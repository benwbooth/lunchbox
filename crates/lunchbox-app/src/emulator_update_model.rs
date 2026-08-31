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
        #[qproperty(i32, actionable_count)]
        #[qproperty(i32, pinned_count)]
        #[qproperty(i32, selected_count)]
        #[qproperty(i32, completed_count)]
        #[qproperty(i32, batch_count)]
        #[qproperty(i32, revision)]
        #[qproperty(QString, message)]
        #[qproperty(QString, check_policy)]
        #[qproperty(bool, prompt_pending)]
        type EmulatorUpdateModel = super::EmulatorUpdateModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut EmulatorUpdateModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut EmulatorUpdateModel>);

        #[qinvokable]
        fn check_if_due(self: Pin<&mut EmulatorUpdateModel>);

        #[qinvokable]
        fn choose_check_policy(self: Pin<&mut EmulatorUpdateModel>, policy: QString);

        #[qinvokable]
        fn dismiss_prompt(self: Pin<&mut EmulatorUpdateModel>);

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
        fn pinned_at(self: &EmulatorUpdateModel, index: i32) -> bool;

        #[qinvokable]
        fn can_pin_at(self: &EmulatorUpdateModel, index: i32) -> bool;

        #[qinvokable]
        fn set_selected(self: Pin<&mut EmulatorUpdateModel>, index: i32, selected: bool);

        #[qinvokable]
        fn select_all(self: Pin<&mut EmulatorUpdateModel>, selected: bool);

        #[qinvokable]
        fn toggle_pin(self: Pin<&mut EmulatorUpdateModel>, index: i32);

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
    Pinned,
    Updating,
    Success,
    Failed,
}

