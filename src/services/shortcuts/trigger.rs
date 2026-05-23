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

pub fn portal_trigger(trigger: &str) -> String {
    let mut portal = String::new();
    for part in trigger.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part {
            "Ctrl" | "Control" => portal.push_str("<Control>"),
            "Shift" => portal.push_str("<Shift>"),
            "Alt" => portal.push_str("<Alt>"),
            "Super" | "Meta" => portal.push_str("<Super>"),
            key if key.starts_with("KP_") => {
                portal.push('<');
                portal.push_str(key);
                portal.push('>');
            }
            other => {
                portal.push('<');
                portal.push_str(other);
                portal.push('>');
            }
        }
    }
    portal
}

pub fn qt_key_sequence(trigger: &str) -> Result<Vec<i32>> {
    let mut keys = Vec::new();
    for part in trigger.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        keys.push(map_qt_key(part)?);
    }
    if keys.is_empty() {
        return Err(anyhow!("empty shortcut trigger"));
    }
    Ok(keys)
}

fn map_qt_key(part: &str) -> Result<i32> {
    match part {
        "Escape" => Ok(QT_KEY_ESCAPE),
        "KP_Add" => Ok(QT_KEY_KP_ADD),
        "KP_Subtract" => Ok(QT_KEY_KP_SUBTRACT),
        "KP_Decimal" => Ok(QT_KEY_KP_DECIMAL),
        "Ctrl" | "Control" => Ok(0x0100_0021), // Qt::Key_Control
        key if key.starts_with("KP_") && key.len() == 4 => {
            let digit = key
                .chars()
                .nth(3)
                .ok_or_else(|| anyhow!("invalid keypad key: {key}"))?;
            if !digit.is_ascii_digit() {
                return Err(anyhow!("invalid keypad key: {key}"));
            }
            Ok(QT_KEY_KEYPAD0 + (digit as i32 - b'0' as i32))
        }
        other => Err(anyhow!("unsupported shortcut key: {other}")),
    }
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
        assert_eq!(keys, vec![QT_KEY_KEYPAD0 + 1]);
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
    fn portal_keypad_trigger() {
        assert_eq!(portal_trigger("KP_1"), "<KP_1>");
        assert_eq!(portal_trigger("Ctrl+KP_Add"), "<Control><KP_Add>");
    }
}
