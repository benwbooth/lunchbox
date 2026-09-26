#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, profile_json)]
        #[qproperty(QString, message)]
        #[qproperty(bool, busy)]
        type GameModsModel = super::GameModsModelRust;
        #[qinvokable]
        fn select_game(self: Pin<&mut GameModsModel>, game: QString);
        #[qinvokable]
        fn reload_profile(self: Pin<&mut GameModsModel>);
        #[qinvokable]
        fn import_patch(self: Pin<&mut GameModsModel>, path: QString);
        #[qinvokable]
        fn import_cheats(self: Pin<&mut GameModsModel>, path: QString);
        #[qinvokable]
        fn export_cheats(self: Pin<&mut GameModsModel>, path: QString);
        #[qinvokable]
        fn change_patch(self: Pin<&mut GameModsModel>, index: i32, action: QString);
        #[qinvokable]
        fn change_cheat(self: Pin<&mut GameModsModel>, index: i32, action: QString);
        #[qinvokable]
        fn save_cheat(self: Pin<&mut GameModsModel>, index: i32, name: QString, code: QString);
        #[qinvokable]
        fn set_base_hash(self: Pin<&mut GameModsModel>, hash: QString);
        #[qinvokable]
        fn enable_cheats(self: Pin<&mut GameModsModel>, enabled: bool);
    }
    impl cxx_qt::Threading for GameModsModel {}
}
use crate::game_mods::{self, Cheat, Profile};
use anyhow::{Context, Result, bail};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::pin::Pin;

