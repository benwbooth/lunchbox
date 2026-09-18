//! One target choice shared by guided review, persistence and launch.
//! Native and libretro scopes are distinct even when their emulator names match.
use crate::{
    controller_catalog::{Catalog, EmulatorProfile, NativeLaunch, catalog},
    emulator::{EmulatorRuntimeKind, RomEmulatorOption, canonical_retroarch_core_name},
    settings::ControllerMappingSettings,
};
use anyhow::{Context, Result, ensure};

/// Standalone emulators whose user-facing name differs from their catalog core
/// key. A launch option carries the emulator's display name (the database
/// `emulators.name`), while native profiles declare the core key used on the
/// wire. This one table feeds both the resolver below and the QML target
/// filter (`controller_catalog_json`), so the dialog and the launch path can
/// never disagree about which profiles an emulator owns. Each entry is also
/// the canonical display name for its core.
pub(crate) const NATIVE_EMULATOR_IDENTITIES: &[(&str, &str)] = &[
    ("Mesen", "mesen2"),
    ("Nestopia UE", "nestopia"),
    ("ADAMEm SDL", "adamem"),
    ("Atari++", "atari-plus-plus"),
    ("GBE+", "gbe-plus"),
    ("Play!", "play"),
    ("Yaba Sanshiro 2", "yaba-sanshiro"),
];

/// Additional accepted labels for a core that already has a canonical display
/// name. Resolution only: the mapping dialog keeps showing the identity name
/// above, so a second label can never rename an emulator in the UI.
pub(crate) const NATIVE_EMULATOR_ALIASES: &[(&str, &str)] = &[
    // The launch path splits VICE by machine; VIC-20 play selects the xvic
    // frontend while the catalog core stays `vice`.
    ("VICE (xvic)", "vice"),
];

fn matches_label(name: &str, label: &str) -> bool {
    name.trim().to_lowercase() == label.trim().to_lowercase()
}

/// Catalog core key for a standalone emulator label. Labels already written as
/// a core key pass through unchanged.
pub(crate) fn native_core_for(label: &str) -> Option<&'static str> {
    NATIVE_EMULATOR_IDENTITIES
        .iter()
        .chain(NATIVE_EMULATOR_ALIASES.iter())
        .find(|(name, _)| matches_label(name, label))
        .map(|(_, core)| *core)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Scope {
    pub retroarch: bool,
    pub core: String,
    pub platform: String,
}

impl Scope {
    pub(crate) fn from_label(emulator: &str, platform: &str) -> Result<Self> {
        // Display decorations (recommendation stars, section prefixes) must
        // never reach identity matching: fail loudly instead of resolving
        // the wrong emulator. See the Gopher64 ★ incident.
        anyhow::ensure!(
            !emulator.contains('★'),
            "Unexpected badge in emulator name; pass the plain emulator name"
        );
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
            canonical_retroarch_core_name(&core).to_owned()
        } else {
            // The bare name "Mesen", for example, selects the Mesen2 contract:
            // its Linux home is ~/.config/Mesen2 and it is the only Mesen with
            // a native controller writer. RetroArch cores never pass here.
            native_core_for(&core).map(str::to_owned).unwrap_or(core)
        };
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
            "nestopia-ue:flatpak-nes" => (&["Nintendo Entertainment System"], 2),
            "punes:flatpak-nes-standard" => (&["Nintendo Entertainment System"], 2),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nestopia_ue_native_name_uses_the_catalog_core_key() {
        let scope = Scope::from_label("Nestopia UE", "Nintendo Entertainment System").unwrap();
        assert!(!scope.retroarch);
        assert_eq!(scope.core, "nestopia");
        assert_eq!(
            scope
                .profile(catalog(), "nestopia-ue:flatpak-nes")
                .unwrap()
                .core,
            "nestopia"
        );
    }

