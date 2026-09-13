//! LinApple native SDL joystick configuration writer.
//!
//! Pinned source: `linappleii/linapple` commit
//! `fa31e11b579edec32dd431c8b400a04e60a21dab`.  LinApple's SDL frontend
//! consumes the `linapple.conf` registry keys `Joystick 0/1`, index, button,
//! and axis values.  This module patches those source-defined keys in a
//! copied configuration; SDL enumeration remains a runtime slot contract and
//! must be probed and rechecked by the launch caller.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "fa31e11b579edec32dd431c8b400a04e60a21dab";
pub(crate) const PROFILE_ID: &str = "linapple:native-sdl-joystick-v1";

/// Values from the source's `joyinfo` table in JoystickFrontend.cpp.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum JoystickMode {
    Disabled = 0,
    HostJoystick = 1,
    KeyboardStandard = 2,
    KeyboardCentering = 3,
    Mouse = 4,
}

impl JoystickMode {
    fn value(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JoystickSelection {
    pub mode: JoystickMode,
    /// SDL enumeration index observed immediately before launch. LinApple
    /// has no persisted GUID/name selector, so this is deliberately not a
    /// stable identity and is only valid with same-launch revalidation.
    pub runtime_sdl_index: Option<u32>,
    pub button_1: u32,
    pub button_2: u32,
    pub axis_x: u32,
    pub axis_y: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InputProfile {
    pub joysticks: [JoystickSelection; 2],
    pub exit_enabled: bool,
    pub exit_buttons: [u32; 2],
}

const FIELDS: [&str; 14] = [
    "Joystick 0",
    "Joystick 1",
    "Joystick 0 Index",
    "Joystick 1 Index",
    "Joystick 0 Button 1",
    "Joystick 0 Button 2",
    "Joystick 1 Button 1",
    "Joystick 0 Axis 0",
    "Joystick 0 Axis 1",
    "Joystick 1 Axis 0",
    "Joystick 1 Axis 1",
    "Joystick Exit Enable",
    "Joystick Exit Button 0",
    "Joystick Exit Button 1",
];

fn values(profile: InputProfile) -> Result<[String; FIELDS.len()]> {
    let mut indices = BTreeSet::new();
    for selection in profile.joysticks {
        if let Some(index) = selection.runtime_sdl_index {
            ensure!(
                indices.insert(index),
                "LinApple SDL joystick index is duplicated"
            );
        } else {
            ensure!(
                !matches!(selection.mode, JoystickMode::HostJoystick),
                "LinApple host joystick mode needs a probed SDL index"
            );
        }
        ensure!(
            selection.button_1 <= 255
                && selection.button_2 <= 255
                && selection.axis_x <= 255
                && selection.axis_y <= 255,
            "LinApple SDL button and axis indices must fit the source uint8-compatible range"
        );
    }
    ensure!(profile.exit_buttons.iter().all(|button| *button <= 255));
    Ok([
        profile.joysticks[0].mode.value().to_string(),
        profile.joysticks[1].mode.value().to_string(),
        profile.joysticks[0]
            .runtime_sdl_index
            .unwrap_or_default()
            .to_string(),
        profile.joysticks[1]
            .runtime_sdl_index
            .unwrap_or_default()
            .to_string(),
        profile.joysticks[0].button_1.to_string(),
        profile.joysticks[0].button_2.to_string(),
        profile.joysticks[1].button_1.to_string(),
        profile.joysticks[0].axis_x.to_string(),
        profile.joysticks[0].axis_y.to_string(),
        profile.joysticks[1].axis_x.to_string(),
        profile.joysticks[1].axis_y.to_string(),
        u32::from(profile.exit_enabled).to_string(),
        profile.exit_buttons[0].to_string(),
        profile.exit_buttons[1].to_string(),
    ])
}

/// Patch only LinApple's source-defined input keys in a complete copied
/// `linapple.conf`. Unknown settings, comments, ordering, and line endings
/// are retained. Missing or duplicate controller keys fail closed rather than
/// relying on the executable's defaults or section fallback behavior.
pub(crate) fn patch_config(baseline: &[u8], profile: InputProfile) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "LinApple config is too large"
    );
    let original = std::str::from_utf8(baseline).context("LinApple config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "LinApple config contains a NUL byte"
    );
    let values = values(profile)?;
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::with_capacity(original.len() + 256);
    let mut seen = [false; FIELDS.len()];

    for line in original.split_inclusive('\n') {
        let (content, ending) = line.strip_suffix('\n').map_or((line, ""), |line| {
            line.strip_suffix('\r')
                .map_or((line, "\n"), |line| (line, "\r\n"))
        });
        let key = content
            .split_once('=')
            .map(|(key, _)| key.trim())
            .and_then(|key| {
                FIELDS
                    .iter()
                    .position(|known| known.eq_ignore_ascii_case(key))
            });
        if let Some(index) = key {
            ensure!(
                !seen[index],
                "LinApple config contains duplicate {} entry",
                FIELDS[index]
            );
            output.push_str(FIELDS[index]);
            output.push_str(" = ");
            output.push_str(&values[index]);
            output.push_str(if ending.is_empty() { newline } else { ending });
            seen[index] = true;
        } else {
            output.push_str(line);
        }
    }
    ensure!(
        seen.iter().all(|present| *present),
        "LinApple config is missing a source-defined controller field"
    );
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "LinApple reads controller modes, SDL enumeration indices, button indices and axis indices from linapple.conf; SDL device identity is runtime enumeration only. Probe and recheck the selected SDL slots immediately before launch, and pass a private copied configuration."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> InputProfile {
        InputProfile {
            joysticks: [
                JoystickSelection {
                    mode: JoystickMode::HostJoystick,
                    runtime_sdl_index: Some(2),
                    button_1: 0,
                    button_2: 1,
                    axis_x: 0,
                    axis_y: 1,
                },
                JoystickSelection {
                    mode: JoystickMode::Disabled,
                    runtime_sdl_index: None,
                    button_1: 0,
                    button_2: 0,
                    axis_x: 0,
                    axis_y: 1,
                },
            ],
            exit_enabled: true,
            exit_buttons: [8, 9],
        }
    }

    fn baseline(newline: &str) -> String {
        let mut text = format!("# keep{newline}[Configuration]{newline}Other = retained{newline}");
        for field in FIELDS {
            text.push_str(&format!("{field} = 0{newline}"));
        }
        text.push_str(&format!("[Preferences]{newline}untouched = yes{newline}"));
        text
    }

    #[test]
    fn patches_source_fields_and_preserves_unknown_config() {
        let output = patch_config(baseline("\r\n").as_bytes(), profile()).unwrap();
        assert!(output.contains("Joystick 0 = 1\r\n"));
        assert!(output.contains("Joystick 0 Index = 2\r\n"));
        assert!(output.contains("Joystick 0 Button 2 = 1\r\n"));
        assert!(output.contains("Joystick Exit Enable = 1\r\n"));
        assert!(output.contains("Other = retained\r\n"));
        assert!(output.contains("untouched = yes\r\n"));
    }

    #[test]
    fn refuses_unprobed_or_duplicate_slots_and_incomplete_baselines() {
        let mut invalid = profile();
        invalid.joysticks[0].runtime_sdl_index = None;
        assert!(patch_config(baseline("\n").as_bytes(), invalid).is_err());

        invalid = profile();
        invalid.joysticks[1].mode = JoystickMode::Mouse;
        invalid.joysticks[1].runtime_sdl_index = Some(2);
        assert!(patch_config(baseline("\n").as_bytes(), invalid).is_err());

        let incomplete = baseline("\n").replace("Joystick 1 Axis 1 = 0\n", "");
        assert!(patch_config(incomplete.as_bytes(), profile()).is_err());
    }
}
