//! JollyCV standalone/native-controller boundary.
//!
//! Pinned upstream source is jgemu/jollycv at
//! 7141b3438c9f6d21e619df56cea76a8d998f38c5.  JollyCV is a Jolly Good API
//! module: it consumes `jg_inputstate` callbacks and exposes settings, but
//! does not serialize host keyboard/gamepad mappings.  Refuse until a host
//! frontend's mapping grammar and same-launch input oracle are available.

pub(crate) const SOURCE_COMMIT: &str = "7141b3438c9f6d21e619df56cea76a8d998f38c5";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "JollyCV controller mapping is unavailable: the pinned module consumes Jolly Good jg_inputstate callbacks and publishes no persistent host mapping grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn callback_only_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
