# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Added
- Embedded ECAPA-TDNN speaker model in the release binary (`build.rs` fetch + `include_bytes!`).
- `Makefile` install target and Flatpak manifest skeleton (`packaging/flatpak/`).
- `THIRD_PARTY_NOTICES.md` for bundled ML models.
- Default tab directory seeding (`01-memes`, `02-music`, `03-effects`) on first layout setup.
- `ci/test-chrome-bindings.sh` for QML binding regression checks.

### Removed
- Bash install layer (`install.sh`, `uninstall.sh`, `sb-play` / `sb-tab` / `sb-stop`, PipeWire setup scripts, systemd unit, sxhkd fragment).

### Changed
- Release binary size is ~55–60 MB (includes embedded ECAPA weights).
- Portal shortcut bind/activate and tab rescan logs demoted to `debug!` level.
- README and PROJECT.md rewritten for GUI-first install (`make install`).

### Fixed
- Global shortcut prompt uses `QtQuick.Dialogs` `MessageDialog` with `QuickDialogs2` linked at build time.
