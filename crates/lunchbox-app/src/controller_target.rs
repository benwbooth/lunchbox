//! One target choice shared by guided review, persistence and launch.
//! Native and libretro scopes are distinct even when their emulator names match.
use crate::{
    controller_catalog::{Catalog, EmulatorProfile, NativeLaunch, catalog},
    emulator::{EmulatorRuntimeKind, RomEmulatorOption, canonical_retroarch_core_name},
    settings::ControllerMappingSettings,
};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

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
    // The identity table also supplies the dialog's display name, so a label
    // that differs from its core key only in case is still meaningful here.
    ("DOSBox-X", "dosbox-x"),
    ("DOSBox Staging", "dosbox-staging"),
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

/// Mapping scope precedence is game, system, core, then legacy profile choices.
/// The target layout is part of the key so a system-wide map is only shared
/// with emulator contracts that use the same controller shape.
pub(crate) fn mapping_key(
    level: &str,
    game_uid: &str,
    scope: &Scope,
    device: &str,
    layout: &str,
) -> Result<String> {
    ensure!(
        !device.is_empty() && device.len() <= 512 && !device.chars().any(char::is_control),
        "Choose a controller first"
    );
    ensure!(
        catalog().layout(layout).is_some(),
        "Choose a target controller first"
    );
    let identity = match level {
        "game" => {
            ensure!(
                !game_uid.trim().is_empty()
                    && game_uid.len() <= 512
                    && !game_uid.chars().any(char::is_control),
                "Choose a game first"
            );
            game_uid.to_owned()
        }
        "system" => scope.platform.clone(),
        "core" => serde_json::to_string(&[
            if scope.retroarch {
                "retroarch"
            } else {
                "native"
            },
            &scope.core,
        ])?,
        _ => anyhow::bail!("Choose game, system, or emulator/core scope"),
    };
    Ok(serde_json::to_string(&[level, &identity, device, layout])?)
}

pub(crate) fn mapping_choices<'a>(
    mapping: &'a ControllerMappingSettings,
    game_uid: &str,
    scope: &Scope,
    device: &str,
    profile: &EmulatorProfile,
    maximum: &str,
) -> Result<Option<(&'a BTreeMap<String, String>, &'static str)>> {
    let levels: &[&str] = match maximum {
        "game" => &["game", "system", "core"],
        "system" => &["system", "core"],
        "core" => &["core"],
        _ => anyhow::bail!("Choose game, system, or emulator/core scope"),
    };
    for level in levels {
        if *level == "game" && game_uid.is_empty() {
            continue;
        }
        let key = mapping_key(level, game_uid, scope, device, &profile.target_layout)?;
        if let Some(choices) = mapping.guided_mapping_overrides.get(&key) {
            return Ok(Some((
                choices,
                match *level {
                    "game" => "game",
                    "system" => "system",
                    _ => "core",
                },
            )));
        }
    }
    Ok(None)
}

