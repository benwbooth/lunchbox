//! Denise standalone-native controller settings writer.
//!
//! Pinned upstream: `piciji/denise@1cb7d45117893c76025d32260edb6008001344f9`.
//! Denise's GUIKIT settings backend persists one `ident:value` record per line
//! in `settings.ini`.  Input mappings use `anded|device-id|group|input|qualifier`
//! records; the udev backend derives `device-id` from the physical device path,
//! vendor and product, rather than from enumeration order.

use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "denise:standalone-native-settings-v1";
pub(crate) const SOURCE_COMMIT: &str = "1cb7d45117893c76025d32260edb6008001344f9";
pub(crate) const SETTINGS_FILE: &str = "settings.ini";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Assignment {
    /// Denise's source-derived physical device id (`Hid::Device::id`).
    pub device_id: u32,
    /// `Hid::Joypad::{Axis,Hat,Trigger,Button}` group id.
    pub group: u32,
    pub input: u32,
    /// `InputMapping::Qualifier` (None/Lo/Hi) numeric value.
    pub qualifier: u32,
}

fn valid_ident(ident: &str) -> bool {
    !ident.is_empty()
        && ident.len() <= 128
        && ident
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

fn mapping_value(assignments: &[Assignment]) -> Result<String> {
    ensure!(
        !assignments.is_empty(),
        "Denise mapping must have an assignment"
    );
    let mut out = String::from("0");
    for assignment in assignments {
        out.push_str(&format!(
            "|{}|{}|{}|{}",
            assignment.device_id, assignment.group, assignment.input, assignment.qualifier
        ));
    }
    Ok(out)
}

/// Render source-shaped `settings.ini` mapping records.  The caller supplies
/// emulator mapping identifiers (for example `joystick_0`) from the same
/// Denise build; unrelated settings and roots must be preserved by the launch
/// layer when this fragment is merged into an existing profile.
pub(crate) fn config_text(mappings: &BTreeMap<String, Vec<Assignment>>) -> Result<String> {
    ensure!(!mappings.is_empty(), "Denise requires at least one mapping");
    let mut out = String::new();
    for (ident, assignments) in mappings {
        ensure!(valid_ident(ident), "invalid Denise setting identifier");
        out.push_str(ident);
        out.push(':');
        out.push_str(&mapping_value(assignments)?);
        out.push_str("\r\n");
    }
    Ok(out)
}

pub(crate) fn source_boundary() -> &'static str {
    "Use the pinned Denise executable and matching settings.ini grammar. Resolve device ids from the same native backend before writing; preserve all unrelated settings, firmware/ROM selections, mounted media, guest-save locations and snapshot/state roots. A rendered fragment is not proof of startup or effective gameplay input."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_source_settings_records() {
        let mappings = BTreeMap::from([(
            "joystick_0".to_string(),
            vec![Assignment {
                device_id: 0x1234,
                group: 3,
                input: 0,
                qualifier: 0,
            }],
        )]);
        let text = config_text(&mappings).unwrap();
        assert_eq!(text, "joystick_0:0|4660|3|0|0\r\n");
        assert_eq!(SETTINGS_FILE, "settings.ini");
        assert!(source_boundary().contains("device ids"));
    }

    #[test]
    fn rejects_untrusted_identifiers_and_empty_bindings() {
        assert!(
            config_text(&BTreeMap::from([(
                "bad:key".into(),
                vec![Assignment {
                    device_id: 1,
                    group: 0,
                    input: 0,
                    qualifier: 0
                }]
            )]))
            .is_err()
        );
        assert!(config_text(&BTreeMap::from([("joystick_0".into(), vec![])])).is_err());
    }
}
