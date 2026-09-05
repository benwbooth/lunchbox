//! Physical evdev codes for SDL 3.2.20's Linux classic joystick backend.
//! SDL removes hat axes from the joystick axis sequence and numbers hats in
//! first-seen order. Neither the jsN suffix nor the joydev axis index is an SDL
//! player/axis number. Other SDL backends require different verified adapters.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

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
}

#[cfg(target_os = "linux")]
pub(crate) fn read(path: &std::path::Path) -> Result<ClassicMap> {
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
    // Linux joystick.h read-only ioctl ABI. Buffer sizes match request sizes.
    for (request, pointer) in [
        (
            0x80016a12u64,
            (&mut button_count as *mut u8).cast::<libc::c_void>(),
        ),
        (0x80016a11, (&mut axis_count as *mut u8).cast()),
        (0x84006a34, buttons.as_mut_ptr().cast()),
        (0x80406a32, axes.as_mut_ptr().cast()),
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
    ClassicMap::from_joydev(
        &buttons[..usize::from(button_count)],
        &axes[..usize::from(axis_count)],
    )
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
}
