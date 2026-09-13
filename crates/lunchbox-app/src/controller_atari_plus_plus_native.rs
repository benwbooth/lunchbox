//! Atari++ native Linux joystick overlay writer.
//!
//! The 1.85 source reads Linux joystick units directly through
//! `/dev/input/js<n>`.  This writer emits only the documented AnalogJoystick
//! path after the caller has probed that exact unit; it never substitutes the
//! SDL numeric-unit or digital/paddle modes.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_VERSION: &str = "1.85";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortOverlay {
    /// Zero-based emulated Atari++ port (0..3).
    pub port: u8,
    /// Zero-based Linux joystick unit, corresponding to /dev/input/js<n>.
    pub unit: u8,
    pub sensitivity: u16,
    pub horizontal_axis: u8,
    pub vertical_axis: u8,
    /// Physical button numbers are one-based in Atari++'s configuration.
    pub buttons: [u8; 4],
}

fn axis_name(axis: u8) -> Result<&'static str> {
    Ok(match axis {
        0 => "XAxis.1",
        1 => "YAxis.1",
        2 => "XAxis.2",
        3 => "YAxis.2",
        _ => anyhow::bail!("Atari++ axis must be 0..3"),
    })
}

fn render(overlay: &PortOverlay) -> Result<Vec<String>> {
    ensure!(overlay.port < 4, "Atari++ joystick port must be 0..3");
    ensure!(overlay.unit < 8, "Atari++ joystick unit must be 0..7");
    ensure!(
        overlay
            .buttons
            .iter()
            .all(|button| (1..=16).contains(button)),
        "Atari++ button must be 1..16"
    );
    ensure!(
        BTreeSet::from(overlay.buttons).len() == overlay.buttons.len(),
        "Atari++ abstract buttons must be distinct"
    );
    let horizontal = axis_name(overlay.horizontal_axis)?;
    let vertical = axis_name(overlay.vertical_axis)?;
    Ok(vec![
        format!(
            "Joystick.{}.Port = AnalogJoystick.{}",
            overlay.port, overlay.unit
        ),
        format!(
            "Joystick.{}.Sensitivity = {}",
            overlay.port, overlay.sensitivity
        ),
        format!("HAxis.{} = {horizontal}", overlay.unit),
        format!("VAxis.{} = {vertical}", overlay.unit),
        format!("First_Button.{} = {}", overlay.unit, overlay.buttons[0]),
        format!("Second_Button.{} = {}", overlay.unit, overlay.buttons[1]),
        format!("Third_Button.{} = {}", overlay.unit, overlay.buttons[2]),
        format!("Fourth_Button.{} = {}", overlay.unit, overlay.buttons[3]),
    ])
}

/// Replace only the explicitly generated Atari++ joystick options and retain
/// all unrelated machine, disk, ROM, and frontend settings.
pub(crate) fn patch_config(baseline: &[u8], overlays: &[PortOverlay]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Atari++ config is too large"
    );
    ensure!(overlays.len() <= 4, "Atari++ has four joystick ports");
    let mut seen_ports = BTreeSet::new();
    let mut seen_units = BTreeSet::new();
    let mut generated = Vec::new();
    let mut keys = BTreeSet::new();
    for overlay in overlays {
        ensure!(
            seen_ports.insert(overlay.port),
            "Atari++ port is duplicated"
        );
        ensure!(
            seen_units.insert(overlay.unit),
            "Atari++ joystick unit is reused"
        );
        keys.insert(format!("Joystick.{}.", overlay.port));
        keys.insert(format!("HAxis.{}", overlay.unit));
        keys.insert(format!("VAxis.{}", overlay.unit));
        for name in [
            "First_Button",
            "Second_Button",
            "Third_Button",
            "Fourth_Button",
        ] {
            keys.insert(format!("{name}.{}", overlay.unit));
        }
        generated.extend(render(overlay)?);
    }
    let text = std::str::from_utf8(baseline).context("Atari++ config is not UTF-8")?;
    let mut kept = Vec::new();
    for line in text.lines() {
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if key.is_some_and(|key| {
            keys.iter().any(|owned| {
                if owned.ends_with('.') {
                    key.starts_with(owned)
                } else {
                    key == owned
                }
            })
        }) {
            continue;
        }
        kept.push(line);
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&generated.join("\n"));
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_linux_analog_contract_without_sdl_substitution() {
        let overlay = PortOverlay {
            port: 0,
            unit: 1,
            sensitivity: 8192,
            horizontal_axis: 0,
            vertical_axis: 1,
            buttons: [1, 2, 3, 4],
        };
        let text = patch_config(
            b"Machine = XL\nJoystick.0.Port = KeypadStick.0\n",
            &[overlay],
        )
        .unwrap();
        assert!(text.contains("Machine = XL"));
        assert!(text.contains("Joystick.0.Port = AnalogJoystick.1"));
        assert!(text.contains("HAxis.1 = XAxis.1"));
        assert!(!text.contains("KeypadStick.0"));
    }
}
