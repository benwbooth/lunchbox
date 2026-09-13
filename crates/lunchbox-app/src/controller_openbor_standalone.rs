//! OpenBOR standalone-native controller boundary.
//!
//! Pinned source: `DCurrent/openbor` at
//! `9d81480f8481fbb9e76b0b5f2a5dfa408376761a`.  OpenBOR does persist a
//! packed `s_savedata` structure in `Saves/<pak>.cfg` (and
//! `Saves/default.cfg`), including the player-1 SDL scancodes.  Its size and
//! fields are conditional on the exact build (`SDL`, `ANDROID`, and version),
//! however, so this catalog does not fabricate a binary writer for an
//! arbitrary executable.  The current `clearbuttons` implementation
//! initializes players 2-4 to `CONTROL_NONE`; joystick virtual-code constants
//! exist but are not assigned there.

pub(crate) const SOURCE_COMMIT: &str = "9d81480f8481fbb9e76b0b5f2a5dfa408376761a";
pub(crate) const PROFILE_ID: &str = "openbor:standalone-native-packed-settings";

/// Refuse until the target OpenBOR executable and its packed-settings ABI
/// have been identified.  A raw byte patch could silently corrupt settings
/// or leave the runtime using a different controller mapping.
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "OpenBOR controller mapping is unavailable: its persisted SDL scancodes are a packed, build-conditional s_savedata binary and no target executable ABI has been verified"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn packed_build_conditional_settings_are_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
