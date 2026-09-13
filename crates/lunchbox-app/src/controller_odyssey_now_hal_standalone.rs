//! Explicit refusal for Odyssey Now: HAL controller serialization.
//!
//! The Unity project has source-defined keyboard, InControl gamepad, and
//! Arduino serial paths, but no portable Lunchbox profile format.  Unity
//! PlayerPrefs keys are application settings, while InControl device slots
//! and serial-port selection are runtime-owned.

pub(crate) const SOURCE_COMMIT: &str = "7eb0c1da4f46728dd22eab189840f8f51f03118b";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Odyssey Now: HAL controller mapping is unavailable: Unity PlayerPrefs and runtime InControl/Arduino device selection do not define a portable stable host-controller profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_device_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
