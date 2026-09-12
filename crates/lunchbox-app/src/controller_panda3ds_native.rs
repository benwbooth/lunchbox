//! Panda3DS native-input boundary.
//!
//! The native SDL frontend in the pinned Panda3DS source opens controller 0
//! directly with `SDL_GameControllerOpen(0)` and hard-wires standard buttons
//! and the left stick.  `config.toml` contains keyboard mappings, but there is
//! no native gamepad mapping table to author.  This module is intentionally a
//! refusal contract: callers must use the libretro core (where Panda exposes
//! a standard retropad) or report native Panda3DS as unsupported.

use anyhow::{Result, bail};

pub(crate) const PROFILE_ID: &str = "panda3ds:standalone-native-unsupported";

pub(crate) fn native_mapping_unavailable() -> Result<()> {
    bail!(
        "Panda3DS native SDL input is not configurable: upstream opens only SDL gamepad 0 and hard-wires its standard controls; use Panda3DS libretro or another frontend"
    )
}

pub(crate) fn launch_contract() -> &'static str {
    "Native Panda3DS may be launched only with a private working directory containing config.toml with General.UsePortableBuild=false; this isolates config while SDL_GetPrefPath keeps saves/app data persistent. No controller mapping is staged."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_path_is_explicitly_unsupported() {
        assert!(native_mapping_unavailable().is_err());
        assert!(launch_contract().contains("No controller mapping"));
    }
}
