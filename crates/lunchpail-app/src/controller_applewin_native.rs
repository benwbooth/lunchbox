//! AppleWin native WinMM joystick-selection writer.
//!
//! Pinned source: AppleWin/AppleWin commit
//! `3e8054b4627624398e4589f7f27b3d40a6b9718e`.  `Joystick.cpp` discovers the
//! first two usable WinMM devices at launch and the configuration layer stores
//! only the emulation method, not a stable physical identity.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const PROFILE_ID: &str = "applewin:native-winmm-v1";
pub(crate) const SOURCE_COMMIT: &str = "3e8054b4627624398e4589f7f27b3d40a6b9718e";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JoystickMode {
    Disabled,
    PcJoystick,
    KeyboardCursors,
    KeyboardNumpad,
    Mouse,
    PcJoystick1Thumbstick2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JoystickSelection {
    pub mode: JoystickMode,
    /// WinMM device ID observed immediately before this launch.  AppleWin
    /// stores only "PC Joystick #1/#2", so this value is a probe contract.
    pub runtime_winmm_id: Option<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InputProfile {
    pub joysticks: [JoystickSelection; 2],
    pub x_trim: i16,
    pub y_trim: i16,
    /// AppleWin persists a three-button bit mask.
    pub autofire_mask: u8,
    pub centering_control: bool,
    pub cursor_control: bool,
    pub swap_buttons: bool,
}

fn mode_value(index: usize, selection: JoystickSelection) -> Result<u32> {
    let value = match selection.mode {
        JoystickMode::Disabled => 0,
        JoystickMode::PcJoystick => 1,
        JoystickMode::KeyboardCursors => 2,
        JoystickMode::KeyboardNumpad => 3,
        JoystickMode::Mouse => 4,
        JoystickMode::PcJoystick1Thumbstick2 => {
            ensure!(index == 1, "AppleWin thumbstick-2 mode is joystick 2 only");
            5
        }
    };
    if matches!(
        selection.mode,
        JoystickMode::PcJoystick | JoystickMode::PcJoystick1Thumbstick2
    ) {
        let id = selection.runtime_winmm_id;
        ensure!(
            id.is_some(),
            "AppleWin PC joystick mode needs a probed WinMM ID"
        );
        ensure!(id.unwrap() <= 15, "AppleWin WinMM ID must be 0 through 15");
    } else {
        ensure!(
            selection.runtime_winmm_id.is_none(),
            "non-PC AppleWin mode cannot carry a WinMM ID"
        );
    }
    Ok(value)
}

fn fields(profile: InputProfile) -> Result<BTreeMap<&'static str, String>> {
    let mut ids = BTreeSet::new();
    let mut values = BTreeMap::new();
    for (index, selection) in profile.joysticks.into_iter().enumerate() {
        if let Some(id) = selection.runtime_winmm_id {
            ensure!(ids.insert(id), "AppleWin WinMM device ID is duplicated");
        }
        values.insert(
            if index == 0 {
                "Joystick0 Emu Type v3"
            } else {
                "Joystick1 Emu Type v3"
            },
            mode_value(index, selection)?.to_string(),
        );
    }
    ensure!(
        profile.autofire_mask <= 7,
        "AppleWin autofire mask must use three bits"
    );
    values.extend([
        ("PDL X-Trim", profile.x_trim.to_string()),
        ("PDL Y-Trim", profile.y_trim.to_string()),
        ("Autofire", profile.autofire_mask.to_string()),
        (
            "Joystick Centering Control",
            u8::from(profile.centering_control).to_string(),
        ),
        (
            "Joystick Cursor Control",
            u8::from(profile.cursor_control).to_string(),
        ),
        (
            "Swap buttons 0 and 1",
            u8::from(profile.swap_buttons).to_string(),
        ),
    ]);
    Ok(values)
}

/// Patch only AppleWin's source-defined `[Configuration]` input keys in a
/// copied `-conf` INI. Registry-backed launches must first select an INI with
/// `-conf`; this writer never edits the user's registry.
pub(crate) fn patch_config(baseline: &[u8], profile: InputProfile) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "AppleWin INI is too large"
    );
    let original = std::str::from_utf8(baseline).context("AppleWin INI is not UTF-8")?;
    ensure!(!original.contains('\0'), "AppleWin INI contains a NUL byte");
    patch_section(original, "Configuration", &fields(profile)?)
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
    let mut closed = false;
    let mut seen = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|s| s.split_once(']').map(|(name, _)| name.trim()))
        {
            if active && !closed {
                for (key, value) in fields {
                    if !seen.contains(key) {
                        output.push_str(&format!("{key}={value}{newline}"));
                    }
                }
                closed = true;
            }
            let matching = header.eq_ignore_ascii_case(section);
            if matching {
                ensure!(!found, "AppleWin INI section is duplicated");
                found = true;
            }
            active = matching;
            output.push_str(line);
        } else if active {
            let key = line.split_once('=').and_then(|(key, _)| {
                fields
                    .keys()
                    .find(|known| key.trim().eq_ignore_ascii_case(known))
                    .copied()
            });
            if let Some(key) = key {
                ensure!(seen.insert(key), "AppleWin INI key is duplicated");
                output.push_str(&format!("{key}={}{newline}", fields[key]));
            } else {
                output.push_str(line);
            }
        } else {
            output.push_str(line);
        }
    }
    if !closed {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        if !active {
            output.push_str(&format!("[{section}]{newline}"));
        }
        for (key, value) in fields {
            if !seen.contains(key) {
                output.push_str(&format!("{key}={value}{newline}"));
            }
        }
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "AppleWin's WinMM probe chooses the first two usable devices and its INI stores only joystick emulation types. Probe and recheck the exact WinMM IDs immediately before launch, then pass a copied -conf INI. Preserve the Apple II model, disk/hard-disk media, firmware, and SaveState.yaml path; this writer does not prove executable startup or guest input."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> InputProfile {
        InputProfile {
            joysticks: [
                JoystickSelection {
                    mode: JoystickMode::PcJoystick,
                    runtime_winmm_id: Some(4),
                },
                JoystickSelection {
                    mode: JoystickMode::KeyboardCursors,
                    runtime_winmm_id: None,
                },
            ],
            x_trim: -2,
            y_trim: 3,
            autofire_mask: 5,
            centering_control: true,
            cursor_control: false,
            swap_buttons: true,
        }
    }

    #[test]
    fn patches_configuration_and_preserves_other_sections() {
        let output = patch_config(
            b"; keep\n[Configuration]\nJoystick0 Emu Type v3=0\nCPU Type=2\n[Other]\nvalue=1\n",
            profile(),
        )
        .unwrap();
        assert!(output.contains("Joystick0 Emu Type v3=1\n"));
        assert!(output.contains("Joystick1 Emu Type v3=2\n"));
        assert!(output.contains("PDL X-Trim=-2\n"));
        assert!(output.contains("Autofire=5\n"));
        assert!(output.contains("CPU Type=2\n"));
        assert!(output.contains("[Other]\nvalue=1\n"));
    }

    #[test]
    fn rejects_unprobed_or_duplicate_winmm_devices() {
        let mut bad = profile();
        bad.joysticks[0].runtime_winmm_id = None;
        assert!(patch_config(b"", bad).is_err());
        bad = profile();
        bad.joysticks[1] = JoystickSelection {
            mode: JoystickMode::PcJoystick,
            runtime_winmm_id: Some(4),
        };
        assert!(patch_config(b"", bad).is_err());
        assert!(patch_config(b"[Configuration]\n[Configuration]\n", profile()).is_err());
    }
}
