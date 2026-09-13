//! Explicit refusal for VirtualBuddy standalone-native controller serialization.
//!
//! VirtualBuddy persists Apple Virtualization VM hardware in a VM bundle and
//! does not publish a portable host-gamepad profile grammar. Its keyboard and
//! pointing-device settings are VM hardware choices, not Lunchbox mappings.

pub(crate) const SOURCE_COMMIT: &str = "683739a7e75921f83c492d15ece1ab89aa66e4cf";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "VirtualBuddy controller mapping is unavailable: the pinned source defines VM keyboard/pointing hardware, not a portable host physical-controller profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn virtual_machine_hardware_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
