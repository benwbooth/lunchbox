//! Explicit refusal for exact Elecbyte MUGEN controller serialization.
//!
//! The official MUGEN documentation is not a pinned source distribution and
//! does not define a stable host controller-profile writer.

pub(crate) const SOURCE_DOC: &str = "https://www.elecbyte.com/mugendocs-11b1/mugen.html";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "MUGEN controller mapping is unavailable: official documentation does not define a stable host profile writer"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn documentation_only_mugen_is_explicitly_refused() {
        assert!(super::super::controller_mugen_standalone::refusal().is_err());
    }
}
