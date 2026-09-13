//! Cxbx-Reloaded native input-profile/settings writer.
//!
//! Pinned source: Cxbx-Reloaded/Cxbx-Reloaded commit
//! `585c49a50af1255ab155099e06f24505f9c5a800`.  `Settings.cpp` uses
//! CSimpleIni sections `input-port-N` and `input-profile-N`; `InputManager`
//! binds the stored qualified device name (`API/id/name`) to the Xbox port.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const PROFILE_ID: &str = "cxbx-reloaded:native-input-profile-v1";
pub(crate) const SOURCE_COMMIT: &str = "585c49a50af1255ab155099e06f24505f9c5a800";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortConfig {
    /// Xbox port number, zero through three.
    pub port: u8,
    /// Source XBOX_INPUT_DEVICE enum value; the caller owns guest topology.
    pub device_type: i32,
    /// Runtime-qualified host name, e.g. `XInput/0/Gamepad`.
    pub device_name: String,
    pub profile_name: String,
    pub top_slot_type: i32,
    pub bottom_slot_type: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InputProfile {
    /// Serialized profile section index, not a device index.
    pub index: u32,
    pub device_type: i32,
    pub profile_name: String,
    /// Runtime-qualified host name selected by the GUI.
    pub device_name: String,
    /// Source-defined control names are intentionally caller supplied: Cxbx
    /// has different control topologies for Duke/S, lightgun, and Steel
    /// Battalion devices.
    pub controls: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Config {
    pub ports: Vec<PortConfig>,
    pub profiles: Vec<InputProfile>,
}

fn safe_value(value: &str, field: &str) -> Result<()> {
    ensure!(!value.contains('\0'), "Cxbx {field} contains NUL");
    ensure!(
        !value.contains(['\r', '\n']),
        "Cxbx {field} contains a newline"
    );
    Ok(())
}

fn quoted(value: &str, field: &str) -> Result<String> {
    safe_value(value, field)?;
    ensure!(
        value.len() <= 49 && !value.contains('"'),
        "Cxbx {field} is too long or contains a quote"
    );
    Ok(format!("\"{value}\""))
}

fn supported_port_type(value: i32) -> bool {
    matches!(value, -1 | 0 | 1 | 2 | 6 | 7 | 9 | 10)
}

fn supported_profile_type(value: i32) -> bool {
    matches!(value, 0 | 1 | 2 | 6 | 7)
}

fn sections(config: &Config) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    ensure!(config.ports.len() <= 4, "Cxbx port count exceeds four");
    let mut output = BTreeMap::new();
    let mut ports = BTreeSet::new();
    for port in &config.ports {
        ensure!(port.port < 4, "Cxbx port must be zero through three");
        ensure!(ports.insert(port.port), "Cxbx port is duplicated");
        ensure!(
            supported_port_type(port.device_type),
            "Cxbx port device type is unsupported"
        );
        ensure!(
            matches!(port.top_slot_type, -1 | 4) && matches!(port.bottom_slot_type, -1 | 4),
            "Cxbx slot type is unsupported"
        );
        safe_value(&port.device_name, "device name")?;
        let mut fields = BTreeMap::new();
        fields.insert("Type".to_owned(), port.device_type.to_string());
        fields.insert("DeviceName".to_owned(), port.device_name.clone());
        fields.insert(
            "ProfileName".to_owned(),
            quoted(&port.profile_name, "profile name")?,
        );
        fields.insert("TopSlot".to_owned(), port.top_slot_type.to_string());
        fields.insert("BottomSlot".to_owned(), port.bottom_slot_type.to_string());
        output.insert(format!("input-port-{}", port.port), fields);
    }
    let mut profile_ids = BTreeSet::new();
    for profile in &config.profiles {
        ensure!(
            profile_ids.insert(profile.index),
            "Cxbx profile index is duplicated"
        );
        ensure!(
            supported_profile_type(profile.device_type),
            "Cxbx profile device type is unsupported"
        );
        ensure!(
            !profile.profile_name.is_empty(),
            "Cxbx profile name is empty"
        );
        safe_value(&profile.device_name, "device name")?;
        let mut fields = BTreeMap::new();
        fields.insert("Type".to_owned(), profile.device_type.to_string());
        fields.insert(
            "ProfileName".to_owned(),
            quoted(&profile.profile_name, "profile name")?,
        );
        fields.insert("DeviceName".to_owned(), profile.device_name.clone());
        for (control, binding) in &profile.controls {
            safe_value(control, "control name")?;
            ensure!(
                !control.is_empty() && !control.contains('='),
                "Cxbx control name is invalid"
            );
            safe_value(binding, "control binding")?;
            fields.insert(control.clone(), binding.clone());
        }
        output.insert(format!("input-profile-{}", profile.index), fields);
    }
    Ok(output)
}

/// Patch the source-defined Cxbx settings/profile sections in a copied
/// `settings.ini`. Unknown sections and keys are retained. Host device names
/// are runtime-qualified names and must be obtained from the same Cxbx input
/// enumeration immediately before launch.
pub(crate) fn patch_config(baseline: &[u8], config: &Config) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "Cxbx settings.ini is too large"
    );
    let original = std::str::from_utf8(baseline).context("Cxbx settings.ini is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Cxbx settings.ini contains a NUL byte"
    );
    patch_sections(original, &sections(config)?)
}

