# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Added
- Adopted `CODING_STANDARDS.md` with CI enforcement (fmt, clippy, tests, QML binding smoke test).
- Centralized cxx-qt property sync in `src/qobjects/controller.rs` (`properties` module) and `docs/cxx-qt-qml.md`.
- Global shortcut status in Settings and a one-time launch prompt to Apply shortcuts.
- `scripts/test-chrome-bindings.sh` for QML binding regression checks.

### Fixed
- Global shortcut prompt uses `QtQuick.Dialogs` `MessageDialog` with `QuickDialogs2` linked at build time.

### Changed
- `AGENTS.md` removed from git tracking; kept locally only.
- README global shortcut instructions updated for portal Apply-only registration.
