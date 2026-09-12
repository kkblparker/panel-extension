# panel-extension

Source for the Calagopus panel extensions maintained in this account, kept separate from the
[`panel`](https://github.com/kkblparker/panel) fork itself so extension work doesn't collide with
upstream syncs there.

Each top-level folder is one extension, laid out the way the panel's own `extensions export`/`add`
commands expect: Rust source (`Cargo.toml`, `Metadata.toml`, `src/`) at the extension's root, and
its frontend counterpart under `frontend/`.

- `dev_kkblparker_pfsensesync` — syncs server allocations to pfSense NAT port-forward rules.
- `dev_kkblparker_reforgerworkshop` — in-panel Workshop browser for Arma Reforger, via reforgermods.net.
- `dev_kkblparker_minecraftworkshop` — in-panel Workshop browser for Minecraft plugins/mods, via Modrinth.
- `dev_kkblparker_steamworkshop` — Steam Workshop mod list manager for Steam-based servers (Arma 3,
  Project Zomboid), via the Steam Web API.

## Using these in a panel checkout

The panel has no dynamic extension loading — an extension's source has to physically live under
`backend-extensions/<id>/` (and be symlinked into `frontend/extensions/<id>/`) at compile time, and
be registered as a path dependency in `backend-extensions/internal-list/`. This repo doesn't change
that; it's meant to be pulled into a panel checkout (e.g. as a git submodule) with each extension
folder here symlinked into place, so the extension source itself never has to live inline in the
panel repo.
