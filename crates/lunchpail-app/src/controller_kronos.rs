//! Kronos native Qt controller grammar and raw SDL2 host-key encoding.
//!
//! Pinned source: `FCare/Kronos` commit
//! `d451a55253e2e75bcef704ec8ade2085d298212c`. `Settings.cpp` uses the
//! `[1.0]` QSettings group; `UIPortManager.cpp` stores pad type and keys under
//! `Input/Port/<port>/Id/<id>/...`; `persdljoy.c` opens every SDL joystick in
//! enumeration order and emits raw button, signed-axis, and cardinal-hat
//! codes. Kronos does not use SDL GameController logical codes here.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "d451a55253e2e75bcef704ec8ade2085d298212c";
pub(crate) const PROFILE_ID: &str = "kronos:standalone-saturn-digital";
pub(crate) const SETTINGS_GROUP: &str = "1.0";
pub(crate) const CONFIG_RELATIVE: &str = "kronos/qt/kronos.ini";
pub(crate) const PER_TYPE_PAD: u32 = 0x02;
pub(crate) const PERSDL_MAX_DEVICES: u32 = 4;

pub(crate) const PAD_BUTTONS: [(u8, &str); 13] = [
    (0, "Up"),
    (1, "Right"),
    (2, "Down"),
    (3, "Left"),
    (4, "R"),
    (5, "L"),
    (6, "Start"),
    (7, "A"),
    (8, "B"),
    (9, "C"),
    (10, "X"),
    (11, "Y"),
    (12, "Z"),
];

pub(crate) const SATURN_ROUTES: [(&str, &str); 13] = [
    ("up", "Up (Key 0)"),
    ("right", "Right (Key 1)"),
    ("down", "Down (Key 2)"),
    ("left", "Left (Key 3)"),
    ("r", "R (Key 4)"),
    ("l", "L (Key 5)"),
    ("start", "Start (Key 6)"),
    ("a", "A (Key 7)"),
    ("b", "B (Key 8)"),
    ("c", "C (Key 9)"),
    ("x", "X (Key 10)"),
    ("y", "Y (Key 11)"),
    ("z", "Z (Key 12)"),
];

pub(crate) fn pad_key_for_target(target: &str) -> Option<u8> {
    Some(match target {
        "up" => 0,
        "right" => 1,
        "down" => 2,
        "left" => 3,
        "r" => 4,
        "l" => 5,
        "start" => 6,
        "a" => 7,
        "b" => 8,
        "c" => 9,
        "x" => 10,
        "y" => 11,
        "z" => 12,
        _ => return None,
    })
}

const SDL_MAX_AXIS_VALUE: u32 = 0x11_0000;
const SDL_MIN_AXIS_VALUE: u32 = 0x10_0000;
const SDL_HAT_VALUE: u32 = 0x20_0000;
const SDL_HAT_UP: u8 = 0x01;
const SDL_HAT_RIGHT: u8 = 0x02;
const SDL_HAT_DOWN: u8 = 0x04;
const SDL_HAT_LEFT: u8 = 0x08;
const DEVICE_SHIFT: u32 = 18;

fn device_bits(device: u32) -> Result<u32, ()> {
    if device >= PERSDL_MAX_DEVICES {
        return Err(());
    }
    Ok(device << DEVICE_SHIFT)
}

pub(crate) fn raw_button_code(device: u32, button: u16) -> Result<u32, ()> {
    Ok(device_bits(device)? | (u32::from(button) + 1))
}

pub(crate) fn raw_axis_code(device: u32, axis: u16, positive: bool) -> Result<u32, ()> {
    let direction = if positive {
        SDL_MAX_AXIS_VALUE
    } else {
        SDL_MIN_AXIS_VALUE
    };
    Ok(device_bits(device)? | direction | u32::from(axis))
}

