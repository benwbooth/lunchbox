//! JSorcerer standalone/native-controller boundary.
//!
//! Source archive `sorcerer-1.30.zip` is pinned by SHA-256 in the emulator
//! record.  JSorcerer forwards AWT KeyEvents to a compiled keyboard matrix;
//! `hardKeyMap` is a runtime toggle, not a persisted controller profile, and
//! no joystick backend or stable mapping file is exposed.

pub(crate) const SOURCE_ARCHIVE_SHA256: &str =
    "2d8e625ff94b1d5c9f39aa00e2763fe2a120149ef14943b4dd283a2ef29d199e";
pub(crate) const ARTIFACT_SHA256: &str =
    "60af6401eb8cace30dad0836fd6fec7eb2c86dc026a1a6c435cdeba2ccde3524";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "JSorcerer controller mapping is unavailable: the pinned Java source has only AWT KeyEvent handling and runtime hardKeyMap, with no persistent mapping grammar or joystick backend"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn awt_only_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_ARCHIVE_SHA256.len(), 64);
        assert_eq!(super::ARTIFACT_SHA256.len(), 64);
    }
}
