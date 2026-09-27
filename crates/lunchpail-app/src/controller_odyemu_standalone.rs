//! Explicit refusal for Odyemu DOS controller serialization.
//!
//! Odyemu is a legacy DOS/Windows artifact normally run through DOSBox. No
//! source-pinned configuration grammar or physical-device identity is known.

pub(crate) const SOURCE_URL: &str = "https://archive.org/details/odyemu";
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Odyemu controller mapping is unavailable: legacy DOS artifact has no source-backed persistent host-controller profile"
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn refuses_unverified_profile() {
        assert!(super::super::controller_odyemu_standalone::refusal().is_err());
    }
}