    #[test]
    fn standalone_mesen_name_uses_the_mesen2_contract() {
        for platform in [
            "Nintendo Entertainment System",
            "NEC TurboGrafx-16",
            "NEC TurboGrafx-CD",
            "PC Engine SuperGrafx",
        ] {
            let scope = Scope::from_label("Mesen", platform).unwrap();
            assert!(!scope.retroarch);
            assert_eq!(scope.core, "mesen2");
        }
        let scope = Scope::from_label("Mesen", "NEC TurboGrafx-CD").unwrap();
        let profile = scope.profile(catalog(), "mesen2:standalone-pce-2").unwrap();
        assert_eq!(profile.core, "mesen2");
        assert_eq!(profile.target_layout, "pce-2");
        // RetroArch mesen cores keep their canonical core name.
        let retro =
            Scope::from_label("RetroArch (mesen)", "Nintendo Entertainment System").unwrap();
        assert!(retro.retroarch);
        assert_eq!(retro.core, "mesen");
    }

    #[test]
    fn native_identity_table_matches_declared_profiles() {
        use std::collections::BTreeSet;
        let native_cores: BTreeSet<&str> = catalog()
            .emulator_profiles
            .iter()
            .filter(|profile| profile.transport != "retropad")
            .map(|profile| profile.core.as_str())
            .collect();
        let mut names = BTreeSet::new();
        for (name, core) in NATIVE_EMULATOR_IDENTITIES {
            assert!(
                names.insert(name.trim().to_lowercase()),
                "duplicate native emulator identity: {name}"
            );
            assert!(
                native_cores.contains(core),
                "native emulator identity {name} names an unknown core {core}"
            );
            assert_ne!(
                &name.trim().to_lowercase(),
                core,
                "identity {name} is redundant: the label is already the core key"
            );
            assert!(
                name.trim() == *name && !name.is_empty(),
                "identity names must be exact, trimmed and non-empty"
            );
        }
        let mut aliases = BTreeSet::new();
        for (name, core) in NATIVE_EMULATOR_ALIASES {
            assert!(
                aliases.insert(name.trim().to_lowercase()),
                "duplicate native emulator alias: {name}"
            );
            assert!(
                native_cores.contains(core),
                "native emulator alias {name} names an unknown core {core}"
            );
            assert!(
                !names.contains(&name.trim().to_lowercase()),
                "alias {name} duplicates an identity"
            );
            assert_ne!(
                &name.trim().to_lowercase(),
                core,
                "alias {name} is redundant: the label is already the core key"
            );
        }
    }

    #[test]
    fn every_native_emulator_resolves_to_its_own_profiles() {
        // The launch path receives `option.emulator_name` (the database display
        // name), so each declared identity and alias must reach profiles.
        for (name, core) in NATIVE_EMULATOR_IDENTITIES
            .iter()
            .chain(NATIVE_EMULATOR_ALIASES.iter())
        {
            let scope = Scope::from_label(name, "Nintendo Entertainment System").unwrap();
            assert!(!scope.retroarch);
            assert_eq!(scope.core, *core, "{name} resolved to the wrong core");
        }
        // Identity keys round-trip through persistence unchanged.
        for (name, core) in NATIVE_EMULATOR_IDENTITIES {
            let scope = Scope::from_label(name, "Nintendo Entertainment System").unwrap();
            assert_eq!(Scope::from_key(&scope.key()).unwrap().core, *core);
        }
        // A core key is never rewritten by an alias entry.
        let scope = Scope::from_label("vice", "Commodore VIC-20").unwrap();
        assert_eq!(scope.core, "vice");
    }

    #[test]
    fn display_badges_never_reach_identity_matching() {
        assert!(Scope::from_label("★ Gopher64", "Nintendo 64").is_err());
        assert!(Scope::from_label("Gopher64", "Nintendo 64").is_ok());
    }

    /// Prints every native profile's core so the checked-in catalog can be
    /// audited against the emulator display names this UI receives. Run
    /// explicitly with `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore = "audit helper; prints the native core contract"]
    fn audit_native_cores() {
        use std::collections::BTreeSet;
        let mut pairs = BTreeSet::new();
        for profile in &catalog().emulator_profiles {
            if profile.transport == "retropad" {
                continue;
            }
            let platforms = profile
                .native_launch
                .as_ref()
                .map(|launch| launch.platforms.join("|"))
                .unwrap_or_default();
            pairs.insert((profile.core.clone(), platforms));
        }
        for (core, platforms) in pairs {
            println!("NATIVE-CORE {core}\t{platforms}");
        }
    }
}
