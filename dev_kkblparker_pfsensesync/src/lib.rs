//! pfSense Firewall Sync — a Calagopus backend extension.
//!
//! Mirrors every in-use server allocation (`IP:port`) into a NAT port-forward
//! rule on a pfSense firewall via the pfSense REST API package, on a periodic
//! reconcile loop. The panel is the source of truth; rules the extension owns
//! are tagged so hand-made pfSense rules are never disturbed.
//!
//! The periodic loop alone means a brand-new server's rule could take up to
//! the configured interval to appear, so we also listen for
//! `ServerEvent::InstallCompleted` and fire an immediate best-effort
//! reconcile pass the moment a new server finishes installing — the point at
//! which it can actually start running anyway. We deliberately do *not* hook
//! into `Server`'s creation transaction (`CreatableModel`) for this: those
//! handlers run inside the server's INSERT transaction, and a slow/unreachable
//! pfSense box would then roll back the entire server creation. The event bus
//! (`EventEmitter::emit`) is fire-and-forget and runs listeners outside any
//! transaction, so a failure here only means we fall back to the periodic
//! loop — it can never block or fail server creation.
//!
//! Configuration comes from `PFSENSE_*` environment variables and/or the
//! admin settings page at `/admin/extensions/dev.kkblparker.pfsensesync`, merged
//! by [`config::Config::load`] on every use — see [`config`] and [`settings`].
//! The module layout (`config` / `pfsense` / `settings` / `routes` / `sync`)
//! is deliberately generic so a future HAProxy module — for handing out
//! per-server subdomains — can sit alongside this one without restructuring.

mod config;
mod pfsense;
mod routes;
mod settings;
mod sync;

use std::sync::Arc;

use shared::{
    State,
    extensions::{
        Extension, ExtensionRouteBuilder, background_tasks::BackgroundTaskBuilder,
        settings::ExtensionSettingsDeserializer,
    },
    models::server::{Server, ServerEvent},
    prelude::EventEmittingModel,
};

use crate::config::Config;
use crate::settings::PfSenseSettingsDeserializer;

/// Name of the reconcile background task (must be `'static`).
const RECONCILE_TASK: &str = "dev.kkblparker.pfsensesync:reconcile";

/// Fallback sleep when a reconcile tick can't even load its own settings
/// (e.g. a transient database hiccup) — short enough to recover quickly,
/// long enough not to hammer the database while it's unhappy.
const SETTINGS_LOAD_RETRY_SECS: u64 = 10;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(PfSenseSettingsDeserializer)
    }

    async fn initialize(&mut self, state: State) {
        match Config::load(&state).await {
            Ok(config) if !config.enabled => tracing::info!(
                "pfSense Firewall Sync loaded but disabled (enable it from /admin/extensions/dev.kkblparker.pfsensesync or PFSENSE_SYNC_ENABLED=true)"
            ),
            Ok(config) => match config.inoperational_reason() {
                Some(reason) => tracing::warn!(
                    "pfSense Firewall Sync is enabled but not operational: {reason}. The reconcile loop will idle until this is fixed."
                ),
                None => tracing::info!(
                    target_url = %config.base_url,
                    interface = %config.interface,
                    interval_secs = config.interval.as_secs(),
                    "pfSense Firewall Sync enabled"
                ),
            },
            Err(err) => {
                tracing::error!("pfSense Firewall Sync: failed to load settings on startup: {err:?}")
            }
        }

        // Best-effort only: `emit` is fire-and-forget and this listener's
        // result is just logged, never propagated back to whatever completed
        // the install. A failed/slow pfSense call here can never affect
        // server installation — it just means this particular server waits
        // for the next periodic reconcile pass.
        Server::register_event_handler(|state, event| async move {
            if let ServerEvent::InstallCompleted {
                successful: true, ..
            } = event.as_ref()
            {
                match Config::load(&state).await {
                    Ok(config) if config.is_operational() => {
                        if let Err(err) = sync::reconcile(&state, &config).await {
                            tracing::error!(
                                "pfSense sync: reconcile triggered by server install completion failed: {err:?}"
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(err) => tracing::error!(
                        "pfSense sync: failed to load settings for install-completed reconcile: {err:?}"
                    ),
                }
            }

            Ok(())
        });
    }

    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder.add_admin_api_router(|router| router.merge(routes::router(&state)))
    }

    async fn initialize_background_tasks(
        &mut self,
        _state: State,
        builder: BackgroundTaskBuilder,
    ) -> BackgroundTaskBuilder {
        // Always registered, regardless of whether the extension is currently
        // enabled: settings can be flipped on/off live from the admin page,
        // and this loop re-loads them fresh every tick, so it just needs to
        // keep ticking (harmlessly idle) for that to take effect without a
        // panel restart.
        builder
            .add_task(RECONCILE_TASK, move |state| async move {
                // The task runner calls this in a tight loop with no delay of
                // its own, so we must always sleep before returning — and
                // never early-return via `?` before that sleep, or a failure
                // would spin the loop.
                match Config::load(&state).await {
                    Ok(config) => {
                        if config.is_operational() {
                            if let Err(err) = sync::reconcile(&state, &config).await {
                                tracing::error!("pfSense sync pass failed: {err:?}");
                            }
                        }

                        tokio::time::sleep(config.interval).await;
                    }
                    Err(err) => {
                        tracing::error!(
                            "pfSense sync: failed to load settings, retrying shortly: {err:?}"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(
                            SETTINGS_LOAD_RETRY_SECS,
                        ))
                        .await;
                    }
                }

                Ok(())
            })
            .await;

        builder
    }
}
