# Global shortcuts — architecture and gotchas

This document captures the operational truth about how Sound Spring's global
shortcuts work on KDE Plasma 6 / Wayland, the failure modes that look like
"our bug" but aren't, and the testing protocol that distinguishes between
them. Read this **before** changing anything in:

- `src/services/shortcuts/portal.rs`
- `src/services/shortcuts/mod.rs`
- `src/cpp/app_identity.cpp`
- `src/main.rs` (the `bind_shortcuts` function)

## TL;DR

1. We bind shortcuts via `xdg-desktop-portal`'s `GlobalShortcuts` interface.
   That's the only supported path.
2. We do **not** make direct D-Bus calls into `org.kde.KGlobalAccel` for
   registration. `kglobalacceld` runs inside `kwin_wayland` on Plasma 6;
   malformed calls crash the whole desktop session.
3. The portal call **must originate from a process in its own systemd cgroup
   scope**, not from a process nested inside another desktop app's scope
   (Cursor IDE, VS Code, Chromium, Electron). Otherwise
   `xdg-desktop-portal-kde` resolves the caller's `app_id` from the parent
   app and silently shares its already-bound portal session.

## How the portal identifies the caller

`xdg-desktop-portal` does not trust D-Bus names or environment variables to
determine which application is calling. It walks the caller's `/proc/<pid>`
cgroup membership and resolves the systemd scope (e.g. `app-cursor-….scope`,
`app-chromium-….scope`, `app-sound-spring-….scope`). The scope name maps to
an `app_id` like `org.chromium.Chromium`, `code-oss`, or `sound-spring`.

If you launch the binary as a child of another desktop app's terminal, your
process inherits that app's cgroup scope. The portal sees the parent app's
`app_id` and treats Sound Spring's `CreateSession` / `BindShortcuts` call as
**another session from the parent app**, not a new application.

## The symptom

When `app_id` is wrong, you'll see all of these together:

- `xdg-desktop-portal-kde[N]: CreateSession ... app_id: "org.chromium.Chromium"`
  (or similar) in `journalctl --user`.
- `BindShortcuts` returns in **~10 ms** instead of seconds (no dialog).
- All 15 shortcut entries come back with empty `trigger_description`.
- Nothing is written to `~/.config/kglobalshortcutsrc`.
- "Sound Spring" never appears in **System Settings → Shortcuts**.
- `qdbus6 --literal org.kde.kglobalaccel /kglobalaccel allComponents` shows
  no `sound-spring` component.

These are **all** explained by the cgroup app_id mismatch. They are **not**
symptoms of a code bug.

## Things that look like fixes but make it worse

Do not, under any circumstances, do the following in response to the above
symptom:

1. **Re-introduce direct KGlobalAccel D-Bus calls.** `setForeignShortcutKeys`,
   `doRegister`, `setShortcut` — all of these are routed to `kglobalacceld`,
   which runs **inside `kwin_wayland`** on Plasma 6. A malformed call
   crashes the compositor and kills the whole user session. This has been
   tried; it left Sound Spring as the only running app with the desktop in
   pieces around it.
2. **Send a "real" Wayland parent_window handle.** `QWaylandShellSurface
   ::externWindowHandle()` is Qt's internal surface UUID, not an
   `xdg_foreign`-exported handle. Sending it makes portal-kde silently
   dismiss the BindShortcuts dialog. Empty parent_window is correct on
   Wayland.
3. **Auto-open System Settings → Shortcuts on Apply.** That window only
   exposes an entry once `[sound-spring]` exists in `kglobalshortcutsrc`,
   which only happens after a successful portal bind under the correct
   `app_id`. Opening it from a broken bind teaches the user nothing.
4. **Add a `purge_kglobalaccel_shortcuts` / cleanup loop.** There's nothing
   to purge — the broken case leaves no `sound-spring` entries anywhere.
5. **Tell the user to log out and log back in.** Session restart doesn't
   change the cgroup scope of a binary you launch from inside another app's
   terminal. Tested.

## The correct test protocol

When verifying global shortcuts work, the binary must end up in a
**top-level `app-sound-spring-*.scope`** directly under `app.slice`. Any
process launched from inside a terminal (Konsole, Cursor, VS Code, GNOME
Terminal) lives in `app-<that-terminal>-*.scope`, and child processes
inherit that scope by default. `systemd-run --user --scope` does **not**
escape it — it creates `run-*.scope` *inside* the calling scope, and
portal-kde resolves to the first `app-*.scope` ancestor, which is the
terminal.

The launchers that actually create a top-level scope go through the user
session bus and call `org.freedesktop.systemd1.Manager.StartTransientUnit`,
which places the new unit directly under `app.slice`:

```bash
# Recommended:
gtk-launch sound-spring

# Equivalent via GIO:
gio launch ~/.local/share/applications/sound-spring.desktop

# Or from KRunner (Alt+Space) — typing "Sound Spring" launches via KIO,
# which uses the same StartTransientUnit path.
```

Avoid these from inside an IDE/terminal — they keep the parent's scope:

```bash
./target/release/sound-spring                       # inherits parent scope
systemd-run --user --scope ./target/release/...    # nests inside parent
```

A bare TTY (Ctrl+Alt+F2) is the only case where running the binary
directly produces a clean scope, because there's no app-*.scope ancestor.

Verification:

```bash
# Should show app_id: "sound-spring", not "org.chromium.Chromium":
journalctl --user -t xdg-desktop-portal-kde --since "1 min ago" \
  | rg 'app_id|CreateSession'

# Should list a sound-spring component after a successful bind+assign:
qdbus6 --literal org.kde.kglobalaccel /kglobalaccel allComponents \
  | tr ',' '\n' | rg sound

# Should contain a [sound-spring] section after assignment in System Settings:
rg '^\[sound' ~/.config/kglobalshortcutsrc
```

If any of those return empty after Apply, **check the cgroup first**:

```bash
cat /proc/$(pgrep -n sound-spring)/cgroup
# Look for app-sound-spring-…scope (correct).
# If you see app-cursor-, app-chromium-, app-org.kde.konsole-, etc., or a
# run-*.scope NESTED inside one, the binary inherited the wrong scope.
# Kill it and relaunch with `gtk-launch sound-spring` per the protocol.
```

## Why the in-app fallback opens System Settings only when useful

`src/main.rs::bind_shortcuts` no longer auto-opens System Settings when the
bind returns zero assigned keys. The previous behavior popped the KCM on
every Apply, even when the KCM had no entry to show, which was actively
confusing. The current behavior is:

- Bind succeeded with keys → state is `Bound`, in-window listener starts.
- Bind succeeded with zero keys → state is `Failed` with a message that
  points at the cgroup root cause. No KCM popup.
- Bind failed entirely → state is `Failed` with the portal error.

## What the codebase intentionally does not do

- No `kglobalaccel.rs` module. It was deleted after `setForeignShortcutKeys`
  crashed the desktop. Read-only queries against `org.kde.KGlobalAccel` are
  also gone — they were unused once cleanup was removed.
- No `BackendCommand::ResetPortalShortcuts`. The portal has no public reset;
  the only safe reset is a `systemd --user restart xdg-desktop-portal-kde`
  by the user.
- No Wayland-private-header `dynamic_cast` in `app_identity.cpp`. The
  Wayland branch returns an empty `QString`; only X11 sends a real handle.

If you find yourself wanting to add any of those back, re-read this file
from the top.
