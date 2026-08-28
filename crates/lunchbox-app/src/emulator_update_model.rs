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
        #[qproperty(i32, row_count)]
        #[qproperty(i32, selected_count)]
        #[qproperty(i32, completed_count)]
        #[qproperty(i32, batch_count)]
        #[qproperty(i32, revision)]
        #[qproperty(QString, message)]
        type EmulatorUpdateModel = super::EmulatorUpdateModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut EmulatorUpdateModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut EmulatorUpdateModel>);

        #[qinvokable]
        fn name_at(self: &EmulatorUpdateModel, index: i32) -> QString;

        #[qinvokable]
        fn source_at(self: &EmulatorUpdateModel, index: i32) -> QString;

        #[qinvokable]
        fn version_at(self: &EmulatorUpdateModel, index: i32) -> QString;

        #[qinvokable]
        fn status_at(self: &EmulatorUpdateModel, index: i32) -> QString;

        #[qinvokable]
        fn status_detail_at(self: &EmulatorUpdateModel, index: i32) -> QString;

        #[qinvokable]
        fn selected_at(self: &EmulatorUpdateModel, index: i32) -> bool;

        #[qinvokable]
        fn set_selected(self: Pin<&mut EmulatorUpdateModel>, index: i32, selected: bool);

        #[qinvokable]
        fn select_all(self: Pin<&mut EmulatorUpdateModel>, selected: bool);

        #[qinvokable]
        fn update_selected(self: Pin<&mut EmulatorUpdateModel>);
    }

    impl cxx_qt::Threading for EmulatorUpdateModel {}
}

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::emulator_manager::{self, ManagedAction, ManagedEmulatorUpdate};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UpdateState {
    Pending,
    Updating,
    Success,
    Failed,
}

impl UpdateState {
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Updating => "UPDATING",
            Self::Success => "UPDATED",
            Self::Failed => "FAILED",
        }
    }

    fn selectable(self) -> bool {
        matches!(self, Self::Pending | Self::Failed)
    }
}

#[derive(Clone, Debug)]
struct UpdateRow {
    update: ManagedEmulatorUpdate,
    selected: bool,
    state: UpdateState,
    status_detail: String,
}

pub struct EmulatorUpdateModelRust {
    initialized: bool,
    busy: bool,
    row_count: i32,
    selected_count: i32,
    completed_count: i32,
    batch_count: i32,
    revision: i32,
    message: QString,
    rows: Vec<UpdateRow>,
}

