//! Physical evdev codes for SDL 3.2.20's Linux classic joystick backend.
//! SDL removes hat axes from the joystick axis sequence and numbers hats in
//! first-seen order. Neither the jsN suffix nor the joydev axis index is an SDL
//! player/axis number. Other SDL backends require different verified adapters.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Public Linux js_corr ABI. Precision is metadata; the driver's value
/// correction uses the type and coefficients. Never write this to the device.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisCorrection {
    pub coefficients: [i32; 8],
    pub precision: i16,
    pub kind: u16,
}

impl AxisCorrection {
    pub fn apply(&self, raw: i32) -> Result<i16> {
        let value = match self.kind {
            0 => raw,
            1 => {
                let [low, high, negative_scale, positive_scale, ..] = self.coefficients;
                ensure!(low <= high, "Unsupported joystick correction interval");
                let (origin, scale) = if raw <= low {
                    (low, negative_scale)
                } else if raw < high {
                    return Ok(0);
                } else {
                    (high, positive_scale)
                };
                // The kernel evaluates signed 32-bit arithmetic. Do not silently
                // use a wider formula where it would disagree with that result.
                raw.checked_sub(origin)
                    .and_then(|v| v.checked_mul(scale))
                    .context("Joystick correction overflows its verified arithmetic")?
                    >> 14
            }
            _ => bail!("Unknown joystick correction type"),
        };
        Ok(value.clamp(-32767, 32767) as i16)
    }
}

/// Values measured in physical evdev units, before kernel joydev correction.
/// Callers validate saved bounds against the current physical device separately.
#[derive(Debug, Clone, Copy)]
pub struct AxisEndpoints {
    pub released: i32,
    pub pressed: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassicMap {
    /// Physical EV_KEY code at each SDL joystick button index.
    pub buttons: Vec<u16>,
    /// Physical EV_ABS code at each SDL joystick axis index (hats excluded).
    pub axes: Vec<u8>,
    /// Physical X-axis EV_ABS code for each SDL hat, in runtime numbering order.
    pub hats: Vec<u8>,
    /// Hat component axes actually present, retaining kernel order.
    pub hat_axes: Vec<u8>,
    /// Keyed by evdev axis code, including hats; never by SDL axis index.
    /// Empty in older snapshots and synthetic numbering-only fixtures.
    #[serde(default)]
    pub axis_corrections: BTreeMap<u8, AxisCorrection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Button(u32),
    Axis(u32),
    HatAxis { index: u32, horizontal: bool },
}

impl ClassicMap {
    pub fn from_joydev(buttons: &[u16], axes: &[u8]) -> Result<Self> {
        ensure!(
            buttons.len() <= 255 && axes.len() <= 64,
            "Invalid joydev counts"
        );
        ensure!(
            buttons.iter().all(|code| *code < 768),
            "Unknown evdev button code"
        );
        ensure!(
            axes.iter().all(|code| *code < 64),
            "Unknown evdev axis code"
        );
        ensure!(
            buttons
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == buttons.len(),
            "Duplicate physical button codes"
        );
        ensure!(
            axes.iter().collect::<std::collections::BTreeSet<_>>().len() == axes.len(),
            "Duplicate physical axis codes"
        );
        let mut result = Self {
            buttons: buttons.to_vec(),
            axes: vec![],
            hats: vec![],
            hat_axes: vec![],
            axis_corrections: BTreeMap::new(),
        };
        for &axis in axes {
            if (16..=23).contains(&axis) {
                result.hat_axes.push(axis);
                let base = axis & !1;
                if !result.hats.contains(&base) {
                    result.hats.push(base);
                }
            } else {
                result.axes.push(axis);
            }
        }
        Ok(result)
    }

    /// Resolve Lunchbox's `(event_type << 16) | code` calibration identity.
    /// Axis rest/pressed values still need calibration/range interpretation.
    pub fn control(&self, encoded: u32) -> Result<Control> {
        let code = (encoded & 0xffff) as u16;
        match encoded >> 16 {
            1 => self
                .buttons
                .iter()
                .position(|c| *c == code)
                .map(|i| Control::Button(i as u32))
                .context("Physical button not present in SDL backend"),
            3 => {
                let code: u8 = code.try_into().context("Invalid physical axis code")?;
                if (16..=23).contains(&code) {
                    ensure!(
                        self.hat_axes.contains(&code),
                        "Physical hat component is not present"
                    );
                    self.hats
                        .iter()
                        .position(|c| *c == (code & !1))
                        .map(|i| Control::HatAxis {
                            index: i as u32,
                            horizontal: code & 1 == 0,
                        })
                        .context("Physical hat not present in SDL backend")
                } else {
                    self.axes
                        .iter()
                        .position(|c| *c == code)
                        .map(|i| Control::Axis(i as u32))
                        .context("Physical axis not present in SDL backend")
                }
            }
            _ => bail!("Unsupported physical event type"),
        }
    }

