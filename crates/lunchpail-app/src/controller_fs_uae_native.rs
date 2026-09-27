//! FS-UAE native input-mapping writer.
//!
//! The pinned upstream source and launcher define `device_event = action`.
//! Device names are normalized by the Python frontend, while universal
//! `controller_*` events avoid physical button-number guesses. This module
//! renders that grammar only; launch code must still probe native SDL input.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "f362278ccd4c60991caac3b4d240d4a3f751bea2";
pub(crate) const PROFILE_ID: &str = "fs-uae:standalone-native-input-v1";

pub(crate) const CONTROLS: [&str; 24] = [
    "dpad_left",
    "dpad_right",
    "dpad_up",
    "dpad_down",
    "lstick_left",
    "lstick_right",
    "lstick_up",
    "lstick_down",
    "lstick_button",
    "rstick_left",
    "rstick_right",
    "rstick_up",
    "rstick_down",
    "rstick_button",
    "south_button",
    "west_button",
    "north_button",
    "east_button",
    "start_button",
    "select_button",
    "left_shoulder",
    "right_shoulder",
    "left_trigger",
    "right_trigger",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DeviceSelector {
    AnyController,
    /// `occurrence` is one-based; `None` means the unsuffixed first device.
    Named {
        normalized_name: String,
        occurrence: Option<u8>,
    },
    JoystickIndex(u8),
    /// `port` is zero-based and rendered as joystick_port_(port + 1).
    PortController(u8),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Mapping {
    pub selector: DeviceSelector,
    /// A universal event, or a measured raw event such as `button_3`.
    pub event: String,
    /// FS-UAE action, for example `action_joy_1_fire_button`.
    pub action: String,
}

fn safe_token(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 256,
        "FS-UAE {what} is empty or too long"
    );
    ensure!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte)),
        "FS-UAE {what} contains a config delimiter"
    );
    Ok(())
}