pub(crate) fn raw_hat_code(device: u32, hat_index: u16, direction: u8) -> Result<u32, ()> {
    if !matches!(
        direction,
        SDL_HAT_UP | SDL_HAT_RIGHT | SDL_HAT_DOWN | SDL_HAT_LEFT
    ) {
        return Err(());
    }
    Ok(device_bits(device)? | SDL_HAT_VALUE | (u32::from(direction) << 4) | u32::from(hat_index))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PadBinding {
    pub port: u8,
    pub id: u8,
    pub bindings: Vec<(u8, u32)>,
}

fn ini_key(path: &str) -> String {
    path.replace('/', "\\")
}

fn fields(pads: &[PadBinding]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !pads.is_empty() && pads.len() <= PERSDL_MAX_DEVICES as usize,
        "Kronos setup needs one to four raw SDL pads"
    );
    let mut slots = BTreeSet::new();
    let mut result = BTreeMap::new();
    for pad in pads {
        ensure!(
            matches!(pad.port, 1 | 2) && matches!(pad.id, 1..=6),
            "Kronos pad port/id is outside the native Saturn topology"
        );
        ensure!(
            slots.insert((pad.port, pad.id)),
            "Kronos pad slot is duplicated"
        );
        let prefix = format!("Input/Port/{}/Id/{}", pad.port, pad.id);
        result.insert(ini_key(&format!("{prefix}/Type")), PER_TYPE_PAD.to_string());
        let mut keys = BTreeSet::new();
        for (key, code) in &pad.bindings {
            ensure!(
                keys.insert(*key) && PAD_BUTTONS.iter().any(|(known, _)| known == key),
                "Kronos pad binding is duplicated or unknown"
            );
            result.insert(
                ini_key(&format!("{prefix}/Controller/{PER_TYPE_PAD}/Key/{key}")),
                code.to_string(),
            );
        }
        ensure!(
            keys.len() == PAD_BUTTONS.len(),
            "Kronos standard pad needs all 13 controls"
        );
    }
    Ok(result)
}

/// Patch only selected pad entries in a copied `kronos.ini`. Machine, BIOS,
/// backup-RAM, cartridge, save-state, media and unrelated input settings stay
/// intact.
pub(crate) fn patch_ini(baseline: &[u8], pads: &[PadBinding]) -> Result<String> {
    ensure!(
        baseline.len() <= 2 * 1024 * 1024,
        "Kronos config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Kronos config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Kronos config contains a NUL byte"
    );
    let fields = fields(pads)?;
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut active = false;
    let mut inserted = false;
    for line in original.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name.trim()))
        {
            if active && !inserted {
                for (key, value) in &fields {
                    output.push_str(&format!("{key}={value}{newline}"));
                }
                inserted = true;
            }
            active = header == SETTINGS_GROUP;
            output.push_str(line);
        } else {
            let owned = active
                && line
                    .split_once('=')
                    .is_some_and(|(key, _)| fields.contains_key(key.trim()));
            if !owned {
                output.push_str(line);
            }
        }
    }
    if active && !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        for (key, value) in &fields {
            output.push_str(&format!("{key}={value}{newline}"));
        }
        inserted = true;
    }
    if !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&format!("[{SETTINGS_GROUP}]{newline}"));
        for (key, value) in &fields {
            output.push_str(&format!("{key}={value}{newline}"));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_pad(port: u8, id: u8, base: u32) -> PadBinding {
        PadBinding {
            port,
            id,
            bindings: PAD_BUTTONS
                .iter()
                .map(|(key, _)| (*key, base + u32::from(*key)))
                .collect(),
        }
    }

    #[test]
    fn raw_codes_match_pinned_persdljoy() {
        assert_eq!(raw_button_code(0, 0), Ok(1));
        assert_eq!(raw_button_code(3, 7), Ok((3 << 18) | 8));
        assert_eq!(raw_axis_code(1, 2, true), Ok((1 << 18) | 0x11_0002));
        assert_eq!(raw_axis_code(1, 2, false), Ok((1 << 18) | 0x10_0002));
        assert_eq!(raw_hat_code(0, 1, SDL_HAT_LEFT), Ok(0x20_0081));
        assert!(raw_button_code(4, 0).is_err());
        assert!(raw_hat_code(0, 0, 3).is_err());
    }

    #[test]
    fn patches_multiple_pads_and_preserves_persistence() {
        let output = patch_ini(
            b"[1.0]\nMemory\\Path=/real/bkram.bin\nInput\\Port\\1\\Id\\1\\Type=0\n[other]\nkeep=yes\n",
            &[full_pad(1, 1, 100), full_pad(2, 6, 200)],
        ).unwrap();
        assert!(output.contains("Memory\\Path=/real/bkram.bin"));
        assert!(output.contains("Input\\Port\\1\\Id\\1\\Type=2"));
        assert!(output.contains("Input\\Port\\1\\Id\\1\\Controller\\2\\Key\\12=112"));
        assert!(output.contains("Input\\Port\\2\\Id\\6\\Controller\\2\\Key\\12=212"));
        assert!(output.contains("[other]\nkeep=yes"));
    }

    #[test]
    fn routes_cover_exact_saturn_pad() {
        assert_eq!(SATURN_ROUTES.len(), PAD_BUTTONS.len());
        for (target, _) in SATURN_ROUTES {
            assert!(pad_key_for_target(target).is_some());
        }
        assert_eq!(pad_key_for_target("mode"), None);
    }
}
