//! Jynx standalone/native-controller boundary.
//!
//! Pinned upstream source is jonathan-markland/Jynx at
//! a02560be8a9b6a13b3a5a33de7e032d8f86dc658.  Linux and Windows frontends
//! map fixed source-defined keyboard matrices.  JynxEmulatorSettings.config
//! stores emulation/display settings, not host key bindings, so no guessed
//! writer is provided.

pub(crate) const SOURCE_COMMIT: &str = "a02560be8a9b6a13b3a5a33de7e032d8f86dc658";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Jynx controller mapping is unavailable: pinned Linux and Windows frontends consume fixed keyboard matrices and expose no persisted host mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixed_matrix_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
