//! Hatari native `hatari.cfg` joystick mappings for the SDL UI, not libretro
//! bindings.
//!
//! Functional contract pinned to hatari/hatari
//! 11964da62914bf232ca84eacf0cedf1d25223e08:
//! - `src/configuration.c`: `[Joystick0]`/`[Joystick1]` sections hold
//!   `nJoystickMode` (1 = JOYSTICK_REALSTICK), `nJoyId` (the SDL device
//!   index bound to the emulated port) and `nJoyBut1/2/3Index` (SDL button
//!   indices for the ST fire buttons); `[ROM]` holds `szTosImageFileName`.
//! - `src/sdl/joy_ui.c` `JoyUI_ReadJoystick`: directions are hardcoded to
//!   SDL axis 0 (X) and axis 1 (Y), overridden by hat 0; buttons come from
//!   the configured indices.
//! - `src/joy.c`: digital thresholds are X ≤ -16384 / X ≥ 16383 and
//!   Y ≤ -16384 / Y ≥ 16383 in SDL axis units.
//! - `src/options.c`: `-c <file>` reads additional configuration values;
//!   `src/paths.c` resolves the configuration home from `$HOME`, so a
//!   private HOME keeps the user's own hatari.cfg and saves untouched.
use anyhow::{Context, Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Direction engagement thresholds from joy.c.
pub(crate) const AXIS_NEGATIVE_THRESHOLD: i32 = -16384;
pub(crate) const AXIS_POSITIVE_THRESHOLD: i32 = 16383;

/// Target controls and their native outputs. The ST fire buttons beyond the
/// first are joyport expansions, not the base CX40 stick.
pub(crate) const CONTROLS: [(&str, &str, bool); 7] = [
    ("up", "Axis 1 low / Hat 0 up", true),
    ("down", "Axis 1 high / Hat 0 down", true),
    ("left", "Axis 0 low / Hat 0 left", true),
    ("right", "Axis 0 high / Hat 0 right", true),
    ("fire", "Fire (nJoyBut1Index)", true),
    ("fire2", "Second fire (nJoyBut2Index)", false),
    ("fire3", "Third fire (nJoyBut3Index)", false),
];

/// One direction or fire binding in Hatari's fixed vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    /// SDL axis 0 or 1, negative or positive half.
    AxisHalf { axis: u32, negative: bool },
    /// Hat 0 direction.
    Hat { direction: u8 },
    /// SDL button index for a fire slot.
    FireButton { slot: u32, button: u32 },
}

impl Binding {
    pub(crate) fn valid(&self) -> Result<()> {
        match *self {
            Binding::AxisHalf { axis, .. } => ensure_axis(axis),
            Binding::Hat { direction } => {
                ensure!(
                    matches!(direction, 1 | 2 | 4 | 8),
                    "Hatari hat direction must be a cardinal SDL mask"
                );
                Ok(())
            }
            Binding::FireButton { slot, .. } => {
                ensure!(
                    (1..=3).contains(&slot),
                    "Hatari only configures three fire buttons"
                );
                Ok(())
            }
        }
    }
}

fn ensure_axis(axis: u32) -> Result<()> {
    anyhow::ensure!(
        axis <= 1,
        "Hatari hardcodes directions to SDL axes 0 and 1; axis {axis} cannot drive a direction"
    );
    Ok(())
}

/// Render the private additional configuration. Only joystick and ROM keys
/// are expressed; every other value keeps Hatari's defaults. Each entry is
/// the emulated port (1–2), the bound SDL device index and the fire slots.
pub(crate) fn config(
    players: &[(usize, i32, &[(usize, u32)])],
    tos_image: &std::path::Path,
) -> Result<String> {
    let mut result = String::new();
    for (port, device, fires) in players {
        let (mode, section) = (1, port - 1);
        result.push_str(&format!(
            "[Joystick{section}]\nnJoystickMode = {mode}\nnJoyId = {device}\n"
        ));
        let mut slots = [None; 3];
        for &(slot, button) in *fires {
            slots[slot - 1] = Some(button);
        }
        for (index, slot) in slots.iter().enumerate() {
            // An unset fire slot is explicitly disabled rather than left to
            // inherit whatever the additional file is merged over.
            let value = slot.map_or(-1, |button| i32::try_from(button).unwrap_or(-1));
            result.push_str(&format!("nJoyBut{}Index = {value}\n", index + 1));
        }
        result.push('\n');
    }
    let tos = tos_image
        .to_str()
        .context("Hatari TOS image path is not UTF-8")?;
    ensure!(
        !tos.contains(['\r', '\n']),
        "Hatari TOS path contains a newline"
    );
    result.push_str(&format!("[ROM]\nszTosImageFileName = {tos}\n"));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding_axis(axis: u32, negative: bool) -> Binding {
        Binding::AxisHalf { axis, negative }
    }

    #[test]
    fn non_primary_axes_cannot_drive_directions() {
        assert!(binding_axis(0, true).valid().is_ok());
        assert!(binding_axis(1, false).valid().is_ok());
        assert!(binding_axis(2, true).valid().is_err());
        assert!(Binding::Hat { direction: 1 }.valid().is_ok());
        assert!(Binding::Hat { direction: 3 }.valid().is_err());
        assert!(Binding::FireButton { slot: 3, button: 9 }.valid().is_ok());
        assert!(Binding::FireButton { slot: 4, button: 9 }.valid().is_err());
    }

    #[test]
    fn config_sets_real_stick_mode_devices_fires_and_tos() {
        let players = [(1_usize, 0_i32, &[(1_usize, 5_u32), (2, 7)][..])];
        let text = config(&players, std::path::Path::new("/roms/tos.img")).unwrap();
        assert!(text.contains("[Joystick0]\nnJoystickMode = 1\nnJoyId = 0\n"));
        assert!(text.contains("nJoyBut1Index = 5\n"));
        assert!(text.contains("nJoyBut2Index = 7\n"));
        assert!(text.contains("nJoyBut3Index = -1\n"));
        assert!(text.ends_with("[ROM]\nszTosImageFileName = /roms/tos.img\n"));

        let two = [(2_usize, 1_i32, &[(1_usize, 0_u32)][..])];
        assert!(
            config(&two, std::path::Path::new("/t.img"))
                .unwrap()
                .contains("[Joystick1]\nnJoystickMode = 1\nnJoyId = 1\n")
        );
    }
}
