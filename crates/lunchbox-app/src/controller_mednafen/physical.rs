//! Raw joydev numbering, including hats as axes; never SDL-remapped indices.
use super::{Input, Polarity};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::linux_classic::{AxisCorrection, AxisEndpoints};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Map {
    pub buttons: Vec<u16>,
    pub axes: Vec<u8>,
    pub corrections: BTreeMap<u8, AxisCorrection>,
}

impl Map {
    #[cfg(target_os = "linux")]
    pub(crate) fn capture(path: &std::path::Path) -> Result<Self> {
        let raw = lunchbox_controller_probe::linux_classic::read_raw(path)?;
        Ok(Self {
            buttons: raw.buttons,
            axes: raw.axes,
            corrections: raw.corrections,
        })
    }

    /// Build complete bindings from a supplied native map and raw evdev state.
    /// State keys use the same (event_type << 16) | code identity as calibration.
    /// The caller owns same-device identity and freshness of all three inputs.
    pub(crate) fn calibrated(
        &self,
        calibration: &crate::controller_catalog::Calibration,
        gamepad: super::profiles::Gamepad,
        native_id: &str,
        released_state: &BTreeMap<u32, i32>,
        threshold_percent: f64,
    ) -> Result<BTreeMap<String, Input>> {
        ensure!(
            calibration.os == "linux",
            "Mednafen native map requires Linux calibration"
        );
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == gamepad.profile_id())
            .context("Mednafen native catalog profile is missing")?;
        let mut bindings = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .context("Mednafen gameplay control has no calibration")?;
            let native = input
                .native
                .as_ref()
                .context("Mednafen requires native physical calibration")?;
            let actual = released_state
                .get(&native.code)
                .context("Mednafen physical released state is absent")?;
            if gamepad == super::profiles::Gamepad::PlayStationDualAnalog
                && super::psx::STICK_PAIRS
                    .iter()
                    .any(|(a, b)| row.output == *a || row.output == *b)
            {
                let axis = input
                    .axis
                    .as_ref()
                    .context("Mednafen analog stick calibration is absent")?;
                let resolved = self.resolve_stick(native.code, axis, *actual)?;
                ensure!(
                    bindings.insert(row.output, resolved).is_none(),
                    "Duplicate Mednafen analog target"
                );
                continue;
            }
            let endpoints = if native.code >> 16 == 3 {
                let axis = input
                    .axis
                    .as_ref()
                    .context("Mednafen axis calibration is absent")?;
                ensure!(
                    *actual == axis.released,
                    "Mednafen axis rest differs from saved calibration"
                );
                Some(AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                })
            } else {
                ensure!(
                    *actual == 0,
                    "Release Mednafen controller buttons before mapping"
                );
                None
            };
            let resolved = self.resolve(native.code, endpoints, threshold_percent)?;
            ensure!(
                bindings.insert(row.output, resolved).is_none(),
                "Duplicate Mednafen gameplay target"
            );
        }
        // Validate completeness and ownership against the supplied native ID.
        gamepad.assignments(native_id, &bindings)?;
        Ok(bindings)
    }

    /// Resolve a proportional stick direction without applying the digital
    /// button threshold. Native joydev correction owns the continuous response.
    pub(crate) fn resolve_stick(
        &self,
        code: u32,
        measurement: &crate::controller_axis::AxisMeasurement,
        actual: i32,
    ) -> Result<Input> {
        measurement.validate()?;
        lunchbox_controller_probe::linux_classic::ClassicMap::from_joydev(
            &self.buttons,
            &self.axes,
        )?;
        ensure!(
            code >> 16 == 3,
            "Mednafen analog sticks require absolute axes"
        );
        let physical = u8::try_from(code & 0xffff)?;
        // Linux ABS_HAT0X through ABS_HAT3Y are discrete directional switches.
        ensure!(
            !(0x10..=0x17).contains(&physical),
            "A hat switch cannot supply a proportional stick"
        );
        ensure!(
            (i64::from(measurement.pressed) - i64::from(measurement.released)).abs() > 2,
            "Mednafen analog stick gesture has no proportional travel"
        );
        let index = self
            .axes
            .iter()
            .position(|value| *value == physical)
            .context("Mednafen analog axis is absent from joydev")?;
        let correction = self
            .corrections
            .get(&physical)
            .context("Mednafen analog axis correction is absent")?;
        let rest = correction.apply(measurement.released)?;
        let current = correction.apply(actual)?;
        let pressed = correction.apply(measurement.pressed)?;
        let low = correction.apply(measurement.minimum)?;
        let high = correction.apply(measurement.maximum)?;
        ensure!(
            (measurement.minimum..=measurement.maximum).contains(&actual)
                && rest == 0
                && current == 0,
            "Mednafen analog stick must be centered in native corrected coordinates"
        );
        ensure!(
            low.min(high) < 0 && low.max(high) > 0 && i32::from(pressed).abs() > 2,
            "Mednafen analog stick needs bipolar corrected travel"
        );
        Ok(Input::Absolute {
            index: index.try_into()?,
            polarity: if pressed > 0 {
                Polarity::Positive
            } else {
                Polarity::Negative
            },
        })
    }

    pub(crate) fn resolve(
        &self,
        code: u32,
        endpoints: Option<AxisEndpoints>,
        threshold_percent: f64,
    ) -> Result<Input> {
        // Validate original joydev maps without using its SDL axis reordering.
        lunchbox_controller_probe::linux_classic::ClassicMap::from_joydev(
            &self.buttons,
            &self.axes,
        )?;
        ensure!(
            threshold_percent.is_finite() && (0.0..=100.0).contains(&threshold_percent),
            "Mednafen axis threshold must be between zero and 100 percent"
        );
        let kind = code >> 16;
        let physical = code & 0xffff;
        if kind == 1 {
            let index = self
                .buttons
                .iter()
                .position(|value| u32::from(*value) == physical)
                .context("Mednafen physical button is absent from joydev")?;
            return Ok(Input::Button(index.try_into()?));
        }
        ensure!(
            kind == 3,
            "Mednafen gamepad requires a physical button or absolute axis"
        );
        let index = self
            .axes
            .iter()
            .position(|value| u32::from(*value) == physical)
            .context("Mednafen physical axis is absent from joydev")?;
        let endpoints = endpoints.context("Mednafen axis needs measured rest/press values")?;
        let correction = self
            .corrections
            .get(&(physical as u8))
            .context("Mednafen joydev correction is not captured")?;
        let released = i32::from(correction.apply(endpoints.released)?);
        let pressed = i32::from(correction.apply(endpoints.pressed)?);
        let positive = pressed > released;
        let polarity = if positive {
            Polarity::Positive
        } else {
            Polarity::Negative
        };
        let magnitude = |value: i32| {
            if positive {
                value.max(0)
            } else {
                (-value).max(0)
            }
        };
        // Joystick.cpp uses a clamped/truncated 15.12 threshold and compares
        // unscaled half-axis magnitude times the binding's unity scale (4096).
        let threshold =
            ((threshold_percent / 100.0 * 32767.0 * 4096.0) as i32).clamp(1, 32767 * 4096);
        ensure!(
            magnitude(released) * 4096 < threshold && magnitude(pressed) * 4096 >= threshold,
            "Mednafen native axis threshold cannot distinguish measured rest and press"
        );
        Ok(Input::Absolute {
            index: index.try_into()?,
            polarity,
        })
    }
}
