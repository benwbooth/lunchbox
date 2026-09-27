//! Explicit refusal for O2EM controller serialization.
//!
//! The inactive SourceForge project publishes legacy native binaries/source,
//! but no verified persistent host-controller profile or stable device ID.

pub(crate) const SOURCE_URL: &str = "http://o2em.sourceforge.net/";
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "O2EM controller mapping is unavailable: no verified writable persistent controller profile or stable host-device selector"
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn refuses_unverified_profile() {
        assert!(super::super::controller_o2em_standalone::refusal().is_err());
    }
}
