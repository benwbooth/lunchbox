//! Explicit refusal for XEBRA controller serialization.
//!
//! XEBRA's PC-pad association is configured interactively at runtime.  The
//! official instructions require probing the host axes and converting their
//! ranges, but do not publish a stable file grammar or device identity that
//! Lunchbox can safely write.

pub(crate) const ARTIFACT_SHA256: &str =
    "5f522e51a0cad7395bdcb070e7c6955327f4bd3449a9c04dc7ed35c6693c1bd2";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "XEBRA controller mapping is unavailable: the official PC instructions require runtime host-axis probing and interactive association, with no verified persistent mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_axis_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::ARTIFACT_SHA256.len(), 64);
    }
}