fn patch_sections(
    original: &str,
    sections: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut active: Option<String> = None;
    let mut seen = BTreeSet::new();
    let mut closed = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|s| s.split_once(']').map(|(name, _)| name.trim().to_owned()))
        {
            if let Some(section) = active.take() {
                if let Some(fields) = sections.get(&section) {
                    for (key, value) in fields {
                        if !seen.contains(&(section.clone(), key.clone())) {
                            output.push_str(&format!("{key}={value}{newline}"));
                        }
                    }
                    closed.insert(section);
                }
            }
            let matching = sections
                .keys()
                .find(|name| name.eq_ignore_ascii_case(&header))
                .cloned();
            if let Some(section) = &matching {
                ensure!(
                    !closed.contains(section),
                    "Cxbx settings section is duplicated"
                );
            }
            active = matching;
            output.push_str(line);
            continue;
        }
        if let Some(section) = &active {
            let key = line.split_once('=').and_then(|(key, _)| {
                sections[section]
                    .keys()
                    .find(|known| key.trim().eq_ignore_ascii_case(known))
                    .cloned()
            });
            if let Some(key) = key {
                ensure!(
                    seen.insert((section.clone(), key.clone())),
                    "Cxbx setting key is duplicated"
                );
                output.push_str(&format!("{key}={}{newline}", sections[section][&key]));
            } else {
                output.push_str(line);
            }
        } else {
            output.push_str(line);
        }
    }
    if let Some(section) = active {
        if let Some(fields) = sections.get(&section) {
            for (key, value) in fields {
                if !seen.contains(&(section.clone(), key.clone())) {
                    output.push_str(&format!("{key}={value}{newline}"));
                }
            }
            closed.insert(section);
        }
    }
    for (section, fields) in sections {
        if !closed.contains(section) {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str(newline);
            }
            output.push_str(&format!("[{section}]{newline}"));
            for (key, value) in fields {
                output.push_str(&format!("{key}={value}{newline}"));
            }
        }
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "Cxbx-Reloaded persists four Xbox ports and arbitrary per-device profiles in settings.ini. DeviceName is an API/id/name qualified runtime identity, while profile controls are source-defined by guest device topology. Enumerate Cxbx's XInput/SDL/DirectInput devices in the same launch and verify the chosen qualified name before exec. Preserve XBE/media, emulated EmuDisk saves, HLE firmware, and unrelated settings; Cxbx has no documented general save-state file."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_port_and_profile_sections_without_touching_other_values() {
        let config = Config {
            ports: vec![PortConfig {
                port: 0,
                device_type: 1,
                device_name: "XInput/0/Gamepad".into(),
                profile_name: "pad".into(),
                top_slot_type: -1,
                bottom_slot_type: -1,
            }],
            profiles: vec![InputProfile {
                index: 0,
                device_type: 1,
                profile_name: "pad".into(),
                device_name: "XInput/0/Gamepad".into(),
                controls: BTreeMap::from([
                    (String::from("A"), String::from("Button A")),
                    (String::from("D Pad Up"), String::from("Pad N")),
                ]),
            }],
        };
        let output = patch_config(b"[gui]\nkeep=yes\n[input-port-0]\nType=-1\nProfileName=\"old\"\n[input-profile-0]\nA=old\n", &config).unwrap();
        assert!(output.contains("Type=1\n"));
        assert!(output.contains("DeviceName=XInput/0/Gamepad\n"));
        assert!(output.contains("ProfileName=\"pad\"\n"));
        assert!(output.contains("A=Button A\nD Pad Up=Pad N\n"));
        assert!(output.contains("[gui]\nkeep=yes\n"));
    }

    #[test]
    fn rejects_invalid_ports_duplicate_indices_and_newline_values() {
        let mut config = Config {
            ports: vec![PortConfig {
                port: 4,
                device_type: 1,
                device_name: "x".into(),
                profile_name: "p".into(),
                top_slot_type: -1,
                bottom_slot_type: -1,
            }],
            profiles: vec![],
        };
        assert!(patch_config(b"", &config).is_err());
        config.ports[0].port = 0;
        config.ports.push(config.ports[0].clone());
        assert!(patch_config(b"", &config).is_err());
        config.ports.truncate(1);
        config.ports[0].device_name = "x\ny".into();
        assert!(patch_config(b"", &config).is_err());
        config.ports[0].device_name = "x".into();
        config.ports[0].device_type = 3;
        assert!(patch_config(b"", &config).is_err());
    }

    #[test]
    fn rejects_duplicate_owned_sections() {
        let config = Config {
            ports: vec![PortConfig {
                port: 0,
                device_type: 1,
                device_name: "XInput/0/Gamepad".into(),
                profile_name: "pad".into(),
                top_slot_type: -1,
                bottom_slot_type: -1,
            }],
            profiles: vec![],
        };
        assert!(patch_config(b"[input-port-0]\n[input-port-0]\n", &config).is_err());
    }
}
