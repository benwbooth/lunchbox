//! Explicit limitation for BGB standalone-native controller serialization.
//!
//! BGB is an official Windows executable (Wine is not a native Linux runtime),
//! and its manual documents the Joypad GUI and numeric controller IDs but no
//! stable portable controller-profile grammar. The official BGB 1.6.6 x64
//! archive was inspected: it contains a generated `bgb.ini`, but no joypad
//! mapping records; the executable is closed and the GUI's persistence format
//! is not authorable from the published artifact. Do not invent keys.

pub(crate) const OFFICIAL_VERSION: &str = "1.6.6";
pub(crate) const ARCHIVE_SHA256: &str =
    "38b97e4496ad85106f59c87a6b0386b7405fbebb3bccc90650279762bd10478c";
pub(crate) const EXECUTABLE_SHA256: &str =
    "f30dbbd104443241dbf9d8cc9d5a08bfbc978778714fb7424701bc2632500947";

/// Refuse until a version-pinned BGB input writer and runtime identity probe
/// are source-backed. Battery/state paths remain user-selected or explicit.
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "BGB standalone controller mapping is unavailable: official BGB 1.6.6 exposes Joypad configuration through its GUI, while the published bgb.ini/archive contains no authorable joypad mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::OFFICIAL_VERSION, "1.6.6");
        assert_eq!(super::ARCHIVE_SHA256.len(), 64);
    }
}
