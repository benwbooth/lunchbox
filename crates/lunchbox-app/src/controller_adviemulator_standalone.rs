//! AdViEmulator standalone controller serialization boundary.
//!
//! The official v1.0 SourceForge archives are legacy 32-bit Linux and Win32
//! binaries.  They expose key capture through a Qt options dialog and an
//! `AdViEmulator.ini` file, but the published artifacts do not expose a
//! stable, reviewable key-name grammar.  Do not guess one from binary strings.

pub(crate) const OFFICIAL_VERSION: &str = "1.0";
pub(crate) const LINUX_ARCHIVE_SHA256: &str =
    "fd4d515ae78c6cd524f7300958c98c495f43834d1a3a24c5316a4de22dff25ad";
pub(crate) const WINDOWS_ARCHIVE_SHA256: &str =
    "f6694923cc10358e608e436f6c792d7fc6990e5494fddad5b73acdd79bca38f6";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "AdViEmulator v1.0 controller mapping is unavailable: official archives expose only a legacy Qt options dialog and do not publish a stable serialized key schema"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_archive_boundary_is_explicit() {
        assert!(super::refusal().is_err());
        assert_eq!(super::OFFICIAL_VERSION, "1.0");
        assert_eq!(super::LINUX_ARCHIVE_SHA256.len(), 64);
        assert_eq!(super::WINDOWS_ARCHIVE_SHA256.len(), 64);
    }
}
