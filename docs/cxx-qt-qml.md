# cxx-qt and QML property bindings

Sound Spring exposes Rust state to QML through cxx-qt `#[qproperty]` fields on
`SoundboardController`. Those fields share storage with the Rust struct.

## Rule

**Do not assign to `#[qproperty]` fields directly when QML must update.**

Direct writes (for example `rust_mut().monitor_muted = true`) change the value
but do not emit Qt NOTIFY signals. Subsequent calls to `set_monitor_muted(true)`
see no change and also skip NOTIFY, so bindings on `controller.monitorMuted`
stay stale.

## Pattern

1. Mutate Rust-only state as needed.
2. Call the sync helpers in `src/qobjects/controller.rs` (`properties` module):
   - `sync_volume_properties` — output/monitor volume and mute
   - `sync_tab_properties` — tab index, count, names
   - `sync_mic_properties` — mic source list
3. Each sync helper finishes with `bump_ui_version`, which increments `uiVersion`
   through its cxx-qt setter so QML bindings that list `controller.uiVersion`
   re-evaluate and read fresh values.

## QML

Chrome bindings (mute buttons, tab highlights, disabled sliders) must depend on
`controller.uiVersion` in binding expressions, not only on the underlying
property.

```qml
checked: {
    controller.uiVersion
    return controller.currentTabIndex === index
}
```

## Tests

- `ci/test-chrome-bindings.qml` — binding refresh pattern
- `ci/test-chrome-bindings.sh` — runs QML test and offscreen launch smoke check
