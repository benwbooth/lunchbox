//! ZEsarUX native Linux real-joystick configuration.
//!
//! The pinned source parses this file as command-line options.  Unlike the
//! SDL-index-only paths, Linux native input accepts an exact device pathname;
//! the writer consequently requires a stable `/dev/input/by-id/` symlink.

use anyhow::{Result, ensure};

pub(crate) const PROFILE_ID: &str = "zesarux:standalone-native-kempston";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
}

impl Binding {
    fn spec(self) -> String {
        match self {
            Self::Button(index) => index.to_string(),
            Self::Axis { index, positive } => {
                format!("{}{}", if positive { '+' } else { '-' }, index)
            }
        }
    }
}

/// ZX joystick controls consumed by ZEsarUX's `realjoystick` event table.
pub(crate) const CONTROLS: [&str; 5] = ["up", "down", "left", "right", "fire"];

fn valid_device_path(path: &str) -> bool {
    path.starts_with("/dev/input/by-id/")
        && !path.ends_with('/')
        && !path
            .chars()
            .any(|c| c == '"' || c == '\n' || c == '\r' || c.is_control())
}

/// Render an isolated ZEsarUX config file.  It uses `--configfile` at launch;
/// save-state/autosnapshot roots are deliberately caller-owned and are not
/// changed by this mapping-only writer.
pub(crate) fn config_text(
    device_path: &str,
    mappings: &std::collections::BTreeMap<String, Binding>,
) -> Result<String> {
    ensure!(
        valid_device_path(device_path),
        "ZEsarUX requires an exact /dev/input/by-id device path"
    );
    ensure!(
        mappings.len() == CONTROLS.len()
            && mappings.keys().all(|key| CONTROLS.contains(&key.as_str())),
        "ZEsarUX requires exactly the five Kempston controls"
    );
    let mut out = format!("--joystickemulated Kempston\n--realjoystickpath \"{device_path}\"\n");
    let mut used = std::collections::BTreeSet::new();
    for control in CONTROLS {
        let binding = *mappings
            .get(control)
            .ok_or_else(|| anyhow::anyhow!("ZEsarUX control {control} is absent"))?;
        ensure!(used.insert(binding), "ZEsarUX joystick input is reused");
        out.push_str("--joystickevent ");
        out.push_str(&binding.spec());
        out.push(' ');
        out.push_str(control);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_source_option_grammar_for_stable_linux_path() {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "up".into(),
            Binding::Axis {
                index: 1,
                positive: false,
            },
        );
        map.insert(
            "down".into(),
            Binding::Axis {
                index: 1,
                positive: true,
            },
        );
        map.insert(
            "left".into(),
            Binding::Axis {
                index: 0,
                positive: false,
            },
        );
        map.insert(
            "right".into(),
            Binding::Axis {
                index: 0,
                positive: true,
            },
        );
        map.insert("fire".into(), Binding::Button(0));
        let text = config_text("/dev/input/by-id/usb-Test-event-joystick", &map).unwrap();
        assert!(text.contains("--realjoystickpath \"/dev/input/by-id/usb-Test-event-joystick\""));
        assert!(text.contains("--joystickevent +0 right"));
    }

    #[test]
    fn rejects_unstable_or_ambiguous_paths() {
        let map = std::collections::BTreeMap::new();
        assert!(config_text("/dev/input/js0", &map).is_err());
        assert!(config_text("/dev/input/by-id/one\"two", &map).is_err());
    }
}