/// Apply the Python launcher normalization documented by FS-UAE.
pub(crate) fn normalize_device_name(name: &str) -> Result<String> {
    ensure!(
        !name.is_empty() && name.len() <= 256,
        "FS-UAE device name is empty or too long"
    );
    let mut out = String::new();
    let mut underscore = false;
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() {
            out.push(byte.to_ascii_lowercase() as char);
            underscore = false;
        } else if !underscore {
            out.push('_');
            underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    ensure!(
        !out.is_empty(),
        "FS-UAE device name has no usable characters"
    );
    Ok(out)
}

impl DeviceSelector {
    fn prefix(&self) -> Result<String> {
        match self {
            Self::AnyController => Ok("controller".into()),
            Self::JoystickIndex(index) => {
                ensure!(*index < 20, "FS-UAE joystick index must be 0..19");
                Ok(format!("joystick_{index}"))
            }
            Self::PortController(port) => {
                ensure!((0..=3).contains(port), "FS-UAE joystick port must be 0..3");
                Ok(format!("joystick_port_{}_controller", port + 1))
            }
            Self::Named {
                normalized_name,
                occurrence,
            } => {
                let normalized = normalize_device_name(normalized_name)?;
                match occurrence {
                    None => Ok(normalized),
                    Some(value) => {
                        ensure!(
                            (1..=255).contains(value),
                            "FS-UAE duplicate occurrence must be 1..255"
                        );
                        Ok(format!("{normalized}_{value}"))
                    }
                }
            }
        }
    }
}

fn event_name(selector: &DeviceSelector, event: &str) -> Result<String> {
    safe_token(event, "event")?;
    let universal = CONTROLS.contains(&event);
    match selector {
        DeviceSelector::AnyController | DeviceSelector::PortController(_) => ensure!(
            universal,
            "FS-UAE controller selectors require a universal event"
        ),
        _ if universal => {}
        _ => {
            let (kind, rest) = event.split_once('_').ok_or_else(|| {
                anyhow::anyhow!("FS-UAE event is not a documented joystick event")
            })?;
            let (index, suffix) = rest
                .split_once('_')
                .map_or((rest, None), |(index, suffix)| (index, Some(suffix)));
            let index: u16 = index
                .parse()
                .map_err(|_| anyhow::anyhow!("FS-UAE raw event index is invalid"))?;
            ensure!(index < 256, "FS-UAE raw event index is out of range");
            match (kind, suffix) {
                ("button", None) => {}
                ("axis", Some("neg" | "pos")) => {}
                ("hat", Some("up" | "down" | "left" | "right")) => {}
                _ => anyhow::bail!("FS-UAE event is not a documented raw joystick event"),
            }
        }
    }
    Ok(format!("{}_{}", selector.prefix()?, event))
}

fn valid_action(action: &str) -> Result<()> {
    safe_token(action, "action")?;
    ensure!(
        action.starts_with("action_"),
        "FS-UAE action must start with action_"
    );
    Ok(())
}

/// Render a mapping-only fragment. Missing controls are intentionally not
/// invented; the caller supplies measured physical events and source actions.
pub(crate) fn config_text(mappings: &[Mapping]) -> Result<String> {
    ensure!(
        !mappings.is_empty() && mappings.len() <= 128,
        "FS-UAE mapping count is invalid"
    );
    let mut seen = BTreeSet::new();
    let mut output = String::new();
    for mapping in mappings {
        let event = event_name(&mapping.selector, &mapping.event)?;
        valid_action(&mapping.action)?;
        ensure!(
            seen.insert(event.clone()),
            "FS-UAE event is mapped more than once"
        );
        output.push_str(&event);
        output.push_str(" = ");
        output.push_str(&mapping.action);
        output.push('\n');
    }
    Ok(output)
}

/// Patch only joystick/controller mapping lines in a copied option file.
pub(crate) fn patch_config(baseline: &[u8], mappings: &[Mapping]) -> Result<String> {
    ensure!(
        baseline.len() <= 8 * 1024 * 1024,
        "FS-UAE config is too large"
    );
    let fragment = config_text(mappings)?;
    let text = std::str::from_utf8(baseline).context("FS-UAE config is not UTF-8")?;
    let mut output = String::new();
    for line in text.lines() {
        let key = line.split_once('=').map(|(key, _)| key.trim());
        if key.is_some_and(|key| key.contains("controller_") || key.contains("joystick_")) {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    output.push_str(&fragment);
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "Use the pinned FS-UAE native executable plus matching Python launcher; probe SDL identity and controller layout immediately before launch. Preserve the user's FS-UAE options, Controllers directory, mounted Amiga media, Kickstarts, Save States and saveimage roots. A rendered fragment is not proof of startup or effective gameplay input."
}

#[cfg(test)]
mod tests {
    use super::*;
    fn map(selector: DeviceSelector, event: &str, action: &str) -> Mapping {
        Mapping {
            selector,
            event: event.into(),
            action: action.into(),
        }
    }

    #[test]
    fn normalizes_names_and_renders_universal_mapping() {
        assert_eq!(
            normalize_device_name("Controller (Xbox 360 Wireless Receiver)").unwrap(),
            "controller_xbox_360_wireless_receiver"
        );
        let text = config_text(&[map(
            DeviceSelector::AnyController,
            "south_button",
            "action_joy_1_fire_button",
        )])
        .unwrap();
        assert_eq!(text, "controller_south_button = action_joy_1_fire_button\n");
    }

    #[test]
    fn supports_exact_name_duplicate_and_port_selectors() {
        let text = config_text(&[
            map(
                DeviceSelector::Named {
                    normalized_name: "Pad One".into(),
                    occurrence: Some(2),
                },
                "button_0",
                "action_joy_1_fire_button",
            ),
            map(
                DeviceSelector::PortController(0),
                "dpad_left",
                "action_key_cursor_left",
            ),
        ])
        .unwrap();
        assert!(text.contains("pad_one_2_button_0 = action_joy_1_fire_button"));
        assert!(text.contains("joystick_port_1_controller_dpad_left = action_key_cursor_left"));
    }

    #[test]
    fn rejects_unsafe_or_duplicate_entries_and_preserves_options() {
        assert!(config_text(&[map(DeviceSelector::AnyController, "button_0", "joy_1")]).is_err());
        assert!(
            config_text(&[map(
                DeviceSelector::JoystickIndex(20),
                "button_0",
                "action_a"
            )])
            .is_err()
        );
        assert!(
            config_text(&[map(
                DeviceSelector::JoystickIndex(0),
                "button_bad",
                "action_a"
            )])
            .is_err()
        );
        assert!(
            config_text(&[
                map(DeviceSelector::AnyController, "south_button", "action_a"),
                map(DeviceSelector::AnyController, "south_button", "action_b")
            ])
            .is_err()
        );
        let patched = patch_config(
            b"kickstart_file=Kickstart.rom\ncontroller_south_button=old\n",
            &[map(
                DeviceSelector::AnyController,
                "south_button",
                "action_new",
            )],
        )
        .unwrap();
        assert!(patched.contains("kickstart_file=Kickstart.rom"));
        assert!(!patched.contains("controller_south_button=old"));
        assert!(patched.contains("controller_south_button = action_new"));
    }
}
