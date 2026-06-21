const QT_KEY_KEYPAD0: i32 = 0x0100_0030;
const QT_KEY_KEYPAD9: i32 = 0x0100_0039;
const QT_KEY_ESCAPE: i32 = 0x0100_0000;
const QT_KEY_KP_ADD: i32 = 0x0100_002b;
const QT_KEY_KP_SUBTRACT: i32 = 0x0100_002d;
const QT_KEY_KP_DECIMAL: i32 = 0x0100_003e;
const QT_KEY_KP_ENTER: i32 = 0x0100_0005;

const QT_SHIFT_MODIFIER: i32 = 0x0200_0000;
const QT_CONTROL_MODIFIER: i32 = 0x0400_0000;
const QT_ALT_MODIFIER: i32 = 0x0800_0000;
const QT_META_MODIFIER: i32 = 0x1000_0000;
const QT_KEYPAD_MODIFIER: i32 = 0x4000_0000;

const QT_KEY_TAB: i32 = 0x0100_0001;
const QT_KEY_BACKSPACE: i32 = 0x0100_0003;
const QT_KEY_RETURN: i32 = 0x0100_0004;
const QT_KEY_INSERT: i32 = 0x0100_0006;
const QT_KEY_DELETE: i32 = 0x0100_0007;
const QT_KEY_PAUSE: i32 = 0x0100_0008;
const QT_KEY_PRINT: i32 = 0x0100_0009;
const QT_KEY_HOME: i32 = 0x0100_0010;
const QT_KEY_END: i32 = 0x0100_0011;
const QT_KEY_LEFT: i32 = 0x0100_0012;
const QT_KEY_UP: i32 = 0x0100_0013;
const QT_KEY_RIGHT: i32 = 0x0100_0014;
const QT_KEY_DOWN: i32 = 0x0100_0015;
const QT_KEY_PAGE_UP: i32 = 0x0100_0016;
const QT_KEY_PAGE_DOWN: i32 = 0x0100_0017;
const QT_KEY_CAPS_LOCK: i32 = 0x0100_0024;
const QT_KEY_SHIFT: i32 = 0x0100_0020;
const QT_KEY_CONTROL: i32 = 0x0100_0021;
const QT_KEY_META: i32 = 0x0100_0022;
const QT_KEY_ALT: i32 = 0x0100_0023;

