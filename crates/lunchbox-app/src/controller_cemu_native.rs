//! Source-accurate Cemu standalone controller-profile writer.
//!
//! Pinned source: cemu-project/Cemu commit
//! `3310f3b8b184d64a62b89fd59088c799432badf5`.
//!
//! Cemu's native profile format is XML, and its SDL controller UUID is the
//! occurrence number followed by SDL's lower-case GUID (`0_<guid>` for the
//! first device).  This module deliberately writes only the VPAD mappings;
//! MLC/save data and keys remain in the caller's normal Cemu data root.

use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "cemu:standalone-vpad";

/// Cemu VPAD mapping IDs from `VPADController::ButtonId`.
pub(crate) const CONTROLS: [(&str, u64); 18] = [
    ("a", 1),
    ("b", 2),
    ("x", 3),
    ("y", 4),
    ("l", 5),
    ("r", 6),
    ("zl", 7),
    ("zr", 8),
    ("plus", 9),
    ("minus", 10),
    ("up", 11),
    ("down", 12),
    ("left", 13),
    ("right", 14),
    ("stick_up", 17),
    ("stick_down", 18),
    ("stick_left", 19),
    ("stick_right", 20),
];

/// SDLController button/axis values from Cemu's `Controller.h`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u64),
    Axis { index: u64, positive: bool },
}

impl Binding {
    fn cemu_value(self) -> Result<u64> {
        match self {
            Self::Button(index) => {
                ensure!(index < 32, "Cemu SDL gamepad button is out of range");
                Ok(index)
            }
            Self::Axis { index, positive } => match (index, positive) {
                (0, true) => Ok(38),
                (1, true) => Ok(39),
                (2, true) => Ok(40),
                (3, true) => Ok(41),
                (4, true) => Ok(42),
                (5, true) => Ok(43),
                (0, false) => Ok(44),
                (1, false) => Ok(45),
                (2, false) => Ok(46),
                (3, false) => Ok(47),
                (4, false) => Ok(48),
                (5, false) => Ok(49),
                _ => anyhow::bail!("Cemu SDL gamepad axis is out of range"),
            },
        }
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Render the XML written by Cemu's input manager for one SDLController VPAD.
pub(crate) fn profile_xml(
    guid_index: u32,
    guid: &str,
    display_name: &str,
    mappings: &BTreeMap<String, Binding>,
) -> Result<Vec<u8>> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
        "Cemu SDL GUID must be 32 hex characters"
    );
    ensure!(
        !display_name.is_empty() && !display_name.chars().any(char::is_control),
        "Cemu display name is invalid"
    );
    ensure!(
        mappings.len() == CONTROLS.len(),
        "Cemu needs every VPAD gameplay mapping"
    );
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<emulated_controller>\n  <type>VPAD</type>\n  <controller>\n    <api>SDLController</api>\n    <uuid>",
    );
    out.push_str(&guid_index.to_string());
    out.push('_');
    out.push_str(&guid.to_ascii_lowercase());
    out.push_str("</uuid>\n    <display_name>");
    out.push_str(&xml_escape(display_name));
    // ControllerBase's default settings in the pinned source use zero rumble
    // and a 0.25 deadzone for each axis family.  Keep the generated profile
    // source-shaped without silently enabling haptics.
    out.push_str("</display_name>\n    <rumble>0</rumble>\n    <axis><deadzone>0.25</deadzone><range>1</range></axis>\n    <rotation><deadzone>0.25</deadzone><range>1</range></rotation>\n    <trigger><deadzone>0.25</deadzone><range>1</range></trigger>\n    <mappings>\n");
    let mut used = std::collections::BTreeSet::new();
    for (name, target) in CONTROLS {
        let binding = mappings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Cemu mapping {name} is absent"))?;
        let value = binding.cemu_value()?;
        ensure!(used.insert(value), "Cemu mapping is reused");
        out.push_str(&format!(
            "      <entry><mapping>{target}</mapping><button>{value}</button></entry>\n"
        ));
    }
    out.push_str(
        "    </mappings>\n    <toggle_display>0</toggle_display>\n  </controller>\n</emulated_controller>\n",
    );
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_uuid_and_vpad_entries() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            map.insert(name.to_owned(), Binding::Button(map.len() as u64));
        }
        let xml = String::from_utf8(
            profile_xml(2, "0123456789abcdef0123456789abcdef", "Pad & one", &map).unwrap(),
        )
        .unwrap();
        assert!(xml.contains("<uuid>2_0123456789abcdef0123456789abcdef</uuid>"));
        assert!(xml.contains("Pad &amp; one"));
        assert!(xml.contains("<rumble>0</rumble>"));
        assert_eq!(xml.matches("<entry>").count(), 18);
    }
}
