//! Atari800 SDL2 input overlay writer.
//!
//! This is deliberately a writer-only module until the launcher has a measured
//! Atari target layout.  Atari800's source contract is exact, but its host
//! identity is SDL display-name plus duplicate-name slot, so a caller must
//! probe the device and provide the action key explicitly.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "fe1d2890d9f05fcecb2fd033d09a5c43f534bebf";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectionSource {
    Auto,
    Hat,
    Axes { first_axis: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortOverlay {
    /// Zero-based Atari800 port, matching SDL2_JOY_PORT_<n>.
    pub port: u8,
    pub device_name: String,
    /// Duplicate-name slot, zero-based, as written by Atari800.
    pub device_slot: u8,
    pub source: DirectionSource,
    /// (SDL button index, Atari800 action enum, Atari key/action value).
    pub fire: (u8, i32, i32),
    pub diagonal_zone: u8,
}

fn safe_name(value: &str) -> Result<()> {
    ensure!(!value.is_empty(), "Atari800 device name cannot be empty");
    ensure!(value.len() <= 256, "Atari800 device name is too long");
    ensure!(
        !value
            .chars()
            .any(|ch| ch == '\n' || ch == '\r' || ch == '='),
        "Atari800 device name contains a config delimiter"
    );
    Ok(())
}

fn render(overlay: &PortOverlay) -> Result<Vec<String>> {
    ensure!(overlay.port < 4, "Atari800 port must be 0..3");
    safe_name(&overlay.device_name)?;
    ensure!(
        overlay.diagonal_zone <= 2,
        "Atari800 diagonal zone must be 0..2"
    );
    ensure!(overlay.fire.0 < 15, "Atari800 fire button must be 0..14");
    ensure!(
        (0..=3).contains(&overlay.fire.1),
        "Atari800 action enum is invalid"
    );
    let mut lines = vec![
        format!("SDL2_JOY_PORT_{}_MODE=4", overlay.port),
        format!(
            "SDL2_JOY_PORT_{}_NAME={}",
            overlay.port, overlay.device_name
        ),
        format!(
            "SDL2_JOY_PORT_{}_SLOT={}",
            overlay.port, overlay.device_slot
        ),
    ];
    match overlay.source {
        DirectionSource::Auto => lines.push(format!("SDL2_JOY_PORT_{}_USE_HAT=-1", overlay.port)),
        DirectionSource::Hat => lines.push(format!("SDL2_JOY_PORT_{}_USE_HAT=1", overlay.port)),
        DirectionSource::Axes { first_axis } => {
            ensure!(first_axis <= 126, "Atari800 axis base is out of range");
            lines.push(format!("SDL2_JOY_PORT_{}_USE_HAT=0", overlay.port));
            lines.push(format!("SDL2_JOY_PORT_{}_AXES={first_axis}", overlay.port));
        }
    }
    lines.push(format!(
        "SDL2_JOY_PORT_{}_DIAGONALS={}",
        overlay.port, overlay.diagonal_zone
    ));
    let mut actions = vec![0; 15];
    let mut keys = vec![0; 15];
    actions[usize::from(overlay.fire.0)] = overlay.fire.1;
    keys[usize::from(overlay.fire.0)] = overlay.fire.2;
    lines.push(format!(
        "SDL2_JOY_PORT_{}_BUTTON_ACTIONS={},",
        overlay.port,
        actions
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ));
    lines.push(format!(
        "SDL2_JOY_PORT_{}_BUTTON_KEYS={},",
        overlay.port,
        keys.iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ));
    Ok(lines)
}

/// Replace only the explicitly named port keys, preserving every unrelated
/// Atari800 setting and all other ports.
pub(crate) fn patch_config(baseline: &[u8], overlays: &[PortOverlay]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Atari800 config is too large"
    );
    ensure!(overlays.len() <= 4, "Atari800 has four joystick ports");
    let mut seen = BTreeSet::new();
    let mut replacement = Vec::new();
    let mut prefixes = BTreeSet::new();
    for overlay in overlays {
        ensure!(seen.insert(overlay.port), "Atari800 port is duplicated");
        prefixes.insert(format!("SDL2_JOY_PORT_{}", overlay.port));
        replacement.extend(render(overlay)?);
    }
    let text = std::str::from_utf8(baseline).context("Atari800 config is not UTF-8")?;
    let mut kept = Vec::new();
    for line in text.lines() {
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if key.is_some_and(|key| prefixes.iter().any(|prefix| key.starts_with(prefix))) {
            continue;
        }
        kept.push(line);
    }
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&replacement.join("\n"));
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_one_measured_port_and_preserves_other_settings() {
        let overlay = PortOverlay {
            port: 0,
            device_name: "Pad One".into(),
            device_slot: 0,
            source: DirectionSource::Axes { first_axis: 0 },
            fire: (0, 1, 0x1000),
            diagonal_zone: 2,
        };
        let text = patch_config(
            b"MACHINE_TYPE=Atari XL\nSDL2_JOY_PORT_0_MODE=1\n",
            &[overlay],
        )
        .unwrap();
        assert!(text.contains("MACHINE_TYPE=Atari XL"));
        assert!(text.contains("SDL2_JOY_PORT_0_MODE=4"));
        assert!(text.contains("SDL2_JOY_PORT_0_AXES=0"));
        assert!(!text.contains("SDL2_JOY_PORT_0_MODE=1"));
    }
}
