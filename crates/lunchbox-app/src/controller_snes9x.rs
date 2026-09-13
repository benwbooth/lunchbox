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

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle_pad(player: u8, joystick: u8) -> configuration::Pad {
        configuration::Pad {
            player,
            bindings: configuration::CONTROLS
                .iter()
                .enumerate()
                .map(|(button, name)| {
                    (
                        (*name).to_owned(),
                        Binding {
                            joystick,
                            input: JoystickInput::Button(button as u16),
                        },
                    )
                })
                .collect(),
        }
    }

    /// Opt-in bridge from the production renderer to the external runtime
    /// oracle. Both paths must be under the system temporary directory so this
    /// cannot be pointed at the user's native or Flatpak Snes9x profile.
    #[test]
    #[ignore = "writes an isolated two-player Snes9x runtime-oracle config"]
    fn write_isolated_runtime_oracle_config() -> Result<()> {
        use anyhow::Context;
        use std::path::PathBuf;

        let source = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_ORACLE_SOURCE_CONFIG")
                .context("Missing isolated Snes9x source config")?,
        );
        let output = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_ORACLE_OUTPUT_CONFIG")
                .context("Missing isolated Snes9x output config")?,
        );
        let temporary = std::env::temp_dir().canonicalize()?;
        let source = source.canonicalize()?;
        let output_parent = output
            .parent()
            .context("Missing isolated Snes9x output parent")?
            .canonicalize()?;
        ensure!(
            source.starts_with(&temporary)
                && output_parent.starts_with(&temporary)
                && output != source,
            "Snes9x runtime oracle may only write a distinct temporary config"
        );
        let joysticks = std::env::var("LUNCHBOX_SNES9X_ORACLE_JOYSTICKS")
            .context("Missing one-based Snes9x oracle joystick numbers")?
            .split(',')
            .map(str::parse::<u8>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            joysticks.len() == 2
                && joysticks[0] != joysticks[1]
                && joysticks.iter().all(|value| (1..=15).contains(value)),
            "Snes9x runtime oracle needs two distinct one-based joystick numbers"
        );
        let original = std::fs::read_to_string(&source)?;
        let rendered = configuration::render(
            &original,
            &[
                oracle_pad(1, joysticks[0] - 1),
                oracle_pad(2, joysticks[1] - 1),
            ],
        )?;
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)?
            .write_all(rendered.as_bytes())?;
        ensure!(
            std::fs::read_to_string(source)? == original,
            "Snes9x runtime oracle changed its isolated source config"
        );
        let newline = if original.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        for (player, joystick) in [(0, joysticks[0]), (1, joysticks[1])] {
            let mut expected = format!("[Joypad {player}]{newline}");
            for (button, control) in configuration::CONTROLS.iter().enumerate() {
                expected.push_str(&format!(
                    "{control} = Joystick {joystick} Button {button}{newline}"
                ));
            }
            ensure!(
                rendered.contains(&expected),
                "Snes9x runtime-oracle output omitted an ordered player binding bank"
            );
        }
        Ok(())
    }

    #[test]
    fn native_setting_and_packed_axis_are_one_based_and_signed() {
        let positive = Binding {
            joystick: 0,
            input: JoystickInput::Axis {
                index: 3,
                positive: true,
                threshold_percent: 50,
            },
        };
        assert_eq!(positive.as_setting().unwrap(), "Joystick 1 Axis 3 + 50%");
        assert_eq!(positive.packed().unwrap(), 0x21320207);
        let negative = Binding {
            joystick: 0,
            input: JoystickInput::Axis {
                index: 3,
                positive: false,
                threshold_percent: 50,
            },
        };
        assert_ne!(
            positive.routing_key().unwrap(),
            negative.routing_key().unwrap()
        );
        let same_direction = Binding {
            joystick: 0,
            input: JoystickInput::Axis {
                index: 3,
                positive: true,
                threshold_percent: 80,
            },
        };
        assert_eq!(
            positive.routing_key().unwrap(),
            same_direction.routing_key().unwrap()
        );
    }

    #[test]
    fn native_binding_bounds_reject_aliasing_values() {
        assert!(
            Binding {
                joystick: 15,
                input: JoystickInput::Button(0)
            }
            .packed()
            .is_err()
        );
        assert!(
            Binding {
                joystick: 0,
                input: JoystickInput::Button(512)
            }
            .packed()
            .is_err()
        );
        assert!(
            Binding {
                joystick: 0,
                input: JoystickInput::Axis {
                    index: 0,
                    positive: true,
                    threshold_percent: 0
                }
            }
            .packed()
            .is_err()
        );
    }
}
