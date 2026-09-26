#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, default_mode)]
        #[qproperty(QString, game_mode)]
        #[qproperty(QString, username)]
        #[qproperty(QString, message)]
        #[qproperty(bool, busy)]
        type RetroAchievementsModel = super::RetroAchievementsModelRust;
        #[qinvokable]
        fn refresh(self: Pin<&mut RetroAchievementsModel>);
        #[qinvokable]
        fn select_game(self: Pin<&mut RetroAchievementsModel>, game: QString);
        #[qinvokable]
        fn choose_default(self: Pin<&mut RetroAchievementsModel>, mode: QString);
        #[qinvokable]
        fn choose_game(self: Pin<&mut RetroAchievementsModel>, mode: QString);
        #[qinvokable]
        fn sign_in(self: Pin<&mut RetroAchievementsModel>, username: QString, password: QString);
        #[qinvokable]
        fn sign_out(self: Pin<&mut RetroAchievementsModel>);
    }
    impl cxx_qt::Threading for RetroAchievementsModel {}
}
use crate::{retroachievements as ra, settings::SettingsStore};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::pin::Pin;

pub struct RetroAchievementsModelRust {
    default_mode: QString,
    game_mode: QString,
    username: QString,
    message: QString,
    busy: bool,
    game: String,
}
impl Default for RetroAchievementsModelRust {
    fn default() -> Self {
        Self {
            default_mode: "emulator".into(),
            game_mode: "inherit".into(),
            username: QString::default(),
            message: QString::default(),
            busy: false,
            game: String::new(),
        }
    }
}
impl qobject::RetroAchievementsModel {
    fn publish_modes(mut self: Pin<&mut Self>) {
        let result = (|| -> anyhow::Result<_> {
            let store = SettingsStore::open_default()?;
            let global = ra::preference(&store, "")?.unwrap_or_default();
            let game = if self.rust().game.is_empty() {
                None
            } else {
                ra::preference(&store, &self.rust().game)?
            };
            Ok((global, game))
        })();
        match result {
            Ok((global, game)) => {
                self.as_mut().set_default_mode(global.key().into());
                self.as_mut()
                    .set_game_mode(game.map(|m| m.key()).unwrap_or("inherit").into());
            }
            Err(error) => self
                .as_mut()
                .set_message(QString::from(format!("{error:#}"))),
        }
    }
    pub fn select_game(mut self: Pin<&mut Self>, game: QString) {
        self.as_mut().rust_mut().game = game.to_string();
        self.as_mut().publish_modes();
    }
    pub fn choose_default(self: Pin<&mut Self>, mode: QString) {
        self.choose(mode, false);
    }
    pub fn choose_game(self: Pin<&mut Self>, mode: QString) {
        self.choose(mode, true);
    }
    fn choose(mut self: Pin<&mut Self>, mode: QString, for_game: bool) {
        if *self.busy() || (for_game && self.rust().game.is_empty()) {
            return;
        }
        let game = if for_game {
            self.rust().game.clone()
        } else {
            String::new()
        };
        let result = SettingsStore::open_default()
            .and_then(|store| ra::save_preference(&store, &game, &mode.to_string()));
        self.as_mut().publish_modes();
        self.set_message(QString::from(match result {
            Ok(()) => "Saved. Applies on the next game launch.".to_owned(),
            Err(error) => format!("{error:#}"),
        }));
    }
    pub fn refresh(mut self: Pin<&mut Self>) {
        self.as_mut().publish_modes();
        self.account_task(|| {
            Ok((
                ra::credentials()?.map(|c| c.username).unwrap_or_default(),
                String::new(),
            ))
        });
    }
    pub fn sign_in(self: Pin<&mut Self>, username: QString, password: QString) {
        let (username, password) = (username.to_string(), password.to_string());
        self.account_task(move || {
            let username = ra::sign_in(&username, &password)?;
            Ok((username, "Signed in. Choose Casual or Hardcore below to enable achievements for RetroArch launches.".into()))
        });
    }
    pub fn sign_out(self: Pin<&mut Self>) {
        self.account_task(|| {
            // Do not leave the global setting demanding missing credentials.
            // Explicit per-game Casual/Hardcore settings still require sign-in.
            ra::sign_out()?;
            ra::save_preference(&SettingsStore::open_default()?, "", "off")?;
            Ok((String::new(), "Signed out of Lunchbox. RetroArch's separately saved account and any running game are unchanged.".into()))
        });
    }
    fn account_task(
        mut self: Pin<&mut Self>,
        task: impl FnOnce() -> anyhow::Result<(String, String)> + Send + 'static,
    ) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message("Contacting the account / credential store…".into());
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = task();
            let _ = thread.queue(move |mut model| {
                model.as_mut().set_busy(false);
                model.as_mut().publish_modes();
                match result {
                    Ok((username, message)) => {
                        model.as_mut().set_username(QString::from(username));
                        model.as_mut().set_message(QString::from(message));
                    }
                    Err(error) => model
                        .as_mut()
                        .set_message(QString::from(format!("{error:#}"))),
                }
            });
        });
    }
}
