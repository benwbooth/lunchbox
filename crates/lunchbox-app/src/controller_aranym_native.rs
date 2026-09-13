//! ARAnyM standalone SDL joystick-selection writer.
//!
//! Pinned source: aranym/aranym commit
//! 5f4ebed6b039ddf42eef1122a315ad9608d20f6e. `src/parameters.cpp` defines
//! `[JOYSTICKS]` selectors and two 17-entry Jaguar joypad button permutations;
//! `src/input.cpp` passes nonnegative selectors to `SDL_JoystickOpen`.
//! Selectors are runtime enumeration indices, so callers must measure and
//! recheck them for the exact child SDL backend immediately before launch.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "5f4ebed6b039ddf42eef1122a315ad9608d20f6e";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum GuestPort {
    Ikbd0,
    Ikbd1,
    JoypadA,
    JoypadB,
}

impl GuestPort {
    fn key(self) -> &'static str {
        match self {
            Self::Ikbd0 => "Ikbd0",
            Self::Ikbd1 => "Ikbd1",
            Self::JoypadA => "JoypadA",
            Self::JoypadB => "JoypadB",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortBinding {
    pub port: GuestPort,
    /// SDL enumeration index measured for the exact launch. `None` writes -1
    /// and disables the port.
    pub runtime_index: Option<u8>,
    /// For JoypadA/JoypadB only: a permutation of 0..16 mapping the first 17
    /// SDL buttons to Jaguar fire, pause, option, keypad and undefined actions.
    pub button_permutation: Option<[u8; 17]>,
}

fn fields(bindings: &[PortBinding]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !bindings.is_empty() && bindings.len() <= 4,
        "ARAnyM joystick binding count is invalid"
    );
    let mut ports = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut result = BTreeMap::new();
    for binding in bindings {
        ensure!(
            ports.insert(binding.port),
            "ARAnyM guest port is duplicated"
        );
        let index = match binding.runtime_index {
            Some(index) => {
                ensure!(index <= 31, "ARAnyM SDL runtime index is out of range");
                ensure!(
                    indices.insert(index),
                    "ARAnyM SDL runtime index is duplicated"
                );
                index as i16
            }
            None => -1,
        };
        result.insert(binding.port.key().into(), index.to_string());

        let joypad = matches!(binding.port, GuestPort::JoypadA | GuestPort::JoypadB);
        ensure!(
            joypad == binding.button_permutation.is_some(),
            "ARAnyM button permutation is required only for JoypadA/JoypadB"
        );
        if let Some(permutation) = binding.button_permutation {
            let unique = permutation.iter().copied().collect::<BTreeSet<_>>();
            ensure!(
                unique.len() == 17 && unique.first() == Some(&0) && unique.last() == Some(&16),
                "ARAnyM joypad buttons must be a permutation of 0 through 16"
            );
            let value = permutation
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            result.insert(
                match binding.port {
                    GuestPort::JoypadA => "JoypadAButtons",
                    GuestPort::JoypadB => "JoypadBButtons",
                    _ => unreachable!(),
                }
                .into(),
                value,
            );
        }
    }
    Ok(result)
}

/// Patch only selected `[JOYSTICKS]` keys in a copied ARAnyM config. The
/// launch layer remains responsible for pinning the executable/SDL backend,
/// validating the measured indices against physical controller identities,
/// and preserving TOS, disks, GEMDOS folders, snapshots and guest saves.
pub(crate) fn patch_joysticks(baseline: &[u8], bindings: &[PortBinding]) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "ARAnyM config is too large");
    let original = std::str::from_utf8(baseline).context("ARAnyM config is not UTF-8")?;
    let fields = fields(bindings)?;
    patch_section(original, "JOYSTICKS", &fields)
}

fn patch_section(
    original: &str,
    section: &str,
    fields: &BTreeMap<String, String>,
) -> Result<String> {
    ensure!(
        !original.contains('\0'),
        "ARAnyM config contains a NUL byte"
    );
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
            active = header == section;
            output.push_str(line);
            if active && !inserted {
                if !output.ends_with('\n') {
                    output.push_str(newline);
                }
                for (key, value) in fields {
                    output.push_str(&format!("{key} = {value}{newline}"));
                }
                inserted = true;
            }
        } else {
            let owned = active
                && line
                    .split_once('=')
                    .is_some_and(|(key, _)| fields.keys().any(|known| key.trim() == known));
            if !owned {
                output.push_str(line);
            }
        }
    }
    if !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&format!("[{section}]{newline}"));
        for (key, value) in fields {
            output.push_str(&format!("{key} = {value}{newline}"));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_measured_index_and_exact_permutation() {
        let permutation = std::array::from_fn(|index| (16 - index) as u8);
        let output = patch_joysticks(
            b"[GLOBAL]\nFastRAM = 64\n[JOYSTICKS]\nJoypadA = -1\nJoypadAButtons = 0 1 2\nIkbd0 = -1\n",
            &[
                PortBinding {
                    port: GuestPort::Ikbd0,
                    runtime_index: Some(0),
                    button_permutation: None,
                },
                PortBinding {
                    port: GuestPort::JoypadA,
                    runtime_index: Some(1),
                    button_permutation: Some(permutation),
                },
            ],
        )
        .unwrap();
        assert!(output.contains("Ikbd0 = 0\nJoypadA = 1\nJoypadAButtons = 16 15 14 13"));
        assert!(output.contains("[GLOBAL]\nFastRAM = 64\n"));
        assert!(!output.contains("JoypadA = -1"));
    }

    #[test]
    fn rejects_stale_or_ambiguous_selection_data() {
        let duplicate = [
            PortBinding {
                port: GuestPort::Ikbd0,
                runtime_index: Some(0),
                button_permutation: None,
            },
            PortBinding {
                port: GuestPort::Ikbd1,
                runtime_index: Some(0),
                button_permutation: None,
            },
        ];
        assert!(patch_joysticks(b"", &duplicate).is_err());
        let mut invalid = std::array::from_fn(|index| index as u8);
        invalid[16] = 15;
        assert!(
            patch_joysticks(
                b"",
                &[PortBinding {
                    port: GuestPort::JoypadB,
                    runtime_index: Some(2),
                    button_permutation: Some(invalid),
                }]
            )
            .is_err()
        );
    }
}
