//! Session configuration replacements. Preparation reads but never edits sources.
use super::{Pad, render, render_profile, select_game_profiles, settings::SavedSetup};
use crate::controller_dolphin::{
    NativeIni, NativePaths, NativeSnapshot, merge_native_ini, profile_choices, snapshot_native,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedConfiguration {
    source: NativeSnapshot,
    /// Absolute native destinations and replacement bytes, for private mounts.
    /// The launcher must never write these bytes to their source destinations.
    pub(crate) replacements: BTreeMap<PathBuf, Vec<u8>>,
}

impl PreparedConfiguration {
    pub(crate) fn verify(&self) -> Result<()> {
        self.source.verify()
    }
}

fn document<'a>(snapshot: &'a NativeSnapshot, path: &Path) -> Result<&'a str> {
    let (_, bytes) = snapshot
        .documents
        .iter()
        .find(|(candidate, _)| candidate == path)
        .with_context(|| format!("Dolphin configuration was not captured: {}", path.display()))?;
    Ok(std::str::from_utf8(bytes)?)
}

fn section(values: &NativeIni, name: &str) -> BTreeMap<String, String> {
    values
        .iter()
        .filter(|((section, _), _)| section == name)
        .map(|((_, key), value)| (key.clone(), value.clone()))
        .collect()
}

pub(crate) fn prepare(setup: &SavedSetup, pads: &[Pad]) -> Result<PreparedConfiguration> {
    setup.validate()?;
    ensure!(
        pads.len() == setup.players.len()
            && pads.iter().all(|pad| setup
                .players
                .iter()
                .any(|player| player.port == pad.port && player.device_qualifier == pad.device)),
        "Dolphin resolved pads do not match the saved setup"
    );
    let id = &setup.game_id;
    let names = [
        format!("{}.ini", &id[..1]),
        format!("{}.ini", &id[..3]),
        format!("{id}.ini"),
        format!("{id}r{}.ini", setup.revision),
    ];
    let paths = NativePaths {
        user: setup.user_directory.clone(),
        assets: setup.system_directory.clone(),
        game_settings: [&setup.system_directory, &setup.user_directory]
            .into_iter()
            .flat_map(|root| {
                names
                    .iter()
                    .map(move |name| root.join("GameSettings").join(name))
            })
            .collect(),
        controller_settings: setup.user_directory.join("Config/GCPadNew.ini"),
        main_settings: setup.user_directory.join("Config/Dolphin.ini"),
    };
    let source = snapshot_native(&paths)?;
    let global = document(&source, &paths.controller_settings)?;
    let (controller, main) = render(global, document(&source, &paths.main_settings)?, pads)?;
    let mut replacements = BTreeMap::from([
        (paths.controller_settings.clone(), controller.into_bytes()),
        (paths.main_settings.clone(), main.into_bytes()),
    ]);
    let mut base = NativeIni::new();
    merge_native_ini(&mut base, global.as_bytes())?;
    let game = source.game_settings(&paths)?;
    let root = paths.user.join("Config/Profiles/GCPad");
    let mut selections = BTreeMap::new();
    for pad in pads {
        let key = format!("padprofile{}", pad.port);
        let mut effective = if let Some(choice) = game.get(&("controls".into(), key.clone())) {
            let choices = profile_choices(&root, choice)?;
            let selected = choices
                .first()
                .context("Dolphin selected profile is missing")?;
            let mut parsed = NativeIni::new();
            merge_native_ini(&mut parsed, document(&source, selected)?.as_bytes())?;
            section(&parsed, "profile")
        } else {
            section(&base, &format!("gcpad{}", pad.port))
        };
        // Modern system and user game layers independently overlay settings;
        // each layer's selected profile is loaded after its embedded values.
        for files in paths.game_settings.chunks(4) {
            let mut layer = NativeIni::new();
            for path in files {
                merge_native_ini(&mut layer, document(&source, path)?.as_bytes())?;
            }
            effective.extend(section(&layer, &format!("gcpad.gcpad{}", pad.port)));
            if let Some(name) = layer.get(&("gcpad.controls".into(), key.clone())) {
                let profile = PathBuf::from(format!(
                    "{}/{name}.ini",
                    root.to_str()
                        .context("Dolphin profile path must be UTF-8")?
                ));
                let mut parsed = NativeIni::new();
                merge_native_ini(&mut parsed, document(&source, &profile)?.as_bytes())?;
                effective.extend(section(&parsed, "profile"));
            }
        }
        let original = super::patch_section("", "Profile", &effective)?;
        // Unique session basename avoids hiding any existing profile file.
        let name = format!("Lunchbox-{}-P{}", uuid::Uuid::new_v4(), pad.port);
        replacements.insert(
            root.join(format!("{name}.ini")),
            render_profile(&original, pad)?.into_bytes(),
        );
        selections.insert(pad.port, name);
    }
    let final_game = paths
        .game_settings
        .last()
        .context("Dolphin revision INI is missing")?;
    replacements.insert(
        final_game.clone(),
        select_game_profiles(document(&source, final_game)?, &selections)?.into_bytes(),
    );
    source.verify()?;
    Ok(PreparedConfiguration {
        source,
        replacements,
    })
}
