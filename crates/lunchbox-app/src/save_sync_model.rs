#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, credentials_saved)]
        #[qproperty(bool, automatic_enabled)]
        #[qproperty(QString, provider)]
        #[qproperty(QString, message)]
        #[qproperty(QString, status)]
        #[qproperty(QString, operation)]
        #[qproperty(i32, conflict_count)]
        #[qproperty(i32, choice_count)]
        #[qproperty(i32, remote_device_count)]
        #[qproperty(i32, revision)]
        type SaveSyncModel = super::SaveSyncModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut SaveSyncModel>);

        #[qinvokable]
        fn save_and_test_credentials(
            self: Pin<&mut SaveSyncModel>,
            provider: QString,
            access_token: QString,
            refresh_token: QString,
            client_id: QString,
            client_secret: QString,
        );

        #[qinvokable]
        fn test_saved_connection(self: Pin<&mut SaveSyncModel>);

        #[qinvokable]
        fn clear_credentials(self: Pin<&mut SaveSyncModel>);

        #[qinvokable]
        fn set_automatic(self: Pin<&mut SaveSyncModel>, enabled: bool);

        #[qinvokable]
        fn begin_sync(
            self: Pin<&mut SaveSyncModel>,
            emulator_slug: QString,
            runtime_platform: QString,
            operation: QString,
        );

        #[qinvokable]
        fn conflict_json(self: &SaveSyncModel, index: i32) -> QString;

        #[qinvokable]
        fn choose_conflict(self: Pin<&mut SaveSyncModel>, index: i32, choice: QString);

        #[qinvokable]
        fn apply_choices(self: Pin<&mut SaveSyncModel>);

        #[qinvokable]
        fn cancel_conflicts(self: Pin<&mut SaveSyncModel>);

        #[qinvokable]
        fn remote_device_json(self: &SaveSyncModel, index: i32) -> QString;

        #[qinvokable]
        fn choose_remote_device(self: Pin<&mut SaveSyncModel>, index: i32);

        #[qinvokable]
        fn cancel_remote_device_selection(self: Pin<&mut SaveSyncModel>);
    }

    impl cxx_qt::Threading for SaveSyncModel {}
}

use std::collections::BTreeMap;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use uuid::Uuid;

use crate::save_cloud::{
    CloudAuth, CloudProfile, CloudProvider, CloudStore, DEFAULT_CLOUD_ROOT, DeviceHead,
};
use crate::save_sync::{ArtifactKey, ConflictChoice, SyncConflict, SyncScope};
use crate::save_sync_service::{AppliedSync, PreparedSync, prepare_sync};

pub struct SaveSyncModelRust {
    initialized: bool,
    busy: bool,
    credentials_saved: bool,
    automatic_enabled: bool,
    provider: QString,
    message: QString,
    status: QString,
    operation: QString,
    conflict_count: i32,
    choice_count: i32,
    remote_device_count: i32,
    revision: i32,
    generation: u64,
    prepared: Option<PreparedSync>,
    choices: BTreeMap<ArtifactKey, ConflictChoice>,
    pending_sync: Option<PendingSync>,
    remote_devices: Vec<DeviceHead>,
    recheck_remote_frontier: bool,
}

impl Default for SaveSyncModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            credentials_saved: false,
            automatic_enabled: false,
            provider: qstring("google_drive"),
            message: qstring(
                "Connect Google Drive, Dropbox, or OneDrive to synchronize captured emulator saves and states.",
            ),
            status: qstring("idle"),
            operation: QString::default(),
            conflict_count: 0,
            choice_count: 0,
            remote_device_count: 0,
            revision: 0,
            generation: 0,
            prepared: None,
            choices: BTreeMap::new(),
            pending_sync: None,
            remote_devices: Vec::new(),
            recheck_remote_frontier: false,
        }
    }
}

#[derive(Clone)]
struct PendingSync {
    emulator_slug: String,
    runtime_platform: String,
    operation: String,
}

