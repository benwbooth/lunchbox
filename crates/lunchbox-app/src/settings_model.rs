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
        #[qproperty(bool, profile_busy)]
        #[qproperty(QString, profile_message)]
        #[qproperty(bool, profile_restore_ready)]
        #[qproperty(QString, profile_restore_summary)]
        #[qproperty(bool, profile_restart_required)]
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
        #[qproperty(QString, seeding_policy)]
        #[qproperty(QString, preferred_region)]
        #[qproperty(i32, region_revision)]
        #[qproperty(QString, version_preference)]
        #[qproperty(i32, media_provider_revision)]
        #[qproperty(bool, shader_busy)]
        #[qproperty(i32, shader_progress)]
        #[qproperty(QString, shader_message)]
        #[qproperty(bool, shader_requires_confirmation)]
        #[qproperty(i32, shader_revision)]
        #[qproperty(bool, controller_enabled)]
        #[qproperty(QString, controller_output_target)]
        #[qproperty(bool, controller_busy)]
        #[qproperty(i32, controller_revision)]
        #[qproperty(QString, controller_status)]
        #[qproperty(bool, controller_profile_editor_open)]
        #[qproperty(QString, controller_profile_editor_id)]
        #[qproperty(QString, controller_profile_editor_name)]
        #[qproperty(QString, controller_profile_editor_layout)]
        #[qproperty(QString, controller_profile_editor_target)]
        #[qproperty(QString, controller_profile_editor_status)]
        #[qproperty(i32, controller_profile_revision)]
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

        #[qinvokable]
        fn export_profile(self: Pin<&mut SettingsModel>, destination: QUrl);

        #[qinvokable]
        fn inspect_profile(self: Pin<&mut SettingsModel>, source: QUrl);

        #[qinvokable]
        fn stage_profile_restore(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn cancel_profile_restore(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn cancel_staged_profile_restore(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn region_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn region_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_region(self: Pin<&mut SettingsModel>, from: i32, to: i32);

        #[qinvokable]
        fn reset_region_priority(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn media_provider_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn media_provider_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn media_provider_description_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_media_provider(self: Pin<&mut SettingsModel>, from: i32, to: i32);

        #[qinvokable]
        fn reset_media_provider_priority(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn refresh_retroarch_shaders(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn install_retroarch_shaders(self: Pin<&mut SettingsModel>, replace_unmanaged: bool);

        #[qinvokable]
        fn cancel_retroarch_shaders(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn shader_target_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn shader_target_path_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn shader_target_detail_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn open_shader_target(self: Pin<&mut SettingsModel>, index: i32);

        #[qinvokable]
        fn refresh_controllers(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn set_controller_mapping_enabled(self: Pin<&mut SettingsModel>, enabled: bool);

        #[qinvokable]
        fn choose_default_controller_target(self: Pin<&mut SettingsModel>, target: QString);

        #[qinvokable]
        fn controller_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_detail_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_action_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_target_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_controller(self: Pin<&mut SettingsModel>, index: i32, direction: i32);

        #[qinvokable]
        fn choose_controller_action(self: Pin<&mut SettingsModel>, index: i32, action: QString);

        #[qinvokable]
        fn choose_controller_profile(self: Pin<&mut SettingsModel>, index: i32, profile: QString);

        #[qinvokable]
        fn choose_controller_target(self: Pin<&mut SettingsModel>, index: i32, target: QString);

        #[qinvokable]
        fn controller_profile_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_profile_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_target_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_target_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_target_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_controller_profile_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn custom_controller_profile_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_controller_profile_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn custom_controller_profile_detail_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn create_controller_profile(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn edit_controller_profile(self: Pin<&mut SettingsModel>, index: i32);

        #[qinvokable]
        fn delete_controller_profile(self: Pin<&mut SettingsModel>, index: i32);

        #[qinvokable]
        fn cancel_controller_profile_edit(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn save_controller_profile(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn select_controller_profile_target(self: Pin<&mut SettingsModel>, target: QString);

        #[qinvokable]
        fn controller_profile_button_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_profile_button_id_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_button_name_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_button_label(self: &SettingsModel, button: QString) -> QString;

        #[qinvokable]
        fn controller_profile_selected_source(self: &SettingsModel) -> QString;

        #[qinvokable]
        fn choose_controller_profile_source(self: Pin<&mut SettingsModel>, source: QString);

        #[qinvokable]
        fn controller_profile_mapping_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn controller_profile_mapping_source_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn controller_profile_mapping_target_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn apply_two_button_controller_preset(self: Pin<&mut SettingsModel>);

        #[qinvokable]
        fn clear_controller_profile_mappings(self: Pin<&mut SettingsModel>);
    }

    impl cxx_qt::Threading for SettingsModel {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::controllers::{self, ControllerDevice, ControllerInventory};
use crate::profile_backup;
use crate::qbittorrent;
use crate::retroarch_shaders::{self, ShaderInstallSummary, ShaderInventory, ShaderProgress};
use crate::settings::{
    self, AppSettings, ControllerButtonMapping, ControllerCustomProfile, ControllerMappingSettings,
    SettingsStore,
};

pub struct SettingsModelRust {
    initialized: bool,
    busy: bool,
    password_saved: bool,
    connection_ok: bool,
    message: QString,
    state_database_path: QString,
    profile_busy: bool,
    profile_message: QString,
    profile_restore_ready: bool,
    profile_restore_summary: QString,
    profile_restart_required: bool,
    profile_restore_source: Option<PathBuf>,
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
    seeding_policy: QString,
    preferred_region: QString,
    region_priority: Vec<String>,
    region_revision: i32,
    version_preference: QString,
    media_provider_priority: Vec<String>,
    media_provider_revision: i32,
    shader_busy: bool,
    shader_progress: i32,
    shader_message: QString,
    shader_requires_confirmation: bool,
    shader_revision: i32,
    shader_inventory: ShaderInventory,
    shader_generation: u64,
    shader_cancel: Option<Arc<AtomicBool>>,
    controller_enabled: bool,
    controller_output_target: QString,
    controller_busy: bool,
    controller_revision: i32,
    controller_status: QString,
    controller_mapping: ControllerMappingSettings,
    controller_inventory: Option<ControllerInventory>,
    controller_profile_editor_open: bool,
    controller_profile_editor_id: QString,
    controller_profile_editor_name: QString,
    controller_profile_editor_layout: QString,
    controller_profile_editor_target: QString,
    controller_profile_editor_status: QString,
    controller_profile_revision: i32,
    controller_profile_editor_mappings: Vec<ControllerButtonMapping>,
    controller_profile_pending_controller_id: Option<String>,
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
            profile_busy: false,
            profile_message: QString::from(
                "Create a portable backup of saved collection state and preferences.",
            ),
            profile_restore_ready: false,
            profile_restore_summary: QString::default(),
            profile_restart_required: false,
            profile_restore_source: None,
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
            seeding_policy: QString::from("follow_client"),
            preferred_region: QString::from("USA"),
            region_priority: crate::region_priority::default_region_priority(),
            region_revision: 0,
            version_preference: QString::from("latest"),
            media_provider_priority: crate::media::default_provider_priority(),
            media_provider_revision: 0,
            shader_busy: false,
            shader_progress: 0,
            shader_message: QString::from(
                "Open Settings to inspect this computer's RetroArch shader folders.",
            ),
            shader_requires_confirmation: false,
            shader_revision: 0,
            shader_inventory: ShaderInventory::default(),
            shader_generation: 0,
            shader_cancel: None,
            controller_enabled: false,
            controller_output_target: QString::from("xb360"),
            controller_busy: false,
            controller_revision: 0,
            controller_status: QString::from("Open controller settings to scan this computer."),
            controller_mapping: ControllerMappingSettings::default(),
            controller_inventory: None,
            controller_profile_editor_open: false,
            controller_profile_editor_id: QString::default(),
            controller_profile_editor_name: QString::from("Custom profile"),
            controller_profile_editor_layout: QString::from("xbox"),
            controller_profile_editor_target: QString::from("South"),
            controller_profile_editor_status: QString::default(),
            controller_profile_revision: 0,
            controller_profile_editor_mappings: Vec::new(),
            controller_profile_pending_controller_id: None,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn shader_ui_probe_enabled() -> bool {
    std::env::args_os().any(|argument| argument == "--retroarch-shader-ui-probe")
}

fn shader_probe_target() -> Option<PathBuf> {
    shader_ui_probe_enabled()
        .then(|| crate::catalog::requested_path("--shader-target", "LUNCHBOX_SHADER_TARGET"))
        .flatten()
}

fn shader_probe_archive_directory() -> Option<PathBuf> {
    shader_ui_probe_enabled()
        .then(|| {
            crate::catalog::requested_path(
                "--shader-archive-directory",
                "LUNCHBOX_SHADER_ARCHIVE_DIRECTORY",
            )
        })
        .flatten()
}

fn format_shader_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
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
        loaded: Result<
            (
                AppSettings,
                bool,
                PathBuf,
                Option<profile_backup::ProfileSummary>,
            ),
            String,
        >,
    ) {
        self.as_mut().set_busy(false);
        match loaded {
            Ok((settings, password_saved, path, restored_profile)) => {
                self.as_mut().apply_settings(settings);
                self.as_mut().set_password_saved(password_saved);
                self.as_mut()
                    .set_state_database_path(qstring(path.to_string_lossy()));
                self.as_mut().set_initialized(true);
                if let Some(summary) = restored_profile {
                    self.as_mut().set_profile_message(qstring(format!(
                        "Profile restored safely at startup: {}. Credentials stayed in this computer's credential store.",
                        summary.description()
                    )));
                }
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

    pub fn region_count(&self) -> i32 {
        i32::try_from(self.rust().region_priority.len()).unwrap_or(i32::MAX)
    }

    pub fn region_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().region_priority.get(index))
            .map(|region| qstring(crate::region_priority::display_name(region)))
            .unwrap_or_default()
    }

    pub fn move_region(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let Ok(from) = usize::try_from(from) else {
            return;
        };
        let Ok(to) = usize::try_from(to) else {
            return;
        };
        let count = self.as_ref().rust().region_priority.len();
        if from >= count || to >= count || from == to {
            return;
        }
        let region = self.as_mut().rust_mut().region_priority.remove(from);
        self.as_mut().rust_mut().region_priority.insert(to, region);
        self.as_mut().sync_primary_region();
        self.as_mut().bump_region_revision();
        self.as_mut().set_connection_ok(false);
        self.as_mut().set_message(qstring(
            "Region priority changed. Save settings to apply it to Minerva results.",
        ));
    }

    pub fn reset_region_priority(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().region_priority =
            crate::region_priority::default_region_priority();
        self.as_mut().sync_primary_region();
        self.as_mut().bump_region_revision();
        self.as_mut().set_connection_ok(false);
        self.as_mut().set_message(qstring(
            "Default region priority restored. Save settings to apply it.",
        ));
    }

    pub fn media_provider_count(&self) -> i32 {
        i32::try_from(self.rust().media_provider_priority.len()).unwrap_or(i32::MAX)
    }

    pub fn media_provider_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().media_provider_priority.get(index))
            .map(|provider| qstring(crate::media::provider_display_name(provider)))
            .unwrap_or_default()
    }

    pub fn media_provider_description_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().media_provider_priority.get(index))
            .map(|provider| qstring(crate::media::provider_description(provider)))
            .unwrap_or_default()
    }

    pub fn move_media_provider(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let Ok(from) = usize::try_from(from) else {
            return;
        };
        let Ok(to) = usize::try_from(to) else {
            return;
        };
        let count = self.as_ref().rust().media_provider_priority.len();
        if from >= count || to >= count || from == to {
            return;
        }
        let provider = self
            .as_mut()
            .rust_mut()
            .media_provider_priority
            .remove(from);
        self.as_mut()
            .rust_mut()
            .media_provider_priority
            .insert(to, provider);
        self.as_mut().bump_media_provider_revision();
        self.as_mut().set_connection_ok(false);
        self.as_mut().set_message(qstring(
            "Media source priority changed. Save settings to reindex cached media.",
        ));
    }

    pub fn reset_media_provider_priority(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().media_provider_priority =
            crate::media::default_provider_priority();
        self.as_mut().bump_media_provider_revision();
        self.as_mut().set_connection_ok(false);
        self.as_mut().set_message(qstring(
            "Default media source priority restored. Save settings to reindex cached media.",
        ));
    }

    pub fn refresh_retroarch_shaders(mut self: Pin<&mut Self>) {
        if *self.as_ref().shader_busy() {
            return;
        }
        self.as_mut().rust_mut().shader_generation =
            self.as_ref().rust().shader_generation.wrapping_add(1);
        let generation = self.as_ref().rust().shader_generation;
        let target_override = shader_probe_target();
        self.as_mut().set_shader_busy(true);
        self.as_mut().set_shader_progress(0);
        self.as_mut()
            .set_shader_message(qstring("Inspecting RetroArch shader folders…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-shader-inventory".into())
            .spawn(move || {
                let result = retroarch_shaders::inventory(target_override)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_shader_refresh(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_shader_busy(false);
            self.as_mut().set_shader_message(qstring(format!(
                "Could not start the RetroArch shader scan: {error}"
            )));
        }
    }

    fn finish_shader_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<ShaderInventory, String>,
    ) {
        if generation != self.as_ref().rust().shader_generation {
            return;
        }
        self.as_mut().set_shader_busy(false);
        match result {
            Ok(inventory) => {
                self.as_mut().apply_shader_inventory(inventory);
                let target_count = self.as_ref().rust().shader_inventory.targets.len();
                let managed_count = self
                    .as_ref()
                    .rust()
                    .shader_inventory
                    .installed_target_count();
                let message = if target_count == 0 {
                    "No RetroArch shader directory could be resolved on this computer.".to_owned()
                } else if managed_count == target_count {
                    format!(
                        "Slang and GLSL packs are managed in {managed_count} RetroArch target{}.",
                        if managed_count == 1 { "" } else { "s" }
                    )
                } else if *self.as_ref().shader_requires_confirmation() {
                    "Existing shader folders were found. Review and confirm before Lunchbox replaces only the official Slang and GLSL pack folders.".to_owned()
                } else {
                    "The official Slang and GLSL packs are ready to install.".to_owned()
                };
                self.as_mut().set_shader_message(qstring(message));
            }
            Err(error) => self.as_mut().set_shader_message(qstring(format!(
                "Could not inspect RetroArch shaders: {error}"
            ))),
        }
        self.as_mut().bump_shader_revision();
    }

    pub fn install_retroarch_shaders(mut self: Pin<&mut Self>, replace_unmanaged: bool) {
        if *self.as_ref().shader_busy() {
            return;
        }
        self.as_mut().rust_mut().shader_generation =
            self.as_ref().rust().shader_generation.wrapping_add(1);
        let generation = self.as_ref().rust().shader_generation;
        let target_override = shader_probe_target();
        let archive_directory = shader_probe_archive_directory();
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().shader_cancel = Some(Arc::clone(&cancel));
        self.as_mut().set_shader_busy(true);
        self.as_mut().set_shader_progress(0);
        self.as_mut()
            .set_shader_message(qstring("Preparing verified RetroArch shader archives…"));
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress: Arc<dyn Fn(ShaderProgress) + Send + Sync> = Arc::new(move |progress| {
            let _ = progress_thread.queue(move |mut model| {
                model.as_mut().update_shader_progress(generation, progress);
            });
        });
        let spawn = std::thread::Builder::new()
            .name("lunchbox-shader-install".into())
            .spawn(move || {
                let result = retroarch_shaders::install(
                    target_override.clone(),
                    archive_directory,
                    replace_unmanaged,
                    &cancel,
                    progress,
                )
                .and_then(|summary| {
                    let inventory = retroarch_shaders::inventory(target_override)?;
                    Ok((summary, inventory))
                })
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_shader_install(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().rust_mut().shader_cancel = None;
            self.as_mut().set_shader_busy(false);
            self.as_mut().set_shader_message(qstring(format!(
                "Could not start the RetroArch shader installer: {error}"
            )));
        }
    }

    pub fn cancel_retroarch_shaders(mut self: Pin<&mut Self>) {
        if let Some(cancel) = self.as_ref().rust().shader_cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
            self.as_mut().set_shader_message(qstring(
                "Cancelling safely; installed packs remain unchanged until publication…",
            ));
        }
    }

    fn update_shader_progress(mut self: Pin<&mut Self>, generation: u64, progress: ShaderProgress) {
        if generation != self.as_ref().rust().shader_generation || !*self.as_ref().shader_busy() {
            return;
        }
        self.as_mut()
            .set_shader_progress(progress.percent.clamp(0, 100));
        self.as_mut().set_shader_message(qstring(progress.message));
    }

    fn finish_shader_install(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<(ShaderInstallSummary, ShaderInventory), String>,
    ) {
        if generation != self.as_ref().rust().shader_generation {
            return;
        }
        self.as_mut().rust_mut().shader_cancel = None;
        self.as_mut().set_shader_busy(false);
        match result {
            Ok((summary, inventory)) => {
                self.as_mut().apply_shader_inventory(inventory);
                self.as_mut().set_shader_progress(100);
                self.as_mut().set_shader_message(qstring(format!(
                    "Installed {} verified shader files ({}) in {} RetroArch target{}{}.",
                    summary.file_count,
                    format_shader_bytes(summary.unpacked_bytes),
                    summary.target_count,
                    if summary.target_count == 1 { "" } else { "s" },
                    if summary.reused_archives == 0 {
                        String::new()
                    } else {
                        format!(" · {} verified archives reused", summary.reused_archives)
                    }
                )));
            }
            Err(error) => {
                self.as_mut().set_shader_progress(0);
                self.as_mut().set_shader_message(qstring(format!(
                    "RetroArch shader installation failed: {error}"
                )));
            }
        }
        self.as_mut().bump_shader_revision();
    }

    fn apply_shader_inventory(mut self: Pin<&mut Self>, inventory: ShaderInventory) {
        self.as_mut()
            .set_shader_requires_confirmation(inventory.requires_confirmation());
        self.as_mut().rust_mut().shader_inventory = inventory;
    }

    fn bump_shader_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().shader_revision().wrapping_add(1);
        self.as_mut().set_shader_revision(revision);
    }

    pub fn shader_target_count(&self) -> i32 {
        i32::try_from(self.rust().shader_inventory.targets.len()).unwrap_or(i32::MAX)
    }

    pub fn shader_target_path_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().shader_inventory.targets.get(index))
            .map(|target| qstring(target.path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn shader_target_detail_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().shader_inventory.targets.get(index))
            .map(|target| qstring(target.detail()))
            .unwrap_or_default()
    }

    pub fn open_shader_target(mut self: Pin<&mut Self>, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            self.as_mut()
                .set_shader_message(qstring("Choose a valid RetroArch shader target."));
            return;
        };
        let path = {
            let this = self.as_ref();
            this.rust()
                .shader_inventory
                .targets
                .get(index)
                .map(|target| target.path.clone())
        };
        let Some(path) = path else {
            self.as_mut()
                .set_shader_message(qstring("Choose a valid RetroArch shader target."));
            return;
        };
        match retroarch_shaders::open_target(&path) {
            Ok(()) => self
                .as_mut()
                .set_shader_message(qstring(format!("Opened {}.", path.display()))),
            Err(error) => self.as_mut().set_shader_message(qstring(format!(
                "Could not open the RetroArch shader folder: {error}"
            ))),
        }
    }

    pub fn refresh_controllers(mut self: Pin<&mut Self>) {
        if *self.as_ref().controller_busy() {
            return;
        }
        self.as_mut().set_controller_busy(true);
        self.as_mut().set_controller_status(qstring(
            "Checking connected controllers and remapping support…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn_result = std::thread::Builder::new()
            .name("lunchbox-controller-inventory".into())
            .spawn(move || {
                let inventory = controllers::controller_inventory();
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_controller_refresh(inventory);
                });
            });
        if let Err(error) = spawn_result {
            self.as_mut().set_controller_busy(false);
            self.as_mut().set_controller_status(qstring(format!(
                "Could not start controller scan: {error}"
            )));
        }
    }

    fn finish_controller_refresh(mut self: Pin<&mut Self>, inventory: ControllerInventory) {
        let status = controller_inventory_status(&inventory);
        if std::env::args().any(|argument| argument == "--controller-ui-probe") {
            println!(
                "LUNCHBOX_CONTROLLER_UI_READY controllers={} managed={} targets={} status={status:?}",
                inventory.controllers.len(),
                inventory.managed_device_count,
                inventory.supported_targets.len()
            );
        }
        self.as_mut().rust_mut().controller_inventory = Some(inventory);
        self.as_mut().set_controller_busy(false);
        self.as_mut().set_controller_status(qstring(status));
        self.as_mut().bump_controller_revision();
    }

    pub fn set_controller_mapping_enabled(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut().set_controller_enabled(enabled);
        self.as_mut().rust_mut().controller_mapping.enabled = enabled;
        self.as_mut().controller_settings_changed();
    }

    pub fn choose_default_controller_target(mut self: Pin<&mut Self>, target: QString) {
        let target = target.to_string();
        let target = target.trim();
        let supported = self
            .as_ref()
            .rust()
            .controller_inventory
            .as_ref()
            .is_some_and(|inventory| {
                inventory
                    .supported_targets
                    .iter()
                    .any(|candidate| candidate.id == target)
            });
        if !supported {
            return;
        }
        self.as_mut().set_controller_output_target(qstring(target));
        self.as_mut().rust_mut().controller_mapping.output_target = target.to_owned();
        self.as_mut().controller_settings_changed();
    }

    pub fn controller_count(&self) -> i32 {
        self.rust()
            .controller_inventory
            .as_ref()
            .map(|inventory| {
                controllers::ordered_controllers(inventory, &self.rust().controller_mapping).len()
            })
            .and_then(|count| i32::try_from(count).ok())
            .unwrap_or(0)
    }

    pub fn controller_name_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| qstring(controller.name))
            .unwrap_or_default()
    }

    pub fn controller_detail_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                let vid_pid = match (
                    controller.vendor_id.as_deref(),
                    controller.product_id.as_deref(),
                ) {
                    (Some(vendor), Some(product)) => format!("VID:PID {vendor}:{product}"),
                    _ => "VID:PID unknown".to_owned(),
                };
                let serial = controller
                    .unique_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("no serial");
                qstring(format!("{vid_pid} · {serial} · {}", controller.stable_id))
            })
            .unwrap_or_default()
    }

    pub fn controller_action_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                qstring(controllers::controller_action(
                    &self.rust().controller_mapping,
                    &controller.stable_id,
                ))
            })
            .unwrap_or_default()
    }

    pub fn controller_profile_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                qstring(controllers::controller_profile(
                    &self.rust().controller_mapping,
                    &controller.stable_id,
                ))
            })
            .unwrap_or_default()
    }

    pub fn controller_target_at(&self, index: i32) -> QString {
        self.controller_at(index)
            .map(|controller| {
                qstring(controllers::controller_target(
                    &self.rust().controller_mapping,
                    &controller.stable_id,
                ))
            })
            .unwrap_or_default()
    }

    pub fn move_controller(mut self: Pin<&mut Self>, index: i32, direction: i32) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        let Some(inventory) = self.as_ref().rust().controller_inventory.clone() else {
            return;
        };
        if controllers::move_controller(
            &inventory,
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            direction as isize,
        ) {
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn choose_controller_action(mut self: Pin<&mut Self>, index: i32, action: QString) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        if controllers::set_controller_action(
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            action.to_string().trim(),
        ) {
            self.as_mut().set_controller_enabled(true);
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn choose_controller_profile(mut self: Pin<&mut Self>, index: i32, profile: QString) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        if profile.to_string().trim() == controllers::PROFILE_CREATE {
            self.as_mut()
                .open_new_controller_profile(Some(controller_id));
            return;
        }
        if controllers::set_controller_profile(
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            profile.to_string().trim(),
        ) {
            self.as_mut().set_controller_enabled(true);
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn choose_controller_target(mut self: Pin<&mut Self>, index: i32, target: QString) {
        let Some(controller_id) = self
            .as_ref()
            .controller_at(index)
            .map(|controller| controller.stable_id)
        else {
            return;
        };
        let Some(inventory) = self.as_ref().rust().controller_inventory.clone() else {
            return;
        };
        if controllers::set_controller_target(
            &inventory,
            &mut self.as_mut().rust_mut().controller_mapping,
            &controller_id,
            target.to_string().trim(),
        ) {
            self.as_mut().set_controller_enabled(true);
            self.as_mut().controller_settings_changed();
        }
    }

    pub fn controller_profile_count(&self) -> i32 {
        let count = 3
            + controllers::built_in_profiles().len()
            + self.rust().controller_mapping.custom_profiles.len();
        i32::try_from(count).unwrap_or(i32::MAX)
    }

    pub fn controller_profile_id_at(&self, index: i32) -> QString {
        controller_profile_option(&self.rust().controller_mapping, index)
            .map(|(id, _)| qstring(id))
            .unwrap_or_default()
    }

    pub fn controller_profile_name_at(&self, index: i32) -> QString {
        controller_profile_option(&self.rust().controller_mapping, index)
            .map(|(_, name)| qstring(name))
            .unwrap_or_default()
    }

    pub fn controller_target_count(&self) -> i32 {
        self.rust()
            .controller_inventory
            .as_ref()
            .map(|inventory| inventory.supported_targets.len().saturating_add(1))
            .and_then(|count| i32::try_from(count).ok())
            .unwrap_or(1)
    }

    pub fn controller_target_id_at(&self, index: i32) -> QString {
        if index == 0 {
            return qstring(controllers::TARGET_INHERIT);
        }
        usize::try_from(index - 1)
            .ok()
            .and_then(|index| {
                self.rust()
                    .controller_inventory
                    .as_ref()?
                    .supported_targets
                    .get(index)
            })
            .map(|target| qstring(&target.id))
            .unwrap_or_default()
    }

    pub fn controller_target_name_at(&self, index: i32) -> QString {
        if index == 0 {
            return qstring("Default target");
        }
        usize::try_from(index - 1)
            .ok()
            .and_then(|index| {
                self.rust()
                    .controller_inventory
                    .as_ref()?
                    .supported_targets
                    .get(index)
            })
            .map(|target| qstring(&target.name))
            .unwrap_or_default()
    }

    pub fn custom_controller_profile_count(&self) -> i32 {
        i32::try_from(self.rust().controller_mapping.custom_profiles.len()).unwrap_or(i32::MAX)
    }

    pub fn custom_controller_profile_id_at(&self, index: i32) -> QString {
        self.custom_controller_profile_at(index)
            .map(|profile| qstring(&profile.id))
            .unwrap_or_default()
    }

    pub fn custom_controller_profile_name_at(&self, index: i32) -> QString {
        self.custom_controller_profile_at(index)
            .map(|profile| qstring(&profile.name))
            .unwrap_or_default()
    }

    pub fn custom_controller_profile_detail_at(&self, index: i32) -> QString {
        self.custom_controller_profile_at(index)
            .map(|profile| {
                let layout = match profile.layout.as_str() {
                    "playstation" => "PlayStation",
                    "generic" => "Generic",
                    _ => "Xbox",
                };
                let mapping_label = match profile.mappings.len() {
                    0 => "identity mapping".to_owned(),
                    1 => "1 remapped button".to_owned(),
                    count => format!("{count} remapped buttons"),
                };
                qstring(format!("{layout} diagram · {mapping_label}"))
            })
            .unwrap_or_default()
    }

    pub fn create_controller_profile(mut self: Pin<&mut Self>) {
        self.as_mut().open_new_controller_profile(None);
    }

    pub fn edit_controller_profile(mut self: Pin<&mut Self>, index: i32) {
        let Some(profile) = self.as_ref().custom_controller_profile_at(index).cloned() else {
            return;
        };
        self.as_mut()
            .set_controller_profile_editor_id(qstring(&profile.id));
        self.as_mut()
            .set_controller_profile_editor_name(qstring(&profile.name));
        self.as_mut()
            .set_controller_profile_editor_layout(qstring(&profile.layout));
        self.as_mut()
            .set_controller_profile_editor_target(qstring("South"));
        self.as_mut()
            .set_controller_profile_editor_status(qstring("Edit the diagram, then save."));
        self.as_mut().rust_mut().controller_profile_editor_mappings = profile.mappings;
        self.as_mut()
            .rust_mut()
            .controller_profile_pending_controller_id = None;
        self.as_mut().set_controller_profile_editor_open(true);
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn delete_controller_profile(mut self: Pin<&mut Self>, index: i32) {
        let Some(profile) = self.as_ref().custom_controller_profile_at(index).cloned() else {
            return;
        };
        if !controllers::remove_custom_profile(
            &mut self.as_mut().rust_mut().controller_mapping,
            &profile.id,
        ) {
            return;
        }
        if self.as_ref().controller_profile_editor_id().to_string() == profile.id {
            self.as_mut().close_controller_profile_editor();
        }
        self.as_mut()
            .set_controller_profile_editor_status(qstring(format!(
                "Deleted profile {:?} and cleared its launch assignments.",
                profile.name
            )));
        self.as_mut().controller_settings_changed();
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn cancel_controller_profile_edit(mut self: Pin<&mut Self>) {
        self.as_mut().close_controller_profile_editor();
        self.as_mut().bump_controller_revision();
    }

    pub fn save_controller_profile(mut self: Pin<&mut Self>) {
        let name = self
            .as_ref()
            .controller_profile_editor_name()
            .to_string()
            .trim()
            .to_owned();
        if name.is_empty() || name.chars().count() > 100 {
            self.as_mut().set_controller_profile_editor_status(qstring(
                "Profile name must contain 1 to 100 characters.",
            ));
            return;
        }
        let layout = self.as_ref().controller_profile_editor_layout().to_string();
        if !matches!(layout.as_str(), "xbox" | "playstation" | "generic") {
            self.as_mut().set_controller_profile_editor_status(qstring(
                "Choose the Xbox, PlayStation, or generic diagram.",
            ));
            return;
        }
        let saved_id = self.as_ref().controller_profile_editor_id().to_string();
        let id = if saved_id.trim().is_empty() {
            format!("custom:{}", uuid::Uuid::new_v4())
        } else {
            saved_id
        };
        let profile = ControllerCustomProfile {
            id: id.clone(),
            name: name.clone(),
            layout,
            mappings: self
                .as_ref()
                .rust()
                .controller_profile_editor_mappings
                .clone(),
        };
        let pending_controller_id = self
            .as_ref()
            .rust()
            .controller_profile_pending_controller_id
            .clone();
        let mut candidate = self.as_ref().rust().controller_mapping.clone();
        if let Some(index) = candidate
            .custom_profiles
            .iter()
            .position(|saved| saved.id == id)
        {
            candidate.custom_profiles[index] = profile;
        } else {
            candidate.custom_profiles.push(profile);
        }
        if let Some(controller_id) = pending_controller_id.as_deref()
            && !controllers::set_controller_profile(&mut candidate, controller_id, &id)
        {
            self.as_mut().set_controller_profile_editor_status(qstring(
                "The new profile could not be assigned to the selected controller.",
            ));
            return;
        }
        if let Err(error) = candidate.validate() {
            self.as_mut()
                .set_controller_profile_editor_status(qstring(error.to_string()));
            return;
        }
        self.as_mut().rust_mut().controller_mapping = candidate;
        if pending_controller_id.is_some() {
            self.as_mut().set_controller_enabled(true);
        }
        self.as_mut().close_controller_profile_editor();
        self.as_mut()
            .set_controller_profile_editor_status(qstring(format!(
                "Saved custom controller profile {name:?}."
            )));
        self.as_mut().controller_settings_changed();
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn select_controller_profile_target(mut self: Pin<&mut Self>, target: QString) {
        let target = target.to_string();
        if !controllers::CONTROLLER_PROFILE_BUTTONS
            .iter()
            .any(|(button, _)| *button == target)
        {
            return;
        }
        self.as_mut()
            .set_controller_profile_editor_target(qstring(target));
        self.as_mut()
            .set_controller_profile_editor_status(QString::default());
        self.as_mut().bump_controller_profile_revision();
    }

    pub fn controller_profile_button_count(&self) -> i32 {
        i32::try_from(controllers::CONTROLLER_PROFILE_BUTTONS.len()).unwrap_or(i32::MAX)
    }

    pub fn controller_profile_button_id_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| controllers::CONTROLLER_PROFILE_BUTTONS.get(index))
            .map(|(id, _)| qstring(id))
            .unwrap_or_default()
    }

    pub fn controller_profile_button_name_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| controllers::CONTROLLER_PROFILE_BUTTONS.get(index))
            .map(|(_, name)| qstring(name))
            .unwrap_or_default()
    }

    pub fn controller_profile_button_label(&self, button: QString) -> QString {
        qstring(controllers::controller_button_label(
            &self.controller_profile_editor_layout().to_string(),
            button.to_string().trim(),
        ))
    }

    pub fn controller_profile_selected_source(&self) -> QString {
        let target = self.controller_profile_editor_target().to_string();
        let profile = ControllerCustomProfile {
            mappings: self.rust().controller_profile_editor_mappings.clone(),
            ..ControllerCustomProfile::default()
        };
        qstring(controllers::custom_profile_source_for_target(
            &profile, &target,
        ))
    }

    pub fn choose_controller_profile_source(mut self: Pin<&mut Self>, source: QString) {
        let target = self.as_ref().controller_profile_editor_target().to_string();
        let source = source.to_string();
        let mut profile = ControllerCustomProfile {
            mappings: self
                .as_ref()
                .rust()
                .controller_profile_editor_mappings
                .clone(),
            ..ControllerCustomProfile::default()
        };
        if controllers::set_custom_profile_mapping(&mut profile, &target, &source) {
            self.as_mut().rust_mut().controller_profile_editor_mappings = profile.mappings;
            self.as_mut()
                .set_controller_profile_editor_status(QString::default());
            self.as_mut().bump_controller_profile_revision();
        }
    }

    pub fn controller_profile_mapping_count(&self) -> i32 {
        i32::try_from(self.rust().controller_profile_editor_mappings.len()).unwrap_or(i32::MAX)
    }

    pub fn controller_profile_mapping_source_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().controller_profile_editor_mappings.get(index))
            .map(|mapping| qstring(&mapping.source_button))
            .unwrap_or_default()
    }

    pub fn controller_profile_mapping_target_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().controller_profile_editor_mappings.get(index))
            .map(|mapping| qstring(&mapping.target_button))
            .unwrap_or_default()
    }

    pub fn apply_two_button_controller_preset(mut self: Pin<&mut Self>) {
        let mut profile = ControllerCustomProfile::default();
        controllers::apply_two_button_profile_preset(&mut profile);
        self.as_mut().rust_mut().controller_profile_editor_mappings = profile.mappings;
        self.as_mut().set_controller_profile_editor_status(qstring(
            "Applied the 2-button X/A preset. Identity mappings remain implicit.",
        ));
        self.as_mut().bump_controller_profile_revision();
        if std::env::args().any(|argument| argument == "--controller-profile-ui-probe") {
            println!(
                "LUNCHBOX_CONTROLLER_PROFILE_UI_READY buttons={} mappings={}",
                controllers::CONTROLLER_PROFILE_BUTTONS.len(),
                self.as_ref()
                    .rust()
                    .controller_profile_editor_mappings
                    .len()
            );
        }
    }

    pub fn clear_controller_profile_mappings(mut self: Pin<&mut Self>) {
        self.as_mut()
            .rust_mut()
            .controller_profile_editor_mappings
            .clear();
        self.as_mut().set_controller_profile_editor_status(qstring(
            "Cleared every remap; the profile now passes buttons through by identity.",
        ));
        self.as_mut().bump_controller_profile_revision();
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

    pub fn export_profile(mut self: Pin<&mut Self>, destination: QUrl) {
        if *self.as_ref().profile_busy() {
            return;
        }
        let Some(destination) = destination
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_profile_message(qstring("Choose a local profile backup file."));
            return;
        };
        let state_path = match settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_profile_message(qstring(format!(
                    "Could not locate the Lunchbox profile: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_message(qstring("Creating a consistent profile snapshot…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-export".into())
            .spawn(move || {
                let result = profile_backup::export_profile(&state_path, &destination)
                    .map(|summary| (summary, destination))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok((summary, destination)) => model.as_mut().set_profile_message(qstring(
                            format!(
                                "Backup verified and saved to {}: {}. Credential-store secrets, ROMs, downloads, media caches, and emulator binaries were not included.",
                                destination.display(),
                                summary.description()
                            ),
                        )),
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Profile backup failed: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start the profile backup worker: {error}"
            )));
        }
    }

    pub fn inspect_profile(mut self: Pin<&mut Self>, source: QUrl) {
        if *self.as_ref().profile_busy() {
            return;
        }
        let Some(source) = source
            .to_local_file()
            .map(|path| PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_profile_message(qstring("Choose a local Lunchbox profile archive."));
            return;
        };
        self.as_mut().rust_mut().profile_restore_source = None;
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_restore_summary(QString::default());
        self.as_mut().set_profile_busy(true);
        self.as_mut().set_profile_message(qstring(
            "Validating the profile manifest, receipts, database, and themes…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-inspect".into())
            .spawn(move || {
                let result = profile_backup::inspect_profile(&source)
                    .map(|summary| (summary, source))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok((summary, source)) => {
                            let description = summary.description();
                            model.as_mut().rust_mut().profile_restore_source = Some(source);
                            model
                                .as_mut()
                                .set_profile_restore_summary(qstring(&description));
                            model.as_mut().set_profile_message(qstring(format!(
                                "Verified profile archive: {description}. Review the replacement before staging it."
                            )));
                            model.as_mut().set_profile_restore_ready(true);
                        }
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Profile archive was rejected: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start profile validation: {error}"
            )));
        }
    }

    pub fn stage_profile_restore(mut self: Pin<&mut Self>) {
        if *self.as_ref().profile_busy() {
            return;
        }
        let Some(source) = self.as_ref().rust().profile_restore_source.clone() else {
            self.as_mut()
                .set_profile_message(qstring("Choose and verify a profile archive first."));
            return;
        };
        let state_path = match settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_profile_message(qstring(format!(
                    "Could not locate the Lunchbox profile: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_message(qstring("Revalidating and staging the profile restore…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-stage".into())
            .spawn(move || {
                let result = profile_backup::stage_restore(&state_path, &source)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok(summary) => {
                            model.as_mut().rust_mut().profile_restore_source = None;
                            model
                                .as_mut()
                                .set_profile_restore_summary(qstring(summary.description()));
                            model.as_mut().set_profile_restart_required(true);
                            model.as_mut().set_profile_message(qstring(
                                "Restore staged safely. Quit Lunchbox to apply it before any models open on the next launch.",
                            ));
                        }
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Profile restore was not staged: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start the profile restore worker: {error}"
            )));
        }
    }

    pub fn cancel_profile_restore(mut self: Pin<&mut Self>) {
        if *self.as_ref().profile_busy() {
            return;
        }
        self.as_mut().rust_mut().profile_restore_source = None;
        self.as_mut().set_profile_restore_ready(false);
        self.as_mut()
            .set_profile_restore_summary(QString::default());
        self.as_mut().set_profile_message(qstring(
            "Profile restore review cancelled; nothing was changed.",
        ));
    }

    pub fn cancel_staged_profile_restore(mut self: Pin<&mut Self>) {
        if *self.as_ref().profile_busy() || !*self.as_ref().profile_restart_required() {
            return;
        }
        let state_path = match settings::state_database_path() {
            Ok(path) => path,
            Err(error) => {
                self.as_mut().set_profile_message(qstring(format!(
                    "Could not locate the staged Lunchbox profile: {error}"
                )));
                return;
            }
        };
        self.as_mut().set_profile_busy(true);
        self.as_mut()
            .set_profile_message(qstring("Cancelling the staged profile restore safely…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-profile-unstage".into())
            .spawn(move || {
                let result = profile_backup::cancel_pending_restore(&state_path)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_profile_busy(false);
                    match result {
                        Ok(removed) => {
                            model.as_mut().set_profile_restart_required(false);
                            model
                                .as_mut()
                                .set_profile_restore_summary(QString::default());
                            model.as_mut().set_profile_message(qstring(if removed {
                                "Staged profile restore cancelled; the current profile was never changed."
                            } else {
                                "No staged profile restore remained; the current profile is unchanged."
                            }));
                        }
                        Err(error) => model.as_mut().set_profile_message(qstring(format!(
                            "Could not cancel the staged profile restore: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_profile_busy(false);
            self.as_mut().set_profile_message(qstring(format!(
                "Could not start the profile restore cancellation worker: {error}"
            )));
        }
    }

    fn apply_settings(mut self: Pin<&mut Self>, settings: AppSettings) {
        let region_priority =
            crate::region_priority::effective_region_priority(&settings.region_priority);
        let media_provider_priority =
            crate::media::effective_provider_priority(&settings.media_provider_priority);
        let controller_mapping = settings.controller_mapping.clone();
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
        self.as_mut()
            .set_seeding_policy(qstring(settings.seeding_policy));
        self.as_mut().rust_mut().region_priority = region_priority;
        self.as_mut().sync_primary_region();
        self.as_mut().bump_region_revision();
        self.as_mut()
            .set_version_preference(qstring(settings.version_preference));
        self.as_mut().rust_mut().media_provider_priority = media_provider_priority;
        self.as_mut().bump_media_provider_revision();
        self.as_mut()
            .set_controller_enabled(controller_mapping.enabled);
        self.as_mut()
            .set_controller_output_target(qstring(&controller_mapping.output_target));
        self.as_mut().rust_mut().controller_mapping = controller_mapping;
        self.as_mut().close_controller_profile_editor();
        self.as_mut().bump_controller_revision();
        self.as_mut().bump_controller_profile_revision();
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
            seeding_policy: self.seeding_policy().to_string(),
            preferred_region: self.preferred_region().to_string(),
            region_priority: self.rust().region_priority.clone(),
            version_preference: self.version_preference().to_string(),
            media_provider_priority: self.rust().media_provider_priority.clone(),
            controller_mapping: {
                let mut mapping = self.rust().controller_mapping.clone();
                mapping.enabled = *self.controller_enabled();
                mapping.output_target = self.controller_output_target().to_string();
                mapping
            },
        };
        settings.validate().map_err(|error| error.to_string())?;
        Ok(settings)
    }

    fn sync_primary_region(mut self: Pin<&mut Self>) {
        let primary = self
            .as_ref()
            .rust()
            .region_priority
            .iter()
            .find(|region| !region.is_empty())
            .cloned()
            .unwrap_or_else(|| "USA".to_owned());
        let legacy_primary = if matches!(
            primary.as_str(),
            "USA" | "Japan" | "Europe" | "World" | "Asia"
        ) {
            primary
        } else {
            "any".to_owned()
        };
        self.as_mut().set_preferred_region(qstring(legacy_primary));
    }

    fn bump_region_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().region_revision().wrapping_add(1);
        self.as_mut().set_region_revision(revision);
    }

    fn bump_media_provider_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().media_provider_revision().wrapping_add(1);
        self.as_mut().set_media_provider_revision(revision);
    }

    fn controller_at(&self, index: i32) -> Option<ControllerDevice> {
        let index = usize::try_from(index).ok()?;
        let inventory = self.rust().controller_inventory.as_ref()?;
        controllers::ordered_controllers(inventory, &self.rust().controller_mapping)
            .get(index)
            .cloned()
    }

    fn custom_controller_profile_at(&self, index: i32) -> Option<&ControllerCustomProfile> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().controller_mapping.custom_profiles.get(index))
    }

    fn open_new_controller_profile(mut self: Pin<&mut Self>, controller_id: Option<String>) {
        self.as_mut()
            .set_controller_profile_editor_id(QString::default());
        self.as_mut()
            .set_controller_profile_editor_name(qstring("Custom profile"));
        self.as_mut()
            .set_controller_profile_editor_layout(qstring("xbox"));
        self.as_mut()
            .set_controller_profile_editor_target(qstring("South"));
        self.as_mut().set_controller_profile_editor_status(qstring(
            "Choose a virtual target on the diagram, then assign its physical source.",
        ));
        self.as_mut()
            .rust_mut()
            .controller_profile_editor_mappings
            .clear();
        self.as_mut()
            .rust_mut()
            .controller_profile_pending_controller_id = controller_id;
        self.as_mut().set_controller_profile_editor_open(true);
        self.as_mut().bump_controller_revision();
        self.as_mut().bump_controller_profile_revision();
        if std::env::args().any(|argument| argument == "--controller-profile-ui-probe") {
            println!(
                "LUNCHBOX_CONTROLLER_PROFILE_UI_OPENED buttons={} mappings=0",
                controllers::CONTROLLER_PROFILE_BUTTONS.len()
            );
        }
    }

    fn close_controller_profile_editor(mut self: Pin<&mut Self>) {
        self.as_mut().set_controller_profile_editor_open(false);
        self.as_mut()
            .rust_mut()
            .controller_profile_pending_controller_id = None;
    }

    fn controller_settings_changed(mut self: Pin<&mut Self>) {
        self.as_mut().bump_controller_revision();
        self.as_mut().set_message(qstring(
            "Controller mapping changed. Save settings to apply it when games launch.",
        ));
    }

    fn bump_controller_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().controller_revision().wrapping_add(1);
        self.as_mut().set_controller_revision(revision);
    }

    fn bump_controller_profile_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().controller_profile_revision().wrapping_add(1);
        self.as_mut().set_controller_profile_revision(revision);
    }
}

