//! Explicit refusal for Wine-wide controller serialization.
//!
//! Wine exposes Windows input APIs to each application and stores registry
//! state in a WINEPREFIX.  That is not a portable, title-independent
//! controller profile contract.

pub(crate) const SOURCE_COMMIT: &str = "788d90c4e1d628fab6672623f0c8094b984ea2fa";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Wine controller mapping is unavailable: DirectInput and registry state are application- and device-specific, so Wine does not provide a stable universal profile serializer"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn compatibility_layer_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