enum SyncResult {
    Skipped(String),
    RemoteDevices(Vec<DeviceHead>),
    Conflicts(PreparedSync),
    Applied(AppliedSync),
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn optional(value: QString) -> Option<String> {
    let value = value.to_string().trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn now_unix_ms() -> Result<i64> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_millis();
    i64::try_from(millis).context("system clock is out of range")
}

fn recovery_base() -> Result<std::path::PathBuf> {
    let state = crate::settings::state_database_path()?;
    let parent = state
        .parent()
        .context("Lunchbox state database has no parent directory")?;
    ensure!(
        parent.is_absolute(),
        "Lunchbox state directory is not absolute"
    );
    Ok(parent.join("save-sync-recovery"))
}

fn connected_store(profile: &CloudProfile) -> Result<CloudStore> {
    profile.validate()?;
    CloudStore::connect(profile.provider, DEFAULT_CLOUD_ROOT, &profile.auth)
}

impl qobject::SaveSyncModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Inspecting the operating-system credential store…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-initialize".into())
            .spawn(move || {
                let result =
                    crate::settings::load_save_cloud_profile().map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_initialize(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_initialized(true);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not inspect cloud-save credentials: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    fn finish_initialize(mut self: Pin<&mut Self>, result: Result<Option<CloudProfile>, String>) {
        self.as_mut().set_busy(false);
        self.as_mut().set_initialized(true);
        match result {
            Ok(Some(profile)) => {
                self.as_mut().set_credentials_saved(true);
                self.as_mut().set_automatic_enabled(profile.automatic);
                self.as_mut().set_provider(qstring(profile.provider.key()));
                self.as_mut().set_status(qstring("idle"));
                self.as_mut().set_message(qstring(format!(
                    "{} credentials are stored securely. Automatic synchronization is {}.",
                    profile.provider.display_name(),
                    if profile.automatic {
                        "enabled"
                    } else {
                        "disabled"
                    }
                )));
            }
            Ok(None) => {
                self.as_mut().set_credentials_saved(false);
                self.as_mut().set_automatic_enabled(false);
                self.as_mut().set_status(qstring("idle"));
                self.as_mut().set_message(qstring(
                    "Connect a cloud provider before enabling automatic save synchronization.",
                ));
            }
            Err(error) => {
                self.as_mut().set_status(qstring("error"));
                self.as_mut().set_message(qstring(format!(
                    "Could not read cloud-save credentials: {error}"
                )));
            }
        }
        self.as_mut().bump_revision();
    }

    pub fn save_and_test_credentials(
        mut self: Pin<&mut Self>,
        provider: QString,
        access_token: QString,
        refresh_token: QString,
        client_id: QString,
        client_secret: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        let provider = match CloudProvider::parse(&provider.to_string()) {
            Ok(provider) => provider,
            Err(error) => {
                self.as_mut().set_message(qstring(error.to_string()));
                return;
            }
        };
        let auth = CloudAuth {
            access_token: optional(access_token),
            refresh_token: optional(refresh_token),
            client_id: optional(client_id),
            client_secret: optional(client_secret),
        };
        if let Err(error) = auth.validate(provider) {
            self.as_mut().set_message(qstring(error.to_string()));
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut().set_status(qstring("busy"));
        self.as_mut().set_message(qstring(format!(
            "Testing authenticated read/write access to {}…",
            provider.display_name()
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-save-credentials".into())
            .spawn(move || {
                let result = (|| {
                    let previous = crate::settings::load_save_cloud_profile()?;
                    let device_id = previous
                        .as_ref()
                        .map(|profile| profile.device_id.clone())
                        .unwrap_or_else(|| format!("device-{}", Uuid::new_v4().simple()));
                    let profile = CloudProfile::new(provider, device_id, true, auth)?;
                    connected_store(&profile)?.probe()?;
                    crate::settings::save_save_cloud_profile(Some(&profile))?;
                    Ok(profile)
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_save_credentials(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not start the cloud connection test: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    fn finish_save_credentials(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<CloudProfile, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(profile) => {
                self.as_mut().set_credentials_saved(true);
                self.as_mut().set_automatic_enabled(true);
                self.as_mut().set_provider(qstring(profile.provider.key()));
                self.as_mut().set_status(qstring("idle"));
                self.as_mut().set_message(qstring(format!(
                    "{} read/write access is verified. Credentials are stored securely and automatic synchronization is enabled.",
                    profile.provider.display_name()
                )));
            }
            Err(error) => {
                self.as_mut().set_status(qstring("error"));
                self.as_mut().set_message(qstring(format!(
                    "Cloud connection failed; credentials were not saved: {error}"
                )));
            }
        }
        self.as_mut().bump_revision();
    }

    pub fn test_saved_connection(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || !*self.as_ref().credentials_saved() {
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut().set_status(qstring("busy"));
        self.as_mut()
            .set_message(qstring("Testing the saved cloud connection…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-test".into())
            .spawn(move || {
                let result = (|| {
                    let profile = crate::settings::load_save_cloud_profile()?
                        .context("no cloud-save credentials are stored")?;
                    connected_store(&profile)?.probe()?;
                    Ok(profile.provider)
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(provider) => {
                            model.as_mut().set_status(qstring("idle"));
                            model.as_mut().set_message(qstring(format!(
                                "{} authenticated read/write access is verified.",
                                provider.display_name()
                            )));
                        }
                        Err(error) => {
                            model.as_mut().set_status(qstring("error"));
                            model.as_mut().set_message(qstring(format!(
                                "Saved cloud connection failed: {error}"
                            )));
                        }
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not start the cloud connection test: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    pub fn clear_credentials(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-clear".into())
            .spawn(move || {
                let result = crate::settings::save_save_cloud_profile(None)
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(()) => {
                            model.as_mut().rust_mut().prepared = None;
                            model.as_mut().rust_mut().choices.clear();
                            model.as_mut().set_credentials_saved(false);
                            model.as_mut().set_automatic_enabled(false);
                            model.as_mut().set_conflict_count(0);
                            model.as_mut().set_choice_count(0);
                            model.as_mut().set_remote_device_count(0);
                            model.as_mut().rust_mut().pending_sync = None;
                            model.as_mut().rust_mut().remote_devices.clear();
                            model.as_mut().rust_mut().recheck_remote_frontier = false;
                            model.as_mut().set_status(qstring("idle"));
                            model.as_mut().set_message(qstring(
                                "Cloud-save credentials were removed from the operating-system credential store.",
                            ));
                        }
                        Err(error) => {
                            model.as_mut().set_status(qstring("error"));
                            model.as_mut().set_message(qstring(format!(
                                "Could not remove cloud-save credentials: {error}"
                            )));
                        }
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not start credential removal: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    pub fn set_automatic(mut self: Pin<&mut Self>, enabled: bool) {
        if *self.as_ref().busy()
            || !*self.as_ref().credentials_saved()
            || enabled == *self.as_ref().automatic_enabled()
        {
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-toggle".into())
            .spawn(move || {
                let result = (|| {
                    let mut profile = crate::settings::load_save_cloud_profile()?
                        .context("no cloud-save credentials are stored")?;
                    profile.automatic = enabled;
                    profile.validate()?;
                    crate::settings::save_save_cloud_profile(Some(&profile))?;
                    Ok(profile.provider)
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(provider) => {
                            model.as_mut().set_automatic_enabled(enabled);
                            model.as_mut().set_status(qstring("idle"));
                            model.as_mut().set_message(qstring(format!(
                                "Automatic {} synchronization is {}.",
                                provider.display_name(),
                                if enabled { "enabled" } else { "disabled" }
                            )));
                        }
                        Err(error) => {
                            model.as_mut().set_status(qstring("error"));
                            model.as_mut().set_message(qstring(format!(
                                "Could not update automatic synchronization: {error}"
                            )));
                        }
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not start the settings update: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    pub fn begin_sync(
        mut self: Pin<&mut Self>,
        emulator_slug: QString,
        runtime_platform: QString,
        operation: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        let emulator_slug = emulator_slug.to_string().trim().to_owned();
        let runtime_platform = runtime_platform.to_string().trim().to_owned();
        let operation = operation.to_string();
        if !matches!(operation.as_str(), "pre_launch" | "post_exit" | "manual") {
            self.as_mut()
                .set_message(qstring("Unknown cloud-save operation."));
            return;
        }
        self.as_mut().start_sync_request(
            PendingSync {
                emulator_slug,
                runtime_platform,
                operation,
            },
            None,
        );
    }

    fn start_sync_request(
        mut self: Pin<&mut Self>,
        pending: PendingSync,
        remote_device_id: Option<String>,
    ) {
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().rust_mut().prepared = None;
        self.as_mut().rust_mut().choices.clear();
        self.as_mut().rust_mut().remote_devices.clear();
        self.as_mut().rust_mut().pending_sync = Some(pending.clone());
        self.as_mut().rust_mut().recheck_remote_frontier = remote_device_id.is_some();
        self.as_mut().set_conflict_count(0);
        self.as_mut().set_choice_count(0);
        self.as_mut().set_remote_device_count(0);
        self.as_mut().set_operation(qstring(&pending.operation));
        self.as_mut().set_busy(true);
        self.as_mut().set_status(qstring("busy"));
        self.as_mut().set_message(qstring(format!(
            "Checking cloud saves for {} before continuing…",
            pending.emulator_slug
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-prepare".into())
            .spawn(move || {
                let result = prepare_and_maybe_apply(
                    &pending.emulator_slug,
                    &pending.runtime_platform,
                    &pending.operation,
                    remote_device_id.as_deref(),
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_sync(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not start cloud synchronization: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    fn finish_sync(mut self: Pin<&mut Self>, generation: u64, result: Result<SyncResult, String>) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(SyncResult::Skipped(message)) => {
                self.as_mut().rust_mut().pending_sync = None;
                self.as_mut().set_status(qstring("skipped"));
                self.as_mut().set_message(qstring(message));
            }
            Ok(SyncResult::RemoteDevices(heads)) => {
                let count = i32::try_from(heads.len()).unwrap_or(i32::MAX);
                self.as_mut().rust_mut().remote_devices = heads;
                self.as_mut().set_remote_device_count(count);
                self.as_mut().set_status(qstring("remote_devices"));
                self.as_mut().set_message(qstring(format!(
                    "{count} remote device histories are available. Choose one to merge first."
                )));
            }
            Ok(SyncResult::Conflicts(prepared)) => {
                let count = i32::try_from(prepared.plan.conflicts.len()).unwrap_or(i32::MAX);
                self.as_mut().rust_mut().prepared = Some(prepared);
                self.as_mut().set_conflict_count(count);
                self.as_mut().set_choice_count(0);
                self.as_mut().set_status(qstring("conflicts"));
                self.as_mut().set_message(qstring(format!(
                    "{count} save conflict{} require an explicit Local or Remote choice.",
                    if count == 1 { "" } else { "s" }
                )));
            }
            Ok(SyncResult::Applied(applied)) => {
                if self.as_ref().rust().recheck_remote_frontier
                    && let Some(pending) = self.as_ref().rust().pending_sync.clone()
                {
                    self.as_mut().rust_mut().recheck_remote_frontier = false;
                    self.as_mut().start_sync_request(pending, None);
                    return;
                }
                self.as_mut().rust_mut().pending_sync = None;
                self.as_mut().set_status(qstring("complete"));
                self.as_mut()
                    .set_message(qstring(applied_message(&applied)));
            }
            Err(error) => {
                self.as_mut().set_status(qstring("error"));
                self.as_mut().set_message(qstring(format!(
                    "Cloud save synchronization failed: {error}"
                )));
            }
        }
        self.as_mut().bump_revision();
    }

    pub fn conflict_json(&self, index: i32) -> QString {
        let Some(prepared) = self.rust().prepared.as_ref() else {
            return qstring("{}");
        };
        let Some(conflict) = usize::try_from(index)
            .ok()
            .and_then(|index| prepared.plan.conflicts.get(index))
        else {
            return qstring("{}");
        };
        let mut value = conflict_json_value(conflict);
        let choice = self
            .rust()
            .choices
            .get(&conflict.key)
            .map(|choice| match choice {
                ConflictChoice::Local => "local",
                ConflictChoice::Remote => "remote",
            })
            .unwrap_or("");
        value["choice"] = serde_json::Value::String(choice.to_owned());
        qstring(value.to_string())
    }

    pub fn choose_conflict(mut self: Pin<&mut Self>, index: i32, choice: QString) {
        if *self.as_ref().busy() || self.as_ref().status().to_string() != "conflicts" {
            return;
        }
        let choice = match choice.to_string().as_str() {
            "local" => ConflictChoice::Local,
            "remote" => ConflictChoice::Remote,
            _ => return,
        };
        let Some(key) = self
            .as_ref()
            .rust()
            .prepared
            .as_ref()
            .and_then(|prepared| {
                usize::try_from(index)
                    .ok()
                    .and_then(|index| prepared.plan.conflicts.get(index))
            })
            .map(|conflict| conflict.key.clone())
        else {
            return;
        };
        self.as_mut().rust_mut().choices.insert(key, choice);
        let count = i32::try_from(self.as_ref().rust().choices.len()).unwrap_or(i32::MAX);
        self.as_mut().set_choice_count(count);
        self.as_mut().bump_revision();
    }

    pub fn apply_choices(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || self.as_ref().status().to_string() != "conflicts" {
            return;
        }
        let Some(prepared) = self.as_ref().rust().prepared.clone() else {
            return;
        };
        let choices = self.as_ref().rust().choices.clone();
        if choices.len() != prepared.plan.conflicts.len() {
            self.as_mut().set_message(qstring(
                "Choose Local or Remote for every conflict before continuing.",
            ));
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut().set_status(qstring("busy"));
        self.as_mut()
            .set_message(qstring("Applying the reviewed save choices…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-save-sync-apply".into())
            .spawn(move || {
                let result = (|| {
                    let profile = crate::settings::load_save_cloud_profile()?
                        .context("cloud-save credentials were removed")?;
                    ensure!(
                        profile.device_id == prepared.device_id,
                        "cloud-save device identity changed after review"
                    );
                    let store = connected_store(&profile)?;
                    prepared.apply(&store, &choices, &recovery_base()?, now_unix_ms()?)
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(applied) => {
                            model.as_mut().rust_mut().prepared = None;
                            model.as_mut().rust_mut().choices.clear();
                            model.as_mut().set_conflict_count(0);
                            model.as_mut().set_choice_count(0);
                            if model.as_ref().rust().recheck_remote_frontier
                                && let Some(pending) = model.as_ref().rust().pending_sync.clone()
                            {
                                model.as_mut().rust_mut().recheck_remote_frontier = false;
                                model.as_mut().start_sync_request(pending, None);
                                return;
                            }
                            model.as_mut().rust_mut().pending_sync = None;
                            model.as_mut().set_status(qstring("complete"));
                            model
                                .as_mut()
                                .set_message(qstring(applied_message(&applied)));
                        }
                        Err(error) => {
                            model.as_mut().set_status(qstring("error"));
                            model.as_mut().set_message(qstring(format!(
                                "Could not apply reviewed save choices: {error}"
                            )));
                        }
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_status(qstring("error"));
            self.as_mut().set_message(qstring(format!(
                "Could not start save application: {error}"
            )));
            self.as_mut().bump_revision();
        }
    }

    pub fn cancel_conflicts(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().prepared = None;
        self.as_mut().rust_mut().choices.clear();
        self.as_mut().rust_mut().pending_sync = None;
        self.as_mut().rust_mut().recheck_remote_frontier = false;
        self.as_mut().set_conflict_count(0);
        self.as_mut().set_choice_count(0);
        self.as_mut().set_status(qstring("cancelled"));
        self.as_mut()
            .set_message(qstring("Cloud save synchronization was cancelled."));
        self.as_mut().bump_revision();
    }

    pub fn remote_device_json(&self, index: i32) -> QString {
        let Some(head) = usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().remote_devices.get(index))
        else {
            return qstring("{}");
        };
        qstring(remote_device_json_value(head).to_string())
    }

    pub fn choose_remote_device(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || self.as_ref().status().to_string() != "remote_devices" {
            return;
        }
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let device_id = {
            let model = self.as_ref();
            let Some(head) = model.rust().remote_devices.get(index) else {
                return;
            };
            head.device_id.clone()
        };
        let Some(pending) = self.as_ref().rust().pending_sync.clone() else {
            return;
        };
        self.as_mut().start_sync_request(pending, Some(device_id));
    }

    pub fn cancel_remote_device_selection(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || self.as_ref().status().to_string() != "remote_devices" {
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().pending_sync = None;
        self.as_mut().rust_mut().remote_devices.clear();
        self.as_mut().rust_mut().recheck_remote_frontier = false;
        self.as_mut().set_remote_device_count(0);
        self.as_mut().set_status(qstring("cancelled"));
        self.as_mut()
            .set_message(qstring("Remote save-history selection was cancelled."));
        self.as_mut().bump_revision();
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let next = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(next);
    }
}

fn prepare_and_maybe_apply(
    emulator_slug: &str,
    runtime_platform: &str,
    operation: &str,
    remote_device_id: Option<&str>,
) -> Result<SyncResult> {
    let Some(profile) = crate::settings::load_save_cloud_profile()? else {
        return Ok(SyncResult::Skipped(
            "Cloud save synchronization is not configured.".to_owned(),
        ));
    };
    if !profile.automatic && operation != "manual" {
        return Ok(SyncResult::Skipped(
            "Automatic cloud save synchronization is disabled.".to_owned(),
        ));
    }
    let store = connected_store(&profile)?;
    let scope = SyncScope::new(emulator_slug, runtime_platform)?;
    if remote_device_id.is_none() {
        let remote_heads =
            crate::save_sync_service::remote_merge_candidates(&store, &scope, &profile.device_id)?;
        if remote_heads.len() > 1 {
            return Ok(SyncResult::RemoteDevices(remote_heads));
        }
    }
    let records = crate::platform_locations::load_records()?;
    let roots = crate::platform_locations::save_route_roots_for_platform(
        &records,
        emulator_slug,
        runtime_platform,
        &crate::platform_locations::LocationBases::detect(),
    )?;
    let prepared = prepare_sync(&store, scope, profile.device_id, roots, remote_device_id)?;
    if prepared.plan.requires_user_choice() {
        return Ok(SyncResult::Conflicts(prepared));
    }
    Ok(SyncResult::Applied(prepared.apply(
        &store,
        &BTreeMap::new(),
        &recovery_base()?,
        now_unix_ms()?,
    )?))
}

fn conflict_json_value(conflict: &SyncConflict) -> serde_json::Value {
    fn side(version: &Option<crate::save_sync::FileVersion>) -> serde_json::Value {
        match version {
            Some(version) => serde_json::json!({
                "exists": true,
                "modified_unix_ms": version.modified_unix_ms,
                "size": version.size,
                "sha256": version.sha256,
            }),
            None => serde_json::json!({ "exists": false }),
        }
    }
    serde_json::json!({
        "key": conflict.key.as_str(),
        "local": side(&conflict.local),
        "remote": side(&conflict.remote),
    })
}

fn remote_device_json_value(head: &DeviceHead) -> serde_json::Value {
    serde_json::json!({
        "device_id": head.device_id,
        "updated_unix_ms": head.updated_unix_ms,
        "manifest_id": head.manifest_id,
    })
}

fn applied_message(applied: &AppliedSync) -> String {
    let count = applied.actions.len();
    if count == 0 {
        "Cloud saves are already synchronized.".to_owned()
    } else if let Some(directory) = &applied.recovery_directory {
        format!(
            "Synchronized {count} save change{}; replaced local files are recoverable from {}.",
            if count == 1 { "" } else { "s" },
            directory.display()
        )
    } else {
        format!(
            "Synchronized {count} save change{}.",
            if count == 1 { "" } else { "s" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_sync::FileVersion;

    #[test]
    fn conflict_payload_keeps_both_timestamps_for_review() {
        let key = ArtifactKey::new(
            crate::save_sync::SaveRoute {
                purpose: crate::save_sync::SavePurpose::Saves,
                root_index: 0,
            },
            "game.sav",
        )
        .unwrap();
        let value = conflict_json_value(&SyncConflict {
            key,
            local: Some(FileVersion::from_bytes(b"local", 1000)),
            remote: Some(FileVersion::from_bytes(b"remote", 2000)),
        });
        assert_eq!(value["local"]["modified_unix_ms"], 1000);
        assert_eq!(value["remote"]["modified_unix_ms"], 2000);
    }

    #[test]
    fn remote_device_payload_identifies_history_and_timestamp() {
        let head = DeviceHead::new(scope_for_test(), "device-b", "b".repeat(64), 2345).unwrap();
        let value = remote_device_json_value(&head);
        assert_eq!(value["device_id"], "device-b");
        assert_eq!(value["updated_unix_ms"], 2345);
        assert_eq!(value["manifest_id"], "b".repeat(64));
    }

    fn scope_for_test() -> SyncScope {
        SyncScope::new("duckstation", "linux").unwrap()
    }
}