pub(crate) fn validate_mapping_key(key: &str) -> Result<()> {
    let [level, identity, device, layout]: [String; 4] = serde_json::from_str(key)?;
    ensure!(
        !identity.is_empty()
            && identity.len() <= 512
            && !identity.chars().any(char::is_control)
            && !device.is_empty()
            && device.len() <= 512
            && !device.chars().any(char::is_control),
        "Invalid guided mapping scope"
    );
    ensure!(
        catalog().layout(&layout).is_some(),
        "Unknown guided mapping target layout"
    );
    match level.as_str() {
        "game" => {}
        "system" => ensure!(
            identity == identity.trim().to_lowercase(),
            "Noncanonical system mapping scope"
        ),
        "core" => {
            let [kind, core]: [String; 2] = serde_json::from_str(&identity)?;
            ensure!(
                matches!(kind.as_str(), "retroarch" | "native")
                    && !core.is_empty()
                    && core == core.trim().to_lowercase(),
                "Invalid core mapping scope"
            );
        }
        _ => anyhow::bail!("Invalid guided mapping level"),
    }
    Ok(())
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

/// A broader mapping can reach a new core/system context without requiring an
/// otherwise redundant target-selection save, but only when its controller
/// layout identifies exactly one non-explicit launch contract.
pub(crate) fn inferred_from_mapping(
    mapping: &ControllerMappingSettings,
    option: &RomEmulatorOption,
    platform: &str,
    game_uid: &str,
) -> Result<Option<&'static EmulatorProfile>> {
    let scope = Scope::for_option(option, platform)?;
    let mut matching = Vec::new();
    for profile in catalog()
        .emulator_profiles
        .iter()
        .filter(|profile| scope.accepts(profile) && !profile.explicit_selection)
    {
        let mut has_mapping = false;
        for device in mapping.calibrations.keys() {
            if mapping_choices(mapping, game_uid, &scope, device, profile, "game")?.is_some() {
                has_mapping = true;
                break;
            }
        }
        if has_mapping {
            matching.push(profile);
        }
    }
    Ok(if matching.len() == 1 {
        matching.pop()
    } else {
        None
    })
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
    fn guided_mapping_scopes_override_in_game_system_core_order() {
        let scope =
            Scope::from_label("RetroArch (fceumm)", "Nintendo Entertainment System").unwrap();
        let profile = scope.profile(catalog(), "retroarch:fceumm:nes").unwrap();
        let mut mapping = ControllerMappingSettings::default();
        let choices = |source: &str| BTreeMap::from([("a".to_owned(), source.to_owned())]);
        for (level, uid, source) in [
            ("core", "", "core-button"),
            ("system", "", "system-button"),
            ("game", "metroid-id", "game-button"),
        ] {
            let key = mapping_key(level, uid, &scope, "pad", &profile.target_layout).unwrap();
            validate_mapping_key(&key).unwrap();
            mapping
                .guided_mapping_overrides
                .insert(key, choices(source));
        }
        let find = |uid: &str, maximum: &str| {
            mapping_choices(&mapping, uid, &scope, "pad", profile, maximum)
                .unwrap()
                .map(|(choices, source)| (choices["a"].clone(), source))
        };
        assert_eq!(
            find("metroid-id", "game"),
            Some(("game-button".into(), "game"))
        );
        assert_eq!(
            find("other-id", "game"),
            Some(("system-button".into(), "system"))
        );
        assert_eq!(
            find("metroid-id", "system"),
            Some(("system-button".into(), "system"))
        );
        assert_eq!(
            find("metroid-id", "core"),
            Some(("core-button".into(), "core"))
        );
        let other_system =
            Scope::from_label("RetroArch (fceumm)", "Nintendo Famicom Disk System").unwrap();
        assert_eq!(
            mapping_key("core", "", &scope, "pad", &profile.target_layout).unwrap(),
            mapping_key("core", "", &other_system, "pad", &profile.target_layout).unwrap()
        );
        assert_ne!(
            mapping_key("system", "", &scope, "pad", &profile.target_layout).unwrap(),
            mapping_key("system", "", &other_system, "pad", &profile.target_layout).unwrap()
        );
        let other_core =
            Scope::from_label("RetroArch (nestopia)", "Nintendo Entertainment System").unwrap();
        assert_eq!(
            mapping_key("system", "", &scope, "pad", &profile.target_layout).unwrap(),
            mapping_key("system", "", &other_core, "pad", &profile.target_layout).unwrap()
        );
        assert_ne!(
            mapping_key("core", "", &scope, "pad", &profile.target_layout).unwrap(),
            mapping_key("core", "", &other_core, "pad", &profile.target_layout).unwrap()
        );
        let calibration: crate::controller_catalog::Calibration =
            serde_json::from_value(serde_json::json!({
                "layout":"nes", "os":"linux", "backend":"gilrs-0.11",
                "bindings":{
                    "a":{"code":1,"kind":"button","direction":0,"logical":"South"},
                    "b":{"code":2,"kind":"button","direction":0,"logical":"West"}
                }
            }))
            .unwrap();
        mapping.calibrations.insert("pad".into(), calibration);
        let option = RomEmulatorOption::retroarch(
            "retroarch-id".into(),
            "RetroArch".into(),
            "fceumm",
            crate::emulator::EmulatorExecutable::Native("/unused/retroarch".into()),
            "/unused/fceumm_libretro.so".into(),
            false,
        );
        assert_eq!(
            inferred_from_mapping(
                &mapping,
                &option,
                "Nintendo Entertainment System",
                "other-id"
            )
            .unwrap()
            .map(|profile| profile.id.as_str()),
            Some("retroarch:fceumm:nes")
        );
    }

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
    fn dosbox_labels_resolve_to_their_mapper_profiles() {
        for (label, core) in [
            ("DOSBox-X", "dosbox-x"),
            ("DOSBox Staging", "dosbox-staging"),
        ] {
            let scope = Scope::from_label(label, "MS-DOS").unwrap();
            assert!(!scope.retroarch);
            assert_eq!(scope.core, core);
            let profile = scope
                .profile(catalog(), &format!("{core}:standalone-dosbox-joystick"))
                .unwrap();
            assert_eq!(profile.transport, format!("{core}-native-settings"));
            let native = profile.native_launch.as_ref().unwrap();
            assert_eq!(native.max_players, 2);
            assert!(native.platforms.iter().any(|platform| platform == "MS-DOS"));
        }
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
                name.trim(),
                *core,
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

    /// The promise this whole module exists to keep: if a core has a fixed
    /// device mode, the launch path must be able to pick one without the user
    /// choosing a mode first. A core whose every fixed-topology profile
    /// demands explicit selection fails closed with "No automatic controller
    /// contract", which is what produced "Could not launch this game:
    /// applying calibrated controller mappings" for RetroArch · Nestopia UE.
    ///
    /// Option-resolved cores (their device count follows a core option) are
    /// not gaps: the launcher seeds the contract and then widens the port
    /// count from the user's own effective options. They are asserted to stay
    /// automatic here, so nobody can quiet this test by flipping one to
    /// explicit.
    #[test]
    fn every_core_with_a_fixed_topology_has_an_automatic_contract() {
        use std::collections::{BTreeMap, BTreeSet};
        // Cores whose device count follows a core option. They must remain
        // automatic: their profile is the one the launcher seeds before it
        // resolves the option value.
        const OPTION_DRIVEN_CORES: &[(&str, &str)] = &[
            ("opera", "opera_active_devices selects 1-8 pads"),
            ("sameboy", "sameboy_model selects 1 or 4 pads (SGB)"),
            ("mednafen_supergrafx", "sgx_multitap selects 1 or 5 pads"),
        ];
        // Cores that cannot be automatic until their catalog data changes,
        // each with the reason the validator or launcher would reject them.
        // These are *modes*, not gaps: the user picks one deliberately and the
        // mapping dialog lists them.
        const KNOWN_EXPLICIT_ONLY_CORES: &[(&str, &str)] = &[
            ("81", "only keyboard cursor/QAOP profiles exist"),
            ("atari800", "machine-specific memory models are the mode"),
            ("b2", "keyboard mapping variants only"),
            ("bk", "BK model guard needs an explicit machine"),
            ("bluemsx", "machine profile is the mode"),
            ("cap32", "keyboard/joystick scheme is the mode"),
            ("citra", "new3ds vs 3ds hardware profile"),
            ("crocods", "single keyboard-plus-joystick profile"),
            ("dolphin", "fixed mixed-trigger topology declared explicit"),
            ("dosbox_pure", "device-direct modes are the choice"),
            ("emuscv", "profile declares per-port devices"),
            ("ep128emu-core", "machine adapter is the mode"),
            ("fmsx", "machine profile is the mode"),
            (
                "freeintv",
                "needs a fresh start before device type is fixed",
            ),
            ("freej2me", "phone keypad layouts are the choice"),
            ("fuse", "joystick interface is the mode"),
            (
                "geolith",
                "cartridge guard requires an explicit AES/MVS mode",
            ),
            ("gw", "single Game & Watch profile"),
            ("hatari", "ST joystick interface is the mode"),
            ("minivmac", "mouse emulation style is the mode"),
            ("mu", "stylus/stick choice"),
            ("np2kai", "machine and mouse handling are the choice"),
            ("o2em", "console/videopac variant is the mode"),
            (
                "panda3ds",
                "needs a fresh start before device type is fixed",
            ),
            ("pcsx2", "needs a fresh start before device type is fixed"),
            ("puae", "machine model is the mode"),
            ("px68k", "machine and joystick scheme are the choice"),
            ("quasi88", "machine model is the mode"),
            ("same_cdi", "pointer mode must be chosen deliberately"),
            (
                "scummvm",
                "single RetroPad-to-cursor contract declared explicit",
            ),
            ("simcp", "patched one/two player interface is the mode"),
            ("skyemu", "GBA vs GB profile"),
            ("steemsse", "embedded ROM joystick mode"),
            ("stella", "detected-joystick variants"),
            ("vice_x128", "joystick port is the mode"),
            ("vice_x64", "joystick port is the mode"),
            ("vice_x64sc", "joystick port is the mode"),
            ("vice_xpet", "virtual keyboard profile"),
            ("vice_xplus4", "joystick port is the mode"),
            ("vice_xvic", "single keyboard-profile joystick port"),
            ("virtual_jaguar", "full keypad layout declared explicit"),
        ];
        let option_driven: BTreeSet<&str> =
            OPTION_DRIVEN_CORES.iter().map(|(core, _)| *core).collect();
        let known: BTreeSet<&str> = KNOWN_EXPLICIT_ONLY_CORES
            .iter()
            .map(|(core, _)| *core)
            .collect();
        let catalog = catalog();
        let mut automatic: BTreeMap<&str, Vec<&EmulatorProfile>> = BTreeMap::new();
        let mut everything: BTreeMap<&str, Vec<&EmulatorProfile>> = BTreeMap::new();
        for profile in catalog
            .emulator_profiles
            .iter()
            .filter(|profile| profile.retroarch_launch.is_some())
        {
            everything
                .entry(profile.core.as_str())
                .or_default()
                .push(profile);
            if !profile.explicit_selection {
                automatic
                    .entry(profile.core.as_str())
                    .or_default()
                    .push(profile);
            }
        }
        let mut unexplained = Vec::new();
        for (core, profiles) in &everything {
            let fixed_topology: Vec<&&EmulatorProfile> = profiles
                .iter()
                .filter(|profile| {
                    profile
                        .retroarch_launch
                        .as_ref()
                        .is_some_and(|launch| launch.player_topology.is_none())
                })
                .collect();
            let Some(auto) = automatic.get(core) else {
                if !known.contains(core) {
                    unexplained.push(format!(
                        "{core} has {} fixed-topology profile(s) but no automatic contract",
                        fixed_topology.len()
                    ));
                }
                continue;
            };
            // An option-driven core must stay automatic: its profile is what
            // the launcher seeds before resolving the option value.
            if option_driven.contains(core) {
                assert!(
                    profiles.iter().any(|profile| profile
                        .retroarch_launch
                        .as_ref()
                        .is_some_and(|launch| launch.player_topology.is_some())),
                    "{core} is listed as option-driven but declares no player topology"
                );
            }
            // An automatic entry must really be defaultable.
            for profile in auto {
                let launch = profile.retroarch_launch.as_ref().unwrap();
                assert!(
                    profile.content_guard.is_none(),
                    "{} is automatic but guards content",
                    profile.id
                );
                assert!(
                    profile.port_devices.is_empty() && profile.port_layouts.is_empty(),
                    "{} is automatic but overrides per-port topology",
                    profile.id
                );
            }
        }
        assert!(
            unexplained.is_empty(),
            "cores that would fail a launch with \"No automatic controller contract\":\n  {}",
            unexplained.join("\n  ")
        );
        // Every listed exception must still exist, so stale entries are caught.
        for (core, _) in KNOWN_EXPLICIT_ONLY_CORES
            .iter()
            .chain(OPTION_DRIVEN_CORES.iter())
        {
            assert!(
                everything.contains_key(core),
                "listed exception {core} has no profiles any more"
            );
        }
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