impl Default for EmulatorUpdateModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            row_count: 0,
            selected_count: 0,
            completed_count: 0,
            batch_count: 0,
            revision: 0,
            message: QString::from("Check for emulator updates when you are ready."),
            rows: Vec::new(),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::EmulatorUpdateModel {
    pub fn initialize(self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.load_async("Checking installed emulator versions…");
    }

    pub fn refresh(self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.load_async("Refreshing available emulator updates…");
    }

    fn load_async(mut self: Pin<&mut Self>, message: &str) {
        self.as_mut().set_busy(true);
        self.as_mut().set_completed_count(0);
        self.as_mut().set_batch_count(0);
        self.as_mut().set_message(qstring(message));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emulator-update-discovery".into())
            .spawn(move || {
                let result = emulator_manager::load_available_emulator_updates()
                    .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_load(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start emulator update discovery: {error}"
            )));
        }
    }

    fn finish_load(
        mut self: Pin<&mut Self>,
        result: Result<emulator_manager::EmulatorUpdateInventory, String>,
    ) {
        self.as_mut().set_busy(false);
        match result {
            Ok(inventory) => {
                let warning_count = inventory.warnings.len();
                let rows = inventory
                    .updates
                    .into_iter()
                    .map(|update| UpdateRow {
                        update,
                        selected: true,
                        state: UpdateState::Pending,
                        status_detail: String::new(),
                    })
                    .collect::<Vec<_>>();
                let count = rows.len();
                self.as_mut().rust_mut().rows = rows;
                self.as_mut().set_row_count(saturating_i32(count));
                self.as_mut().set_selected_count(saturating_i32(count));
                self.as_mut().set_completed_count(0);
                self.as_mut().set_batch_count(0);
                self.as_mut().set_initialized(true);
                self.as_mut().bump_revision();
                let message = if count == 0 && warning_count == 0 {
                    "All supported emulator installations are up to date.".to_owned()
                } else if count == 0 {
                    format!(
                        "No updates found. {warning_count} source{} could not be checked.",
                        plural(warning_count)
                    )
                } else if warning_count == 0 {
                    format!("{count} emulator update{} available.", plural(count))
                } else {
                    format!(
                        "{count} update{} available; {warning_count} source{} could not be checked.",
                        plural(count),
                        plural(warning_count)
                    )
                };
                self.as_mut().set_message(qstring(message));
                println!(
                    "LUNCHBOX_EMULATOR_UPDATES_READY updates={count} warnings={warning_count}"
                );
            }
            Err(error) => {
                eprintln!("LUNCHBOX_EMULATOR_UPDATES_FAILED error={error}");
                self.as_mut().set_message(qstring(format!(
                    "Could not check emulator updates: {error}"
                )));
            }
        }
    }

    pub fn name_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.update.display_name.as_str())
                .unwrap_or(""),
        )
    }

    pub fn source_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.update.source_label.as_str())
                .unwrap_or(""),
        )
    }

    pub fn version_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| {
                    let current = if row.update.current_version.is_empty() {
                        "installed build"
                    } else {
                        row.update.current_version.as_str()
                    };
                    let available = if row.update.available_version.is_empty() {
                        "new build"
                    } else {
                        row.update.available_version.as_str()
                    };
                    format!("{current}  →  {available}")
                })
                .unwrap_or_default(),
        )
    }

    pub fn status_at(&self, index: i32) -> QString {
        qstring(self.row(index).map(|row| row.state.label()).unwrap_or(""))
    }

    pub fn status_detail_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.status_detail.as_str())
                .unwrap_or(""),
        )
    }

    pub fn selected_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(|row| row.selected)
    }

    pub fn set_selected(mut self: Pin<&mut Self>, index: i32, selected: bool) {
        if *self.as_ref().busy() {
            return;
        }
        if let Ok(index) = usize::try_from(index) {
            let mut rust = self.as_mut().rust_mut();
            if let Some(row) = rust.rows.get_mut(index)
                && row.state.selectable()
            {
                row.selected = selected;
                self.as_mut().update_selected_count();
                self.as_mut().bump_revision();
            }
        }
    }

    pub fn select_all(mut self: Pin<&mut Self>, selected: bool) {
        if *self.as_ref().busy() {
            return;
        }
        for row in &mut self.as_mut().rust_mut().rows {
            if row.state.selectable() {
                row.selected = selected;
            }
        }
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
    }

    pub fn update_selected(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let selected = self
            .as_ref()
            .rust()
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.selected && row.state.selectable())
            .map(|(index, row)| (index, row.update.clone()))
            .collect::<Vec<_>>();
        if selected.is_empty() {
            self.as_mut()
                .set_message(qstring("Select at least one pending emulator update."));
            return;
        }
        let total = selected.len();
        self.as_mut().set_busy(true);
        self.as_mut().set_completed_count(0);
        self.as_mut().set_batch_count(saturating_i32(total));
        self.as_mut().set_message(qstring(format!(
            "Updating {total} emulator installation{}…",
            plural(total)
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emulator-update-batch".into())
            .spawn(move || {
                let mut succeeded = 0usize;
                let mut failed = 0usize;
                for (index, update) in selected {
                    let started_thread = qt_thread.clone();
                    let _ = started_thread.queue(move |mut model| {
                        model.as_mut().mark_updating(index);
                    });
                    let result =
                        emulator_manager::perform_action(&update.row, ManagedAction::Update)
                            .map_err(|error| format!("{error:#}"));
                    if result.is_ok() {
                        succeeded += 1;
                    } else {
                        failed += 1;
                    }
                    let completed_thread = qt_thread.clone();
                    let _ = completed_thread.queue(move |mut model| {
                        model.as_mut().finish_update(index, result);
                    });
                }
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_batch(succeeded, failed);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start emulator updates: {error}"
            )));
        }
    }

    fn mark_updating(mut self: Pin<&mut Self>, index: usize) {
        if let Some(row) = self.as_mut().rust_mut().rows.get_mut(index) {
            row.state = UpdateState::Updating;
            row.status_detail.clear();
        }
        self.as_mut().bump_revision();
    }

    fn finish_update(mut self: Pin<&mut Self>, index: usize, result: Result<String, String>) {
        if let Some(row) = self.as_mut().rust_mut().rows.get_mut(index) {
            match result {
                Ok(message) => {
                    row.state = UpdateState::Success;
                    row.selected = false;
                    row.status_detail = message;
                }
                Err(error) => {
                    row.state = UpdateState::Failed;
                    row.status_detail = error;
                }
            }
        }
        let completed = self.as_ref().completed_count().saturating_add(1);
        self.as_mut().set_completed_count(completed);
        self.as_mut().update_selected_count();
        self.as_mut().bump_revision();
    }

    fn finish_batch(mut self: Pin<&mut Self>, succeeded: usize, failed: usize) {
        self.as_mut().set_busy(false);
        let message = if failed == 0 {
            format!(
                "Updated {succeeded} emulator installation{}. Refresh to verify installed versions.",
                plural(succeeded)
            )
        } else {
            format!("Updated {succeeded}; {failed} failed. Failed rows retain their error details.")
        };
        self.as_mut().set_message(qstring(message));
        println!("LUNCHBOX_EMULATOR_UPDATE_BATCH succeeded={succeeded} failed={failed}");
    }

    fn row(&self, index: i32) -> Option<&UpdateRow> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().rows.get(index))
    }

    fn update_selected_count(mut self: Pin<&mut Self>) {
        let count = self
            .as_ref()
            .rust()
            .rows
            .iter()
            .filter(|row| row.selected && row.state.selectable())
            .count();
        self.as_mut().set_selected_count(saturating_i32(count));
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn plural(value: usize) -> &'static str {
    if value == 1 { "" } else { "s" }
}