    pub fn validate_counts(&self, gamepad: &crate::bindings::ResolvedGamepad) -> Result<()> {
        ensure!(
            self.buttons.len() == gamepad.joystick_buttons as usize
                && self.axes.len() == gamepad.joystick_axes as usize
                && self.hats.len() == gamepad.joystick_hats as usize,
            "Physical numbering disagrees with target SDL runtime"
        );
        Ok(())
    }

    /// Physical calibration -> SDL classic joystick input. SDL forwards the
    /// kernel-corrected axis value; hats use its sign, not the evdev raw sign.
    pub fn digital_input(
        &self,
        encoded: u32,
        measured: Option<AxisEndpoints>,
    ) -> Result<crate::duckstation::DigitalInput> {
        use crate::duckstation::DigitalInput;
        let control = self.control(encoded)?;
        if let Control::Button(index) = control {
            ensure!(
                measured.is_none(),
                "A physical button cannot carry an axis measurement"
            );
            return Ok(DigitalInput::Button(index));
        }
        let measured =
            measured.context("This axis needs a saved physical rest/pressed measurement")?;
        let code = (encoded & 0xffff) as u8;
        let correction = self
            .axis_corrections
            .get(&code)
            .context("No verified kernel correction for this axis; refresh the runtime probe")?;
        let released = correction.apply(measured.released)?;
        let pressed = correction.apply(measured.pressed)?;
        ensure!(
            released != pressed,
            "Physical gesture is lost inside the kernel joystick dead zone"
        );
        match control {
            Control::Axis(index) => Ok(DigitalInput::Axis {
                index,
                released,
                pressed,
            }),
            Control::HatAxis { index, horizontal } => {
                ensure!(released == 0, "A calibrated hat must return to its center");
                let direction = match (horizontal, pressed.signum()) {
                    (true, -1) => 8,
                    (true, 1) => 2,
                    (false, -1) => 1,
                    (false, 1) => 4,
                    _ => bail!("Hat did not activate"),
                };
                Ok(DigitalInput::Hat { index, direction })
            }
            Control::Button(_) => unreachable!(),
        }
    }
}

#[cfg(target_os = "linux")]
pub fn read(path: &std::path::Path) -> Result<ClassicMap> {
    use std::os::{fd::AsRawFd, unix::fs::FileTypeExt};
    ensure!(
        path.parent() == Some(std::path::Path::new("/dev/input")),
        "Expected /dev/input/jsN"
    );
    let suffix = path
        .file_name()
        .and_then(|p| p.to_str())
        .and_then(|p| p.strip_prefix("js"))
        .context("Expected a joystick path")?;
    ensure!(
        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()),
        "Expected numeric joystick path"
    );
    let file = std::fs::File::open(path).context("Opening joystick numbering read-only")?;
    ensure!(
        file.metadata()?.file_type().is_char_device(),
        "Joystick path is not a character device"
    );
    let mut buttons = [0u16; 512];
    let mut axes = [0u8; 64];
    let mut button_count = 0u8;
    let mut axis_count = 0u8;
    let mut corrections = [AxisCorrection::default(); 64];
    // Linux joystick.h read-only ioctl ABI. Buffer sizes match request sizes.
    for (request, pointer) in [
        (
            0x80016a12u64,
            (&mut button_count as *mut u8).cast::<libc::c_void>(),
        ),
        (0x80016a11, (&mut axis_count as *mut u8).cast()),
        (0x84006a34, buttons.as_mut_ptr().cast()),
        (0x80406a32, axes.as_mut_ptr().cast()),
        // JSIOCGCORR encodes ONE struct's size, but returns nabs structs.
        // The buffer covers ABS_CNT to match the kernel's copy length.
        (0x80246a22, corrections.as_mut_ptr().cast()),
    ] {
        let result = unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, pointer) };
        ensure!(
            result >= 0,
            "Reading joystick numbering: {}",
            std::io::Error::last_os_error()
        );
    }
    ensure!(
        usize::from(axis_count) <= axes.len(),
        "Invalid joystick axis count"
    );
    let mut result = ClassicMap::from_joydev(
        &buttons[..usize::from(button_count)],
        &axes[..usize::from(axis_count)],
    )?;
    result.axis_corrections = axes[..usize::from(axis_count)]
        .iter()
        .copied()
        .zip(corrections)
        .collect();
    // Re-read both numbering and correction to reject a concurrent jscal/map
    // change instead of combining two different physical coordinate systems.
    let mut verify_axes = [0u8; 64];
    let mut verify_buttons = [0u16; 512];
    let mut verify_corrections = [AxisCorrection::default(); 64];
    for (request, pointer) in [
        (
            0x80406a32u64,
            verify_axes.as_mut_ptr().cast::<libc::c_void>(),
        ),
        (0x84006a34, verify_buttons.as_mut_ptr().cast()),
        (0x80246a22, verify_corrections.as_mut_ptr().cast()),
    ] {
        ensure!(
            unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, pointer) } >= 0,
            "Cannot recheck joystick calibration"
        );
    }
    ensure!(
        axes == verify_axes && buttons == verify_buttons && corrections == verify_corrections,
        "Joystick numbering or correction changed during the probe"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interspersed_hat_axes_do_not_shift_sdl_stick_indices() {
        let map = ClassicMap::from_joydev(&[305, 304, 307], &[0, 19, 18, 1, 16, 17, 5]).unwrap();
        assert_eq!(map.buttons, [305, 304, 307]);
        assert_eq!(map.axes, [0, 1, 5]);
        assert_eq!(map.hats, [18, 16]);
        assert_eq!(map.control((1 << 16) | 304).unwrap(), Control::Button(1));
        assert_eq!(map.control((3 << 16) | 5).unwrap(), Control::Axis(2));
        assert_eq!(
            map.control((3 << 16) | 17).unwrap(),
            Control::HatAxis {
                index: 1,
                horizontal: false
            }
        );
        assert!(map.control((1 << 16) | 300).is_err());
        assert!(map.control((3 << 16) | 511).is_err());
        assert!(map.control((2 << 16) | 1).is_err());
    }
    #[test]
    fn duplicate_and_out_of_range_physical_codes_fail() {
        assert!(ClassicMap::from_joydev(&[304, 304], &[]).is_err());
        assert!(ClassicMap::from_joydev(&[], &[0, 0]).is_err());
        assert!(ClassicMap::from_joydev(&[768], &[]).is_err());
        assert!(ClassicMap::from_joydev(&[], &[64]).is_err());
        let single = ClassicMap::from_joydev(&[], &[16]).unwrap();
        assert!(single.control((3 << 16) | 16).is_ok());
        assert!(single.control((3 << 16) | 17).is_err());
    }

    #[test]
    fn correction_abi_and_signed_piecewise_arithmetic() {
        assert_eq!(std::mem::size_of::<AxisCorrection>(), 36);
        assert_eq!(std::mem::offset_of!(AxisCorrection, precision), 32);
        assert_eq!(std::mem::offset_of!(AxisCorrection, kind), 34);
        let raw = AxisCorrection::default();
        assert_eq!(raw.apply(i32::MIN).unwrap(), -32767);
        assert_eq!(raw.apply(i32::MAX).unwrap(), 32767);
        let correction = AxisCorrection {
            coefficients: [-100, 100, 16384, 32768, 0, 0, 0, 0],
            kind: 1,
            precision: 16,
        };
        for (raw, expected) in [
            (-101, -1),
            (-100, 0),
            (0, 0),
            (100, 0),
            (101, 2),
            (20000, 32767),
        ] {
            assert_eq!(correction.apply(raw).unwrap(), expected);
        }
        let reversed = AxisCorrection {
            coefficients: [128, 128, -4194304, -4194304, 0, 0, 0, 0],
            kind: 1,
            precision: 0,
        };
        assert_eq!(reversed.apply(0).unwrap(), 32767);
        assert_eq!(reversed.apply(255).unwrap(), -32512);
        assert!(AxisCorrection { kind: 42, ..raw }.apply(0).is_err());
        assert!(correction.apply(i32::MAX).is_err());
    }

    #[test]
    fn physical_axis_to_sdl_uses_correction_and_excludes_hats_from_axis_indices() {
        let mut map = ClassicMap::from_joydev(&[304], &[16, 17, 2]).unwrap();
        assert!(
            map.digital_input(
                0x30002,
                Some(AxisEndpoints {
                    released: 0,
                    pressed: 255
                })
            )
            .is_err()
        );
        // Kernel default for a typical 0..255 trigger has its neutral interval
        // near the midpoint: physical rest 0 is negative, NOT SDL zero.
        map.axis_corrections.insert(
            2,
            AxisCorrection {
                coefficients: [127, 127, 4227330, 4227330, 0, 0, 0, 0],
                kind: 1,
                precision: 0,
            },
        );
        assert_eq!(
            map.digital_input(
                0x30002,
                Some(AxisEndpoints {
                    released: 0,
                    pressed: 255
                })
            )
            .unwrap(),
            crate::duckstation::DigitalInput::Axis {
                index: 0,
                released: -32767,
                pressed: 32767
            }
        );
        map.axis_corrections.insert(
            17,
            AxisCorrection {
                coefficients: [0, 0, -16384, -16384, 0, 0, 0, 0],
                kind: 1,
                precision: 0,
            },
        );
        assert_eq!(
            map.digital_input(
                0x30011,
                Some(AxisEndpoints {
                    released: 0,
                    pressed: 1
                })
            )
            .unwrap(),
            crate::duckstation::DigitalInput::Hat {
                index: 0,
                direction: 1
            }
        );
        assert!(
            map.digital_input(
                0x30011,
                Some(AxisEndpoints {
                    released: 1,
                    pressed: 0
                })
            )
            .is_err()
        );
        assert!(map.digital_input(0x30002, None).is_err());
        assert!(
            map.digital_input(
                0x10130,
                Some(AxisEndpoints {
                    released: 0,
                    pressed: 1
                })
            )
            .is_err()
        );
    }
}
