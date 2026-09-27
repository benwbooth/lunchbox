//! Explicit refusal for Proton controller serialization.

pub(crate) const SOURCE_COMMIT: &str = "5b89db940e0ebe3a137a6009a3589232fe084c09";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Proton controller mapping is unavailable: Proton is a Wine compatibility layer and does not own a stable game-specific host-controller profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn compatibility_layer_boundary_is_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
