//! Explicit refusal for Devector standalone-native controller serialization.
//!
//! Pinned source: parallelno/Devector
//! `ba0790a4a4a2b7300d6eb4869d11caf2b7047122`.  The source has a hard-coded
//! SDL keyboard table and initializes gamepad support for ImGui navigation,
//! but no emulated gamepad mapping or persistent device selector.

pub(crate) const SOURCE_COMMIT: &str = "ba0790a4a4a2b7300d6eb4869d11caf2b7047122";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Devector standalone controller mapping is unavailable: upstream has no persistent controller profile or emulated gamepad binding schema"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_native_profile_is_explicitly_refused() {
        assert!(super::super::controller_devector_standalone::refusal().is_err());
    }
}
