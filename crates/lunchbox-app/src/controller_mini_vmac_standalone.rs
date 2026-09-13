//! Explicit refusal for Mini vMac controller serialization.
//!
//! Mini vMac is generated per Macintosh model/build and its official
//! distribution does not publish a stable host-controller profile format.

pub(crate) const SOURCE_URL: &str = "https://www.gryphel.com/c/minivmac/";
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Mini vMac controller mapping is unavailable: generated builds expose keyboard-oriented Macintosh input without a stable writable host-gamepad profile"
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn refuses_unverified_profile() {
        assert!(super::super::controller_mini_vmac_standalone::refusal().is_err());
    }
}
