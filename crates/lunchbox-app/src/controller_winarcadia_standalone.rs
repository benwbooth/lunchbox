//! Explicit refusal for WinArcadia standalone-native controller serialization.
//!
//! WinArcadia's Windows source uses DirectInput device enumeration, guest
//! port assignment, and an interactive game-specific button rearrangement.
//! The source archive does not by itself provide a validated Lunchbox writer
//! contract for mutating those settings across installed builds.

pub(crate) const SOURCE_ARCHIVE_SHA256: &str =
    "061544e10b182ad06e0e084ecd1460303917e52f4877fee9ca4fb4ab221565f8";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "WinArcadia controller mapping is unavailable: DirectInput identity, guest-port assignment and game-specific button rearrangement require runtime probing and a verified serializer"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn directinput_mapping_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_ARCHIVE_SHA256.len(), 64);
    }
}
