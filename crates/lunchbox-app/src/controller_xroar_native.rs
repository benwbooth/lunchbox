//! XRoar standalone SDL3 controller-profile writer.
//!
//! Pinned source: xroar/xroar commit
//! 0229f97a636c3c80d51fd27e7d145d792f0a8932. `src/joystick.c` prints
//! `joy` profile blocks, while `src/sdl3/joystick_sdl3.c` defines the exact
//! `physical:<device>,<axis>` and `physical:<device>,%<button-mask>` grammar.
//! The device component is a runtime enumeration index and must be measured
//! and rechecked for the exact child process immediately before launch.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "0229f97a636c3c80d51fd27e7d145d792f0a8932";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum GuestPort {
    Right,
    Left,
}

impl GuestPort {
    fn profile_name(self) -> &'static str {
        match self {
            Self::Right => "lunchbox-right",
            Self::Left => "lunchbox-left",
        }
    }

    fn selector(self) -> &'static str {
        match self {
            Self::Right => "joy-right",
            Self::Left => "joy-left",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PortProfile {
    pub port: GuestPort,
    /// SDL3 enumeration index measured for this launch.
    pub runtime_index: u8,
    /// X and Y logical SDL_GamepadAxis values (0 through 5).
    pub axes: [u8; 2],
    /// Two non-empty masks over SDL_GamepadButton values 0 through 26.
    pub button_masks: [u32; 2],
}

fn validate(profile: PortProfile) -> Result<()> {
    ensure!(
        profile.runtime_index <= 31,
        "XRoar SDL runtime index is out of range"
    );
    ensure!(
        profile.axes.iter().all(|axis| *axis <= 5) && profile.axes[0] != profile.axes[1],
        "XRoar axes must be distinct SDL3 axes 0 through 5"
    );
    const VALID_BUTTONS: u32 = (1_u32 << 27) - 1;
    ensure!(
        profile
            .button_masks
            .iter()
            .all(|mask| *mask != 0 && (*mask & !VALID_BUTTONS) == 0),
        "XRoar button masks must select SDL3 buttons 0 through 26"
    );
    Ok(())
}

/// Append source-shaped joystick profile declarations to a copied
/// `xroar.conf`. XRoar processes declarations in order, so these final
/// `lunchbox-right` / `lunchbox-left` profiles and selectors supersede earlier
/// declarations without deleting machine, ROM, tape, disk or snapshot config.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[PortProfile]) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "XRoar config is too large");
    ensure!(
        !profiles.is_empty() && profiles.len() <= 2,
        "XRoar profile count is invalid"
    );
    let original = std::str::from_utf8(baseline).context("XRoar config is not UTF-8")?;
    ensure!(!original.contains('\0'), "XRoar config contains a NUL byte");
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut ports = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut output = original.to_owned();
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    output.push_str(&format!(
        "# Lunchbox measured SDL3 joystick profiles{newline}"
    ));
    for profile in profiles {
        validate(*profile)?;
        ensure!(ports.insert(profile.port), "XRoar guest port is duplicated");
        ensure!(
            indices.insert(profile.runtime_index),
            "XRoar SDL runtime index is duplicated"
        );
        let name = profile.port.profile_name();
        output.push_str(&format!("joy {name}{newline}"));
        output.push_str(&format!(
            "  joy-desc 'Lunchbox measured SDL3 controller'{newline}"
        ));
        for (axis_number, physical_axis) in profile.axes.iter().enumerate() {
            output.push_str(&format!(
                "  joy-axis {axis_number}='physical:{},{}'{newline}",
                profile.runtime_index, physical_axis
            ));
        }
        for (button_number, mask) in profile.button_masks.iter().enumerate() {
            output.push_str(&format!(
                "  joy-button {button_number}='physical:{},%{}'{newline}",
                profile.runtime_index, mask
            ));
        }
        output.push_str(newline);
        output.push_str(&format!("{} {name}{newline}", profile.port.selector()));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_exact_physical_profile_grammar() {
        let output = patch_config(
            b"machine dragon64\nrompath /opt/roms\n",
            &[PortProfile {
                port: GuestPort::Right,
                runtime_index: 2,
                axes: [0, 1],
                button_masks: [(1 << 0) | (1 << 9), 1 << 1],
            }],
        )
        .unwrap();
        assert!(output.contains("machine dragon64\nrompath /opt/roms\n"));
        assert!(output.contains("joy lunchbox-right\n"));
        assert!(output.contains("joy-axis 0='physical:2,0'\n"));
        assert!(output.contains("joy-button 0='physical:2,%513'\n"));
        assert!(output.ends_with("joy-right lunchbox-right\n"));
    }

    #[test]
    fn rejects_duplicate_ports_indices_and_invalid_controls() {
        let base = PortProfile {
            port: GuestPort::Left,
            runtime_index: 0,
            axes: [0, 1],
            button_masks: [1, 2],
        };
        assert!(patch_config(b"", &[base, base]).is_err());
        let invalid = PortProfile {
            port: GuestPort::Right,
            runtime_index: 1,
            axes: [6, 1],
            button_masks: [1, 1 << 27],
        };
        assert!(patch_config(b"", &[invalid]).is_err());
    }
}
