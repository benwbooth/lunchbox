//! Explicit refusal for LoopyMSE controller serialization.
//!
//! Pinned source: PSI-Rockin/LoopyMSE
//! `f6ad0bb9e40eafb0907bcd74223ad5b17c7f66cd`.  The SDL frontend translates
//! a fixed keyboard table (Z/X/A/S/Q/W, arrows, and Enter); it does not expose
//! a persistent controller-map format or physical-device selector.

pub(crate) const SOURCE_COMMIT: &str = "f6ad0bb9e40eafb0907bcd74223ad5b17c7f66cd";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "LoopyMSE controller mapping is unavailable: upstream exposes fixed keyboard bindings, not a persistent gamepad profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixed_keyboard_input_is_explicitly_refused() {
        assert!(super::super::controller_loopymse_standalone::refusal().is_err());
    }
}
