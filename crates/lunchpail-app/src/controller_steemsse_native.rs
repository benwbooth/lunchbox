//! Steem SSE native joystick profile serialization.
//!
//! Pinned to the SourceForge SVN-origin tree at
//! `d2c604006c94686ba98faff252d25586089fd099`. Steem stores joystick
//! profiles in its Windows-format `steem.ini` via `ConfigStoreFile`:
//! `[Joystick 1]` through `[Joystick 8]`, with `DirID0` through `DirID5`
//! holding Up/Down/Left/Right/Fire/AutoFire. A `DirID` is not a persistent
//! device identity; its high byte contains the measured runtime joystick
//! index. Callers must probe and recheck that index immediately before launch.

use anyhow::{Context, Result, ensure};

pub(crate) const SOURCE_COMMIT: &str = "d2c604006c94686ba98faff252d25586089fd099";
pub(crate) const PROFILE_ID: &str = "steemsse:standalone-native-joystick-v1";
pub(crate) const MAX_JOYSTICKS: u8 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Port {
    /// Steem profile section, one through eight.
    pub player: u8,
    /// Measured native joystick enumeration slot, zero through seven.
    pub device_index: u8,
    pub x_axis: u8,
    pub y_axis: u8,
    pub fire_button: u8,
}

fn dir_id(low: u8, device_index: u8, negative: bool) -> i32 {
    let high = 10 + device_index * 10 + u8::from(negative);
    i32::from(u16::from_le_bytes([low, high]))
}

fn button_id(button: u8, device_index: u8) -> i32 {
    dir_id(100 + button, device_index, false)
}

fn key_lines(prefix: &str, port: Port) -> String {
    let mut out = String::new();
    for (key, value) in [
        ("Type", 0),
        ("ToggleKey", 0),
        ("AnyFireOnJoy", 0),
        ("DeadZone", 50),
        ("AutoFireSpeed", 0),
    ] {
        out.push_str(&format!("{prefix}{key}={value}\r\n"));
    }
    let values = [
        ("DirID0", dir_id(port.y_axis + 1, port.device_index, true)),
        ("DirID1", dir_id(port.y_axis + 1, port.device_index, false)),
        ("DirID2", dir_id(port.x_axis + 1, port.device_index, true)),
        ("DirID3", dir_id(port.x_axis + 1, port.device_index, false)),
        ("DirID4", button_id(port.fire_button, port.device_index)),
        ("DirID5", 0xffff),
    ];
    for (key, value) in values {
        out.push_str(&format!("{prefix}{key}={value}\r\n"));
    }
    out
}

/// Patch the active setup's joystick sections in a copied Steem INI.
/// Existing text and unrelated settings are retained; duplicate keys match
/// `ConfigStoreFile`'s last-key-wins behavior.
pub(crate) fn patch_config(baseline: &[u8], setup: u8, ports: &[Port]) -> Result<String> {
    ensure!(baseline.len() <= 4 * 1024 * 1024, "Steem INI is too large");
    ensure!(setup <= 2, "Steem joystick setup must be 0, 1, or 2");
    ensure!(
        !ports.is_empty() && ports.len() <= 8,
        "Steem supports up to eight joystick profiles"
    );
    let text = std::str::from_utf8(baseline).context("Steem INI is not UTF-8")?;
    ensure!(!text.contains('\0'), "Steem INI contains a NUL byte");
    let mut seen = [false; 9];
    for port in ports {
        ensure!(
            (1..=8).contains(&port.player),
            "Steem joystick profile is out of range"
        );
        ensure!(
            !seen[usize::from(port.player)],
            "Steem joystick profile is duplicated"
        );
        seen[usize::from(port.player)] = true;
        ensure!(
            port.device_index < MAX_JOYSTICKS,
            "Steem native joystick index is out of range"
        );
        ensure!(
            port.x_axis < 6 && port.y_axis < 6 && port.x_axis != port.y_axis,
            "Steem axis is out of range or duplicated"
        );
        ensure!(
            port.fire_button < 100,
            "Steem button index cannot be encoded in DirID"
        );
    }

    let prefix = if setup == 0 {
        String::new()
    } else {
        format!("{setup}_")
    };
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut output = text.to_owned();
    if !output.is_empty() && !output.ends_with(['\n', '\r']) {
        output.push_str(newline);
    }
    output.push_str(&format!("[Joysticks]{newline}Setup={setup}{newline}"));
    for port in ports {
        output.push_str(&format!("{newline}[Joystick {}]{newline}", port.player));
        output.push_str(&key_lines(&prefix, *port).replace("\r\n", newline));
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "Steem DirID encodes the measured native joystick slot, not a stable physical identity; probe and recheck the slot immediately before launch. Preserve the baseline INI, RunDir, TOS/EmuTOS selection, disk images and guest-save roots."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_source_dirids_and_preserves_baseline() {
        let baseline = b"[Machine and TOS]\r\nTOS=tos.img\r\n";
        let text = patch_config(
            baseline,
            0,
            &[Port {
                player: 1,
                device_index: 2,
                x_axis: 0,
                y_axis: 1,
                fire_button: 0,
            }],
        )
        .unwrap();
        assert!(text.starts_with("[Machine and TOS]"));
        assert!(text.contains("[Joystick 1]\r\nType=0\r\nToggleKey=0\r\nAnyFireOnJoy=0\r\nDeadZone=50\r\nAutoFireSpeed=0\r\nDirID0=7938\r\nDirID1=7682\r\nDirID2=7937\r\nDirID3=7681\r\nDirID4=7780\r\nDirID5=65535"));
        assert!(text.contains("[Joysticks]\r\nSetup=0"));
    }

    #[test]
    fn rejects_invalid_runtime_slots() {
        let bad = Port {
            player: 1,
            device_index: 8,
            x_axis: 0,
            y_axis: 1,
            fire_button: 0,
        };
        assert!(patch_config(b"", 0, &[bad]).is_err());
    }
}
