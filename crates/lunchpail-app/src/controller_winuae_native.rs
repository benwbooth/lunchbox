//! WinUAE native Windows input configuration writer.
//!
//! Pinned upstream source: `tonioni/WinUAE`
//! `d9a702b536283ea968c0de7c1991b44181120575`.  The source writes joystick
//! device records as `input.1.joystick.N.{name,friendlyname,axis.*,button.*}`
//! and selects Amiga ports with `joyportN=joyM`, `joyportNmode=gamepad`.
//! Device matching is name-based (with a configurable match mask), so callers
//! must provide names measured from the same WinUAE runtime.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "d9a702b536283ea968c0de7c1991b44181120575";
pub(crate) const PROFILE_ID: &str = "winuae:standalone-native-input-v1";

pub(crate) const CONTROLS: [&str; 7] = ["up", "down", "left", "right", "fire", "fire2", "fire3"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Axis(u8),
    Button(u8),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Device {
    /// WinUAE's zero-based joystick device number used in `joyN` and
    /// `input.1.joystick.N`; this number is only accepted after a live probe.
    pub index: u8,
    /// `get_uniquename()` used by `input.*.joystick.*.name`.
    pub unique_name: String,
    /// `get_friendlyname()` used by `friendlyname` and `joyportfriendlyname`.
    pub friendly_name: String,
    pub bindings: BTreeMap<String, Binding>,
}

fn valid_name(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 256,
        "WinUAE {what} is empty or too long"
    );
    ensure!(
        !value
            .chars()
            .any(|ch| ch == '\n' || ch == '\r' || ch == '\0'),
        "WinUAE {what} contains a control character"
    );
    Ok(())
}

fn event_name(port: u8, control: &str) -> Result<&'static str> {
    ensure!((0..=3).contains(&port), "WinUAE joystick port must be 0..3");
    match (port, control) {
        (0, "left") | (0, "right") => Ok("JOY1_HORIZ"),
        (0, "up") | (0, "down") => Ok("JOY1_VERT"),
        (0, "fire") => Ok("JOY1_FIRE_BUTTON"),
        (0, "fire2") => Ok("JOY1_2ND_BUTTON"),
        (0, "fire3") => Ok("JOY1_3RD_BUTTON"),
        (1, "left") | (1, "right") => Ok("JOY2_HORIZ"),
        (1, "up") | (1, "down") => Ok("JOY2_VERT"),
        (1, "fire") => Ok("JOY2_FIRE_BUTTON"),
        (1, "fire2") => Ok("JOY2_2ND_BUTTON"),
        (1, "fire3") => Ok("JOY2_3RD_BUTTON"),
        // Ports 2 and 3 are parallel adapters. Their events are source-
        // defined too, but this classic gamepad writer intentionally keeps
        // the normal two Amiga joystick ports unambiguous.
        _ => anyhow::bail!("WinUAE classic gamepad mapping supports ports 0 and 1"),
    }
}

fn validate_device(port: u8, device: &Device) -> Result<()> {
    ensure!(
        (0..=1).contains(&port),
        "WinUAE classic gamepad port must be 0 or 1"
    );
    valid_name(&device.unique_name, "unique device name")?;
    valid_name(&device.friendly_name, "friendly device name")?;
    ensure!(
        device.bindings.len() == CONTROLS.len(),
        "WinUAE needs all seven classic joystick controls"
    );
    for control in CONTROLS {
        let binding = device
            .bindings
            .get(control)
            .ok_or_else(|| anyhow::anyhow!("WinUAE control {control} is absent"))?;
        match binding {
            Binding::Axis(axis) => ensure!(*axis < 32, "WinUAE axis must be 0..31"),
            Binding::Button(button) => ensure!(*button < 32, "WinUAE button must be 0..31"),
        }
    }
    // WinUAE's Joy1/Joy2 axis events are full two-way axes. Both directions
    // therefore must be measured from the same physical axis.
    ensure!(
        device.bindings["left"] == device.bindings["right"],
        "WinUAE left/right must share one measured axis"
    );
    ensure!(
        device.bindings["up"] == device.bindings["down"],
        "WinUAE up/down must share one measured axis"
    );
    ensure!(
        device.bindings["left"] != device.bindings["up"],
        "WinUAE horizontal and vertical axes must differ"
    );
    Ok(())
}

