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
        #[qproperty(QString, seeding_policy)]
        #[qproperty(QString, preferred_region)]
        #[qproperty(i32, region_revision)]
        #[qproperty(QString, version_preference)]
        #[qproperty(bool, controller_enabled)]
        #[qproperty(QString, controller_output_target)]
        #[qproperty(bool, controller_busy)]
        #[qproperty(i32, controller_revision)]
        #[qproperty(QString, controller_status)]
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
        fn region_count(self: &SettingsModel) -> i32;

        #[qinvokable]
        fn region_at(self: &SettingsModel, index: i32) -> QString;

        #[qinvokable]
        fn move_region(self: Pin<&mut SettingsModel>, from: i32, to: i32);

        #[qinvokable]
        fn reset_region_priority(self: Pin<&mut SettingsModel>);

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
    }

    impl cxx_qt::Threading for SettingsModel {}
}

use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::controllers::{self, ControllerDevice, ControllerInventory};
use crate::qbittorrent;
use crate::settings::{self, AppSettings, ControllerMappingSettings, SettingsStore};

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
    seeding_policy: QString,
    preferred_region: QString,
    region_priority: Vec<String>,
    region_revision: i32,
    version_preference: QString,
    controller_enabled: bool,
    controller_output_target: QString,
    controller_busy: bool,
    controller_revision: i32,
    controller_status: QString,
    controller_mapping: ControllerMappingSettings,
    controller_inventory: Option<ControllerInventory>,
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
            seeding_policy: QString::from("follow_client"),
            preferred_region: QString::from("USA"),
            region_priority: crate::region_priority::default_region_priority(),
            region_revision: 0,
            version_preference: QString::from("latest"),
            controller_enabled: false,
            controller_output_target: QString::from("xb360"),
            controller_busy: false,
            controller_revision: 0,
            controller_status: QString::from("Open controller settings to scan this computer."),
            controller_mapping: ControllerMappingSettings::default(),
            controller_inventory: None,
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
        let count = 2
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
        let region_priority =
            crate::region_priority::effective_region_priority(&settings.region_priority);
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
        self.as_mut()
            .set_controller_enabled(controller_mapping.enabled);
        self.as_mut()
            .set_controller_output_target(qstring(&controller_mapping.output_target));
        self.as_mut().rust_mut().controller_mapping = controller_mapping;
        self.as_mut().bump_controller_revision();
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

    fn controller_at(&self, index: i32) -> Option<ControllerDevice> {
        let index = usize::try_from(index).ok()?;
        let inventory = self.rust().controller_inventory.as_ref()?;
        controllers::ordered_controllers(inventory, &self.rust().controller_mapping)
            .get(index)
            .cloned()
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
            let profile = mapping.custom_profiles.get(index - built_in.len())?;
            Some((profile.id.clone(), profile.name.clone()))
        }
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
