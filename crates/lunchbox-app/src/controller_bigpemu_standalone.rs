//! BigPEmu standalone controller serialization boundary.
//!
//! The official BigPEmu 1.221 Linux archive and manual expose controller
//! configuration through the GUI and `.bigpcfg` properties / `-setcfgprop`,
//! but the published artifact does not define a complete stable controller
//! property schema.  Do not invent property names; use the emulator GUI or
//! a version-specific runtime oracle before adding a writer.

pub(crate) const OFFICIAL_VERSION: &str = "1.221";
pub(crate) const LINUX_ARCHIVE_SHA256: &str =
    "c0ff610f21d5f55c01c404ef991d89f769a5b3b893c8570c033bcef44121ee57";
pub(crate) const LINUX_ARCHIVE_FNV1A64: &str = "C1B241BBFA5135CB";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "BigPEmu 1.221 controller mapping is unavailable: the official archive documents GUI/.bigpcfg configuration and -setcfgprop, but publishes no complete stable controller property schema"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn mapping_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::OFFICIAL_VERSION, "1.221");
        assert_eq!(super::LINUX_ARCHIVE_SHA256.len(), 64);
    }
}
