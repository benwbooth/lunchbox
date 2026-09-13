//! Explicit refusal for Visual Pinball standalone controller serialization.
//!
//! Visual Pinball's source exposes device and action mapping semantics, but a
//! mapping depends on runtime SDL device identities and table-specific actions.
//! Lunchbox therefore does not synthesize or overwrite VP configuration.

pub(crate) const SOURCE_COMMIT: &str = "93d93d9653d7fdc295967cb5f30fff33b734e1fa";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Visual Pinball controller mapping is unavailable: runtime device identity and table-specific actions must be resolved by the VP frontend; no deterministic writer is implemented"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_device_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
