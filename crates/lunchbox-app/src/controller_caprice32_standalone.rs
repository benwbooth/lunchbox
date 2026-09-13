//! Caprice32 standalone SDL joystick-enablement writer.
//!
//! Pinned source: ColinPitrat/caprice32 commit
//! 6c12c4c92360065cdc229ac9ada7551f941436b8. `cap32.cpp` persists the
//! `[system]` joystick switches, opens the first eight SDL devices, and
//! `keyboard.cpp` routes event instance 0/1 to CPC joystick 0/1 with fixed
//! axes 0/1 (also 2/3) and buttons 0/1. There is no device selector to write,
//! so callers must prove that the selected pads occupy those exact instances
//! for the exact child process.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "6c12c4c92360065cdc229ac9ada7551f941436b8";
pub(crate) const PROFILE_ID: &str = "caprice32:standalone-native-fixed-sdl-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FixedSdlSelection {
    /// SDL event instance IDs observed for CPC joystick 0 and optionally 1.
    /// The pinned mapper recognizes only instance IDs 0 and 1.
    pub instance_ids: [Option<u8>; 2],
    /// One-based values persisted by Caprice32; internally it subtracts one.
    pub menu_button: u8,
    pub virtual_keyboard_button: u8,
}

fn fields(selection: FixedSdlSelection) -> Result<BTreeMap<&'static str, String>> {
    ensure!(
        selection.instance_ids[0] == Some(0),
        "Caprice32 CPC joystick 0 requires measured SDL instance 0"
    );
    ensure!(
        selection.instance_ids[1].is_none() || selection.instance_ids[1] == Some(1),
        "Caprice32 CPC joystick 1 requires measured SDL instance 1"
    );
    ensure!(
        (1..=64).contains(&selection.menu_button)
            && (1..=64).contains(&selection.virtual_keyboard_button),
        "Caprice32 global buttons must be one-based values from 1 through 64"
    );
    ensure!(
        selection.menu_button != selection.virtual_keyboard_button,
        "Caprice32 menu and virtual-keyboard buttons must differ"
    );
    Ok(BTreeMap::from([
        ("joysticks", "1".into()),
        ("joystick_emulation", "0".into()),
        ("joystick_menu_button", selection.menu_button.to_string()),
        (
            "joystick_vkeyboard_button",
            selection.virtual_keyboard_button.to_string(),
        ),
    ]))
}

/// Patch only Caprice32's source-defined `[system]` joystick keys in a copied
/// `cap32.cfg`. Device ordering is not encoded by this file: the launch layer
/// must re-probe the exact SDL backend and fail if the intended controller(s)
/// do not have event instance IDs 0 and optionally 1 immediately before exec.
pub(crate) fn patch_config(baseline: &[u8], selection: FixedSdlSelection) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "Caprice32 config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Caprice32 config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Caprice32 config contains a NUL byte"
    );
    let fields = fields(selection)?;
    patch_section(original, "system", &fields)
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
                ensure!(!found, "Caprice32 config section is duplicated");
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
                ensure!(seen.insert(key), "Caprice32 config key is duplicated");
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
    "The pinned executable has a fixed first-two-SDL-device contract, not a persistent device identity grammar. Re-probe event instance IDs for the exact child immediately before launch. Preserve all other cap32.cfg settings, writable .dsk media and .sna snapshots; Caprice32 libretro and native Flatpak remain separate contracts."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enables_fixed_sdl_mapping_and_preserves_other_sections() {
        let output = patch_config(
            b"[system]\njoysticks=0\njoystick_emulation=1\nspeed=4\n[video]\nscr_scale=3\n",
            FixedSdlSelection {
                instance_ids: [Some(0), Some(1)],
                menu_button: 9,
                virtual_keyboard_button: 10,
            },
        )
        .unwrap();
        assert!(output.contains("joysticks=1\n"));
        assert!(output.contains("joystick_emulation=0\n"));
        assert!(output.contains("joystick_menu_button=9\n"));
        assert!(output.contains("speed=4\n"));
        assert!(output.contains("[video]\nscr_scale=3\n"));
    }

    #[test]
    fn rejects_unmatched_instances_and_button_collisions() {
        let mut selection = FixedSdlSelection {
            instance_ids: [Some(2), None],
            menu_button: 9,
            virtual_keyboard_button: 10,
        };
        assert!(patch_config(b"", selection).is_err());
        selection.instance_ids = [Some(0), None];
        selection.virtual_keyboard_button = 9;
        assert!(patch_config(b"", selection).is_err());
        selection.virtual_keyboard_button = 10;
        assert!(patch_config(b"[system]\n[system]\n", selection).is_err());
    }
}
