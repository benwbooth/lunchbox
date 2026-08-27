#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");
        type QString = cxx_qt_lib::QString;
        type QUrl = cxx_qt_lib::QUrl;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, password_saved)]
        #[qproperty(bool, connection_ok)]
        #[qproperty(QString, message)]
        #[qproperty(QString, state_database_path)]
        #[qproperty(QString, qbittorrent_host)]
        #[qproperty(i32, qbittorrent_port)]
        #[qproperty(bool, qbittorrent_use_https)]
        #[qproperty(QString, qbittorrent_username)]
        #[qproperty(QString, qbittorrent_password)]
        #[qproperty(QString, rom_directory)]
        #[qproperty(QString, qbittorrent_container_rom_directory)]
        #[qproperty(QString, torrent_library_directory)]
        #[qproperty(QString, qbittorrent_container_torrent_library_directory)]
        #[qproperty(bool, download_entire_torrent)]
        #[qproperty(QString, file_link_mode)]
        type SettingsModel = super::SettingsModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn save(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn test_connection(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn clear_password(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn set_directory(self: Pin<&mut SettingsModel>, field: QString, url: QUrl);
    }

    impl cxx_qt::Threading for SettingsModel {}
}

use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::Threading;
use cxx_qt_lib::{QString, QUrl};

use crate::qbittorrent;
use crate::settings::{self, AppSettings, SettingsStore};

pub struct SettingsModelRust {
    initialized: bool,
    busy: bool,
    password_saved: bool,
    connection_ok: bool,
    message: QString,
    state_database_path: QString,
    qbittorrent_host: QString,
    qbittorrent_port: i32,
    qbittorrent_use_https: bool,
    qbittorrent_username: QString,
    qbittorrent_password: QString,
    rom_directory: QString,
    qbittorrent_container_rom_directory: QString,
    torrent_library_directory: QString,
    qbittorrent_container_torrent_library_directory: QString,
    download_entire_torrent: bool,
    file_link_mode: QString,
}

impl Default for SettingsModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            password_saved: false,
            connection_ok: false,
            message: QString::from("Loading settings…"),
            state_database_path: QString::default(),
            qbittorrent_host: QString::from("127.0.0.1"),
            qbittorrent_port: 8080,
            qbittorrent_use_https: false,
            qbittorrent_username: QString::default(),
            qbittorrent_password: QString::default(),
            rom_directory: QString::default(),
            qbittorrent_container_rom_directory: QString::default(),
            torrent_library_directory: QString::default(),
            qbittorrent_container_torrent_library_directory: QString::default(),
            download_entire_torrent: false,
            file_link_mode: QString::from("symlink"),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::SettingsModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring("Loading settings…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-settings-load".into())
            .spawn(move || {
                let loaded = load_settings().map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_initialize(loaded);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start settings worker: {error}")));
        }
    }

    fn finish_initialize(
        mut self: Pin<&mut Self>,
        loaded: Result<(AppSettings, bool, PathBuf), String>,
    ) {
        self.as_mut().set_busy(false);
        match loaded {
            Ok((settings, password_saved, path)) => {
                self.as_mut().apply_settings(settings);
                self.as_mut().set_password_saved(password_saved);
                self.as_mut()
                    .set_state_database_path(qstring(path.to_string_lossy()));
                self.as_mut().set_initialized(true);
                self.as_mut().set_message(qstring(if password_saved {
                    "Settings loaded; the qBittorrent password is in your system credential store."
                } else {
                    "Settings loaded. Configure qBittorrent to enable Minerva downloads."
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load settings: {error}"))),
        }
    }

    pub fn save(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let settings = match self.as_ref().settings_snapshot() {
            Ok(settings) => settings,
            Err(error) => {
                self.as_mut().set_connection_ok(false);
                self.as_mut().set_message(qstring(error));
                return;
            }
        };
        let password = self.as_ref().qbittorrent_password().to_string();
        self.as_mut().set_busy(true);
        self.as_mut().set_connection_ok(false);
        self.as_mut().set_message(qstring("Saving settings…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-settings-save".into())
            .spawn(move || {
                let password_changed = !password.is_empty();
                let saved = save_settings(&settings, &password).map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_save(saved, password_changed);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start settings worker: {error}")));
        }
    }

    fn finish_save(
        mut self: Pin<&mut Self>,
        saved: Result<PathBuf, String>,
        password_changed: bool,
    ) {
        self.as_mut().set_busy(false);
        match saved {
            Ok(path) => {
                if password_changed {
                    self.as_mut().set_password_saved(true);
                    self.as_mut().set_qbittorrent_password(QString::default());
                }
                self.as_mut()
                    .set_state_database_path(qstring(path.to_string_lossy()));
                self.as_mut().set_message(qstring(
                    "Settings saved. Test the connection before downloading.",
                ));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save settings: {error}"))),
        }
    }

    pub fn test_connection(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let settings = match self.as_ref().settings_snapshot() {
            Ok(settings) => settings,
            Err(error) => {
                self.as_mut().set_connection_ok(false);
                self.as_mut().set_message(qstring(error));
                return;
            }
        };
        let entered_password = self.as_ref().qbittorrent_password().to_string();
        self.as_mut().set_busy(true);
        self.as_mut().set_connection_ok(false);
        self.as_mut()
            .set_message(qstring("Testing qBittorrent connection…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-qbittorrent-test".into())
            .spawn(move || {
                let tested = effective_password(entered_password)
                    .and_then(|password| qbittorrent::test_connection(&settings, &password))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_test(tested);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start connection worker: {error}"
            )));
        }
    }

    fn finish_test(mut self: Pin<&mut Self>, tested: Result<String, String>) {
        self.as_mut().set_busy(false);
        match tested {
            Ok(version) => {
                self.as_mut().set_connection_ok(true);
                self.as_mut()
                    .set_message(qstring(format!("Connected to qBittorrent {version}.")));
            }
            Err(error) => {
                self.as_mut().set_connection_ok(false);
                self.as_mut()
                    .set_message(qstring(format!("Connection failed: {error}")));
            }
        }
    }

    pub fn clear_password(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_connection_ok(false);
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-password-clear".into())
            .spawn(move || {
                let cleared = settings::save_password("").map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_busy(false);
                    match cleared {
                        Ok(()) => {
                            model.as_mut().set_password_saved(false);
                            model.as_mut().set_qbittorrent_password(QString::default());
                            model
                                .as_mut()
                                .set_message(qstring("Saved qBittorrent password removed."));
                        }
                        Err(error) => model
                            .as_mut()
                            .set_message(qstring(format!("Could not remove password: {error}"))),
                    }
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start password worker: {error}")));
        }
    }

    pub fn set_directory(mut self: Pin<&mut Self>, field: QString, url: QUrl) {
        let Some(path) = url.to_local_file() else {
            self.as_mut()
                .set_message(qstring("Choose a local filesystem directory."));
            return;
        };
        match field.to_string().as_str() {
            "rom" => self.as_mut().set_rom_directory(path),
            "torrent" => self.as_mut().set_torrent_library_directory(path),
            _ => self
                .as_mut()
                .set_message(qstring("Unknown settings directory field.")),
        }
    }

    fn apply_settings(mut self: Pin<&mut Self>, settings: AppSettings) {
        self.as_mut()
            .set_qbittorrent_host(qstring(settings.qbittorrent_host));
        self.as_mut()
            .set_qbittorrent_port(i32::from(settings.qbittorrent_port));
        self.as_mut()
            .set_qbittorrent_use_https(settings.qbittorrent_use_https);
        self.as_mut()
            .set_qbittorrent_username(qstring(settings.qbittorrent_username));
        self.as_mut()
            .set_rom_directory(qstring(settings.rom_directory.to_string_lossy()));
        self.as_mut()
            .set_qbittorrent_container_rom_directory(qstring(
                settings.qbittorrent_container_rom_directory,
            ));
        self.as_mut().set_torrent_library_directory(qstring(
            settings.torrent_library_directory.to_string_lossy(),
        ));
        self.as_mut()
            .set_qbittorrent_container_torrent_library_directory(qstring(
                settings.qbittorrent_container_torrent_library_directory,
            ));
        self.as_mut()
            .set_download_entire_torrent(settings.download_entire_torrent);
        self.as_mut()
            .set_file_link_mode(qstring(settings.file_link_mode));
    }

    fn settings_snapshot(&self) -> Result<AppSettings, String> {
        let port = u16::try_from(*self.qbittorrent_port())
            .map_err(|_| "qBittorrent port must be between 1 and 65535".to_owned())?;
        let settings = AppSettings {
            qbittorrent_host: self.qbittorrent_host().to_string(),
            qbittorrent_port: port,
            qbittorrent_use_https: *self.qbittorrent_use_https(),
            qbittorrent_username: self.qbittorrent_username().to_string(),
            rom_directory: PathBuf::from(self.rom_directory().to_string()),
            qbittorrent_container_rom_directory: self
                .qbittorrent_container_rom_directory()
                .to_string(),
            torrent_library_directory: PathBuf::from(self.torrent_library_directory().to_string()),
            qbittorrent_container_torrent_library_directory: self
                .qbittorrent_container_torrent_library_directory()
                .to_string(),
            download_entire_torrent: *self.download_entire_torrent(),
            file_link_mode: self.file_link_mode().to_string(),
        };
        settings.validate().map_err(|error| error.to_string())?;
        Ok(settings)
    }
}

fn load_settings() -> anyhow::Result<(AppSettings, bool, PathBuf)> {
    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    let password_saved = settings::load_password()?.is_some();
    Ok((settings, password_saved, store.path().to_owned()))
}

fn save_settings(settings: &AppSettings, password: &str) -> anyhow::Result<PathBuf> {
    let store = SettingsStore::open_default()?;
    store.save(settings)?;
    if !password.is_empty() {
        settings::save_password(password)?;
    }
    Ok(store.path().to_owned())
}

fn effective_password(entered_password: String) -> anyhow::Result<String> {
    if entered_password.is_empty() {
        Ok(settings::load_password()?.unwrap_or_default())
    } else {
        Ok(entered_password)
    }
}
