//! Explicit refusal for XM8 native controller serialization.
//!
//! XM8 persists settings, including joystick-to-key values, in a private
//! binary `setting.bin`; SDL joystick devices are opened in enumeration order
//! and read with fixed axis/button semantics.  The binary is not a stable,
//! source-defined physical-controller profile format.

pub(crate) const SOURCE_COMMIT: &str = "2c4bf025840711a696d7d9ef55f3be3d0f3c84f9";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "XM8 native controller mapping is unavailable: setting.bin is a private binary settings blob, while SDL joystick identity is enumeration-order based and input semantics are hard-coded"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn private_binary_settings_are_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
