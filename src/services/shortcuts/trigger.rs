use anyhow::{anyhow, Result};

const QT_KEY_KEYPAD0: i32 = 0x0100_0030;
const QT_KEY_KEYPAD9: i32 = 0x0100_0039;
const QT_KEY_ESCAPE: i32 = 0x0100_0000;
const QT_KEY_KP_ADD: i32 = 0x0100_002b;
const QT_KEY_KP_SUBTRACT: i32 = 0x0100_002d;
const QT_KEY_KP_DECIMAL: i32 = 0x0100_003e;

const QT_SHIFT_MODIFIER: i32 = 0x0200_0000;
const QT_CONTROL_MODIFIER: i32 = 0x0400_0000;
const QT_ALT_MODIFIER: i32 = 0x0800_0000;
const QT_KEYPAD_MODIFIER: i32 = 0x4000_0000;

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
            "Ctrl" | "Control" => "Ctrl".to_string(),
            "Alt" => "Alt".to_string(),
            "Shift" => "Shift".to_string(),
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
            "Ctrl" | "Control" => "Ctrl",
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

    let key_part = map_event_key(key, modifiers, native_scan_code)?;
    parts.push(key_part);
    Some(parts.join("+"))
}

/// Map a numpad key press to a play slot (1-10) when no chord modifiers are held.
pub fn play_slot_from_qt_key(key: i32, modifiers: i32, native_scan_code: u32) -> Option<i32> {
    if modifiers & (QT_CONTROL_MODIFIER | QT_SHIFT_MODIFIER | QT_ALT_MODIFIER) != 0 {
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
    if key >= QT_KEY_KEYPAD0 && key <= QT_KEY_KEYPAD9 {
        let digit = key - QT_KEY_KEYPAD0;
        return Some(format!("KP_{digit}"));
    }

    match key {
        QT_KEY_KP_ADD => return Some("KP_Add".into()),
        QT_KEY_KP_SUBTRACT => return Some("KP_Subtract".into()),
        _ => {}
    }

    if let Some(trigger) = map_native_scan_code(native_scan_code) {
        if is_keypad_digit(&trigger) {
            return Some(trigger);
        }
        if key != QT_KEY_KP_DECIMAL && key != 0x2e && key != 0x2c {
            return Some(trigger);
        }
    }

    if key == QT_KEY_KP_DECIMAL {
        return Some("KP_Decimal".into());
    }

    let keypad = modifiers & QT_KEYPAD_MODIFIER != 0;
    if keypad {
        if (0x30..=0x39).contains(&key) {
            let digit = key as u8 - b'0';
            return Some(format!("KP_{digit}"));
        }
        match key {
            0x2b => return Some("KP_Add".into()),
            0x2d => return Some("KP_Subtract".into()),
            0x2e | 0x2c => return Some("KP_Decimal".into()),
            _ => {}
        }
    }

    map_native_scan_code(native_scan_code)
}

fn is_keypad_digit(trigger: &str) -> bool {
    trigger.len() == 4
        && trigger.starts_with("KP_")
        && trigger.as_bytes().get(3).is_some_and(|b| b.is_ascii_digit())
}

/// Qt on Linux reports X11 keycodes; convert to evdev before lookup.
fn map_native_scan_code(native_scan_code: u32) -> Option<String> {
    if native_scan_code == 0 {
        return None;
    }

    let evdev = native_scan_code.saturating_sub(8);
    if evdev != native_scan_code {
        if let Some(trigger) = map_evdev_scancode(evdev) {
            return Some(trigger);
        }
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
        _ => None,
    }
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
                other => other.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn qt_key_sequence(trigger: &str) -> Result<Vec<i32>> {
    let parts: Vec<&str> = trigger
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(anyhow!("empty shortcut trigger"));
    }

    let mut modifiers = 0i32;
    let mut key: Option<i32> = None;
    for part in parts {
        match part {
            "Ctrl" | "Control" => modifiers |= QT_CONTROL_MODIFIER,
            "Shift" => modifiers |= QT_SHIFT_MODIFIER,
            "Alt" => modifiers |= QT_ALT_MODIFIER,
            "KP_0" => {
                modifiers |= QT_KEYPAD_MODIFIER;
                key = Some(0x30);
            }
            "KP_1" | "KP_2" | "KP_3" | "KP_4" | "KP_5" | "KP_6" | "KP_7" | "KP_8" | "KP_9" => {
                modifiers |= QT_KEYPAD_MODIFIER;
                key = Some(part.as_bytes()[3] as i32);
            }
            "KP_Add" => {
                modifiers |= QT_KEYPAD_MODIFIER;
                key = Some(0x2b);
            }
            "KP_Subtract" => {
                modifiers |= QT_KEYPAD_MODIFIER;
                key = Some(0x2d);
            }
            "KP_Decimal" => {
                modifiers |= QT_KEYPAD_MODIFIER;
                key = Some(0x2e);
            }
            "Escape" => key = Some(QT_KEY_ESCAPE),
            other => return Err(anyhow!("unsupported shortcut key: {other}")),
        }
    }

    let key = key.ok_or_else(|| anyhow!("shortcut trigger missing key: {trigger}"))?;
    Ok(vec![modifiers | key])
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
            trigger_from_qt(QT_KEY_KEYPAD0 + 4, 0, 83),
            Some("KP_4".into())
        );
        assert_eq!(trigger_from_qt(QT_KEY_KP_DECIMAL, 0, 91), Some("KP_Decimal".into()));
    }

    #[test]
    fn play_slot_from_qt_keypad() {
        assert_eq!(play_slot_from_qt_key(QT_KEY_KEYPAD0 + 1, 0, 0), Some(1));
        assert_eq!(play_slot_from_qt_key(QT_KEY_KEYPAD0, 0, 0), Some(10));
        assert_eq!(play_slot_from_qt_key(0, 0, 87), Some(1));
        assert_eq!(play_slot_from_qt_key(0, 0, 79), Some(7));
        assert_eq!(
            play_slot_from_qt_key(QT_KEY_KP_ADD, QT_CONTROL_MODIFIER, 0),
            None
        );
    }

    #[test]
    fn keypad_one_sequence() {
        let keys = qt_key_sequence("KP_1").unwrap();
        assert_eq!(keys, vec![QT_KEYPAD_MODIFIER | 0x31]);
    }

    #[test]
    fn qt_key_sequence_matches_kde_numpad_format() {
        assert_eq!(
            qt_key_sequence("Ctrl+KP_Add").unwrap(),
            vec![QT_CONTROL_MODIFIER | QT_KEYPAD_MODIFIER | 0x2b]
        );
        assert_eq!(
            qt_key_sequence("KP_Decimal").unwrap(),
            vec![QT_KEYPAD_MODIFIER | 0x2e]
        );
    }

    #[test]
    fn trigger_from_qt_keypad_digit() {
        assert_eq!(trigger_from_qt(QT_KEY_KEYPAD0 + 1, 0, 0), Some("KP_1".into()));
        assert_eq!(
            trigger_from_qt(0x31, QT_KEYPAD_MODIFIER, 0),
            Some("KP_1".into())
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
    fn portal_keypad_trigger() {
        assert_eq!(portal_trigger("KP_1"), "NUM+1");
        assert_eq!(portal_trigger("KP_0"), "NUM+0");
        assert_eq!(portal_trigger("Ctrl+KP_Add"), "CTRL+NUM+plus");
        assert_eq!(portal_trigger("Ctrl+KP_Subtract"), "CTRL+NUM+minus");
        assert_eq!(portal_trigger("KP_Decimal"), "NUM+period");
        assert_eq!(portal_trigger("Meta+1"), "LOGO+1");
        assert!(
            !portal_trigger("KP_1").contains("KP_"),
            "numpad digits use NUM modifier for KDE KGlobalAccel"
        );
    }
}
