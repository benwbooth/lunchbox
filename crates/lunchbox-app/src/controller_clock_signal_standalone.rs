//! Clock Signal (CLK) standalone controller serialization boundary.
//!
//! All three catalog slugs (`clk-clock-signal`, `clock-signal`, and
//! `clock-signal-clk`) are aliases of Tom Harte's single CLK project. The
//! pinned SDL frontend opens every SDL joystick in enumeration order and
//! forwards axes 0/1, hats, and raw button indices directly to the emulated
//! machine. The pinned macOS frontend does the same and explicitly marks
//! configurable mapping as TODO. Neither frontend consumes a persisted
//! controller mapping file, so inventing one would not be faithful.

pub(crate) const SOURCE_COMMIT: &str = "096de57445920ecf16cf066979422e34aceb843a";

/// No writer is exposed until the upstream frontend defines persisted input
/// settings. Same-launch callers must measure and validate SDL device order;
/// this boundary is unrelated to device enumeration failure.
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Clock Signal controller mapping is unavailable: pinned SDL and macOS frontends map live SDL devices directly and publish no persisted mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn aliases_share_the_same_source_boundary() {
        assert!(super::refusal().is_err());
        assert_eq!(
            super::SOURCE_COMMIT,
            "096de57445920ecf16cf066979422e34aceb843a"
        );
    }
}
