//! macOS-on-Hyper-V controller boundary.
//!
//! Pinned source: `balopez83/macOS_On_Hyper-V`
//! `cacf043c6b362c621037d44a85ad15feca9ef7aa`. This is an archived Windows
//! Hyper-V/OpenCore VM support project, not a game emulator. Hyper-V input is
//! configured by VM-manager devices and the project publishes no portable
//! gamepad mapping format.

pub(crate) const SOURCE_COMMIT: &str = "cacf043c6b362c621037d44a85ad15feca9ef7aa";
pub(crate) const EFI_CONFIG_SHA256: &str =
    "b12badfafb109c1438a6080913671d9c2f60e018868db24ad1b707b8669e33fd";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "macOS on Hyper-V controller mapping is unavailable: the project configures Hyper-V VM devices and OpenCore EFI files, not a portable emulator controller profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn vm_support_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
        assert_eq!(super::EFI_CONFIG_SHA256.len(), 64);
    }
}
