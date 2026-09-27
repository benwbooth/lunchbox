//! Explicit refusal for PK201 controller serialization.

pub(crate) const ARTIFACT_SHA256: &str =
    "b0350fdb7f5cbee609189cdfdf574b0ab89bb536ea44ddbbde3ce3e14b1e6673";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "PK201 controller mapping is unavailable: legacy Windows artifact documents ROM files but no persistent host profile grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_artifact_is_explicitly_refused() {
        assert!(super::super::controller_pk201_standalone::refusal().is_err());
    }
}
