//! Explicit refusal for a standalone Virtual Jaguar controller writer.
//!
//! The maintained Virtual Jaguar implementation is a libretro core.  Input,
//! configuration, saves and states are owned by the RetroArch frontend; the
//! standalone record must not invent a second native serializer or duplicate
//! the `retroarch:virtual_jaguar` controller profile.

pub(crate) const SOURCE_COMMIT: &str = "f9a3c89f58836cb2c45a42ad4edfac047050f30a";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Virtual Jaguar standalone controller mapping is unavailable: the maintained implementation is a libretro core and RetroArch owns its controller mapping/configuration"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn libretro_frontend_boundary_fails_closed() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
