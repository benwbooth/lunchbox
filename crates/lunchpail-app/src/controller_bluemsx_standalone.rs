//! blueMSX standalone-native controller serialization.
//!
//! Pinned blueMSX source exposes a native `bluemsx.ini` property tree and a
//! Windows keyboard-profile directory. The profile grammar can be rendered
//! when the caller has explicitly resolved a DirectInput runtime slot. SDL and
//! Linux remain unsupported because their source-backed frontends do not
//! expose a native joystick backend/profile writer.

use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "aef17c6c7e4a6cb93c58f6425569c4586a887be";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Port {
    /// Emulated joystick port, one or two.
    pub player: u8,
    /// DirectInput enumeration slot, one through eight. This is deliberately
    /// explicit: blueMSX does not persist a stable physical identity.
    pub device_slot: u8,
    pub button1: u8,
    pub button2: Option<u8>,
}

/// Render the native Windows keyboard-profile sections used by
/// `Win32keyboard.c`. The output is intended for `<root>/Keyboard Config`.
pub(crate) fn profile_ini(ports: &[Port]) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 2,
        "blueMSX has two joystick ports"
    );
    let mut out = String::new();
    let mut players = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for port in ports {
        ensure!(
            (1..=2).contains(&port.player) && players.insert(port.player),
            "blueMSX joystick port is invalid"
        );
        ensure!(
            (1..=8).contains(&port.device_slot) && slots.insert(port.device_slot),
            "blueMSX DirectInput slot is invalid"
        );
        ensure!(
            (1..=28).contains(&port.button1),
            "blueMSX button1 is invalid"
        );
        if let Some(button) = port.button2 {
            ensure!((1..=28).contains(&button), "blueMSX button2 is invalid");
        }
        let table = port.player;
        let event_prefix = format!("joy{}-", port.player);
        let value_prefix = format!("J{} ", port.device_slot);
        out.push_str(&format!("[Keymapping-{table}]\n"));
        for (event, value) in [
            ("up", format!("{}UP", value_prefix)),
            ("down", format!("{}DOWN", value_prefix)),
            ("left", format!("{}LEFT", value_prefix)),
            ("right", format!("{}RIGHT", value_prefix)),
        ] {
            // Win32keyboard.c writes the event name with one trailing space;
            // IniFileParser matches that exact key prefix on load.
            out.push_str(&format!("{}{event} ={value}\n", event_prefix));
        }
        out.push_str(&format!(
            "{}button1 ={}BT {}\n",
            event_prefix, value_prefix, port.button1
        ));
        if let Some(button) = port.button2 {
            out.push_str(&format!(
                "{}button2 ={}BT {}\n",
                event_prefix, value_prefix, button
            ));
        }
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_native_directinput_profile_sections() {
        let ini = super::profile_ini(&[super::Port {
            player: 1,
            device_slot: 2,
            button1: 1,
            button2: Some(2),
        }])
        .unwrap();
        assert!(ini.contains("[Keymapping-1]"));
        assert!(ini.contains("joy1-up =J2 UP"));
        assert!(ini.contains("joy1-button2 =J2 BT 2"));
    }
}
