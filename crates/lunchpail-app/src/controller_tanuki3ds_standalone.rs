//! Explicit refusal for Tanuki3DS gamepad serialization.
//!
//! Pinned source: burhanr13/Tanuki3DS
//! `879a1e788ab90f449ac539f72e69ca6f3ba376d9`. The `[Input]` section of
//! `ctremu.ini` carries keyboard scancodes only (`config.inc`); no gamepad,
//! joystick, or SDL mapping table exists in the pinned source.

pub(crate) const SOURCE_COMMIT: &str = "879a1e788ab90f449ac539f72e69ca6f3ba376d9";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Tanuki3DS gamepad mapping is unavailable: ctremu.ini [Input] carries keyboard scancodes only and no gamepad mapping table exists in the pinned source"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn keyboard_only_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
