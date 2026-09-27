//! Explicit refusal for JPCSP controller serialization.
//!
//! Pinned source: jpcsp/jpcsp
//! `cd20cf312b358b4260f26f6754f9c62926c70ba6`. Controller mappings live in
//! `Setting.properties` as JInput component name strings
//! (`Controller.java`, `Settings.java`: `controller.controllerName` plus
//! `controllerNameIndex` and per-action component names). Device identity is
//! a name string plus enumeration index with no stable physical identity,
//! so this catalog refuses to treat a fuzzy name match as an identity link.

pub(crate) const SOURCE_COMMIT: &str = "cd20cf312b358b4260f26f6754f9c62926c70ba6";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "JPCSP controller mapping is unavailable: JInput name-plus-index identity is not a stable physical identity"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn name_based_identity_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
