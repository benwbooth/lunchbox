//! Magnavox Odyssey DS standalone-native controller boundary.
//!
//! GameBrew's preserved 2010 artifact is Nintendo DS homebrew. It documents
//! DS buttons (R/L player selection and A ball reset), but no desktop binary,
//! source, or desktop persistence format.

pub(crate) const ARTIFACT_URL: &str = "https://dlhb.gamebrew.org/dshomebrew2/magnavoxodysseyds.rar";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Magnavox Odyssey DS controller mapping is unavailable: the preserved artifact is Nintendo DS homebrew with no source-backed desktop controller or persistence contract"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn ds_artifact_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert!(super::ARTIFACT_URL.ends_with("magnavoxodysseyds.rar"));
    }
}
