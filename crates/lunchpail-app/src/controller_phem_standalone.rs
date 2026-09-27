//! Explicit refusal for PHEM desktop controller serialization.

pub(crate) const SOURCE_COMMIT: &str = "7f2268390e1e60bd2e51c7dff617a05ca84c51f5";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "PHEM desktop controller mapping is unavailable: upstream is an Android Gradle/JNI application"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn android_only_profile_is_explicitly_refused() {
        assert!(super::super::controller_phem_standalone::refusal().is_err());
    }
}
