//! 8-Bit Wonders platform/controller boundary.
//!
//! Pinned source: eightbitwonders/app commit
//! 3b72d139099a513eb2d3fe5fe9b775fd11aeb06f.  The upstream project is an
//! Android application around VICE.  Its README documents hardware input and
//! save states, but no desktop config file or portable controller serializer.

pub(crate) const SOURCE_COMMIT: &str = "3b72d139099a513eb2d3fe5fe9b775fd11aeb06f";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "8-Bit Wonders controller mapping is unavailable for the desktop host matrix: the pinned upstream artifact is an Android VICE frontend and publishes no portable desktop mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn android_frontend_boundary_is_explicit() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
