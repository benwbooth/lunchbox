//! SameBoy SDL v1.0.3 native control arrays, not libretro bindings.
//! Pin: 208ba4afabffab9edde416f2dbb8ae459e34adb8, SDL/configuration.h,
//! SDL/gui.c get_joypad_button/get_joypad_axis.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod configuration;
pub(crate) mod isolation;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod physical;
#[cfg(target_os = "linux")]
pub(crate) mod preferences;
pub(crate) mod routing;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

pub(crate) const CONTROLS: [&str; 8] = ["right", "left", "up", "down", "a", "b", "select", "start"];

pub(crate) struct Bindings {
    pub(crate) buttons: [u8; 32],
    pub(crate) axes: [u8; 2],
}

#[derive(Clone, Copy)]
pub(crate) enum Input {
    Button(u32),
    Axis {
        index: u32,
        released: i16,
        pressed: i16,
    },
    Hat {
        index: u32,
        direction: u8,
    },
}

impl Bindings {
    pub(crate) fn calibrated(
        controls: &BTreeMap<String, Input>,
        button_count: u32,
        axis_count: u32,
        hat_count: u32,
    ) -> Result<Self> {
        ensure!(
            button_count <= 255 && axis_count <= 255,
            "SameBoy cannot reserve a disabled native byte index"
        );
        ensure!(
            controls.len() == 8 && CONTROLS.iter().all(|key| controls.contains_key(*key)),
            "SameBoy requires every Game Boy control"
        );
        let mut result = Self {
            buttons: [255; 32],
            axes: [255; 2],
        };
        let mut buttons = BTreeSet::new();
        let mut directional_axes = [None; 4];
        for (logical, key) in CONTROLS.iter().enumerate() {
            match controls[*key] {
                Input::Button(index) => {
                    ensure!(
                        index < button_count && buttons.insert(index),
                        "SameBoy button is unavailable or shared"
                    );
                    result.buttons[logical] = index.try_into()?;
                }
                Input::Axis {
                    index,
                    released,
                    pressed,
                } => {
                    ensure!(
                        logical < 4 && index < axis_count,
                        "SameBoy axes can only map directional pairs"
                    );
                    let positive = matches!(logical, 0 | 3); // right/down
                    ensure!(
                        (-0x3800..0x3800).contains(&released)
                            && released != -0x3800
                            && if positive {
                                pressed > 0x4000
                            } else {
                                pressed < -0x4000
                            },
                        "SameBoy fixed hysteresis/polarity cannot represent measured axis"
                    );
                    directional_axes[logical] = Some(index);
                }
                Input::Hat { index, direction } => {
                    ensure!(
                        logical < 4 && index < hat_count && direction == [2, 8, 1, 4][logical],
                        "SameBoy hats have fixed cardinal gameplay directions"
                    );
                    // All hat events are accepted natively, without an index
                    // filter. No configurable per-hat byte exists to write.
                }
            }
        }
        for (axis, first, second) in [(0, 0, 1), (1, 2, 3)] {
            ensure!(
                directional_axes[first] == directional_axes[second],
                "SameBoy requires opposite directions on the same native axis"
            );
            if let Some(index) = directional_axes[first] {
                result.axes[axis] = index.try_into()?;
            }
        }
        ensure!(
            result.axes[0] == 255 || result.axes[1] == 255 || result.axes[0] != result.axes[1],
            "SameBoy horizontal and vertical axes must be distinct"
        );
        Ok(result)
    }

    /// Native byte arrays have no tagged disabled value: reserve 255 only
    /// after proving no physical button/axis can generate that index.
    /// Hats are hardwired by the frontend and are not disabled by these arrays.
    pub(crate) fn buttons(
        controls: &BTreeMap<String, u32>,
        button_count: u32,
        axis_count: u32,
    ) -> Result<Self> {
        ensure!(
            button_count <= 255 && axis_count <= 255,
            "SameBoy needs an unused byte index to disable inherited bindings"
        );
        ensure!(
            controls.len() == CONTROLS.len()
                && CONTROLS.iter().all(|key| controls.contains_key(*key)),
            "SameBoy needs all eight standard Game Boy controls"
        );
        let mut owners = BTreeSet::new();
        let mut buttons = [255; 32];
        for (index, control) in CONTROLS.iter().enumerate() {
            let physical = controls[*control];
            ensure!(
                physical < button_count && owners.insert(physical),
                "SameBoy button is unavailable or has competing gameplay owners"
            );
            buttons[index] = physical.try_into()?;
        }
        // Clear native menu/turbo/rewind/slow-motion/hotkey/rapid-fire slots.
        Ok(Self {
            buttons,
            axes: [255; 2],
        })
    }
}
