//! Portable absolute-device records; capture and frontend routing are separate.
use super::absolute_calibration::{AbsoluteProjection, AbsoluteRectangleCalibration};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsoluteAxisBounds {
    /// Scalar evdev ABS code, excluding slot-based multitouch coordinates.
    pub code: u16,
    pub minimum: i32,
    pub maximum: i32,
}

impl AbsoluteAxisBounds {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.code <= 0x28,
            "Absolute scalar calibration does not support multitouch slots"
        );
        ensure!(
            self.minimum < self.maximum,
            "Absolute axis bounds must have positive extent"
        );
        Ok(())
    }
    fn contains(self, value: i32) -> bool {
        (self.minimum..=self.maximum).contains(&value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsoluteDeviceSettings {
    pub event_path: PathBuf,
    pub input_identity: PathBuf,
    pub x_axis: AbsoluteAxisBounds,
    pub y_axis: AbsoluteAxisBounds,
    pub calibration: AbsoluteRectangleCalibration,
}

impl AbsoluteDeviceSettings {
    pub fn validate(&self) -> Result<()> {
        for path in [&self.event_path, &self.input_identity] {
            ensure!(
                path.is_absolute()
                    && path.as_os_str().len() <= 4096
                    && path
                        .components()
                        .all(|part| matches!(part, Component::RootDir | Component::Normal(_))),
                "Absolute device paths must be bounded normalized absolute paths"
            );
        }
        self.x_axis.validate()?;
        self.y_axis.validate()?;
        ensure!(
            self.x_axis.code != self.y_axis.code,
            "Absolute X/Y need distinct physical axes"
        );
        self.calibration.validate()?;
        let (horizontal, vertical) = if self.calibration.swap_xy {
            (self.y_axis, self.x_axis)
        } else {
            (self.x_axis, self.y_axis)
        };
        ensure!(
            horizontal.contains(self.calibration.left)
                && horizontal.contains(self.calibration.right)
                && vertical.contains(self.calibration.top)
                && vertical.contains(self.calibration.bottom),
            "Absolute calibrated edges exceed their recorded physical bounds"
        );
        Ok(())
    }

    /// Revalidate device identity and live axis metadata before using this in
    /// a runtime reader. Out-of-hardware-range sentinel protocols need an
    /// explicit adapter; they are not silently interpreted as offscreen shots.
    pub fn project_raw(&self, raw_x: i32, raw_y: i32) -> Result<AbsoluteProjection> {
        self.validate()?;
        ensure!(
            self.x_axis.contains(raw_x) && self.y_axis.contains(raw_y),
            "Absolute sample exceeds recorded hardware bounds; native sentinel semantics are unresolved"
        );
        self.calibration.project(raw_x, raw_y)
    }

    pub fn project_libretro_raw(
        &self,
        raw_x: i32,
        raw_y: i32,
    ) -> Result<super::absolute_calibration::LibretroAbsoluteProjection> {
        // Preserve the same saved bounds and hardware-sentinel rejection as the
        // unsigned preview; a new coordinate space is not a new input protocol.
        self.project_raw(raw_x, raw_y)?;
        self.calibration.project_libretro(raw_x, raw_y)
    }
}

pub fn validate_devices(devices: &[AbsoluteDeviceSettings]) -> Result<()> {
    ensure!(
        devices.len() <= 16,
        "At most 16 absolute devices may be saved"
    );
    let mut paths = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for device in devices {
        device.validate()?;
        ensure!(
            paths.insert(&device.event_path) && identities.insert(&device.input_identity),
            "Duplicate absolute device path or identity"
        );
    }
    Ok(())
}
