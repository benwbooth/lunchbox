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
        #[qproperty(i32, busy_index)]
        #[qproperty(i32, row_count)]
        #[qproperty(i32, installed_count)]
        #[qproperty(i32, available_count)]
        #[qproperty(i32, recovered_count)]
        #[qproperty(i32, revision)]
        #[qproperty(QString, message)]
        #[qproperty(QString, recent_activity)]
        #[qproperty(QString, recent_activity_detail)]
        #[qproperty(bool, recent_activity_attention)]
        #[qproperty(QString, search)]
        #[qproperty(QString, status_filter)]
        #[qproperty(QString, platform_filter)]
        #[qproperty(i32, operation_revision)]
        type EmulatorManagerModel = super::EmulatorManagerModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut EmulatorManagerModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut EmulatorManagerModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut EmulatorManagerModel>,
            search: QString,
            status_filter: QString,
        );

        #[qinvokable]
        fn set_platform_scope(self: Pin<&mut EmulatorManagerModel>, platform: QString);

        #[qinvokable]
        fn name_at(self: &EmulatorManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn detail_at(self: &EmulatorManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn source_at(self: &EmulatorManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn status_at(self: &EmulatorManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn recommended_at(self: &EmulatorManagerModel, index: i32) -> bool;

        #[qinvokable]
        fn can_install_at(self: &EmulatorManagerModel, index: i32) -> bool;

        #[qinvokable]
        fn can_update_at(self: &EmulatorManagerModel, index: i32) -> bool;

        #[qinvokable]
        fn can_uninstall_at(self: &EmulatorManagerModel, index: i32) -> bool;

        #[qinvokable]
        fn uninstall_confirmation_at(self: &EmulatorManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn verify_recovery_probe(self: &EmulatorManagerModel, dialog_visible: bool) -> bool;

        #[qinvokable]
        fn install_at(self: Pin<&mut EmulatorManagerModel>, index: i32);

        #[qinvokable]
        fn update_at(self: Pin<&mut EmulatorManagerModel>, index: i32);

        #[qinvokable]
        fn uninstall_at(self: Pin<&mut EmulatorManagerModel>, index: i32);
    }

    impl cxx_qt::Threading for EmulatorManagerModel {}
}

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::emulator_manager::{self, ManagedAction, ManagedEmulator};

pub struct EmulatorManagerModelRust {
    initialized: bool,
    busy: bool,
    busy_index: i32,
    row_count: i32,
    installed_count: i32,
    available_count: i32,
    recovered_count: i32,
    revision: i32,
    message: QString,
    recent_activity: QString,
    recent_activity_detail: QString,
    recent_activity_attention: bool,
    search: QString,
    status_filter: QString,
    platform_filter: QString,
    operation_revision: i32,
    all_rows: Vec<ManagedEmulator>,
    rows: Vec<ManagedEmulator>,
}

impl Default for EmulatorManagerModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            busy_index: -1,
            row_count: 0,
            installed_count: 0,
            available_count: 0,
            recovered_count: 0,
            revision: 0,
            message: QString::from("Open Emulator Manager to detect installed runtimes."),
            recent_activity: QString::default(),
            recent_activity_detail: QString::default(),
            recent_activity_attention: false,
            search: QString::default(),
            status_filter: QString::from("all"),
            platform_filter: QString::default(),
            operation_revision: 0,
            all_rows: Vec::new(),
            rows: Vec::new(),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::EmulatorManagerModel {
    pub fn initialize(self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.load_async("Detecting installed emulators…");
    }

    pub fn refresh(self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.load_async("Refreshing emulator installations…");
    }

    fn load_async(mut self: Pin<&mut Self>, message: &str) {
        self.as_mut().set_busy(true);
        self.as_mut().set_busy_index(-1);
        self.as_mut().set_message(qstring(message));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emulator-discovery".into())
            .spawn(move || {
                let result = emulator_manager::recover_interrupted_emulator_operations()
                    .and_then(|recovered| {
                        let rows = emulator_manager::load_managed_emulators()?;
                        let recent = emulator_manager::recent_emulator_lifecycle_operation()?;
                        Ok((rows, recovered.len(), recent))
                    })
                    .map_err(|error| format!("{error:#}"));
                if let Ok((rows, recovered, _)) = &result {
                    println!(
                        "LUNCHBOX_EMULATOR_DISCOVERY_READY runtimes={} recovered={recovered}",
                        rows.len()
                    );
                }
                if let Err(error) = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_load(result);
                }) {
                    eprintln!("LUNCHBOX_EMULATOR_MANAGER_QUEUE_FAILED error={error}");
                }
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start emulator discovery: {error}"
            )));
        }
    }

    fn finish_load(
        mut self: Pin<&mut Self>,
        result: Result<
            (
                Vec<ManagedEmulator>,
                usize,
                Option<crate::settings::EmulatorLifecycleOperation>,
            ),
            String,
        >,
    ) {
        match result {
            Ok((rows, recovered, recent)) => {
                let detected = rows.len();
                self.as_mut().set_recovered_count(saturating_i32(recovered));
                self.as_mut().set_recent_operation(recent.as_ref());
                self.as_mut().rust_mut().all_rows = rows;
                self.as_mut().refilter();
                self.as_mut().set_initialized(true);
                let installed = *self.as_ref().installed_count();
                let total = self.as_ref().rust().all_rows.len();
                let uninstallable = self
                    .as_ref()
                    .rust()
                    .all_rows
                    .iter()
                    .filter(|row| row.can_uninstall())
                    .count();
                let mut message = format!(
                    "Detected {installed} installed runtime{} across {total} managed emulator choices.",
                    if installed == 1 { "" } else { "s" }
                );
                if recovered > 0 {
                    message.push_str(&format!(
                        " Recovered {recovered} interrupted operation{} and rescanned the exact installation state.",
                        if recovered == 1 { "" } else { "s" }
                    ));
                }
                self.as_mut().set_message(qstring(message));
                println!(
                    "LUNCHBOX_EMULATOR_MANAGER_READY runtimes={detected} installed={installed} uninstallable={uninstallable}"
                );
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMULATOR_MANAGER_FAILED error={error}");
                self.as_mut()
                    .set_message(qstring(format!("Could not load emulator catalog: {error}")));
            }
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_busy_index(-1);
    }

    pub fn apply_filter(mut self: Pin<&mut Self>, search: QString, status_filter: QString) {
        self.as_mut().set_search(search);
        self.as_mut().set_status_filter(status_filter);
        self.as_mut().refilter();
    }

    pub fn set_platform_scope(mut self: Pin<&mut Self>, platform: QString) {
        self.as_mut().set_platform_filter(platform);
        self.as_mut().refilter();
    }

    fn refilter(mut self: Pin<&mut Self>) {
        let search = self
            .as_ref()
            .search()
            .to_string()
            .trim()
            .to_ascii_lowercase();
        let filter = self.as_ref().status_filter().to_string();
        let platform_key =
            crate::catalog::normalize_platform_key(&self.as_ref().platform_filter().to_string());
        let mut rows = self
            .as_ref()
            .rust()
            .all_rows
            .iter()
            .filter(|row| {
                row.is_compatible_with(&platform_key)
                    && (search.is_empty()
                        || row.name.to_ascii_lowercase().contains(&search)
                        || row.package_id.to_ascii_lowercase().contains(&search))
                    && match filter.as_str() {
                        "installed" => row.installed,
                        "available" => !row.installed && row.manager_available,
                        "managed" => row.managed,
                        _ => true,
                    }
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .is_recommended_for(&platform_key)
                .cmp(&left.is_recommended_for(&platform_key))
                .then_with(|| right.installed.cmp(&left.installed))
                .then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
        });
        let installed = self
            .as_ref()
            .rust()
            .all_rows
            .iter()
            .filter(|row| row.installed && row.is_compatible_with(&platform_key))
            .count();
        let available = self
            .as_ref()
            .rust()
            .all_rows
            .iter()
            .filter(|row| {
                !row.installed && row.manager_available && row.is_compatible_with(&platform_key)
            })
            .count();
        let row_count = rows.len();
        self.as_mut().rust_mut().rows = rows;
        self.as_mut().set_row_count(saturating_i32(row_count));
        self.as_mut().set_installed_count(saturating_i32(installed));
        self.as_mut().set_available_count(saturating_i32(available));
        let revision = self.as_ref().revision().saturating_add(1);
        self.as_mut().set_revision(revision);
    }

    pub fn name_at(&self, index: i32) -> QString {
        qstring(self.row(index).map(|row| row.name.as_str()).unwrap_or(""))
    }

    pub fn detail_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(ManagedEmulator::detail)
                .unwrap_or_default(),
        )
    }

    pub fn source_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.source_label.as_str())
                .unwrap_or(""),
        )
    }

    pub fn status_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(ManagedEmulator::status_label)
                .unwrap_or(""),
        )
    }

    pub fn recommended_at(&self, index: i32) -> bool {
        let platform_key =
            crate::catalog::normalize_platform_key(&self.platform_filter().to_string());
        self.row(index)
            .is_some_and(|row| row.is_recommended_for(&platform_key))
    }

    pub fn can_install_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(ManagedEmulator::can_install)
    }

    pub fn can_update_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(ManagedEmulator::can_update)
    }

    pub fn can_uninstall_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(ManagedEmulator::can_uninstall)
    }

    pub fn verify_recovery_probe(&self, dialog_visible: bool) -> bool {
        let activity = self.recent_activity().to_string();
        let ready = dialog_visible
            && *self.initialized()
            && *self.recovered_count() == 1
            && *self.recent_activity_attention()
            && activity.contains("RetroArch")
            && activity.contains("interrupted");
        let evidence = format!(
            "visible={dialog_visible} initialized={} recovered={} attention={} activity={activity:?}",
            *self.initialized(),
            *self.recovered_count(),
            *self.recent_activity_attention(),
        );
        if ready {
            println!("LUNCHBOX_EMULATOR_RECOVERY_UI_READY {evidence}");
        } else {
            eprintln!(
                "LUNCHBOX_EMULATOR_RECOVERY_UI_FAILED {evidence} message={:?}",
                self.message().to_string()
            );
        }
        ready
    }

    pub fn uninstall_confirmation_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(ManagedEmulator::uninstall_confirmation)
                .unwrap_or_else(|| "Select an emulator first.".to_owned()),
        )
    }

    pub fn install_at(self: Pin<&mut Self>, index: i32) {
        self.perform_at(index, ManagedAction::Install);
    }

    pub fn update_at(self: Pin<&mut Self>, index: i32) {
        self.perform_at(index, ManagedAction::Update);
    }

    pub fn uninstall_at(self: Pin<&mut Self>, index: i32) {
        self.perform_at(index, ManagedAction::Uninstall);
    }

    fn perform_at(mut self: Pin<&mut Self>, index: i32, action: ManagedAction) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(row) = self.as_ref().row(index).cloned() else {
            self.as_mut()
                .set_message(qstring("Select an emulator first."));
            return;
        };
        let verb = match action {
            ManagedAction::Install => "Installing",
            ManagedAction::Update => "Updating",
            ManagedAction::Uninstall => "Uninstalling",
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_busy_index(index);
        self.as_mut()
            .set_message(qstring(format!("{verb} {}…", row.name)));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emulator-lifecycle".into())
            .spawn(move || {
                let result = emulator_manager::perform_action(&row, action)
                    .and_then(|message| {
                        let rows = emulator_manager::load_managed_emulators()?;
                        let recent = emulator_manager::recent_emulator_lifecycle_operation()?;
                        Ok((message, rows, recent))
                    })
                    .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_action(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_busy_index(-1);
            self.as_mut().set_message(qstring(format!(
                "Could not start emulator operation: {error}"
            )));
        }
    }

    fn finish_action(
        mut self: Pin<&mut Self>,
        result: Result<
            (
                String,
                Vec<ManagedEmulator>,
                Option<crate::settings::EmulatorLifecycleOperation>,
            ),
            String,
        >,
    ) {
        match result {
            Ok((message, rows, recent)) => {
                self.as_mut().rust_mut().all_rows = rows;
                self.as_mut().refilter();
                self.as_mut().set_recent_operation(recent.as_ref());
                self.as_mut().set_message(qstring(message));
                let revision = self.as_ref().operation_revision().wrapping_add(1);
                self.as_mut().set_operation_revision(revision);
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMULATOR_OPERATION_FAILED error={error}");
                self.as_mut()
                    .set_message(qstring(format!("Emulator operation failed: {error}")));
            }
        }
        self.as_mut().set_busy(false);
        self.as_mut().set_busy_index(-1);
    }

    fn row(&self, index: i32) -> Option<&ManagedEmulator> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().rows.get(index))
    }

    fn set_recent_operation(
        mut self: Pin<&mut Self>,
        operation: Option<&crate::settings::EmulatorLifecycleOperation>,
    ) {
        let Some(operation) = operation else {
            self.as_mut().set_recent_activity(QString::default());
            self.as_mut().set_recent_activity_detail(QString::default());
            self.as_mut().set_recent_activity_attention(false);
            return;
        };
        self.as_mut()
            .set_recent_activity(qstring(recent_operation_label(operation)));
        self.as_mut()
            .set_recent_activity_detail(qstring(&operation.detail));
        self.as_mut()
            .set_recent_activity_attention(operation.status != "succeeded");
    }
}

