//! Explicit refusal for M88kai controller serialization.
//!
//! M88kai is distributed as a Windows-only executable.  The inspectable M88
//! source family (`rururutan/m88` commit `1c48d83070202eef43ab00db757131d0cc4768cc`)
//! has an `M88.ini` joystick-enable/port mode and a fixed Win32 joystick
//! backend, but no remappable physical-device identity or binding grammar.
//! The official M88kai distribution site was unavailable during capture, so
//! this boundary does not pretend the family source is an exact M88kai oracle.

pub(crate) const ARTIFACT_SHA256: &str =
    "305bf4a6918f472ba29169425048354d6d324244b93d0fe97a0abb35c9c4b3f7";
pub(crate) const SOURCE_FAMILY_COMMIT: &str = "1c48d83070202eef43ab00db757131d0cc4768cc";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "M88kai controller mapping is unavailable: the Windows artifact has no source-backed remappable controller profile or stable device selector"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn artifact_only_controller_input_is_explicitly_refused() {
        assert!(super::super::controller_m88kai_standalone::refusal().is_err());
    }
}