impl UpdateState {
    fn label(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Pinned => "PINNED",
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
    actionable_count: i32,
    pinned_count: i32,
    selected_count: i32,
    completed_count: i32,
    batch_count: i32,
    revision: i32,
    message: QString,
    check_policy: QString,
    prompt_pending: bool,
    scheduled_check: bool,
    rows: Vec<UpdateRow>,
}

impl Default for EmulatorUpdateModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            row_count: 0,
            actionable_count: 0,
            pinned_count: 0,
            selected_count: 0,
            completed_count: 0,
            batch_count: 0,
            revision: 0,
            message: QString::from("Check for emulator updates when you are ready."),
            check_policy: QString::from("daily"),
            prompt_pending: false,
            scheduled_check: false,
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
        self.load_async("Checking installed emulator versions…", false);
    }

    pub fn refresh(self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.load_async("Refreshing available emulator updates…", false);
    }

    pub fn check_if_due(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        let preferences = match crate::settings::SettingsStore::open_default()
            .and_then(|store| store.emulator_update_preferences())
        {
            Ok(preferences) => preferences,
            Err(error) => {
                self.as_mut().set_message(qstring(format!(
                    "Could not load emulator update preferences: {error}"
                )));
                return;
            }
        };
        self.as_mut()
            .set_check_policy(qstring(&preferences.check_policy));
        if preferences.check_is_due(crate::settings::unix_timestamp()) {
            self.load_async("Checking for emulator updates in the background…", true);
        }
    }

    pub fn choose_check_policy(mut self: Pin<&mut Self>, policy: QString) {
        let policy = policy.to_string();
        match crate::settings::SettingsStore::open_default()
            .and_then(|store| store.set_emulator_update_check_policy(&policy))
        {
            Ok(()) => {
                self.as_mut().set_check_policy(qstring(&policy));
                self.as_mut().set_message(qstring(match policy.as_str() {
                    "daily" => "Lunchbox will check daily and ask before applying updates.",
                    "weekly" => "Lunchbox will check weekly and ask before applying updates.",
                    _ => "Automatic emulator update checks are disabled.",
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not save emulator update preferences: {error}"
            ))),
        }
    }

    pub fn dismiss_prompt(mut self: Pin<&mut Self>) {
        self.as_mut().set_prompt_pending(false);
    }

    fn load_async(mut self: Pin<&mut Self>, message: &str, scheduled: bool) {
        self.as_mut().rust_mut().scheduled_check = scheduled;
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
        let scheduled = self.as_ref().rust().scheduled_check;
        self.as_mut().rust_mut().scheduled_check = false;
        match result {
            Ok(inventory) => {
                let warning_count = inventory.warnings.len();
                let rows = update_rows(inventory.updates);
                let count = rows.len();
                let pinned_count = rows
                    .iter()
                    .filter(|row| row.state == UpdateState::Pinned)
                    .count();
                let actionable_count = count.saturating_sub(pinned_count);
                self.as_mut().rust_mut().rows = rows;
                self.as_mut().set_row_count(saturating_i32(count));
                self.as_mut()
                    .set_actionable_count(saturating_i32(actionable_count));
                self.as_mut().set_pinned_count(saturating_i32(pinned_count));
                self.as_mut()
                    .set_selected_count(saturating_i32(actionable_count));
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
                    update_inventory_message(actionable_count, pinned_count)
                } else {
                    format!(
                        "{} {warning_count} source{} could not be checked.",
                        update_inventory_message(actionable_count, pinned_count),
                        plural(warning_count)
                    )
                };
                self.as_mut().set_message(qstring(message));
                if scheduled {
                    if let Err(error) =
                        crate::settings::SettingsStore::open_default().and_then(|store| {
                            store.record_emulator_update_check(crate::settings::unix_timestamp())
                        })
                    {
                        eprintln!("LUNCHBOX_EMULATOR_UPDATE_CHECK_RECORD_FAILED error={error:#}");
                    }
                    self.as_mut().set_prompt_pending(actionable_count > 0);
                }
                println!(
                    "LUNCHBOX_EMULATOR_UPDATES_READY updates={actionable_count} pinned={pinned_count} warnings={warning_count}"
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
                .map(|row| {
                    if !row.status_detail.is_empty() {
                        row.status_detail.clone()
                    } else if let Some(pin) = &row.update.pin {
                        format!(
                            "Lunchbox will skip this installation in update batches; it was pinned at {}. External package-manager updates are unaffected.",
                            pin.pinned_version
                        )
                    } else {
                        String::new()
                    }
                })
                .unwrap_or_default(),
        )
    }

    pub fn selected_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(|row| row.selected)
    }

    pub fn pinned_at(&self, index: i32) -> bool {
        self.row(index)
            .is_some_and(|row| row.state == UpdateState::Pinned)
    }

    pub fn can_pin_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(|row| {
            !row.update.current_version.trim().is_empty()
                && !matches!(row.state, UpdateState::Updating | UpdateState::Success)
        })
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

    pub fn toggle_pin(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() {
            return;
        }
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let Some(row) = self.as_ref().rust().rows.get(index).cloned() else {
            return;
        };
        if row.update.current_version.trim().is_empty()
            || matches!(row.state, UpdateState::Updating | UpdateState::Success)
        {
            self.as_mut().set_message(qstring(
                "Lunchbox can pin only an exact detected installed version.",
            ));
            return;
        }
        let was_pinned = row.state == UpdateState::Pinned;
        let update = row.update.clone();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(if was_pinned {
            format!("Unpinning {}…", update.display_name)
        } else {
            format!(
                "Pinning {} at {}…",
                update.display_name, update.current_version
            )
        }));
        let qt_thread = self.as_ref().qt_thread();
        let worker_update = update.clone();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emulator-update-pin".into())
            .spawn(move || {
                let result = crate::settings::SettingsStore::open_default()
                    .and_then(|store| {
                        if was_pinned {
                            store.remove_emulator_update_pin(
                                &worker_update.row.host_system_slug,
                                &worker_update.row.manager,
                                &worker_update.row.package_id,
                                &worker_update.row.install_path,
                            )?;
                            Ok(None)
                        } else {
                            store
                                .set_emulator_update_pin(
                                    &worker_update.row.host_system_slug,
                                    &worker_update.row.manager,
                                    &worker_update.row.package_id,
                                    &worker_update.row.install_path,
                                    &worker_update.current_version,
                                )
                                .map(Some)
                        }
                    })
                    .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model
                        .as_mut()
                        .finish_toggle_pin(index, was_pinned, update, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the emulator version-pin change: {error}"
            )));
        }
    }

    fn finish_toggle_pin(
        mut self: Pin<&mut Self>,
        index: usize,
        was_pinned: bool,
        update: ManagedEmulatorUpdate,
        result: Result<Option<crate::settings::EmulatorUpdatePin>, String>,
    ) {
        self.as_mut().set_busy(false);
        match result {
            Ok(pin) => {
                if index >= self.as_ref().rust().rows.len() {
                    self.as_mut().set_message(qstring(
                        "The emulator update list changed before the pin could be applied.",
                    ));
                    return;
                }
                let row = &mut self.as_mut().rust_mut().rows[index];
                row.update.pin = pin;
                row.state = if was_pinned {
                    UpdateState::Pending
                } else {
                    UpdateState::Pinned
                };
                row.selected = false;
                row.status_detail.clear();
                self.as_mut().update_counts();
                self.as_mut().bump_revision();
                println!(
                    "LUNCHBOX_EMULATOR_UPDATE_PIN_CHANGED action={} manager={} package={} pinned={}",
                    if was_pinned { "unpin" } else { "pin" },
                    update.row.manager,
                    update.row.package_id,
                    self.as_ref().pinned_count()
                );
                self.as_mut().set_message(qstring(if was_pinned {
                    format!(
                        "{} is unpinned. Select it if you want to apply the available update.",
                        update.display_name
                    )
                } else {
                    format!(
                        "{} is pinned at {}. Lunchbox will not offer it in update batches.",
                        update.display_name, update.current_version
                    )
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not change the emulator version pin: {error}"
            ))),
        }
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
        self.as_mut().set_prompt_pending(false);
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
        self.as_mut().update_counts();
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
        self.as_mut().update_counts();
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

    fn update_counts(mut self: Pin<&mut Self>) {
        let pinned = self
            .as_ref()
            .rust()
            .rows
            .iter()
            .filter(|row| row.state == UpdateState::Pinned)
            .count();
        let actionable = self
            .as_ref()
            .rust()
            .rows
            .iter()
            .filter(|row| row.state.selectable())
            .count();
        self.as_mut().set_pinned_count(saturating_i32(pinned));
        self.as_mut()
            .set_actionable_count(saturating_i32(actionable));
        self.as_mut().update_selected_count();
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

fn update_rows(updates: Vec<ManagedEmulatorUpdate>) -> Vec<UpdateRow> {
    updates
        .into_iter()
        .map(|update| {
            let pinned = update.pin.is_some();
            UpdateRow {
                update,
                selected: !pinned,
                state: if pinned {
                    UpdateState::Pinned
                } else {
                    UpdateState::Pending
                },
                status_detail: String::new(),
            }
        })
        .collect()
}

fn update_inventory_message(actionable: usize, pinned: usize) -> String {
    match (actionable, pinned) {
        (0, pinned) => format!(
            "No actionable emulator updates; {pinned} available update{} pinned.",
            plural(pinned)
        ),
        (actionable, 0) => {
            format!(
                "{actionable} emulator update{} available.",
                plural(actionable)
            )
        }
        (actionable, pinned) => format!(
            "{actionable} emulator update{} available; {pinned} pinned.",
            plural(actionable)
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::emulator_manager::ManagedEmulator;
    use crate::settings::EmulatorUpdatePin;

    fn update(name: &str, install_path: &str, pinned: bool) -> ManagedEmulatorUpdate {
        let row = ManagedEmulator {
            emulator_id: format!("{name}-id"),
            name: name.to_owned(),
            host_system_slug: "linux".to_owned(),
            manager: "flatpak".to_owned(),
            package_id: format!("org.example.{name}"),
            metadata_json: String::new(),
            source_label: "Flatpak".to_owned(),
            installed: true,
            managed: false,
            manager_available: true,
            version: "1.0".to_owned(),
            install_path: install_path.to_owned(),
            compatible_platform_keys: BTreeSet::new(),
            recommended_platform_keys: BTreeSet::new(),
        };
        ManagedEmulatorUpdate {
            row,
            display_name: name.to_owned(),
            source_label: format!("Flatpak · {install_path}"),
            current_version: "1.0".to_owned(),
            available_version: "2.0".to_owned(),
            pin: pinned.then(|| EmulatorUpdatePin {
                host_system_slug: "linux".to_owned(),
                manager: "flatpak".to_owned(),
                package_id: format!("org.example.{name}"),
                install_path: install_path.to_owned(),
                pinned_version: "1.0".to_owned(),
                pinned_at: 10,
            }),
        }
    }

    #[test]
    fn pinned_updates_stay_visible_but_are_not_selected_or_actionable() {
        let rows = update_rows(vec![
            update("RetroArch", "user", true),
            update("MAME", "system", false),
        ]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].state, UpdateState::Pinned);
        assert!(!rows[0].selected);
        assert!(!rows[0].state.selectable());
        assert_eq!(rows[1].state, UpdateState::Pending);
        assert!(rows[1].selected);
        assert!(rows[1].state.selectable());
        assert_eq!(
            update_inventory_message(1, 1),
            "1 emulator update available; 1 pinned."
        );
        assert_eq!(
            update_inventory_message(0, 2),
            "No actionable emulator updates; 2 available updates pinned."
        );
    }
}
