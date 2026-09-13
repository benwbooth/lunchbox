//! Explicit refusal for MEMU persistent controller serialization.
//!
//! Pinned source: Memotech-Bill/MEMU@86eb58dfaae32ab73d36297e24daf2b8929567c4.
//! MEMU accepts -joy/-joy-buttons at launch, but its config-save routine has
//! the joystick persistence block disabled (#if 0). Therefore an adapter must
//! not claim that writing memu.cfg persists a controller profile.

pub(crate) const SOURCE_COMMIT: &str = "86eb58dfaae32ab73d36297e24daf2b8929567c4";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "MEMU persistent controller mapping is unavailable: -joy-buttons is a launch option and joystick serialization is disabled in config.c"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn disabled_config_persistence_is_explicitly_refused() {
        assert!(super::super::controller_memu_standalone::refusal().is_err());
    }
}
