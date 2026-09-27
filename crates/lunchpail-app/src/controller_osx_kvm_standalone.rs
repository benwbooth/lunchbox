//! OSX-KVM standalone QEMU controller boundary.
//!
//! Pinned source: `kholia/OSX-KVM` at
//! `4c378a4b5e0b219783683012bec680325eb40719`.  The supplied boot scripts
//! create QEMU USB keyboard/tablet devices and pass an AppleSMC OSK inline;
//! they do not define a persistent host controller profile or device
//! selector.  OpenCore and OVMF are boot media, not a controller grammar.
//! The repository's notes mention QEMU monitor `savevm`, but do not define a
//! repository-owned state-slot or file-path contract.

pub(crate) const SOURCE_COMMIT: &str = "4c378a4b5e0b219783683012bec680325eb40719";
pub(crate) const PROFILE_ID: &str = "osx-kvm:standalone-qemu-fixed-usb-input";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "OSX-KVM controller mapping is unavailable: pinned QEMU scripts expose generic USB keyboard/tablet devices but no persistent host controller profile or stable device selector"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn generic_qemu_input_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
