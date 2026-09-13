//! Explicit refusal for MasterGear native controller serialization.
//!
//! MasterGear 4.9 is distributed as stripped Windows/Linux binaries; the
//! official site states that source is not publicly available.  Windows has
//! a GUI joystick selector, but no inspectable persistence grammar, so no
//! source-backed writer is safe to implement.

pub(crate) const VERSION: &str = "4.9";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "MasterGear controller mapping is unavailable: the official 4.9 binaries are stripped and do not publish a writable profile format"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn unavailable_profile_is_explicitly_refused() {
        assert!(super::super::controller_mastergear_standalone::refusal().is_err());
    }
}
