//! EKA2L1 native SDL2 keybind-profile writer.
//!
//! Pinned source: EKA2L1/EKA2L1 commit
//! `8dd86cffc59d12c59661acecdfddfab5ffc810db`.  `config.cpp` serializes
//! `config.yml` and `bindings/<current-keybind-profile>.yml`; the SDL2
//! controller backend uses the enumerated joystick index as `controller_id`.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const PROFILE_ID: &str = "eka2l1:native-sdl2-keybind-v1";
pub(crate) const SOURCE_COMMIT: &str = "8dd86cffc59d12c59661acecdfddfab5ffc810db";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindingSource {
    Keyboard { keycode: u32 },
    Mouse { button: u32 },
    Controller { controller_id: i32, button_id: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyBinding {
    /// EKA2L1 guest scan/key code. The source casts this to std_scan_code.
    pub target: u32,
    pub source: BindingSource,
}

fn validate_name(name: &str) -> Result<()> {
    ensure!(!name.is_empty(), "EKA2L1 keybind profile name is empty");
    ensure!(name.len() <= 128, "EKA2L1 keybind profile name is too long");
    ensure!(
        name.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)),
        "EKA2L1 profile name must be a safe YAML scalar and file stem"
    );
    ensure!(
        name != "." && name != "..",
        "EKA2L1 profile name is invalid"
    );
    Ok(())
}

fn validate_binding(binding: KeyBinding) -> Result<()> {
    ensure!(
        binding.target <= u16::MAX as u32,
        "EKA2L1 guest key code is out of range"
    );
    match binding.source {
        BindingSource::Keyboard { .. } | BindingSource::Mouse { .. } => {}
        BindingSource::Controller {
            controller_id,
            button_id,
        } => {
            ensure!(
                controller_id >= 0,
                "EKA2L1 controller ID must be non-negative"
            );
            ensure!(
                button_id >= 0 && (button_id <= 20 || (300..=311).contains(&button_id)),
                "EKA2L1 controller button code is unsupported"
            );
        }
    }
    Ok(())
}

/// Patch only the top-level `current-keybind-profile` scalar in `config.yml`.
/// The returned string is written beside the copied EKA2L1 install/data root;
/// the caller writes `bindings_yaml` to `bindings/<profile>.yml`.
pub(crate) fn patch_config(
    config_baseline: &[u8],
    profile_name: &str,
    bindings: &[KeyBinding],
) -> Result<(String, String)> {
    validate_name(profile_name)?;
    ensure!(
        bindings.len() <= 4096,
        "EKA2L1 keybind profile is too large"
    );
    let mut targets = BTreeSet::new();
    for binding in bindings {
        validate_binding(*binding)?;
        ensure!(
            targets.insert(binding.target),
            "EKA2L1 keybind target is duplicated"
        );
    }
    ensure!(
        config_baseline.len() <= 4 * 1024 * 1024,
        "EKA2L1 config.yml is too large"
    );
    let original =
        std::str::from_utf8(config_baseline).context("EKA2L1 config.yml is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "EKA2L1 config.yml contains a NUL byte"
    );
    let config = patch_profile_scalar(original, profile_name)?;
    Ok((config, serialize_bindings(bindings)))
}

fn patch_profile_scalar(original: &str, profile_name: &str) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut seen = false;
    for line in original.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let key = body.split_once(':').map(|(key, _)| key.trim());
        if key == Some("current-keybind-profile") {
            ensure!(!seen, "EKA2L1 current-keybind-profile is duplicated");
            seen = true;
            output.push_str("current-keybind-profile: ");
            output.push_str(profile_name);
            output.push_str(newline);
        } else {
            output.push_str(line);
        }
    }
    if !seen {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str("current-keybind-profile: ");
        output.push_str(profile_name);
        output.push_str(newline);
    }
    Ok(output)
}

/// Serialize the exact YAML object shape consumed by EKA2L1's `keybind_profile`.
/// The file is owned by the selected profile, so unrelated `config.yml`, data,
/// firmware, media, and save/state files are not touched.
pub(crate) fn serialize_bindings(bindings: &[KeyBinding]) -> String {
    let mut output = String::new();
    for binding in bindings {
        output.push_str("- source:\n    type: ");
        match binding.source {
            BindingSource::Keyboard { keycode } => {
                output.push_str("key\n    data:\n      keycode: ");
                output.push_str(&keycode.to_string());
            }
            BindingSource::Mouse { button } => {
                output.push_str("mouse\n    data:\n      keycode: ");
                output.push_str(&button.to_string());
            }
            BindingSource::Controller {
                controller_id,
                button_id,
            } => {
                output.push_str("controller\n    data:\n      controller_id: ");
                output.push_str(&controller_id.to_string());
                output.push_str("\n      button_id: ");
                output.push_str(&button_id.to_string());
            }
        }
        output.push_str("\n  target: ");
        output.push_str(&binding.target.to_string());
        output.push('\n');
    }
    if bindings.is_empty() {
        "[]\n".to_owned()
    } else {
        output
    }
}

pub(crate) fn source_boundary() -> &'static str {
    "EKA2L1 stores a selected current-keybind-profile in config.yml and a complete YAML sequence in bindings/<profile>.yml. The SDL2 backend uses the process enumeration index as controller_id and maps standard buttons 0..20 plus virtual axis/trigger codes; probe and recheck SDL indices immediately before launch. Preserve the data/drives save tree, installed device/Z-drive firmware, ROM/media, and unrelated config keys."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_profile_name_and_serializes_guest_keyboard_and_controller_actions() {
        let (config, bindings) = patch_config(
            b"cpu: dynarmic\r\ncurrent-keybind-profile: default\r\nkeep: true\r\n",
            "arcade",
            &[
                KeyBinding {
                    target: 632,
                    source: BindingSource::Keyboard { keycode: 16777220 },
                },
                KeyBinding {
                    target: 633,
                    source: BindingSource::Controller {
                        controller_id: 1,
                        button_id: 11,
                    },
                },
            ],
        )
        .unwrap();
        assert!(config.contains("current-keybind-profile: arcade\r\nkeep: true\r\n"));
        assert!(
            bindings.contains("type: key\n    data:\n      keycode: 16777220\n  target: 632\n")
        );
        assert!(bindings.contains("type: controller\n    data:\n      controller_id: 1\n      button_id: 11\n  target: 633\n"));
    }

    #[test]
    fn rejects_duplicate_targets_invalid_controller_codes_and_path_names() {
        let duplicate = [
            KeyBinding {
                target: 1,
                source: BindingSource::Keyboard { keycode: 1 },
            },
            KeyBinding {
                target: 1,
                source: BindingSource::Keyboard { keycode: 2 },
            },
        ];
        assert!(patch_config(b"", "default", &duplicate).is_err());
        assert!(patch_config(b"", "../escape", &[]).is_err());
        assert!(patch_config(b"", "#yaml-comment", &[]).is_err());
        assert!(
            patch_config(
                b"",
                "default",
                &[KeyBinding {
                    target: 1,
                    source: BindingSource::Controller {
                        controller_id: 0,
                        button_id: 99
                    }
                }]
            )
            .is_err()
        );
    }
}
