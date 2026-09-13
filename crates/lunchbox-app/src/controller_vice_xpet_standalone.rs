//! Explicit refusal for the standalone VICE PET (`xpet`) controller path.
//!
//! This is deliberately separate from controller_vice_native: xpet's native
//! contract is PET keyboard/virtual-keyboard input, not the two-port joystick
//! mapping written by the generic VICE integration.

pub(crate) const SOURCE_COMMIT: &str = "d322f7a8d6c269b97162c74e73214c58eaad9a71";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "VICE xpet controller mapping is unavailable: xpet uses the PET keyboard/virtual keyboard and its machine-specific keymap state is not the generic VICE joystick .vjm contract"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn pet_keyboard_boundary_does_not_reuse_joystick_writer() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
