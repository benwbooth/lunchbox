//! VICE xvic (VIC-20) controller boundary.
//!
//! xvic is a native VICE machine with one emulated joystick port.  Its SDL
//! joystick map is the same `.vjm` grammar as the generic native VICE path;
//! keep this module as a machine-specific adapter so xvic cannot be confused
//! with xpet or with the libretro `vice_xvic` core.

use anyhow::{Result, ensure};

pub(crate) const SOURCE_COMMIT: &str = "d322f7a8d6c269b97162c74e73214c58eaad9a71";
pub(crate) const MACHINE: &str = "xvic";
pub(crate) const JOYSTICK_PORTS: usize = 1;

pub(crate) use crate::controller_vice_native::{
    Binding, DIGITAL_THRESHOLD, PINS, RELEASE_MARGIN, joymap, pin, required_controls,
};

/// Render the private VICE resource file for xvic's single joystick port.
/// The actual resource grammar is owned by the shared native VICE writer.
pub(crate) fn config(devices: &[u32]) -> Result<String> {
    ensure!(
        devices.len() == JOYSTICK_PORTS,
        "VICE xvic exposes exactly one native joystick port"
    );
    Ok(crate::controller_vice_native::config(devices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xvic_reuses_native_vice_grammar_but_limits_the_machine_to_one_port() {
        assert_eq!(MACHINE, "xvic");
        assert_eq!(JOYSTICK_PORTS, 1);
        assert_eq!(config(&[0]).unwrap(), "JoyDevice1=4\n");
        assert!(config(&[]).is_err());
        assert!(config(&[0, 1]).is_err());
        assert_eq!(
            Binding::Button(2).line(0, pin("fire").unwrap()),
            "0 1 2 1 16"
        );
        assert_eq!(SOURCE_COMMIT.len(), 40);
    }
}
