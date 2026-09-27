//! Explicit refusal for PCSX-ReARMed native controller serialization.

pub(crate) const SOURCE_COMMIT: &str = "d7d741db1d974cf8dd05a23149a1f12758fc1894";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "PCSX-ReARMed controller mapping is unavailable: frontend-owned input/config format is not established for this core"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn frontend_owned_profile_is_explicitly_refused() {
        assert!(super::super::controller_pcsx_rearmed_standalone::refusal().is_err());
    }
}
