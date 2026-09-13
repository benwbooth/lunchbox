//! Explicit refusal for Provenance controller serialization on desktop hosts.

pub(crate) const SOURCE_COMMIT: &str = "93f49b4079df204fcd77f29883d00bb5bde629d4";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Provenance controller mapping is unavailable: the pinned project targets Apple platforms and publishes no desktop controller persistence contract"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn apple_only_boundary_is_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
