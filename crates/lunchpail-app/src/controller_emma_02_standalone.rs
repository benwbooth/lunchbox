//! Explicit refusal for Emma 02 generic controller serialization.
//!
//! Pinned source: etxmato/emma_02
//! `6cef5f299a32206d939afaaabb56d4c5efc6c4fa`.  Emma 02 persists wx keycodes
//! and emulated-machine-specific keypad layout INIs; it does not expose a
//! stable generic host gamepad profile or physical-device identity.

pub(crate) const SOURCE_COMMIT: &str = "6cef5f299a32206d939afaaabb56d4c5efc6c4fa";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Emma 02 generic controller mapping is unavailable: upstream exposes machine-specific keyboard/keypad files, not a stable host gamepad profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn machine_specific_input_is_explicitly_refused() {
        assert!(super::super::controller_emma_02_standalone::refusal().is_err());
    }
}
