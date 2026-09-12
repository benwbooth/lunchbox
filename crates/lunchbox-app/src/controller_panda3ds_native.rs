//! Panda3DS native-input boundary.
//!
//! Pinned source: wheremyfoodat/Panda3DS commit
//! `5aaa1d26565c834a6f1999026260e559f54aacf1`.
//!
//! The native SDL frontend in the pinned Panda3DS source opens controller 0
//! directly with `SDL_GameControllerOpen(0)` and hard-wires standard buttons
//! and the left stick. `config.toml` contains keyboard mappings, but there is
//! no native gamepad mapping table to author. This module emits the exact
//! keyboard file and refuses only the native gamepad mapping claim.

use anyhow::{Result, bail, ensure};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "panda3ds:standalone-native-unsupported";
pub(crate) const SOURCE_COMMIT: &str = "5aaa1d26565c834a6f1999026260e559f54aacf1";

/// Panda's Qt/SDL frontends do have an authorable keyboard map. This is the
/// exact `InputMappings::serialize` shape used by `controls_qt.toml`; it does
/// not alter the hard-wired SDL GameController 0 path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyboardBinding {
    /// One of HID::Keys::keyToName outputs, for example `A` or `D-Pad Up`.
    pub control: String,
    /// Qt QKeySequence text accepted by the source deserializer, e.g. `L`.
    pub key: String,
}

pub(crate) fn keyboard_mapping_toml(bindings: &[KeyboardBinding]) -> Result<String> {
    ensure!(
        !bindings.is_empty(),
        "Panda3DS needs at least one keyboard binding"
    );
    let known = [
        "A",
        "B",
        "Select",
        "Start",
        "D-Pad Right",
        "D-Pad Left",
        "D-Pad Up",
        "D-Pad Down",
        "R",
        "L",
        "X",
        "Y",
        "ZL",
        "ZR",
        "CirclePad Right",
        "CirclePad Left",
        "CirclePad Up",
        "CirclePad Down",
    ];
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for binding in bindings {
        ensure!(
            known.contains(&binding.control.as_str()),
            "Panda3DS keyboard control is unknown"
        );
        ensure!(
            !binding.key.is_empty()
                && binding.key.len() <= 64
                && !binding.key.contains(['"', '\r', '\n']),
            "Panda3DS keyboard key is invalid"
        );
        grouped
            .entry(binding.control.clone())
            .or_default()
            .push(binding.key.clone());
    }
    let mut out = String::from(
        "[Metadata]\nName = \"Lunchbox\"\nDevice = \"Lunchbox\"\nFrontend = \"Qt\"\n\n[Mappings]\n",
    );
    for (control, keys) in grouped {
        if control
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            out.push_str(&control);
        } else {
            out.push('"');
            out.push_str(&control);
            out.push('"');
        }
        out.push_str(" = [");
        for (index, key) in keys.iter().enumerate() {
            if index != 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&key.replace('\\', "\\\\").replace('"', "\\\""));
            out.push('"');
        }
        out.push_str("]\n");
    }
    Ok(out)
}

pub(crate) fn native_mapping_unavailable() -> Result<()> {
    bail!(
        "Panda3DS native SDL gamepad input is not configurable: upstream opens only SDL gamepad 0 and hard-wires its standard controls; keyboard mappings can be authored in controls_qt.toml"
    )
}

pub(crate) fn launch_contract() -> &'static str {
    "Native Panda3DS may be launched only with a private working directory containing config.toml with General.UsePortableBuild=false; this isolates config while SDL_GetPrefPath keeps saves/app data persistent. No native gamepad mapping is staged."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_path_is_explicitly_unsupported() {
        assert!(native_mapping_unavailable().is_err());
        assert!(launch_contract().contains("No native gamepad mapping"));
        let text = keyboard_mapping_toml(&[KeyboardBinding {
            control: "A".into(),
            key: "L".into(),
        }])
        .unwrap();
        assert!(text.contains("A = [\"L\"]"));
        let text = keyboard_mapping_toml(&[KeyboardBinding {
            control: "D-Pad Up".into(),
            key: "Up".into(),
        }])
        .unwrap();
        assert!(text.contains("\"D-Pad Up\" = [\"Up\"]"));
    }
}
