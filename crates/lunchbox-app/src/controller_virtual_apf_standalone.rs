//! Explicit refusal for the standalone Virtual APF controller path.
//!
//! The official Virtual APF release is a Windows-only legacy binary.  It
//! exposes keyboard/joystick assignment through dialogs and an opaque INI
//! file, but publishes no stable portable mapping grammar or source-backed
//! serializer.  Do not guess at that format.

pub(crate) const SOURCE_ARCHIVE_SHA256: &str =
    "a2a4b5a2781e3858251deb0aea6d6d07b48099b61870a405539d8a590bd933ee";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Virtual APF controller mapping is unavailable: the standalone Windows binary exposes dialog-owned keyboard/joystick state and an opaque INI format without a portable source-backed serializer"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn opaque_virtual_apf_mapping_fails_closed() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_ARCHIVE_SHA256.len(), 64);
    }
}
