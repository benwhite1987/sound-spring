use anyhow::{anyhow, Result};

const QT_KEY_META: i32 = 0x0100_0022;
const QT_KEY_ESCAPE: i32 = 0x0100_0000;

pub fn portal_trigger(trigger: &str) -> String {
    trigger.replace("Meta", "SUPER")
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
        "Meta" | "Super" | "Logo" => Ok(QT_KEY_META),
        "Escape" => Ok(QT_KEY_ESCAPE),
        "Bracket Left" => Ok(0x5b),
        "Bracket Right" => Ok(0x5d),
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
            Ok(part.as_bytes()[0] as i32)
        }
        other => Err(anyhow!("unsupported shortcut key: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_one_sequence() {
        let keys = qt_key_sequence("Meta+1").unwrap();
        assert_eq!(keys, vec![QT_KEY_META, b'1' as i32]);
    }
}