pub fn trigger_display(trigger: &str) -> String {
    trigger
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| match part {
            "KP_0" => "Num 0".to_string(),
            key if key.starts_with("KP_") && key.len() == 4 => {
                format!("Num {}", key.chars().nth(3).unwrap_or('?'))
            }
            "KP_Add" => "Num +".to_string(),
            "KP_Subtract" => "Num -".to_string(),
            "KP_Decimal" => "Num .".to_string(),
            "KP_Multiply" => "Num *".to_string(),
            "KP_Divide" => "Num /".to_string(),
            "KP_Enter" => "Num Enter".to_string(),
            "KP_End" => "Num End".to_string(),
            "KP_Down" => "Num ↓".to_string(),
            "KP_PageDown" | "KP_Next" => "Num PgDown".to_string(),
            "KP_Left" => "Num ←".to_string(),
            "KP_Begin" | "KP_Clear" => "Num Clear".to_string(),
            "KP_Right" => "Num →".to_string(),
            "KP_Home" => "Num Home".to_string(),
            "KP_Up" => "Num ↑".to_string(),
            "KP_PageUp" | "KP_Prior" => "Num PgUp".to_string(),
            "KP_Insert" => "Num Insert".to_string(),
            "KP_Delete" => "Num Delete".to_string(),
            "Ctrl" | "Control" => "Ctrl".to_string(),
            "Alt" => "Alt".to_string(),
            "Shift" => "Shift".to_string(),
            "Meta" | "Super" => "Meta".to_string(),
            "Return" | "Enter" => "Return".to_string(),
            "BackSpace" | "Backspace" => "Backspace".to_string(),
            "bracketleft" => "[".to_string(),
            "bracketright" => "]".to_string(),
            "semicolon" => ";".to_string(),
            "apostrophe" => "'".to_string(),
            "comma" => ",".to_string(),
            "period" => ".".to_string(),
            "slash" => "/".to_string(),
            "backslash" => "\\".to_string(),
            "minus" => "-".to_string(),
            "equal" => "=".to_string(),
            "grave" => "`".to_string(),
            "space" => "Space".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn qt_shortcut_sequence(trigger: &str) -> String {
    trigger
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| match part {
            "KP_0" => "Num+0",
            "KP_1" => "Num+1",
            "KP_2" => "Num+2",
            "KP_3" => "Num+3",
            "KP_4" => "Num+4",
            "KP_5" => "Num+5",
            "KP_6" => "Num+6",
            "KP_7" => "Num+7",
            "KP_8" => "Num+8",
            "KP_9" => "Num+9",
            "KP_Add" => "Num++",
            "KP_Subtract" => "Num+-",
            "KP_Decimal" => "Num+.",
            "KP_Multiply" => "Num+*",
            "KP_Divide" => "Num+/",
            "KP_Enter" => "Num+Enter",
            "KP_End" => "Num+End",
            "KP_Down" => "Num+Down",
            "KP_PageDown" | "KP_Next" => "Num+PgDown",
            "KP_Left" => "Num+Left",
            "KP_Begin" | "KP_Clear" => "Num+Clear",
            "KP_Right" => "Num+Right",
            "KP_Home" => "Num+Home",
            "KP_Up" => "Num+Up",
            "KP_PageUp" | "KP_Prior" => "Num+PgUp",
            "KP_Insert" => "Num+Insert",
            "KP_Delete" => "Num+Delete",
            "Ctrl" | "Control" => "Ctrl",
            "Meta" | "Super" => "Meta",
            other => other,
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Build an internal trigger string from a Qt key event.
pub fn trigger_from_qt(key: i32, modifiers: i32, native_scan_code: u32) -> Option<String> {
    if key == QT_KEY_ESCAPE {
        return None;
    }

    let mut parts = Vec::new();
    if modifiers & QT_CONTROL_MODIFIER != 0 {
        parts.push("Ctrl".to_string());
    }
    if modifiers & QT_SHIFT_MODIFIER != 0 {
        parts.push("Shift".to_string());
    }
    if modifiers & QT_ALT_MODIFIER != 0 {
        parts.push("Alt".to_string());
    }
    if modifiers & QT_META_MODIFIER != 0 {
        parts.push("Meta".to_string());
    }

    let key_part = map_event_key(key, modifiers, native_scan_code)?;
    parts.push(key_part);
    Some(parts.join("+"))
}

/// Map a numpad key press to a play slot (1-10) when no chord modifiers are held.
pub fn play_slot_from_qt_key(key: i32, modifiers: i32, native_scan_code: u32) -> Option<i32> {
    if modifiers & (QT_CONTROL_MODIFIER | QT_SHIFT_MODIFIER | QT_ALT_MODIFIER | QT_META_MODIFIER)
        != 0
    {
        return None;
    }
    let trigger = map_event_key(key, modifiers, native_scan_code)?;
    match trigger.as_str() {
        "KP_0" => Some(10),
        key if key.starts_with("KP_") && key.len() == 4 => key[3..].parse().ok(),
        _ => None,
    }
}

fn map_event_key(key: i32, modifiers: i32, native_scan_code: u32) -> Option<String> {
    if is_modifier_key(key) {
        return None;
    }

    let keypad = modifiers & QT_KEYPAD_MODIFIER != 0;

    if (QT_KEY_KEYPAD0..=QT_KEY_KEYPAD9).contains(&key) {
        let offset = key - QT_KEY_KEYPAD0;
        if keypad {
            return Some(format!("KP_{offset}"));
        }
        return Some(format!("F{}", offset + 1));
    }

    match key {
        QT_KEY_KP_ADD => return Some("KP_Add".into()),
        QT_KEY_KP_SUBTRACT => return Some("KP_Subtract".into()),
        QT_KEY_KP_ENTER => return Some("KP_Enter".into()),
        QT_KEY_KP_DECIMAL => return Some("KP_Decimal".into()),
        _ => {}
    }

    if keypad {
        if (0x30..=0x39).contains(&key) {
            let digit = key as u8 - b'0';
            return Some(format!("KP_{digit}"));
        }
        match key {
            0x2a => return Some("KP_Multiply".into()),
            0x2b => return Some("KP_Add".into()),
            0x2d => return Some("KP_Subtract".into()),
            0x2e | 0x2c => return Some("KP_Decimal".into()),
            0x2f => return Some("KP_Divide".into()),
            _ => {}
        }
    }

    if let Some(trigger) = map_native_scan_code(native_scan_code) {
        return Some(trigger);
    }

    qt_key_to_internal(key)
}

fn is_modifier_key(key: i32) -> bool {
    matches!(
        key,
        QT_KEY_SHIFT
            | QT_KEY_CONTROL
            | QT_KEY_ALT
            | QT_KEY_META
            | QT_KEY_CAPS_LOCK
            | 0x0100_0025 // NumLock
            | 0x0100_0026 // ScrollLock
    )
}

fn qt_key_to_internal(key: i32) -> Option<String> {
    if (0x30..=0x39).contains(&key) {
        return Some(((key as u8) as char).to_string());
    }
    if (0x41..=0x5a).contains(&key) || (0x61..=0x7a).contains(&key) {
        return Some(((key as u8) as char).to_ascii_uppercase().to_string());
    }

    match key {
        QT_KEY_TAB => Some("Tab".into()),
        QT_KEY_BACKSPACE => Some("Backspace".into()),
        QT_KEY_RETURN => Some("Return".into()),
        QT_KEY_INSERT => Some("Insert".into()),
        QT_KEY_DELETE => Some("Delete".into()),
        QT_KEY_PAUSE => Some("Pause".into()),
        QT_KEY_PRINT => Some("Print".into()),
        QT_KEY_HOME => Some("Home".into()),
        QT_KEY_END => Some("End".into()),
        QT_KEY_LEFT => Some("Left".into()),
        QT_KEY_UP => Some("Up".into()),
        QT_KEY_RIGHT => Some("Right".into()),
        QT_KEY_DOWN => Some("Down".into()),
        QT_KEY_PAGE_UP => Some("Page_Up".into()),
        QT_KEY_PAGE_DOWN => Some("Page_Down".into()),
        0x20 => Some("space".into()),
        0x21 => Some("exclam".into()),
        0x22 => Some("quotedbl".into()),
        0x23 => Some("numbersign".into()),
        0x24 => Some("dollar".into()),
        0x25 => Some("percent".into()),
        0x26 => Some("ampersand".into()),
        0x27 => Some("apostrophe".into()),
        0x28 => Some("parenleft".into()),
        0x29 => Some("parenright".into()),
        0x2a => Some("asterisk".into()),
        0x2b => Some("plus".into()),
        0x2c => Some("comma".into()),
        0x2d => Some("minus".into()),
        0x2e => Some("period".into()),
        0x2f => Some("slash".into()),
        0x3a => Some("colon".into()),
        0x3b => Some("semicolon".into()),
        0x3c => Some("less".into()),
        0x3d => Some("equal".into()),
        0x3e => Some("greater".into()),
        0x3f => Some("question".into()),
        0x40 => Some("at".into()),
        0x5b => Some("bracketleft".into()),
        0x5c => Some("backslash".into()),
        0x5d => Some("bracketright".into()),
        0x5e => Some("asciicircum".into()),
        0x5f => Some("underscore".into()),
        0x60 => Some("grave".into()),
        0x7b => Some("braceleft".into()),
        0x7c => Some("bar".into()),
        0x7d => Some("braceright".into()),
        0x7e => Some("asciitilde".into()),
        _ => None,
    }
}

fn internal_to_portal_key(part: &str) -> String {
    if part.len() == 1 {
        let ch = part.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return ch.to_ascii_lowercase().to_string();
        }
        if ch.is_ascii_digit() {
            return part.to_string();
        }
    }
    part.to_string()
}

fn portal_numpad_keysym_to_internal(keysym: &str) -> Option<String> {
    match keysym {
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
            Some(format!("KP_{keysym}"))
        }
        "plus" => Some("KP_Add".into()),
        "minus" => Some("KP_Subtract".into()),
        "period" => Some("KP_Decimal".into()),
        "asterisk" => Some("KP_Multiply".into()),
        "slash" => Some("KP_Divide".into()),
        "Return" => Some("KP_Enter".into()),
        _ => None,
    }
}

fn portal_keysym_to_internal(keysym: &str) -> String {
    if keysym.len() == 1 {
        let ch = keysym.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return ch.to_ascii_uppercase().to_string();
        }
    }
    match keysym {
        "Return" | "Enter" => "Return".into(),
        "BackSpace" | "Backspace" => "Backspace".into(),
        "PageUp" | "Prior" => "Page_Up".into(),
        "PageDown" | "Next" => "Page_Down".into(),
        other => {
            if other.starts_with('F')
                && other[1..].chars().all(|c| c.is_ascii_digit())
                && other.len() > 1
            {
                other.to_string()
            } else {
                other.to_string()
            }
        }
    }
}

/// Parse an XDG / KDE shortcut string into the internal trigger format.
pub fn trigger_from_portal(portal: &str) -> Option<String> {
    let parts: Vec<&str> = portal
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Vec::new();
    let mut num_modifier = false;
    let mut key: Option<String> = None;

    for part in parts {
        match part {
            "CTRL" | "Control" => modifiers.push("Ctrl".to_string()),
            "ALT" => modifiers.push("Alt".to_string()),
            "SHIFT" => modifiers.push("Shift".to_string()),
            "LOGO" | "Super" | "Meta" => modifiers.push("Meta".to_string()),
            "CAPS" => {}
            "NUM" => num_modifier = true,
            keysym => {
                if key.is_some() {
                    return None;
                }
                key = Some(if num_modifier {
                    portal_numpad_keysym_to_internal(keysym)?
                } else {
                    portal_keysym_to_internal(keysym)
                });
            }
        }
    }

    let key = key?;
    modifiers.push(key);
    Some(modifiers.join("+"))
}

/// Qt on Linux reports X11 keycodes; convert to evdev before lookup.
fn map_native_scan_code(native_scan_code: u32) -> Option<String> {
    if native_scan_code == 0 {
        return None;
    }

    let evdev = native_scan_code.saturating_sub(8);
    if evdev != native_scan_code {
        if evdev == 69 {
            // KEY_NUMLOCK — X11 keycode 77 must not fall through to evdev 77 (KP_6).
            return None;
        }
        if let Some(trigger) = map_evdev_scancode(evdev) {
            return Some(trigger);
        }
        return None;
    }

    map_evdev_scancode(native_scan_code)
}

fn map_evdev_scancode(code: u32) -> Option<String> {
    match code {
        82 => Some("KP_0".into()),
        79 => Some("KP_1".into()),
        80 => Some("KP_2".into()),
        81 => Some("KP_3".into()),
        75 => Some("KP_4".into()),
        76 => Some("KP_5".into()),
        77 => Some("KP_6".into()),
        71 => Some("KP_7".into()),
        72 => Some("KP_8".into()),
        73 => Some("KP_9".into()),
        78 => Some("KP_Add".into()),
        74 => Some("KP_Subtract".into()),
        83 | 91 => Some("KP_Decimal".into()),
        55 => Some("KP_Multiply".into()),
        98 => Some("KP_Divide".into()),
        96 => Some("KP_Enter".into()),
        _ => None,
    }
}

fn portal_key_token(part: &str) -> Option<String> {
    match part {
        "KP_0" => Some("NUM+0".into()),
        "KP_1" => Some("NUM+1".into()),
        "KP_2" => Some("NUM+2".into()),
        "KP_3" => Some("NUM+3".into()),
        "KP_4" => Some("NUM+4".into()),
        "KP_5" => Some("NUM+5".into()),
        "KP_6" => Some("NUM+6".into()),
        "KP_7" => Some("NUM+7".into()),
        "KP_8" => Some("NUM+8".into()),
        "KP_9" => Some("NUM+9".into()),
        // KDE KGlobalAccel stores numpad operators as NUM+plus/minus/period (shown as Num++ etc.)
        "KP_Add" => Some("NUM+plus".into()),
        "KP_Subtract" => Some("NUM+minus".into()),
        "KP_Decimal" => Some("NUM+period".into()),
        "KP_Multiply" => Some("NUM+asterisk".into()),
        "KP_Divide" => Some("NUM+slash".into()),
        "KP_Enter" => Some("NUM+Return".into()),
        // NumLock-OFF equivalents: same physical keys, distinct X11 keysyms.
        //
        // portal-kde's XdgShortcut::parse (src/xdgshortcut.cpp upstream) calls
        // libxkbcommon's `xkb_keysym_from_name()` to resolve the key portion
        // of the trigger string. That means we must send **XKB keysym names
        // from /usr/include/X11/keysymdef.h**, NOT Qt's QKeySequence names.
        // The differences that bit us during testing:
        //   - Qt::Key_Insert is "Ins" / Qt's longer form is "Insert" — XKB is
        //     "Insert" (XK_Insert). Use "Insert".
        //   - Qt::Key_Delete is "Del" / "Delete" — XKB is "Delete". Use "Delete".
        //   - Qt::Key_PageUp is "PgUp" / "Page Up" — XKB names are "Prior"
        //     (XK_Prior) and "Page_Up" with underscore. Use "Prior".
        //   - Qt::Key_PageDown is "PgDown" / "Page Down" — XKB names are "Next"
        //     (XK_Next) and "Page_Down". Use "Next".
        //   - Qt::Key_Clear matches XK_Clear directly — "Clear" works.
        // Any rejection logs `unknown key "<name>"` and drops the entire bind
        // to 0/N assigned keys.
        "KP_End" => Some("NUM+End".into()),
        "KP_Down" => Some("NUM+Down".into()),
        "KP_PageDown" | "KP_Next" => Some("NUM+Next".into()),
        "KP_Left" => Some("NUM+Left".into()),
        "KP_Begin" | "KP_Clear" => Some("NUM+Clear".into()),
        "KP_Right" => Some("NUM+Right".into()),
        "KP_Home" => Some("NUM+Home".into()),
        "KP_Up" => Some("NUM+Up".into()),
        "KP_PageUp" | "KP_Prior" => Some("NUM+Prior".into()),
        "KP_Insert" => Some("NUM+Insert".into()),
        "KP_Delete" => Some("NUM+Delete".into()),
        _ => None,
    }
}

/// Maps a numpad shortcut trigger to its NumLock-OFF X11 keysym equivalent.
/// Modifier prefixes (Ctrl, Alt, Shift, Meta) are preserved verbatim.
/// Returns `None` for triggers that don't depend on NumLock state
/// (operators like KP_Add and non-numpad keys).
pub fn numlock_off_alt(trigger: &str) -> Option<String> {
    let parts: Vec<&str> = trigger
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let (key, modifiers) = parts.split_last()?;
    let alt = match *key {
        "KP_1" => "KP_End",
        "KP_2" => "KP_Down",
        "KP_3" => "KP_PageDown",
        "KP_4" => "KP_Left",
        "KP_5" => "KP_Begin",
        "KP_6" => "KP_Right",
        "KP_7" => "KP_Home",
        "KP_8" => "KP_Up",
        "KP_9" => "KP_PageUp",
        "KP_0" => "KP_Insert",
        "KP_Decimal" => "KP_Delete",
        _ => return None,
    };
    let mut out: Vec<&str> = modifiers.to_vec();
    out.push(alt);
    Some(out.join("+"))
}

pub fn portal_trigger(trigger: &str) -> String {
    trigger
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            if let Some(portal) = portal_key_token(part) {
                return portal;
            }
            match part {
                "Ctrl" | "Control" => "CTRL".to_string(),
                "Shift" => "SHIFT".to_string(),
                "Alt" => "ALT".to_string(),
                "Super" | "Meta" => "LOGO".to_string(),
                other => internal_to_portal_key(other),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x11_scancode_offset_maps_keypad_digits() {
        assert_eq!(trigger_from_qt(0, 0, 79), Some("KP_7".into()));
        assert_eq!(trigger_from_qt(0, 0, 80), Some("KP_8".into()));
        assert_eq!(trigger_from_qt(0, 0, 81), Some("KP_9".into()));
        assert_eq!(trigger_from_qt(0, 0, 83), Some("KP_4".into()));
        assert_eq!(trigger_from_qt(0, 0, 87), Some("KP_1".into()));
    }

    #[test]
    fn qt_keysym_overrides_misleading_scancode() {
        assert_eq!(
            trigger_from_qt(QT_KEY_KEYPAD0 + 4, QT_KEYPAD_MODIFIER, 83),
            Some("KP_4".into())
        );
        assert_eq!(
            trigger_from_qt(QT_KEY_KP_DECIMAL, 0, 91),
            Some("KP_Decimal".into())
        );
    }

    #[test]
    fn numlock_key_does_not_map_to_kp_6() {
        const QT_KEY_NUM_LOCK: i32 = 0x0100_0025;
        assert_eq!(trigger_from_qt(QT_KEY_NUM_LOCK, 0, 77), None);
        assert_eq!(play_slot_from_qt_key(QT_KEY_NUM_LOCK, 0, 77), None);
        assert_eq!(trigger_from_qt(0, 0, 77), None);
    }

    #[test]
    fn kp_decimal_wins_over_misleading_scancode() {
        assert_eq!(
            trigger_from_qt(QT_KEY_KP_DECIMAL, 0, 83),
            Some("KP_Decimal".into())
        );
        assert_eq!(
            trigger_from_qt(QT_KEY_KP_DECIMAL, QT_CONTROL_MODIFIER, 83),
            Some("Ctrl+KP_Decimal".into())
        );
    }

    #[test]
    fn play_slot_from_qt_keypad() {
        assert_eq!(
            play_slot_from_qt_key(QT_KEY_KEYPAD0 + 1, QT_KEYPAD_MODIFIER, 0),
            Some(1)
        );
        assert_eq!(
            play_slot_from_qt_key(QT_KEY_KEYPAD0, QT_KEYPAD_MODIFIER, 0),
            Some(10)
        );
        assert_eq!(play_slot_from_qt_key(0, 0, 87), Some(1));
        assert_eq!(play_slot_from_qt_key(0, 0, 79), Some(7));
        assert_eq!(
            play_slot_from_qt_key(QT_KEY_KP_ADD, QT_CONTROL_MODIFIER, 0),
            None
        );
    }

    #[test]
    fn trigger_from_qt_keypad_digit() {
        assert_eq!(
            trigger_from_qt(QT_KEY_KEYPAD0 + 1, QT_KEYPAD_MODIFIER, 0),
            Some("KP_1".into())
        );
        assert_eq!(
            trigger_from_qt(0x31, QT_KEYPAD_MODIFIER, 0),
            Some("KP_1".into())
        );
        assert_eq!(
            trigger_from_qt(QT_KEY_KEYPAD0 + 1, 0, 0),
            Some("F2".into())
        );
        assert_eq!(
            trigger_from_qt(QT_KEY_KP_ADD, QT_CONTROL_MODIFIER, 0),
            Some("Ctrl+KP_Add".into())
        );
    }

    #[test]
    fn qt_shortcut_maps_keypad() {
        assert_eq!(qt_shortcut_sequence("KP_1"), "Num+1");
        assert_eq!(qt_shortcut_sequence("Ctrl+KP_Add"), "Ctrl+Num++");
        assert_eq!(trigger_display("Ctrl+KP_Decimal"), "Ctrl+Num .");
    }

    #[test]
    fn portal_alt_numpad_trigger() {
        assert_eq!(portal_trigger("Alt+KP_Add"), "ALT+NUM+plus");
        assert_eq!(portal_trigger("Alt+KP_Subtract"), "ALT+NUM+minus");
        assert_eq!(qt_shortcut_sequence("Alt+KP_Add"), "Alt+Num++");
        assert_eq!(trigger_display("Alt+KP_Subtract"), "Alt+Num -");
    }

    #[test]
    fn numlock_off_alt_maps_numpad_digits_and_preserves_modifiers() {
        assert_eq!(numlock_off_alt("KP_1").as_deref(), Some("KP_End"));
        assert_eq!(numlock_off_alt("KP_0").as_deref(), Some("KP_Insert"));
        assert_eq!(numlock_off_alt("KP_5").as_deref(), Some("KP_Begin"));
        assert_eq!(numlock_off_alt("KP_Decimal").as_deref(), Some("KP_Delete"));
        assert_eq!(
            numlock_off_alt("Ctrl+KP_3").as_deref(),
            Some("Ctrl+KP_PageDown")
        );
        assert_eq!(
            numlock_off_alt("Ctrl+Alt+KP_7").as_deref(),
            Some("Ctrl+Alt+KP_Home")
        );
        // Operators and non-numpad keys: no NumLock-off variant.
        assert_eq!(numlock_off_alt("KP_Add"), None);
        assert_eq!(numlock_off_alt("Ctrl+KP_Subtract"), None);
        assert_eq!(numlock_off_alt("KP_Divide"), None);
        assert_eq!(numlock_off_alt("F1"), None);
        assert_eq!(numlock_off_alt(""), None);
    }

    #[test]
    fn portal_trigger_emits_xkb_keysym_names_for_numlock_off_alts() {
        // portal-kde's XdgShortcut::parse() resolves the key portion via
        // libxkbcommon's xkb_keysym_from_name(), so the strings must match
        // X11/keysymdef.h XKB names — NOT Qt's QKeySequence names.
        // PageUp/PageDown specifically must be "Prior"/"Next" (Qt's PgUp/PgDown
        // and PageUp/PageDown are both rejected by xkb_keysym_from_name).
        assert_eq!(portal_trigger("KP_End"), "NUM+End");
        assert_eq!(portal_trigger("KP_Insert"), "NUM+Insert");
        assert_eq!(portal_trigger("Ctrl+KP_Delete"), "CTRL+NUM+Delete");
        assert_eq!(portal_trigger("KP_PageDown"), "NUM+Next");
        assert_eq!(portal_trigger("KP_PageUp"), "NUM+Prior");
        assert_eq!(portal_trigger("KP_Begin"), "NUM+Clear");
    }



    #[test]
    fn portal_keypad_trigger() {
        assert_eq!(portal_trigger("KP_1"), "NUM+1");
        assert_eq!(portal_trigger("KP_0"), "NUM+0");
        assert_eq!(portal_trigger("Ctrl+KP_Add"), "CTRL+NUM+plus");
        assert_eq!(portal_trigger("Ctrl+KP_Subtract"), "CTRL+NUM+minus");
        assert_eq!(portal_trigger("KP_Decimal"), "NUM+period");
        assert_eq!(portal_trigger("KP_Multiply"), "NUM+asterisk");
        assert_eq!(portal_trigger("KP_Divide"), "NUM+slash");
        assert_eq!(portal_trigger("KP_Enter"), "NUM+Return");
        assert_eq!(portal_trigger("Meta+1"), "LOGO+1");
        assert_eq!(portal_trigger("Meta+A"), "LOGO+a");
        assert!(
            !portal_trigger("KP_1").contains("KP_"),
            "numpad digits use NUM modifier for KDE KGlobalAccel"
        );
    }

    #[test]
    fn trigger_from_qt_letters_and_meta() {
        assert_eq!(
            trigger_from_qt(0x31, QT_META_MODIFIER, 0),
            Some("Meta+1".into())
        );
        assert_eq!(trigger_from_qt(0x41, 0, 0), Some("A".into()));
        assert_eq!(
            trigger_from_qt(0x5d, QT_CONTROL_MODIFIER, 0),
            Some("Ctrl+bracketright".into())
        );
        assert_eq!(
            trigger_from_qt(QT_KEY_KEYPAD0, QT_SHIFT_MODIFIER, 0),
            Some("Shift+F1".into())
        );
    }

    #[test]
    fn portal_round_trip() {
        let samples = [
            "KP_1",
            "Ctrl+KP_Add",
            "Meta+1",
            "Ctrl+Shift+F1",
            "Alt+bracketleft",
        ];
        for trigger in samples {
            let portal = portal_trigger(trigger);
            assert_eq!(
                trigger_from_portal(&portal).as_deref(),
                Some(trigger),
                "round-trip failed for {trigger} -> {portal}"
            );
        }
    }
}
