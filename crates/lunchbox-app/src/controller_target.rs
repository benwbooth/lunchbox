//! One target choice shared by guided review, persistence and launch.
//! Native and libretro scopes are distinct even when their emulator names match.
use crate::{
    controller_catalog::{Catalog, EmulatorProfile, NativeLaunch, catalog},
    emulator::{EmulatorRuntimeKind, RomEmulatorOption, canonical_retroarch_core_name},
    settings::ControllerMappingSettings,
};
use anyhow::{Context, Result, ensure};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Scope {
    pub retroarch: bool,
    pub core: String,
    pub platform: String,
}

impl Scope {
    pub(crate) fn from_label(emulator: &str, platform: &str) -> Result<Self> {
        let label = emulator.trim().to_lowercase();
        let retroarch = label.starts_with("retroarch");
        let core = if retroarch {
            label
                .strip_suffix(')')
                .and_then(|s| s.rsplit_once('('))
                .map(|(_, core)| core.trim().trim_end_matches("_libretro"))
                .context("Choose a specific RetroArch core")?
        } else {
            label.as_str()
        };
        Self::new(retroarch, core, platform)
    }

    pub(crate) fn for_option(option: &RomEmulatorOption, platform: &str) -> Result<Self> {
        let retroarch = option.runtime_kind == EmulatorRuntimeKind::RetroArch;
        Self::new(
            retroarch,
            if retroarch {
                &option.core_name
            } else {
                &option.emulator_name
            },
            platform,
        )
    }

    fn new(retroarch: bool, core: &str, platform: &str) -> Result<Self> {
        let core = core.trim().to_lowercase();
        let core = if retroarch {
            canonical_retroarch_core_name(&core)
        } else {
            &core
        }
        .to_owned();
        let scope = Self {
            retroarch,
            core,
            platform: platform.trim().to_lowercase(),
        };
        ensure!(
            [&scope.core, &scope.platform]
                .iter()
                .all(|part| !part.is_empty()
                    && part.len() <= 256
                    && !part.chars().any(char::is_control)),
            "Choose an emulator and system first"
        );
        Ok(scope)
    }

    pub(crate) fn key(&self) -> String {
        serde_json::to_string(&[
            if self.retroarch {
                "retroarch"
            } else {
                "native"
            },
            &self.core,
            &self.platform,
        ])
        .expect("string scope serializes")
    }

    pub(crate) fn from_key(key: &str) -> Result<Self> {
        let [kind, core, platform]: [String; 3] = serde_json::from_str(key)?;
        ensure!(
            matches!(kind.as_str(), "native" | "retroarch"),
            "Invalid controller target scope"
        );
        let scope = Self::new(kind == "retroarch", &core, &platform)?;
        ensure!(scope.key() == key, "Noncanonical controller target scope");
        Ok(scope)
    }

    pub(crate) fn accepts(&self, profile: &EmulatorProfile) -> bool {
        if (profile.transport == "retropad") != self.retroarch {
            return false;
        }
        let core = profile.core.to_lowercase();
        let core = if self.retroarch {
            canonical_retroarch_core_name(&core)
        } else {
            &core
        };
        if core != self.core {
            return false;
        }
        let platforms = if self.retroarch {
            profile
                .retroarch_launch
                .as_ref()
                .map(|launch| &launch.platforms)
        } else {
            profile
                .native_launch
                .as_ref()
                .map(|launch| &launch.platforms)
        };
        if !platforms.is_some_and(|values| {
            values
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&self.platform))
        }) {
            return false;
        }
        // Ordinary arcade controls only. The separate advanced peripheral editor
        // remains available, but its modes do not leak into this player-first UI.
        !matches!(
            self.platform.as_str(),
            "arcade" | "sega naomi" | "sega naomi 2" | "sammy atomiswave"
        ) || matches!(
            profile.target_layout.as_str(),
            "arcade-six-button" | "arcade-eight-button"
        )
    }

    pub(crate) fn profile<'a>(&self, db: &'a Catalog, id: &str) -> Result<&'a EmulatorProfile> {
        db.emulator_profiles.iter().find(|p| p.id == id && self.accepts(p))
            .context("The saved target controller is not applicable to this emulator and system. Choose it again in Controller setup.")
    }
}

