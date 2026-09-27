//! SDL2 Linux evdev numbering and axis conversion, separate from joydev.
//! Source: SDL 3eba0b6f8a21392f47b1b53a476e7633048de9b1,
//! ConfigJoystick, GuessIfAxesAreDigitalHat and AxisCorrect.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Linux input.h read-only ioctl ABI. No events are read and no device state is
/// written. Flags must be the effective hints from the target SDL2 runtime.
#[cfg(target_os = "linux")]
pub fn read(
    path: &std::path::Path,
    joystick_deadzones: bool,
    hat_deadzones: bool,
    digital_hats: bool,
) -> Result<EvdevMap> {
    use std::os::{fd::AsRawFd, unix::fs::FileTypeExt};
    ensure!(
        path.parent() == Some(std::path::Path::new("/dev/input")),
        "Expected an exact /dev/input/eventN path"
    );
    let index = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("event"))
        .context("Expected an evdev event node")?;
    ensure!(
        !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()),
        "Expected a numeric evdev event node"
    );
    let file = std::fs::File::open(path).context("Opening evdev capabilities read-only")?;
    ensure!(
        file.metadata()?.file_type().is_char_device(),
        "Evdev path is not a character device"
    );
    let mut keys = [0u8; 96];
    let mut axes = [0u8; 8];
    let mut relative = [0u8; 8];
    // EVIOCGBIT(EV_KEY,96), EVIOCGBIT(EV_ABS,8), EVIOCGBIT(EV_REL,8).
    for (request, pointer) in [
        (0x80604521u64, keys.as_mut_ptr()),
        (0x80084523, axes.as_mut_ptr()),
        (0x80084522, relative.as_mut_ptr()),
    ] {
        ensure!(
            unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, pointer) } >= 0,
            "Reading evdev capabilities: {}",
            std::io::Error::last_os_error()
        );
    }
    let bit = |bytes: &[u8], code: usize| bytes[code / 8] & (1 << (code % 8)) != 0;
    let keys = (0..768u16)
        .filter(|code| bit(&keys, *code as usize))
        .collect();
    let mut metadata = BTreeMap::new();
    for code in 0..64u8 {
        if !bit(&axes, code as usize) {
            continue;
        }
        // struct input_absinfo: value, minimum, maximum, fuzz, flat, resolution.
        let mut info = [0i32; 6];
        let request = 0x80184540u64 + u64::from(code);
        ensure!(
            unsafe {
                libc::ioctl(
                    file.as_raw_fd(),
                    request as libc::c_ulong,
                    info.as_mut_ptr(),
                )
            } >= 0,
            "Reading evdev axis metadata: {}",
            std::io::Error::last_os_error()
        );
        metadata.insert(
            code,
            AxisInfo {
                minimum: info[1],
                maximum: info[2],
                fuzz: info[3],
                flat: info[4],
                resolution: info[5],
            },
        );
    }
    EvdevMap::from_capabilities(
        &keys,
        metadata,
        joystick_deadzones,
        hat_deadzones,
        digital_hats,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisInfo {
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvdevMap {
    pub buttons: Vec<u16>,
    pub axes: Vec<u8>,
    pub hats: Vec<u8>,
    pub metadata: BTreeMap<u8, AxisInfo>,
    pub joystick_deadzones: bool,
    pub hat_deadzones: bool,
    pub digital_hats: bool,
}

impl EvdevMap {
    pub fn control(&self, encoded: u32) -> Result<crate::linux_classic::Control> {
        use crate::linux_classic::Control;
        let code = (encoded & 0xffff) as u16;
        match encoded >> 16 {
            1 => self
                .buttons
                .iter()
                .position(|button| *button == code)
                .map(|index| Control::Button(index as u32))
                .context("Physical button is absent from SDL2 evdev numbering"),
            3 => {
                let code = u8::try_from(code).context("Invalid physical evdev axis code")?;
                ensure!(
                    self.metadata.contains_key(&code),
                    "Physical axis has no evdev metadata"
                );
                if let Some(index) = self.hats.iter().position(|base| *base == (code & !1)) {
                    Ok(Control::HatAxis {
                        index: index as u32,
                        horizontal: code & 1 == 0,
                    })
                } else {
                    self.axes
                        .iter()
                        .position(|axis| *axis == code)
                        .map(|index| Control::Axis(index as u32))
                        .context("Physical axis is absent from SDL2 evdev numbering")
                }
            }
            _ => anyhow::bail!("Unsupported evdev physical input type"),
        }
    }

    fn hat_component(&self, axis: u8, raw: i32) -> Result<i8> {
        let info = self
            .metadata
            .get(&axis)
            .context("Missing evdev hat metadata")?;
        ensure!(
            (info.minimum..=info.maximum).contains(&raw),
            "Evdev hat measurement exceeds the captured bounds"
        );
        Ok(
            if raw < 0 && (raw <= info.minimum || !self.hat_deadzones || raw < info.minimum / 3) {
                -1
            } else if raw > 0
                && (raw >= info.maximum || !self.hat_deadzones || raw > info.maximum / 3)
            {
                1
            } else {
                0
            },
        )
    }

    pub fn digital_input(
        &self,
        encoded: u32,
        measured: Option<crate::linux_classic::AxisEndpoints>,
    ) -> Result<crate::duckstation::DigitalInput> {
        use crate::{duckstation::DigitalInput, linux_classic::Control};
        let control = self.control(encoded)?;
        if let Control::Button(index) = control {
            ensure!(
                measured.is_none(),
                "A physical button cannot carry axis measurements"
            );
            return Ok(DigitalInput::Button(index));
        }
        let measured =
            measured.context("Physical evdev gesture needs release and press measurements")?;
        let code = (encoded & 0xffff) as u8;
        match control {
            Control::Axis(index) => {
                let info = self
                    .metadata
                    .get(&code)
                    .context("Missing evdev axis metadata")?;
                ensure!(
                    (info.minimum..=info.maximum).contains(&measured.released)
                        && (info.minimum..=info.maximum).contains(&measured.pressed),
                    "Evdev gesture exceeds current axis bounds"
                );
                let released = self.axis_value(code, measured.released)?;
                let pressed = self.axis_value(code, measured.pressed)?;
                ensure!(
                    released != pressed,
                    "Evdev gesture is lost in SDL2's deadzone"
                );
                Ok(DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                })
            }
            Control::HatAxis { index, horizontal } => {
                ensure!(
                    self.hat_component(code, measured.released)? == 0,
                    "Evdev hat calibration does not return to neutral"
                );
                let direction = match (horizontal, self.hat_component(code, measured.pressed)?) {
                    (true, -1) => 8,
                    (true, 1) => 2,
                    (false, -1) => 1,
                    (false, 1) => 4,
                    _ => anyhow::bail!(
                        "Evdev hat gesture does not cross SDL2's activation threshold"
                    ),
                };
                Ok(DigitalInput::Hat { index, direction })
            }
            Control::Button(_) => unreachable!("buttons returned before axis conversion"),
        }
    }

    pub fn from_capabilities(
        keys: &BTreeSet<u16>,
        metadata: BTreeMap<u8, AxisInfo>,
        joystick_deadzones: bool,
        hat_deadzones: bool,
        digital_hats: bool,
    ) -> Result<Self> {
        ensure!(
            keys.iter().all(|key| *key < 768) && metadata.keys().all(|axis| *axis < 64),
            "Invalid Linux input capability code"
        );
        ensure!(
            metadata.values().all(|axis| axis.minimum <= axis.maximum
                && axis.flat >= 0
                && axis.fuzz >= 0
                && axis.resolution >= 0),
            "Invalid evdev axis metadata"
        );
        // SDL's loops exclude KEY_MAX/ABS_MAX themselves, as in the pinned source.
        let buttons = (288..767u16)
            .chain(0..288)
            .filter(|key| keys.contains(key))
            .collect();
        let mut hats = Vec::new();
        for base in [16u8, 18, 20, 22] {
            let components: Vec<_> = [base, base + 1]
                .iter()
                .filter_map(|axis| metadata.get(axis))
                .collect();
            if !components.is_empty()
                && (digital_hats
                    || components
                        .iter()
                        .all(|axis| axis.minimum == -1 && axis.maximum == 1)
                    || components
                        .iter()
                        .all(|axis| axis.fuzz == 0 && axis.flat == 0 && axis.resolution == 0))
            {
                hats.push(base);
            }
        }
        let axes = (0..63u8)
            .filter(|axis| metadata.contains_key(axis) && !hats.contains(&(axis & !1)))
            .collect();
        Ok(Self {
            buttons,
            axes,
            hats,
            metadata,
            joystick_deadzones,
            hat_deadzones,
            digital_hats,
        })
    }

    pub fn axis_value(&self, physical_axis: u8, raw: i32) -> Result<i16> {
        ensure!(
            self.axes.contains(&physical_axis),
            "Input is not an SDL2 evdev analog axis"
        );
        let info = self
            .metadata
            .get(&physical_axis)
            .context("Missing evdev axis metadata")?;
        let value = if info.minimum == info.maximum {
            raw
        } else if self.joystick_deadzones {
            let sum = info
                .maximum
                .checked_add(info.minimum)
                .context("Evdev axis midpoint overflow")?;
            let flat2 = info
                .flat
                .checked_mul(2)
                .context("Evdev deadzone overflow")?;
            let low = sum
                .checked_sub(flat2)
                .context("Evdev lower deadzone overflow")?;
            let high = sum
                .checked_add(flat2)
                .context("Evdev upper deadzone overflow")?;
            let denominator = info
                .maximum
                .checked_sub(info.minimum)
                .and_then(|range| {
                    info.flat
                        .checked_mul(4)
                        .and_then(|flat| range.checked_sub(flat))
                })
                .context("Evdev axis scale overflow")?;
            let coefficient = if denominator == 0 {
                0
            } else {
                (1i32 << 28) / denominator
            };
            let doubled = raw.checked_mul(2).context("Evdev sample overflow")?;
            if doubled > low && doubled < high {
                return Ok(0);
            }
            doubled
                .checked_sub(if doubled > low { high } else { low })
                .and_then(|value| value.checked_mul(coefficient))
                .context("Evdev correction overflow")?
                >> 13
        } else {
            let range = info
                .maximum
                .checked_sub(info.minimum)
                .context("Evdev axis range overflow")?;
            let offset = raw
                .checked_sub(info.minimum)
                .context("Evdev axis offset overflow")?;
            ((offset as f32 * (65535.0f32 / range as f32)) - 32768.0 + 0.5).floor() as i32
        };
        Ok(value.clamp(-32768, 32767) as i16)
    }
}
