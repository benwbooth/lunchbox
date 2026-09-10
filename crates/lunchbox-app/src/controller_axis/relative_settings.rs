//! Portable storage for an explicitly identified relative device. Validation
//! checks saved structure only; the native session must reopen the exact device.
use anyhow::{Result, ensure};
use std::collections::BTreeSet;
use std::path::{Component, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelativeMotionCalibration {
    pub x_percent: u16,
    pub y_percent: u16,
    pub invert_x: bool,
    pub invert_y: bool,
    /// Swap physical X/Y before applying output-axis sensitivity/inversion.
    #[serde(default)]
    pub swap_xy: bool,
}

impl Default for RelativeMotionCalibration {
    fn default() -> Self {
        Self {
            x_percent: 100,
            y_percent: 100,
            invert_x: false,
            invert_y: false,
            swap_xy: false,
        }
    }
}

impl RelativeMotionCalibration {
    pub fn validate(self) -> Result<()> {
        ensure!(
            (1..=1000).contains(&self.x_percent) && (1..=1000).contains(&self.y_percent),
            "Relative mouse sensitivity must be 1–1000 percent per axis"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelativeDeviceSettings {
    pub event_path: PathBuf,
    pub input_identity: PathBuf,
    pub axes: Vec<u16>,
    /// Physical mouse-button code and its output code. A list detects duplicates.
    pub buttons: Vec<(u16, u16)>,
    /// Explicit opt-in: grab the physical source for the session lifetime.
    /// This does not exclude the virtual output from desktop input handling.
    #[serde(default)]
    pub exclusive_source: bool,
    #[serde(default)]
    pub motion: RelativeMotionCalibration,
}

impl RelativeDeviceSettings {
    /// Check an output after the saved axis swap/button remap. This validates
    /// configuration only: it neither opens hardware nor resolves emulator IDs.
    pub fn supports_output(&self, event_type: u16, code: u16) -> Result<bool> {
        self.validate()?;
        Ok(match event_type {
            1 => self.buttons.iter().any(|(_, output)| *output == code),
            2 if matches!(code, 0 | 1 | 6 | 8) => {
                let source = if self.motion.swap_xy && code <= 1 {
                    code ^ 1
                } else {
                    code
                };
                self.axes.contains(&source)
            }
            _ => false,
        })
    }

    pub fn validate(&self) -> Result<()> {
        for path in [&self.event_path, &self.input_identity] {
            ensure!(
                path.is_absolute()
                    && path.as_os_str().len() <= 4096
                    && path
                        .components()
                        .all(|part| matches!(part, Component::RootDir | Component::Normal(_))),
                "Relative device paths must be normalized absolute paths"
            );
        }
        ensure!(
            self.axes.len() <= 4
                && self.buttons.len() <= 8
                && (!self.axes.is_empty() || !self.buttons.is_empty()),
            "Invalid relative device control count"
        );
        let mut axes = BTreeSet::new();
        for axis in &self.axes {
            ensure!(
                matches!(axis, 0 | 1 | 6 | 8) && axes.insert(axis),
                "Invalid or duplicate relative axis"
            );
        }
        let mut sources = BTreeSet::new();
        let mut outputs = BTreeSet::new();
        for (source, output) in &self.buttons {
            ensure!(
                (0x110..=0x117).contains(source)
                    && (0x110..=0x117).contains(output)
                    && sources.insert(source)
                    && outputs.insert(output),
                "Invalid or duplicate relative button mapping"
            );
        }
        self.motion.validate()
    }
}

pub fn validate_devices(devices: &[RelativeDeviceSettings]) -> Result<()> {
    ensure!(
        devices.len() <= 16,
        "At most 16 relative devices may be saved"
    );
    let mut paths = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for device in devices {
        device.validate()?;
        ensure!(
            paths.insert(&device.event_path) && identities.insert(&device.input_identity),
            "Duplicate saved relative device path or identity"
        );
    }
    Ok(())
}
