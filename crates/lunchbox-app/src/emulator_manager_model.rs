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
        #[qproperty(i32, revision)]
        #[qproperty(QString, message)]
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
    revision: i32,
    message: QString,
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
            revision: 0,
            message: QString::from("Open Emulator Manager to detect installed runtimes."),
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
                let result = emulator_manager::load_managed_emulators()
                    .map_err(|error| format!("{error:#}"));
                if let Ok(rows) = &result {
                    println!("LUNCHBOX_EMULATOR_DISCOVERY_READY runtimes={}", rows.len());
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

    fn finish_load(mut self: Pin<&mut Self>, result: Result<Vec<ManagedEmulator>, String>) {
        self.as_mut().set_busy(false);
        self.as_mut().set_busy_index(-1);
        match result {
            Ok(rows) => {
                let detected = rows.len();
                self.as_mut().rust_mut().all_rows = rows;
                self.as_mut().refilter();
                self.as_mut().set_initialized(true);
                let installed = *self.as_ref().installed_count();
                let total = self.as_ref().rust().all_rows.len();
                self.as_mut().set_message(qstring(format!(
                    "Detected {installed} installed runtime{} across {total} managed emulator choices.",
                    if installed == 1 { "" } else { "s" }
                )));
                println!(
                    "LUNCHBOX_EMULATOR_MANAGER_READY runtimes={detected} installed={installed}"
                );
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMULATOR_MANAGER_FAILED error={error}");
                self.as_mut()
                    .set_message(qstring(format!("Could not load emulator catalog: {error}")));
            }
        }
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
                        emulator_manager::load_managed_emulators().map(|rows| (message, rows))
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
        result: Result<(String, Vec<ManagedEmulator>), String>,
    ) {
        self.as_mut().set_busy(false);
        self.as_mut().set_busy_index(-1);
        match result {
            Ok((message, rows)) => {
                self.as_mut().rust_mut().all_rows = rows;
                self.as_mut().refilter();
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
    }

    fn row(&self, index: i32) -> Option<&ManagedEmulator> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().rows.get(index))
    }
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
    fn model_rows_keep_external_installs_out_of_uninstall_actions() {
        let managed = row("Managed", true, true);
        let external = row("External", true, false);
        let available = row("Available", false, false);
        assert!(managed.can_uninstall());
        assert!(!external.can_uninstall());
        assert!(external.can_update());
        assert!(available.can_install());
    }

    #[test]
    fn compatibility_is_exact_and_recommended_within_the_platform_scope() {
        let nes = row("Mesen", false, false);
        assert!(nes.is_compatible_with("nintendo-entertainment-system"));
        assert!(nes.is_recommended_for("nintendo-entertainment-system"));
        assert!(!nes.is_compatible_with("nintendo-switch"));
        assert!(nes.is_compatible_with(""));
    }
}
