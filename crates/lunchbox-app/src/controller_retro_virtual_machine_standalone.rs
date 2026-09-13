//! Explicit refusal for Retro Virtual Machine controller serialization.

pub(crate) const SOURCE_COMMIT: &str = "e2b3cbeda8f96d92947a1a1d004e2548da95db33";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Retro Virtual Machine controller mapping is unavailable: the current v2 desktop distribution is not source-published and the pinned v1 source's HID mapping bytes are not a stable cross-version Lunchbox contract"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn closed_source_current_release_boundary_is_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