fn recent_operation_label(operation: &crate::settings::EmulatorLifecycleOperation) -> String {
    let action = match (operation.action.as_str(), operation.status.as_str()) {
        ("install", "succeeded") => "Installed",
        ("update", "succeeded") => "Updated",
        ("uninstall", "succeeded") => "Uninstalled",
        ("install", _) => "Install",
        ("update", _) => "Update",
        ("uninstall", _) => "Uninstall",
        _ => "Operation",
    };
    let status = match operation.status.as_str() {
        "succeeded" => "complete",
        "failed" => "failed",
        "interrupted" => "interrupted",
        "cancelled" => "cancelled",
        "running" => "in progress",
        _ => "unknown",
    };
    format!("{action} {} · {status}", operation.display_name)
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn row(name: &str, installed: bool, managed: bool) -> ManagedEmulator {
        ManagedEmulator {
            emulator_id: name.to_ascii_lowercase(),
            name: name.into(),
            host_system_slug: "linux".into(),
            manager: "flatpak".into(),
            package_id: format!("org.example.{name}"),
            metadata_json: "{}".into(),
            source_label: "Flatpak".into(),
            installed,
            managed,
            manager_available: true,
            version: String::new(),
            install_path: String::new(),
            compatible_platform_keys: BTreeSet::from(["nintendo-entertainment-system".into()]),
            recommended_platform_keys: BTreeSet::from(["nintendo-entertainment-system".into()]),
        }
    }

    #[test]
    fn model_rows_allow_exact_package_manager_uninstalls() {
        let managed = row("Managed", true, true);
        let external = row("External", true, false);
        let available = row("Available", false, false);
        assert!(managed.can_uninstall());
        assert!(external.can_uninstall());
        assert!(external.can_update());
        assert!(available.can_install());
    }

    #[test]
    fn only_exact_package_managers_can_remove_external_installations() {
        for manager in ["flatpak", "winget", "homebrew"] {
            let mut external = row(manager, true, false);
            external.manager = manager.into();
            assert!(external.can_uninstall(), "{manager}");
            assert!(
                external
                    .uninstall_confirmation()
                    .contains(&external.package_id)
            );
        }

        for manager in ["nix", "appimage", "github", "direct", "libretro"] {
            let mut external = row(manager, true, false);
            external.manager = manager.into();
            assert!(!external.can_uninstall(), "{manager}");
        }
    }

    #[test]
    fn compatibility_is_exact_and_recommended_within_the_platform_scope() {
        let nes = row("Mesen", false, false);
        assert!(nes.is_compatible_with("nintendo-entertainment-system"));
        assert!(nes.is_recommended_for("nintendo-entertainment-system"));
        assert!(!nes.is_compatible_with("nintendo-switch"));
        assert!(nes.is_compatible_with(""));
    }

    #[test]
    fn recent_operation_labels_distinguish_success_and_interruption() {
        let mut operation = crate::settings::EmulatorLifecycleOperation {
            id: uuid::Uuid::new_v4().to_string(),
            emulator_id: "retroarch".into(),
            display_name: "RetroArch".into(),
            host_system_slug: "linux".into(),
            manager: "nix".into(),
            package_id: "retroarch".into(),
            install_path: "retroarch".into(),
            action: "update".into(),
            status: "succeeded".into(),
            detail: "Updated RetroArch".into(),
            started_at: 1,
            finished_at: Some(2),
        };
        assert_eq!(
            recent_operation_label(&operation),
            "Updated RetroArch · complete"
        );
        operation.status = "interrupted".into();
        assert_eq!(
            recent_operation_label(&operation),
            "Update RetroArch · interrupted"
        );
    }
}