/// Render a source-shaped WinUAE `.uae` fragment for one or two measured
/// devices. Input event flags use `.0`, the source's no-qualifier spelling.
pub(crate) fn config_text(devices: &[(u8, Device)]) -> Result<String> {
    ensure!(
        !devices.is_empty() && devices.len() <= 2,
        "WinUAE supports one or two classic joystick ports"
    );
    let mut ports = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut out = String::new();
    out.push_str("input.devicematchflags=7\n");
    for (port, device) in devices {
        ensure!(ports.insert(*port), "WinUAE joystick port is duplicated");
        ensure!(
            indices.insert(device.index),
            "WinUAE joystick device is assigned twice"
        );
        validate_device(*port, device)?;
        let n = device.index;
        out.push_str(&format!(
            "input.1.joystick.{n}.name={}\n",
            device.unique_name
        ));
        out.push_str(&format!(
            "input.1.joystick.{n}.friendlyname={}\n",
            device.friendly_name
        ));
        out.push_str(&format!(
            "input.1.joystick.{n}.empty=0\ninput.1.joystick.{n}.disabled=0\n"
        ));
        for control in CONTROLS {
            let binding = device.bindings[control];
            let (kind, number) = match binding {
                Binding::Axis(axis) => ("axis", axis),
                Binding::Button(button) => ("button", button),
            };
            out.push_str(&format!(
                "input.1.joystick.{n}.{kind}.{number}={}.0\n",
                event_name(*port, control)?
            ));
        }
        out.push_str(&format!("joyport{port}=joy{n}\njoyport{port}mode=gamepad\njoyportfriendlyname{port}={}\njoyportname{port}={}\n", device.friendly_name, device.unique_name));
    }
    Ok(out)
}

/// Replace only keys owned by this writer in a copied `.uae` file. The
/// baseline remains authoritative for ROM, Kickstart, media and save paths.
pub(crate) fn patch_config(baseline: &[u8], devices: &[(u8, Device)]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "WinUAE config is too large"
    );
    let fragment = config_text(devices)?;
    let text = std::str::from_utf8(baseline).context("WinUAE config is not UTF-8")?;
    let mut output = String::new();
    for line in text.lines() {
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if key.is_some_and(|key| {
            key == "input.devicematchflags"
                || key.starts_with("input.1.joystick.")
                || key.starts_with("joyport")
        }) {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    output.push_str(&fragment);
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "Use the pinned WinUAE executable and Windows input backend only; verify unique/friendly names and every axis/button in a live session immediately before launch. Preserve the user's Kickstarts, mounted Amiga media, configuration, Save States and saveimage roots. Wine remains a Windows runtime, not native Linux compatibility; a rendered fragment is not proof of startup or effective input."
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device() -> Device {
        Device {
            index: 2,
            unique_name: "HID#VID_1234&PID_5678".into(),
            friendly_name: "Pad One".into(),
            bindings: [
                ("up", Binding::Axis(1)),
                ("down", Binding::Axis(1)),
                ("left", Binding::Axis(0)),
                ("right", Binding::Axis(0)),
                ("fire", Binding::Button(0)),
                ("fire2", Binding::Button(1)),
                ("fire3", Binding::Button(2)),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
        }
    }

    #[test]
    fn emits_source_device_identity_event_and_port_keys() {
        let text = config_text(&[(0, device())]).unwrap();
        assert!(text.contains("input.devicematchflags=7"));
        assert!(text.contains("input.1.joystick.2.name=HID#VID_1234&PID_5678"));
        assert!(text.contains("input.1.joystick.2.axis.0=JOY1_HORIZ.0"));
        assert!(text.contains("input.1.joystick.2.button.1=JOY1_2ND_BUTTON.0"));
        assert!(text.contains("joyport0=joy2\njoyport0mode=gamepad"));
    }

    #[test]
    fn rejects_ambiguous_or_unsafe_profiles_and_preserves_baseline() {
        let mut bad = device();
        bad.bindings.insert("left".into(), Binding::Axis(3));
        assert!(config_text(&[(0, bad)]).is_err());
        assert!(config_text(&[(0, device()), (0, device())]).is_err());
        let patched = patch_config(
            b"kickstart_rom_file=K.rom\njoyport0=none\n",
            &[(0, device())],
        )
        .unwrap();
        assert!(patched.contains("kickstart_rom_file=K.rom"));
        assert!(!patched.contains("joyport0=none"));
        assert!(patched.contains("joyport0=joy2"));
    }
}
