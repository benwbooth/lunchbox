//! Explicit refusal for NetherSX2 desktop controller serialization.

pub(crate) const SOURCE_COMMIT: &str = "98ddad36d6c367a045b65fd06fdb26ba4c611387";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "NetherSX2 desktop controller mapping is unavailable: pinned upstream is an Android AetherSX2 patch/distribution"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn android_only_nethersx2_is_explicitly_refused() {
        assert!(super::super::controller_nethersx2_standalone::refusal().is_err());
    }
}
