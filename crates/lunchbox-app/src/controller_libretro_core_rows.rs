//! Explicit refusals for libretro-core rows.
//!
//! The following records describe libretro cores whose Linux input path is
//! the RetroArch frontend (`retroarch.cfg` plus `autoconfig/` profiles).
//! They are out of native-adapter scope and covered by the RetroArch core
//! queue instead:
//!
//! * `gearlynx` (libretro/gearlynx): record paths are retroarch-only.
//! * `freeintv` (libretro/FreeIntv): record paths are retroarch-only.
//! * `freechaf` (libretro/FreeChaF): record paths are retroarch-only.
//! * `emuscv` (MaaaX-EmuSCV/libretro-emuscv): record paths are retroarch-only.
//! * `np2kai`: the captured record (`AZO234/NP2kai` identity evidence) pins
//!   only the RetroArch frontend convention; no standalone input grammar
//!   was captured.
//! * `dosbox-pure` (schellingb/dosbox-pure): libretro core by design; its
//!   input path is the RetroArch frontend.
//! * `gw` (MADrigal simulators libretro core): record paths are
//!   retroarch-only.

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "libretro-core input is frontend-owned: configure it through RetroArch, not a native adapter"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn libretro_rows_are_frontend_owned() {
        assert!(super::refusal().is_err());
    }
}
