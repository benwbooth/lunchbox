//! ColEm 5.6 standalone controller serialization boundary.
//!
//! The exact official `ColEm56-Source.zip` archive shows the Unix port maps
//! keyboard events in `EMULib/Unix/LibUnix.c`; it has no persisted Unix
//! controller mapping file.  Windows physical joystick selection is handled
//! by the closed platform port and is not represented by the portable source.
//! Refuse rather than fabricate a cross-platform config writer.

pub(crate) const OFFICIAL_VERSION: &str = "5.6";
pub(crate) const SOURCE_ARCHIVE_SHA256: &str =
    "11e536e20892b89140c24e3163bbe7b2c032e31535f6df0334768144b1deb351";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "ColEm 5.6 controller mapping is unavailable: the portable source maps Unix keyboard events directly and publishes no persisted controller mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn mapping_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::OFFICIAL_VERSION, "5.6");
        assert_eq!(super::SOURCE_ARCHIVE_SHA256.len(), 64);
    }
}