pub struct GameModsModelRust {
    profile_json: QString,
    message: QString,
    busy: bool,
    game: String,
    profile: Profile,
    generation: u64,
}
impl Default for GameModsModelRust {
    fn default() -> Self {
        Self {
            profile_json: QString::from(serde_json::to_string(&Profile::default()).unwrap()),
            message: QString::default(),
            busy: false,
            game: String::new(),
            profile: Profile::default(),
            generation: 0,
        }
    }
}
impl qobject::GameModsModel {
    pub fn reload_profile(mut self: Pin<&mut Self>) {
        let game = self.rust().game.clone();
        self.as_mut().rust_mut().game.clear();
        self.select_game(game.into());
    }
    pub fn select_game(mut self: Pin<&mut Self>, game: QString) {
        if self.rust().game == game.to_string() {
            return;
        }
        self.as_mut().rust_mut().generation += 1;
        self.as_mut().rust_mut().game = game.to_string();
        self.as_mut().set_message(QString::default());
        let result = crate::settings::SettingsStore::open_default()
            .and_then(|store| Profile::load(&store, &game.to_string()));
        match result {
            Ok(profile) => self.as_mut().publish(profile),
            Err(error) => {
                self.as_mut().publish(Profile::default());
                self.as_mut().set_message(QString::from(format!(
                    "Cannot load patches/cheats: {error:#}"
                )));
            }
        }
    }
    fn publish(mut self: Pin<&mut Self>, profile: Profile) {
        self.as_mut()
            .set_profile_json(QString::from(serde_json::to_string(&profile).unwrap()));
        self.as_mut().rust_mut().profile = profile;
    }
    fn edit(mut self: Pin<&mut Self>, change: impl FnOnce(&mut Profile) -> Result<()>) {
        if *self.busy() {
            return;
        }
        let mut profile = self.rust().profile.clone();
        let result = change(&mut profile).and_then(|()| {
            profile.save(
                &crate::settings::SettingsStore::open_default()?,
                &self.rust().game,
            )
        });
        match result {
            Ok(()) => {
                self.as_mut().publish(profile);
                self.as_mut()
                    .set_message(QString::from("Saved. Changes apply on the next launch."));
            }
            Err(error) => self
                .as_mut()
                .set_message(QString::from(format!("{error:#}"))),
        }
    }
    pub fn change_patch(self: Pin<&mut Self>, index: i32, action: QString) {
        self.edit(|p| {
            let i = usize::try_from(index)?;
            anyhow::ensure!(i < p.patches.len(), "Patch no longer exists");
            match action.to_string().as_str() {
                "toggle" => p.patches[i].enabled = !p.patches[i].enabled,
                "remove" => {
                    p.patches.remove(i);
                }
                "up" if i > 0 => p.patches.swap(i, i - 1),
                "down" if i + 1 < p.patches.len() => p.patches.swap(i, i + 1),
                _ => bail!("Invalid patch action"),
            }
            Ok(())
        });
    }
    pub fn change_cheat(self: Pin<&mut Self>, index: i32, action: QString) {
        self.edit(|p| {
            let i = usize::try_from(index)?;
            anyhow::ensure!(i < p.cheats.len(), "Cheat no longer exists");
            match action.to_string().as_str() {
                "toggle" => p.cheats[i].enabled = !p.cheats[i].enabled,
                "remove" => {
                    p.cheats.remove(i);
                }
                _ => bail!("Invalid cheat action"),
            }
            Ok(())
        });
    }
    pub fn save_cheat(self: Pin<&mut Self>, index: i32, name: QString, code: QString) {
        self.edit(|p| {
            if index < 0 {
                p.cheats.push(Cheat {
                    name: name.to_string().trim().to_owned(),
                    code: code.to_string().trim().to_owned(),
                    ..Cheat::default()
                });
            } else {
                let cheat = p
                    .cheats
                    .get_mut(index as usize)
                    .context("Cheat no longer exists")?;
                cheat.name = name.to_string().trim().to_owned();
                cheat.code = code.to_string().trim().to_owned();
            }
            Ok(())
        });
    }
    pub fn set_base_hash(self: Pin<&mut Self>, hash: QString) {
        self.edit(|p| {
            p.base_sha256 = hash.to_string().trim().to_ascii_lowercase();
            Ok(())
        });
    }
    pub fn enable_cheats(self: Pin<&mut Self>, enabled: bool) {
        self.edit(|p| {
            p.cheats_enabled = enabled;
            Ok(())
        });
    }
    pub fn import_patch(self: Pin<&mut Self>, path: QString) {
        self.import_file(path, false);
    }
    pub fn import_cheats(self: Pin<&mut Self>, path: QString) {
        self.import_file(path, true);
    }
    fn import_file(mut self: Pin<&mut Self>, path: QString, cheats: bool) {
        if *self.busy() || self.rust().game.is_empty() || path.is_empty() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_message(QString::from("Importing…"));
        let path = match local_path(&path.to_string()) {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_busy(false);
                self.as_mut()
                    .set_message(QString::from(format!("{error:#}")));
                return;
            }
        };
        let game = self.rust().game.clone();
        let generation = self.rust().generation;
        let mut profile = self.rust().profile.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = (|| -> Result<Profile> {
                if cheats {
                    anyhow::ensure!(
                        std::fs::metadata(&path)?.len() <= 4 * 1024 * 1024,
                        "Cheat file exceeds 4 MiB"
                    );
                    profile
                        .cheats
                        .extend(game_mods::import_cheats(&std::fs::read_to_string(&path)?)?);
                } else {
                    let patch = game_mods::import_patch(&path)?;
                    anyhow::ensure!(
                        !profile.patches.iter().any(|p| p.sha256 == patch.sha256),
                        "This patch is already imported"
                    );
                    profile.patches.push(patch);
                }
                profile.save(&crate::settings::SettingsStore::open_default()?, &game)?;
                Ok(profile)
            })();
            let _ = thread.queue(move |mut model| {
                model.as_mut().set_busy(false);
                if model.rust().generation != generation {
                    // Returning to this game during an import must show the
                    // newly persisted profile, not a stale pre-import copy.
                    if model.rust().game == game {
                        if let Ok(profile) = result {
                            model.as_mut().publish(profile);
                        }
                    }
                    return;
                }
                match result {
                    Ok(profile) => {
                        model.as_mut().publish(profile);
                        model.as_mut().set_message(QString::from(
                            "Imported, initially disabled. Enable only the patches/codes you want.",
                        ));
                    }
                    Err(error) => model
                        .as_mut()
                        .set_message(QString::from(format!("Import failed: {error:#}"))),
                }
            });
        });
    }
    pub fn export_cheats(mut self: Pin<&mut Self>, path: QString) {
        if path.is_empty() {
            return;
        }
        let result = game_mods::export_cheats(&self.rust().profile).and_then(|text| {
            std::fs::write(local_path(&path.to_string())?, text).map_err(Into::into)
        });
        self.as_mut().set_message(QString::from(match result {
            Ok(()) => "Cheat file exported.".into(),
            Err(error) => format!("Export failed: {error:#}"),
        }));
    }
}

fn local_path(value: &str) -> Result<std::path::PathBuf> {
    url::Url::parse(value)?
        .to_file_path()
        .map_err(|_| anyhow::anyhow!("Choose a local file"))
}