pub(crate) fn selected(
    mapping: &ControllerMappingSettings,
    option: &RomEmulatorOption,
    platform: &str,
) -> Result<Option<&'static EmulatorProfile>> {
    let scope = Scope::for_option(option, platform)?;
    mapping
        .guided_target_selections
        .get(&scope.key())
        .map(|id| scope.profile(catalog(), id))
        .transpose()
}

/// Explicit native contracts, not a guess based on similar controller artwork.
/// Player limits describe the configurations the corresponding writer can
/// produce. A profile does not imply automatic runtime discovery or host support.
pub(crate) fn add_native_metadata(db: &mut Catalog) -> Result<()> {
    for profile in &mut db.emulator_profiles {
        let (platforms, max_players): (&[&str], usize) = match profile.id.as_str() {
            "duckstation:digital-controller" | "duckstation:analog-controller" => {
                (&["Sony Playstation"], 2)
            }
            "ppsspp:standalone-psp" => (&["Sony PSP"], 1),
            "dolphin:standalone-gamecube" => (&["Nintendo GameCube"], 4),
            "mgba:standalone-gba" | "mednafen:standalone-gba" => {
                (&["Nintendo Game Boy Advance"], 1)
            }
            "mgba:standalone-gameboy"
            | "sameboy:standalone-sdl-gameboy"
            | "mednafen:standalone-gb" => (&["Nintendo Game Boy", "Nintendo Game Boy Color"], 1),
            "snes9x:standalone-gtk-snes" => (&["Super Nintendo Entertainment System"], 5),
            "fceux:standalone-qt-nes" => (
                &[
                    "Nintendo Entertainment System",
                    "Nintendo Famicom Disk System",
                ],
                4,
            ),
            "mednafen:standalone-psx" | "mednafen:standalone-psx-dualanalog" => {
                (&["Sony Playstation"], 8)
            }
            "mednafen:standalone-saturn" => (&["Sega Saturn"], 12),
            "mednafen:standalone-md3" | "mednafen:standalone-md6" => {
                (&["Sega Genesis", "Sega Mega Drive"], 8)
            }
            "mednafen:standalone-snes" | "mednafen:standalone-snes-faust" => {
                (&["Super Nintendo Entertainment System"], 8)
            }
            "mednafen:standalone-nes" => (
                &[
                    "Nintendo Entertainment System",
                    "Nintendo Famicom Disk System",
                ],
                4,
            ),
            "mednafen:standalone-pce2"
            | "mednafen:standalone-pce6"
            | "mednafen:standalone-pce-fast2"
            | "mednafen:standalone-pce-fast6" => (
                &[
                    "NEC TurboGrafx-16",
                    "NEC TurboGrafx-CD",
                    "NEC PC Engine SuperGrafx",
                    "PC Engine",
                ],
                5,
            ),
            "mednafen:standalone-sms" => (&["Sega Master System"], 2),
            "mednafen:standalone-gg" => (&["Sega Game Gear"], 1),
            "mednafen:standalone-vb" => (&["Nintendo Virtual Boy"], 1),
            "mednafen:standalone-wswan" => (&["WonderSwan", "WonderSwan Color"], 1),
            "mednafen:standalone-ngp" => (&["SNK Neo Geo Pocket", "SNK Neo Geo Pocket Color"], 1),
            "mednafen:standalone-lynx" => (&["Atari Lynx"], 1),
            _ => continue,
        };
        ensure!(
            profile.retroarch_launch.is_none() && profile.transport != "retropad",
            "Native metadata attached to libretro profile"
        );
        profile.native_launch = Some(NativeLaunch {
            platforms: platforms.iter().map(|s| (*s).to_owned()).collect(),
            max_players,
        });
    }
    Ok(())
}
