//! Explicit refusal for OdyWeb controller serialization.
//!
//! OdyWeb is a self-contained browser HTML/JavaScript artifact.  Its keyboard
//! state and sliders live in the page; the overlay picker reads a local file
//! into memory.  No persistent controller, save, state, BIOS, or key grammar
//! is provided for a native launcher to write.

pub(crate) const SOURCE_URL: &str =
    "https://prehistoricgaming.com/en/odyweb-the-html-magnavox-odyssey-simulator/";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "OdyWeb controller mapping is unavailable: the browser artifact has no persistent host-controller profile or native configuration writer"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn browser_artifact_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert!(super::SOURCE_URL.contains("odyweb"));
    }
}