fn controller_inventory_status(inventory: &ControllerInventory) -> String {
    let provider = &inventory.provider;
    let mut status = if provider.available && provider.service_accessible {
        let raw_version = provider.version.as_deref().unwrap_or("unknown version");
        let version = raw_version
            .strip_prefix("inputplumber ")
            .or_else(|| raw_version.strip_prefix("InputPlumber "))
            .unwrap_or(raw_version);
        format!(
            "InputPlumber {version} · {} connected game controllers · {} managed devices · {} virtual targets",
            inventory.controllers.len(),
            inventory.managed_device_count,
            inventory.supported_targets.len()
        )
    } else {
        provider.message.clone().unwrap_or_else(|| {
            "Native controller remapping is not available on this computer.".to_owned()
        })
    };
    if !inventory.warnings.is_empty() {
        status.push_str(" · ");
        status.push_str(&inventory.warnings.join("; "));
    }
    status
}

fn controller_profile_option(
    mapping: &ControllerMappingSettings,
    index: i32,
) -> Option<(String, String)> {
    match index {
        0 => Some((
            controllers::PROFILE_INHERIT.to_owned(),
            "Use game, system, or default profile".to_owned(),
        )),
        1 => Some((
            controllers::PROFILE_NONE.to_owned(),
            "Off (no button remap)".to_owned(),
        )),
        _ => {
            let index = usize::try_from(index - 2).ok()?;
            let built_in = controllers::built_in_profiles();
            if let Some(profile) = built_in.get(index) {
                return Some((profile.id.clone(), profile.name.clone()));
            }
            let custom_index = index.checked_sub(built_in.len())?;
            if let Some(profile) = mapping.custom_profiles.get(custom_index) {
                return Some((profile.id.clone(), profile.name.clone()));
            }
            (custom_index == mapping.custom_profiles.len()).then(|| {
                (
                    controllers::PROFILE_CREATE.to_owned(),
                    "Create new profile…".to_owned(),
                )
            })
        }
    }
}

fn load_settings() -> anyhow::Result<(
    AppSettings,
    bool,
    PathBuf,
    Option<profile_backup::ProfileSummary>,
)> {
    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    let password_saved = settings::load_password()?.is_some();
    let restored_profile = match profile_backup::take_applied_notice(store.path()) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("LUNCHBOX_PROFILE_RESTORE_NOTICE_FAILED error={error:#}");
            None
        }
    };
    Ok((
        settings,
        password_saved,
        store.path().to_owned(),
        restored_profile,
    ))
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
