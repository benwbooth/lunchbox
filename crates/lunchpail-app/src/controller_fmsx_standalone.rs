//! Explicit refusal for fMSX standalone-native controller serialization.
//!
//! The upstream standalone program accepts joystick type and system-ROM
//! directory through command-line options.  Its Unix SDL frontend maps the
//! first two runtime SDL controllers to MSX ports and has no persisted input
//! profile or physical-device identity.

/// Refuse to invent a persistent fMSX mapping file or device selector.
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "fMSX standalone controller mapping is unavailable: upstream exposes only runtime SDL controller slots and -joy type options, not a persisted native profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_sdl_slot_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
