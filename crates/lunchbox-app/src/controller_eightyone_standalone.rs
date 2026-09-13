//! EightyOne native WinMM joystick-selection writer.
//!
//! Pinned source: charlierobson/EightyOne commit
//! 4de3cdb8cff0060ad03e43ae2681b0628a9c5f5d. `main_.cpp` reads/writes
//! `[MAIN]` connection, autofire, and `JoystickNController` values, while
//! `Joystick.cpp` probes WinMM IDs 0 through 15 and polls the stored IDs.
//! Those IDs are runtime slots, so the exact Windows process must be probed
//! and rechecked immediately before launch.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const PROFILE_ID: &str = "eightyone:standalone-native-winmm-v1";
pub(crate) const SOURCE_COMMIT: &str = "4de3cdb8cff0060ad03e43ae2681b0628a9c5f5d";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PortSelection {
    /// WinMM ID measured for the exact launch. `None` disconnects the port.
    pub runtime_index: Option<u8>,
    pub autofire: bool,
}

fn fields(ports: [PortSelection; 2]) -> Result<BTreeMap<&'static str, String>> {
    let mut indices = BTreeSet::new();
    for port in ports {
        if let Some(index) = port.runtime_index {
            ensure!(index <= 15, "EightyOne WinMM index must be 0 through 15");
            ensure!(indices.insert(index), "EightyOne WinMM index is duplicated");
        } else {
            ensure!(
                !port.autofire,
                "EightyOne autofire needs a connected controller"
            );
        }
    }
    Ok(BTreeMap::from([
        (
            "ConnectJoystick1",
            u8::from(ports[0].runtime_index.is_some()).to_string(),
        ),
        (
            "ConnectJoystick2",
            u8::from(ports[1].runtime_index.is_some()).to_string(),
        ),
        (
            "EnableJoystick1AutoFire",
            u8::from(ports[0].autofire).to_string(),
        ),
        (
            "EnableJoystick2AutoFire",
            u8::from(ports[1].autofire).to_string(),
        ),
        (
            "Joystick1Controller",
            ports[0]
                .runtime_index
                .map_or_else(|| "-1".into(), |index| index.to_string()),
        ),
        (
            "Joystick2Controller",
            ports[1]
                .runtime_index
                .map_or_else(|| "-1".into(), |index| index.to_string()),
        ),
    ]))
}

/// Patch only the six source-defined `[MAIN]` joystick selector keys in a
/// copied EightyOne INI. The caller keeps the user's selected machine and
/// joystick interface; this function only binds physical WinMM controllers.
pub(crate) fn patch_config(baseline: &[u8], ports: [PortSelection; 2]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "EightyOne INI is too large"
    );
    let original = std::str::from_utf8(baseline).context("EightyOne INI is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "EightyOne INI contains a NUL byte"
    );
    patch_section(original, "MAIN", &fields(ports)?)
}

fn patch_section(original: &str, section: &str, fields: &BTreeMap<&str, String>) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut active = false;
    let mut found = false;
    let mut inserted = false;
    let mut seen = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name.trim()))
        {
            if active && !inserted {
                for (key, value) in fields {
                    if !seen.contains(*key) {
                        output.push_str(&format!("{key}={value}{newline}"));
                    }
                }
                inserted = true;
            }
            let matching = header.eq_ignore_ascii_case(section);
            if matching {
                ensure!(!found, "EightyOne INI section is duplicated");
                found = true;
            }
            active = matching;
            output.push_str(line);
        } else if active {
            let owned = line.split_once('=').and_then(|(key, _)| {
                fields
                    .keys()
                    .find(|known| key.trim().eq_ignore_ascii_case(known))
                    .copied()
            });
            if let Some(key) = owned {
                ensure!(seen.insert(key), "EightyOne INI key is duplicated");
                output.push_str(&format!("{key}={}{newline}", fields[key]));
            } else {
                output.push_str(line);
            }
        } else {
            output.push_str(line);
        }
    }
    if !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        if !active {
            output.push_str(&format!("[{section}]{newline}"));
        }
        for (key, value) in fields {
            if !seen.contains(*key) {
                output.push_str(&format!("{key}={value}{newline}"));
            }
        }
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "EightyOne persists WinMM slots but no stable physical identity. Resolve and recheck IDs 0 through 15 for the exact native Windows process immediately before launch. Preserve the selected machine/guest joystick interface, programmable keys, media and state paths. Wine is a Windows runtime rather than native Linux support."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_measured_winmm_slots_and_preserves_unrelated_ini() {
        let output = patch_config(
            b"[MAIN]\r\nJoystick1Controller=-1\r\nBorderNormal=1\r\n[KB]\r\nUseNumericPadForJoystick1=0\r\n",
            [
                PortSelection {
                    runtime_index: Some(3),
                    autofire: true,
                },
                PortSelection {
                    runtime_index: None,
                    autofire: false,
                },
            ],
        )
        .unwrap();
        assert!(output.contains("Joystick1Controller=3\r\n"));
        assert!(output.contains("ConnectJoystick1=1\r\n"));
        assert!(output.contains("EnableJoystick1AutoFire=1\r\n"));
        assert!(output.contains("Joystick2Controller=-1\r\n"));
        assert!(output.contains("BorderNormal=1\r\n"));
        assert!(output.contains("[KB]\r\nUseNumericPadForJoystick1=0\r\n"));
    }

    #[test]
    fn rejects_stale_or_ambiguous_slots() {
        assert!(
            patch_config(
                b"",
                [
                    PortSelection {
                        runtime_index: Some(16),
                        autofire: false,
                    },
                    PortSelection {
                        runtime_index: None,
                        autofire: false,
                    },
                ]
            )
            .is_err()
        );
        assert!(
            patch_config(
                b"",
                [
                    PortSelection {
                        runtime_index: Some(1),
                        autofire: false,
                    },
                    PortSelection {
                        runtime_index: Some(1),
                        autofire: false,
                    },
                ]
            )
            .is_err()
        );
        assert!(
            patch_config(
                b"[MAIN]\n[MAIN]\n",
                [
                    PortSelection {
                        runtime_index: Some(0),
                        autofire: false,
                    },
                    PortSelection {
                        runtime_index: None,
                        autofire: false,
                    },
                ]
            )
            .is_err()
        );
    }
}
