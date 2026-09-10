//! Shared physical-to-SDL2 translation. Emulator control names belong to callers.
use crate::linux_classic::{AxisEndpoints, ClassicMap, Control};
use crate::sdl2::{ControlCounts, Device};
use crate::sdl2_evdev::EvdevMap;
use anyhow::{Context, Result, bail, ensure};

#[derive(Clone, Copy)]
pub enum PhysicalMap<'a> {
    Classic(&'a ClassicMap),
    Evdev(&'a EvdevMap),
}

impl<'a> PhysicalMap<'a> {
    /// Require one explicit backend and measured SDL counts, never infer a
    /// backend from a controller name or interchange joydev/evdev numbering.
    pub fn from_device(device: &'a Device) -> Result<Self> {
        let map = match (&device.linux_classic, &device.linux_evdev) {
            (Some(map), None) => Self::Classic(map),
            (None, Some(map)) => Self::Evdev(map),
            (None, None) => bail!(
                "Selected SDL2 device needs a matching physical backend probe; HID requires its own translation"
            ),
            (Some(_), Some(_)) => bail!("Ambiguous SDL2 physical backend metadata"),
        };
        map.validate_counts(
            device
                .controls
                .as_ref()
                .context("SDL2 control counts have not been inspected")?,
        )?;
        Ok(map)
    }

    pub fn validate_counts(self, counts: &ControlCounts) -> Result<()> {
        let (buttons, axes, hats) = match self {
            Self::Classic(map) => (map.buttons.len(), map.axes.len(), map.hats.len()),
            Self::Evdev(map) => (map.buttons.len(), map.axes.len(), map.hats.len()),
        };
        ensure!(
            buttons == counts.buttons as usize
                && axes == counts.axes as usize
                && hats == counts.hats as usize,
            "Physical input numbering disagrees with the selected SDL2 device"
        );
        Ok(())
    }

    pub fn control(self, encoded: u32) -> Result<Control> {
        match self {
            Self::Classic(map) => map.control(encoded),
            Self::Evdev(map) => map.control(encoded),
        }
    }

    pub fn digital_input(
        self,
        encoded: u32,
        measured: Option<AxisEndpoints>,
    ) -> Result<crate::duckstation::DigitalInput> {
        match self {
            Self::Classic(map) => map.digital_input(encoded, measured),
            Self::Evdev(map) => map.digital_input(encoded, measured),
        }
    }

    pub fn axis_value(self, code: u8, raw: i32) -> Result<i16> {
        match self {
            Self::Classic(map) => map
                .axis_corrections
                .get(&code)
                .context("No kernel correction for the measured axis")?
                .apply(raw),
            Self::Evdev(map) => map.axis_value(code, raw),
        }
    }
}
