//! Explicit refusal for XM7/SDL native controller serialization.
//!
//! The reviewed XM7-for-SDL port stores joystick type, rapid-fire and key
//! codes in `XM7.INI`, but opens SDL joystick devices by enumeration index and
//! hard-codes the axis/button interpretation in the event backend.  That is
//! not a stable physical-device profile grammar.

pub(crate) const SOURCE_COMMIT: &str = "37f83f77d94d7510162952755be40b7e6f5d0d45";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "XM7 native controller mapping is unavailable: XM7.INI stores guest joystick options and key codes, while the SDL port selects physical devices by unstable enumeration index and hard-codes axis/button semantics"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn enumeration_bound_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
