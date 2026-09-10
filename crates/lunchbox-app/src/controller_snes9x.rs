//! Snes9x GTK 1.63 native binding representation.
//! Source: snes9xgit/snes9x 921f9f7b83660eb44ad263022a57a4a029057c37,
//! gtk/src/gtk_binding.{cpp,h}. Separate from libretro and Qt frontends.
use anyhow::{Result, ensure};

pub(crate) mod configuration;
pub(crate) mod isolation;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod physical;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JoystickInput {
    Button(u16),
    Axis {
        index: u16,
        positive: bool,
        threshold_percent: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Binding {
    /// Zero-based native joystick index, not an SDL instance ID.
    pub joystick: u8,
    pub input: JoystickInput,
}

impl Binding {
    /// The native packed representation reserves 4 bits for one-based devices
    /// and encodes axes above button code 511. Reject overflow rather than
    /// allowing one binding to alias another device or a keyboard modifier.
    pub(crate) fn packed(self) -> Result<u32> {
        ensure!(
            self.joystick < 15,
            "Snes9x GTK joystick index exceeds native binding range"
        );
        let (code, threshold) = match self.input {
            JoystickInput::Button(button) => {
                ensure!(
                    button < 512,
                    "Snes9x GTK button overlaps native axis encoding"
                );
                (u32::from(button), 0)
            }
            JoystickInput::Axis {
                index,
                positive,
                threshold_percent,
            } => {
                ensure!(
                    index <= 32511 && (1..=100).contains(&threshold_percent),
                    "Snes9x GTK axis or threshold is outside the native range"
                );
                (
                    512 + 2 * u32::from(index) + u32::from(positive),
                    u32::from(threshold_percent),
                )
            }
        };
        Ok(0x20000000 | ((u32::from(self.joystick) + 1) << 24) | (threshold << 16) | code)
    }

    pub(crate) fn as_setting(self) -> Result<String> {
        self.packed()?;
        let device = self.joystick + 1;
        Ok(match self.input {
            JoystickInput::Button(button) => format!("Joystick {device} Button {button}"),
            JoystickInput::Axis {
                index,
                positive,
                threshold_percent,
            } => format!(
                "Joystick {device} Axis {index} {} {threshold_percent}%",
                if positive { '+' } else { '-' }
            ),
        })
    }

    /// Native equality/mapping ownership ignores threshold bits. Different
    /// thresholds on the same axis direction do not create distinct controls.
    pub(crate) fn routing_key(self) -> Result<u32> {
        Ok(self.packed()? & !0x00ff0000)
    }
}
