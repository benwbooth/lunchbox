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
        #[qproperty(i32, customized_count)]
        #[qproperty(i32, revision)]
        #[qproperty(QString, message)]
        #[qproperty(QString, search)]
        #[qproperty(QString, scope_filter)]
        #[qproperty(QString, customization_filter)]
        #[qproperty(bool, editor_open)]
        #[qproperty(QString, editor_title)]
        #[qproperty(QString, editor_identity)]
        #[qproperty(QString, editor_default_template)]
        #[qproperty(QString, editor_extra_arguments)]
        #[qproperty(QString, editor_command_template)]
        #[qproperty(QString, editor_effective_summary)]
        #[qproperty(QString, editor_status)]
        #[qproperty(i32, editor_revision)]
        #[qproperty(bool, launch_profile_preview_valid)]
        #[qproperty(QString, launch_profile_preview_runtime)]
        #[qproperty(QString, launch_profile_preview_message)]
        #[qproperty(i32, launch_profile_preview_argument_count)]
        #[qproperty(i32, launch_profile_preview_revision)]
        type LaunchProfileManagerModel = super::LaunchProfileManagerModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut LaunchProfileManagerModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut LaunchProfileManagerModel>);

        #[qinvokable]
        fn apply_filter(
            self: Pin<&mut LaunchProfileManagerModel>,
            search: QString,
            scope_filter: QString,
            customization_filter: QString,
        );

        #[qinvokable]
        fn scope_at(self: &LaunchProfileManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn emulator_at(self: &LaunchProfileManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn runtime_at(self: &LaunchProfileManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn default_template_at(self: &LaunchProfileManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn customization_at(self: &LaunchProfileManagerModel, index: i32) -> QString;

        #[qinvokable]
        fn customized_at(self: &LaunchProfileManagerModel, index: i32) -> bool;

        #[qinvokable]
        fn edit_at(self: Pin<&mut LaunchProfileManagerModel>, index: i32);

        #[qinvokable]
        fn close_editor(self: Pin<&mut LaunchProfileManagerModel>);

        #[qinvokable]
        fn save_editor(
            self: Pin<&mut LaunchProfileManagerModel>,
            extra_arguments: QString,
            command_template: QString,
        );

        #[qinvokable]
        fn clear_editor(self: Pin<&mut LaunchProfileManagerModel>);

        #[qinvokable]
        fn update_launch_profile_preview(
            self: Pin<&mut LaunchProfileManagerModel>,
            extra_arguments: QString,
            command_template: QString,
        );

        #[qinvokable]
        fn launch_profile_preview_argument_at(
            self: &LaunchProfileManagerModel,
            index: i32,
        ) -> QString;
    }

    impl cxx_qt::Threading for LaunchProfileManagerModel {}
}

use std::collections::HashMap;
use std::pin::Pin;

use anyhow::{Context, Result, bail};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::emulator::{
    EmulatorRuntimeKind, LaunchCommandPreview, default_rom_extra_argument_insert_index,
    default_rom_launch_template_for, effective_launch_preview_values, launch_template_placeholders,
    preview_launch_command,
};
use crate::settings::{EmulatorLaunchProfile, ResolvedLaunchCustomization, SettingsStore};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LaunchProfileKey {
    scope_kind: String,
    scope_key: String,
    emulator_id: String,
    runtime_kind: String,
    core_name: String,
}

impl From<&EmulatorLaunchProfile> for LaunchProfileKey {
    fn from(profile: &EmulatorLaunchProfile) -> Self {
        Self {
            scope_kind: profile.scope_kind.clone(),
            scope_key: profile.scope_key.clone(),
            emulator_id: profile.emulator_id.clone(),
            runtime_kind: profile.runtime_kind.clone(),
            core_name: profile.core_name.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LaunchProfileRow {
    key: LaunchProfileKey,
    scope_label: String,
    platform_name: String,
    emulator_name: String,
    default_template: String,
    preview_extra_insert_index: usize,
    profile: Option<EmulatorLaunchProfile>,
}

impl LaunchProfileRow {
    fn customized(&self) -> bool {
        self.profile.as_ref().is_some_and(|profile| {
            !profile.extra_arguments.is_empty() || !profile.command_template.is_empty()
        })
    }

    fn runtime_label(&self) -> String {
        if self.key.runtime_kind == "retroarch" {
            format!("RetroArch · {}", self.key.core_name)
        } else {
            "Standalone".to_owned()
        }
    }

    fn customization_summary(&self) -> String {
        let Some(profile) = &self.profile else {
            return "Built-in".to_owned();
        };
        match (
            profile.extra_arguments.is_empty(),
            profile.command_template.is_empty(),
        ) {
            (false, false) => "Arguments + template".to_owned(),
            (false, true) => "Extra arguments".to_owned(),
            (true, false) => "Command template".to_owned(),
            (true, true) => "Built-in".to_owned(),
        }
    }

    fn search_text(&self) -> String {
        let profile = self.profile.as_ref();
        format!(
            "{} {} {} {} {} {} {}",
            self.scope_label,
            self.platform_name,
            self.emulator_name,
            self.runtime_label(),
            self.default_template,
            profile
                .map(|value| value.extra_arguments.as_str())
                .unwrap_or(""),
            profile
                .map(|value| value.command_template.as_str())
                .unwrap_or("")
        )
        .to_ascii_lowercase()
    }
}

pub struct LaunchProfileManagerModelRust {
    initialized: bool,
    busy: bool,
    row_count: i32,
    customized_count: i32,
    revision: i32,
    message: QString,
    search: QString,
    scope_filter: QString,
    customization_filter: QString,
    editor_open: bool,
    editor_title: QString,
    editor_identity: QString,
    editor_default_template: QString,
    editor_extra_arguments: QString,
    editor_command_template: QString,
    editor_effective_summary: QString,
    editor_status: QString,
    editor_revision: i32,
    launch_profile_preview_valid: bool,
    launch_profile_preview_runtime: QString,
    launch_profile_preview_message: QString,
    launch_profile_preview_argument_count: i32,
    launch_profile_preview_revision: i32,
    all_rows: Vec<LaunchProfileRow>,
    rows: Vec<LaunchProfileRow>,
    selected_key: Option<LaunchProfileKey>,
    launch_profile_preview_arguments: Vec<String>,
    launch_profile_preview_fallback_extra_arguments: String,
    launch_profile_preview_fallback_command_template: String,
}

impl Default for LaunchProfileManagerModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            row_count: 0,
            customized_count: 0,
            revision: 0,
            message: QString::from("Open Launch Commands to load exact runtime profiles."),
            search: QString::default(),
            scope_filter: QString::from("all"),
            customization_filter: QString::from("all"),
            editor_open: false,
            editor_title: QString::default(),
            editor_identity: QString::default(),
            editor_default_template: QString::default(),
            editor_extra_arguments: QString::default(),
            editor_command_template: QString::default(),
            editor_effective_summary: QString::default(),
            editor_status: QString::default(),
            editor_revision: 0,
            launch_profile_preview_valid: false,
            launch_profile_preview_runtime: QString::default(),
            launch_profile_preview_message: QString::default(),
            launch_profile_preview_argument_count: 0,
            launch_profile_preview_revision: 0,
            all_rows: Vec::new(),
            rows: Vec::new(),
            selected_key: None,
            launch_profile_preview_arguments: Vec::new(),
            launch_profile_preview_fallback_extra_arguments: String::new(),
            launch_profile_preview_fallback_command_template: String::new(),
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::LaunchProfileManagerModel {
    pub fn initialize(self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.load_async("Loading launch command profiles…");
    }

    pub fn refresh(self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.load_async("Refreshing launch command profiles…");
    }

    fn load_async(mut self: Pin<&mut Self>, message: &str) {
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(message));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-launch-profile-load".into())
            .spawn(move || {
                let result = load_launch_profile_rows().map_err(|error| format!("{error:#}"));
                if let Err(error) = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_load(result);
                }) {
                    eprintln!("LUNCHBOX_LAUNCH_PROFILE_MANAGER_QUEUE_FAILED error={error}");
                }
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start launch profile worker: {error}"
            )));
        }
    }

    fn finish_load(mut self: Pin<&mut Self>, result: Result<Vec<LaunchProfileRow>, String>) {
        self.as_mut().set_busy(false);
        match result {
            Ok(rows) => {
                self.as_mut().rust_mut().all_rows = rows;
                self.as_mut().refilter();
                self.as_mut().set_initialized(true);
                let total = self.as_ref().rust().all_rows.len();
                let customized = *self.as_ref().customized_count();
                self.as_mut().set_message(qstring(format!(
                    "{total} exact global and platform runtime profiles · {customized} customized"
                )));
                println!(
                    "LUNCHBOX_LAUNCH_PROFILE_MANAGER_READY rows={total} customized={customized}"
                );
            }
            Err(error) => {
                eprintln!("LUNCHBOX_LAUNCH_PROFILE_MANAGER_FAILED error={error}");
                self.as_mut()
                    .set_message(qstring(format!("Could not load launch profiles: {error}")));
            }
        }
    }

    pub fn apply_filter(
        mut self: Pin<&mut Self>,
        search: QString,
        scope_filter: QString,
        customization_filter: QString,
    ) {
        self.as_mut().set_search(search);
        self.as_mut().set_scope_filter(scope_filter);
        self.as_mut().set_customization_filter(customization_filter);
        self.as_mut().refilter();
    }

    fn refilter(mut self: Pin<&mut Self>) {
        let search = self
            .as_ref()
            .search()
            .to_string()
            .trim()
            .to_ascii_lowercase();
        let scope_filter = self.as_ref().scope_filter().to_string();
        let customization_filter = self.as_ref().customization_filter().to_string();
        let rows = filter_rows(
            &self.as_ref().rust().all_rows,
            &search,
            &scope_filter,
            &customization_filter,
        );
        let row_count = rows.len();
        let customized_count = self
            .as_ref()
            .rust()
            .all_rows
            .iter()
            .filter(|row| row.customized())
            .count();
        self.as_mut().rust_mut().rows = rows;
        self.as_mut().set_row_count(saturating_i32(row_count));
        self.as_mut()
            .set_customized_count(saturating_i32(customized_count));
        let revision = self.as_ref().revision().saturating_add(1);
        self.as_mut().set_revision(revision);
    }

    pub fn scope_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.scope_label.as_str())
                .unwrap_or(""),
        )
    }

    pub fn emulator_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.emulator_name.as_str())
                .unwrap_or(""),
        )
    }

    pub fn runtime_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.runtime_label())
                .unwrap_or_default(),
        )
    }

    pub fn default_template_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.default_template.as_str())
                .unwrap_or(""),
        )
    }

    pub fn customization_at(&self, index: i32) -> QString {
        qstring(
            self.row(index)
                .map(|row| row.customization_summary())
                .unwrap_or_default(),
        )
    }

    pub fn customized_at(&self, index: i32) -> bool {
        self.row(index).is_some_and(LaunchProfileRow::customized)
    }

    pub fn edit_at(mut self: Pin<&mut Self>, index: i32) {
        let Some(row) = self.row(index).cloned() else {
            return;
        };
        self.as_mut().rust_mut().selected_key = Some(row.key.clone());
        self.as_mut().set_editor_open(true);
        self.as_mut().set_editor_title(qstring(format!(
            "{} · {}",
            row.emulator_name,
            row.runtime_label()
        )));
        self.as_mut().set_editor_identity(qstring(format!(
            "{} · exact emulator ID {}",
            row.scope_label, row.key.emulator_id
        )));
        self.as_mut()
            .set_editor_default_template(qstring(&row.default_template));
        self.as_mut().set_editor_extra_arguments(qstring(
            row.profile
                .as_ref()
                .map(|profile| profile.extra_arguments.as_str())
                .unwrap_or(""),
        ));
        self.as_mut().set_editor_command_template(qstring(
            row.profile
                .as_ref()
                .map(|profile| profile.command_template.as_str())
                .unwrap_or(""),
        ));
        match resolved_for_row(&row) {
            Ok(resolved) => self
                .as_mut()
                .set_editor_effective_summary(qstring(effective_summary(&resolved))),
            Err(error) => self.as_mut().set_editor_effective_summary(qstring(format!(
                "Could not resolve inherited values: {error:#}"
            ))),
        }
        self.as_mut().set_editor_status(QString::default());
        let exact_extra_arguments = row
            .profile
            .as_ref()
            .map(|profile| profile.extra_arguments.as_str())
            .unwrap_or("");
        let exact_command_template = row
            .profile
            .as_ref()
            .map(|profile| profile.command_template.as_str())
            .unwrap_or("");
        let fallback = preview_fallback_for_row(&row, &self.as_ref().rust().all_rows);
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_fallback_extra_arguments = fallback.extra_arguments.clone();
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_fallback_command_template = fallback.command_template.clone();
        let (effective_extra_arguments, effective_command_template) =
            effective_launch_preview_values(
                exact_extra_arguments,
                exact_command_template,
                &fallback.extra_arguments,
                &fallback.command_template,
            );
        let preview = preview_for_row(
            &row,
            &effective_extra_arguments,
            &effective_command_template,
        );
        self.as_mut().publish_launch_profile_preview(preview);
        self.as_mut().bump_editor_revision();
    }

    pub fn close_editor(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().selected_key = None;
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_fallback_extra_arguments
            .clear();
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_fallback_command_template
            .clear();
        self.as_mut().set_editor_open(false);
        self.as_mut().set_editor_status(QString::default());
        self.as_mut().clear_launch_profile_preview();
        self.as_mut().bump_editor_revision();
    }

    pub fn save_editor(
        mut self: Pin<&mut Self>,
        extra_arguments: QString,
        command_template: QString,
    ) {
        let Some(row) = self.selected_row().cloned() else {
            self.as_mut()
                .set_editor_status(qstring("Choose a launch profile row first."));
            return;
        };
        let extra_arguments = extra_arguments.to_string();
        let command_template = command_template.to_string();
        if let Err(error) = validate_template_for_row(&row, &command_template) {
            self.as_mut().set_editor_status(qstring(format!(
                "Could not save the launch profile: {error}"
            )));
            return;
        }
        let profile = EmulatorLaunchProfile {
            scope_kind: row.key.scope_kind.clone(),
            scope_key: row.key.scope_key.clone(),
            emulator_id: row.key.emulator_id.clone(),
            runtime_kind: row.key.runtime_kind.clone(),
            core_name: row.key.core_name.clone(),
            extra_arguments,
            command_template,
            updated_at: 0,
        };
        match SettingsStore::open_default()
            .and_then(|store| store.set_emulator_launch_profile(&profile))
        {
            Ok(()) => {
                self.as_mut().update_profile_row(
                    &row.key,
                    if profile.extra_arguments.trim().is_empty()
                        && profile.command_template.trim().is_empty()
                    {
                        None
                    } else {
                        Some(EmulatorLaunchProfile {
                            extra_arguments: profile.extra_arguments.trim().to_owned(),
                            command_template: profile.command_template.trim().to_owned(),
                            ..profile
                        })
                    },
                );
                self.as_mut().set_editor_status(qstring(format!(
                    "Saved the exact {} profile.",
                    row.scope_label.to_ascii_lowercase()
                )));
                println!(
                    "LUNCHBOX_LAUNCH_PROFILE_MANAGER_SAVED scope={} emulator={} runtime={} core={}",
                    row.key.scope_kind,
                    row.key.emulator_id,
                    row.key.runtime_kind,
                    row.key.core_name
                );
            }
            Err(error) => self.as_mut().set_editor_status(qstring(format!(
                "Could not save the launch profile: {error:#}"
            ))),
        }
    }

    pub fn clear_editor(mut self: Pin<&mut Self>) {
        let Some(row) = self.selected_row().cloned() else {
            self.as_mut()
                .set_editor_status(qstring("Choose a launch profile row first."));
            return;
        };
        match SettingsStore::open_default().and_then(|store| {
            store.clear_emulator_launch_profile(
                &row.key.scope_kind,
                &row.key.scope_key,
                &row.key.emulator_id,
                &row.key.runtime_kind,
                &row.key.core_name,
            )
        }) {
            Ok(()) => {
                self.as_mut().update_profile_row(&row.key, None);
                self.as_mut()
                    .set_editor_status(qstring("Cleared this exact profile; inheritance applies."));
            }
            Err(error) => self.as_mut().set_editor_status(qstring(format!(
                "Could not clear the launch profile: {error:#}"
            ))),
        }
    }

    pub fn update_launch_profile_preview(
        mut self: Pin<&mut Self>,
        extra_arguments: QString,
        command_template: QString,
    ) {
        let fallback_extra_arguments = self
            .as_ref()
            .rust()
            .launch_profile_preview_fallback_extra_arguments
            .clone();
        let fallback_command_template = self
            .as_ref()
            .rust()
            .launch_profile_preview_fallback_command_template
            .clone();
        let (effective_extra_arguments, effective_command_template) =
            effective_launch_preview_values(
                &extra_arguments.to_string(),
                &command_template.to_string(),
                &fallback_extra_arguments,
                &fallback_command_template,
            );
        let result = self
            .selected_row()
            .cloned()
            .context("Choose a launch profile row first.")
            .and_then(|row| {
                preview_for_row(
                    &row,
                    &effective_extra_arguments,
                    &effective_command_template,
                )
            });
        self.as_mut().publish_launch_profile_preview(result);
    }

    pub fn launch_profile_preview_argument_at(&self, index: i32) -> QString {
        qstring(
            usize::try_from(index)
                .ok()
                .and_then(|index| self.rust().launch_profile_preview_arguments.get(index))
                .map(String::as_str)
                .unwrap_or(""),
        )
    }

    fn publish_launch_profile_preview(
        mut self: Pin<&mut Self>,
        result: Result<LaunchCommandPreview>,
    ) {
        match result {
            Ok(preview) => {
                let argument_count = preview.arguments.len();
                let message = preview.summary();
                self.as_mut().set_launch_profile_preview_valid(true);
                self.as_mut()
                    .set_launch_profile_preview_runtime(qstring(preview.runtime));
                self.as_mut()
                    .set_launch_profile_preview_message(qstring(message));
                self.as_mut().rust_mut().launch_profile_preview_arguments = preview.arguments;
                self.as_mut()
                    .set_launch_profile_preview_argument_count(saturating_i32(argument_count));
            }
            Err(error) => {
                self.as_mut().set_launch_profile_preview_valid(false);
                self.as_mut()
                    .set_launch_profile_preview_runtime(QString::default());
                self.as_mut()
                    .set_launch_profile_preview_message(qstring(format!(
                        "Check this command: {error}"
                    )));
                self.as_mut()
                    .rust_mut()
                    .launch_profile_preview_arguments
                    .clear();
                self.as_mut().set_launch_profile_preview_argument_count(0);
            }
        }
        let revision = self
            .as_ref()
            .launch_profile_preview_revision()
            .wrapping_add(1);
        self.as_mut().set_launch_profile_preview_revision(revision);
    }

    fn clear_launch_profile_preview(mut self: Pin<&mut Self>) {
        self.as_mut().set_launch_profile_preview_valid(false);
        self.as_mut()
            .set_launch_profile_preview_runtime(QString::default());
        self.as_mut()
            .set_launch_profile_preview_message(QString::default());
        self.as_mut()
            .rust_mut()
            .launch_profile_preview_arguments
            .clear();
        self.as_mut().set_launch_profile_preview_argument_count(0);
        let revision = self
            .as_ref()
            .launch_profile_preview_revision()
            .wrapping_add(1);
        self.as_mut().set_launch_profile_preview_revision(revision);
    }

    fn update_profile_row(
        mut self: Pin<&mut Self>,
        key: &LaunchProfileKey,
        profile: Option<EmulatorLaunchProfile>,
    ) {
        if let Some(row) = self
            .as_mut()
            .rust_mut()
            .all_rows
            .iter_mut()
            .find(|row| row.key == *key)
        {
            row.profile = profile;
        }
        self.as_mut().refilter();
        let total = self.as_ref().rust().all_rows.len();
        let customized = *self.as_ref().customized_count();
        self.as_mut().set_message(qstring(format!(
            "{total} exact global and platform runtime profiles · {customized} customized"
        )));
        if let Some(index) = self
            .as_ref()
            .rust()
            .rows
            .iter()
            .position(|row| row.key == *key)
        {
            self.as_mut().edit_at(saturating_i32(index));
        } else {
            self.as_mut().close_editor();
        }
    }

    fn bump_editor_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().editor_revision().saturating_add(1);
        self.as_mut().set_editor_revision(revision);
    }

    fn row(&self, index: i32) -> Option<&LaunchProfileRow> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().rows.get(index))
    }

    fn selected_row(&self) -> Option<&LaunchProfileRow> {
        let key = self.rust().selected_key.as_ref()?;
        self.rust().all_rows.iter().find(|row| row.key == *key)
    }
}

fn load_launch_profile_rows() -> Result<Vec<LaunchProfileRow>> {
    let database = crate::catalog::requested_database_path()
        .context("No canonical database found. Pass --database PATH or set LUNCHBOX_DATABASE.")?;
    let connection = crate::catalog::open_read_only(&database, "Lunchbox emulator catalog")?;
    let profiles = SettingsStore::open_default()?
        .emulator_launch_profiles()?
        .into_iter()
        .filter(|profile| matches!(profile.scope_kind.as_str(), "global" | "platform"))
        .map(|profile| (LaunchProfileKey::from(&profile), profile))
        .collect::<HashMap<_, _>>();

    let mut rows = Vec::new();
    {
        let mut statement = connection
            .prepare("SELECT id, name FROM emulators ORDER BY name COLLATE NOCASE, id")?;
        for result in statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (emulator_id, emulator_name) = result?;
            rows.push(make_row(
                LaunchProfileKey {
                    scope_kind: "global".to_owned(),
                    scope_key: String::new(),
                    emulator_id,
                    runtime_kind: EmulatorRuntimeKind::Standalone.key().to_owned(),
                    core_name: String::new(),
                },
                "All platforms",
                "",
                &emulator_name,
                EmulatorRuntimeKind::Standalone,
                &profiles,
            ));
        }
    }
    {
        let mut statement = connection.prepare(
            "SELECT DISTINCT e.id, e.name, ep.core_name
             FROM emulator_platforms ep
             JOIN emulators e ON e.id=ep.emulator_id
             WHERE ep.core_name<>''
             ORDER BY e.name COLLATE NOCASE, e.id, ep.core_name",
        )?;
        for result in statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })? {
            let (emulator_id, emulator_name, core_name) = result?;
            rows.push(make_row(
                LaunchProfileKey {
                    scope_kind: "global".to_owned(),
                    scope_key: String::new(),
                    emulator_id,
                    runtime_kind: EmulatorRuntimeKind::RetroArch.key().to_owned(),
                    core_name,
                },
                "All platforms",
                "",
                &emulator_name,
                EmulatorRuntimeKind::RetroArch,
                &profiles,
            ));
        }
    }
    {
        let mut statement = connection.prepare(
            "SELECT DISTINCT p.canonical_name, p.normalized_name, e.id, e.name
             FROM emulator_platforms ep
             JOIN emulators e ON e.id=ep.emulator_id
             JOIN platforms p ON p.id=ep.platform_id
             ORDER BY p.canonical_name COLLATE NOCASE, e.name COLLATE NOCASE, e.id",
        )?;
        for result in statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })? {
            let (platform_name, platform_key, emulator_id, emulator_name) = result?;
            rows.push(make_row(
                LaunchProfileKey {
                    scope_kind: "platform".to_owned(),
                    scope_key: platform_key,
                    emulator_id,
                    runtime_kind: EmulatorRuntimeKind::Standalone.key().to_owned(),
                    core_name: String::new(),
                },
                &platform_name,
                &platform_name,
                &emulator_name,
                EmulatorRuntimeKind::Standalone,
                &profiles,
            ));
        }
    }
    {
        let mut statement = connection.prepare(
            "SELECT p.canonical_name, p.normalized_name, e.id, e.name, ep.core_name
             FROM emulator_platforms ep
             JOIN emulators e ON e.id=ep.emulator_id
             JOIN platforms p ON p.id=ep.platform_id
             WHERE ep.core_name<>''
             ORDER BY p.canonical_name COLLATE NOCASE, e.name COLLATE NOCASE,
                      e.id, ep.core_name",
        )?;
        for result in statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })? {
            let (platform_name, platform_key, emulator_id, emulator_name, core_name) = result?;
            rows.push(make_row(
                LaunchProfileKey {
                    scope_kind: "platform".to_owned(),
                    scope_key: platform_key,
                    emulator_id,
                    runtime_kind: EmulatorRuntimeKind::RetroArch.key().to_owned(),
                    core_name,
                },
                &platform_name,
                &platform_name,
                &emulator_name,
                EmulatorRuntimeKind::RetroArch,
                &profiles,
            ));
        }
    }
    rows.sort_by(|left, right| {
        left.key
            .scope_kind
            .cmp(&right.key.scope_kind)
            .then_with(|| {
                left.platform_name
                    .to_ascii_lowercase()
                    .cmp(&right.platform_name.to_ascii_lowercase())
            })
            .then_with(|| {
                left.emulator_name
                    .to_ascii_lowercase()
                    .cmp(&right.emulator_name.to_ascii_lowercase())
            })
            .then_with(|| left.key.runtime_kind.cmp(&right.key.runtime_kind))
            .then_with(|| left.key.core_name.cmp(&right.key.core_name))
    });
    Ok(rows)
}

fn make_row(
    key: LaunchProfileKey,
    scope_label: &str,
    platform_name: &str,
    emulator_name: &str,
    runtime_kind: EmulatorRuntimeKind,
    profiles: &HashMap<LaunchProfileKey, EmulatorLaunchProfile>,
) -> LaunchProfileRow {
    let preview_extra_insert_index =
        default_rom_extra_argument_insert_index(emulator_name, runtime_kind, platform_name);
    LaunchProfileRow {
        profile: profiles.get(&key).cloned(),
        default_template: default_rom_launch_template_for(
            emulator_name,
            runtime_kind,
            platform_name,
        ),
        preview_extra_insert_index,
        key,
        scope_label: scope_label.to_owned(),
        platform_name: platform_name.to_owned(),
        emulator_name: emulator_name.to_owned(),
    }
}

fn preview_for_row(
    row: &LaunchProfileRow,
    extra_arguments: &str,
    command_template: &str,
) -> Result<LaunchCommandPreview> {
    validate_template_for_row(row, command_template)?;
    let available_placeholders = launch_template_placeholders(&row.default_template)?;
    preview_launch_command(
        &row.emulator_name,
        &row.default_template,
        extra_arguments,
        command_template,
        &available_placeholders,
        row.preview_extra_insert_index,
    )
}

fn preview_fallback_for_row(
    row: &LaunchProfileRow,
    rows: &[LaunchProfileRow],
) -> ResolvedLaunchCustomization {
    if row.key.scope_kind != "platform" {
        return ResolvedLaunchCustomization::default();
    }
    let global = rows.iter().find(|candidate| {
        candidate.key.scope_kind == "global"
            && candidate.key.emulator_id == row.key.emulator_id
            && candidate.key.runtime_kind == row.key.runtime_kind
            && candidate.key.core_name == row.key.core_name
    });
    let Some(profile) = global.and_then(|candidate| candidate.profile.as_ref()) else {
        return ResolvedLaunchCustomization::default();
    };
    ResolvedLaunchCustomization {
        extra_arguments: profile.extra_arguments.clone(),
        argument_scope: (!profile.extra_arguments.is_empty())
            .then_some("global".to_owned())
            .unwrap_or_default(),
        command_template: profile.command_template.clone(),
        template_scope: (!profile.command_template.is_empty())
            .then_some("global".to_owned())
            .unwrap_or_default(),
    }
}

fn filter_rows(
    rows: &[LaunchProfileRow],
    search: &str,
    scope_filter: &str,
    customization_filter: &str,
) -> Vec<LaunchProfileRow> {
    rows.iter()
        .filter(|row| {
            (search.is_empty() || row.search_text().contains(search))
                && match scope_filter {
                    "global" | "platform" => row.key.scope_kind == scope_filter,
                    _ => true,
                }
                && match customization_filter {
                    "customized" => row.customized(),
                    "built-in" => !row.customized(),
                    _ => true,
                }
        })
        .cloned()
        .collect()
}

fn resolved_for_row(row: &LaunchProfileRow) -> Result<ResolvedLaunchCustomization> {
    SettingsStore::open_default()?.resolve_launch_customization(
        "",
        &row.platform_name,
        &row.key.emulator_id,
        &row.key.runtime_kind,
        &row.key.core_name,
    )
}

fn effective_summary(resolved: &ResolvedLaunchCustomization) -> String {
    format!(
        "Effective arguments: {} · command template: {}",
        if resolved.extra_arguments.is_empty() {
            "built-in"
        } else {
            resolved.argument_scope.as_str()
        },
        if resolved.command_template.is_empty() {
            "built-in"
        } else {
            resolved.template_scope.as_str()
        }
    )
}

fn validate_template_for_row(row: &LaunchProfileRow, command_template: &str) -> Result<()> {
    let requested = launch_template_placeholders(command_template)?;
    if requested.is_empty() {
        return Ok(());
    }
    let available = launch_template_placeholders(&row.default_template)?;
    let unavailable = requested
        .into_iter()
        .filter(|placeholder| !available.contains(placeholder))
        .collect::<Vec<_>>();
    if !unavailable.is_empty() {
        bail!(
            "placeholder{} {} {} not available for this exact runtime; use {}",
            if unavailable.len() == 1 { "" } else { "s" },
            unavailable
                .iter()
                .map(|placeholder| format!("%{{{placeholder}}}"))
                .collect::<Vec<_>>()
                .join(", "),
            if unavailable.len() == 1 { "is" } else { "are" },
            available
                .iter()
                .map(|placeholder| {
                    if placeholder == "file" {
                        "%f".to_owned()
                    } else {
                        format!("%{{{placeholder}}}")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(scope: &str, platform: &str, customized: bool) -> LaunchProfileRow {
        let key = LaunchProfileKey {
            scope_kind: scope.into(),
            scope_key: platform.to_ascii_lowercase(),
            emulator_id: "emulator-id".into(),
            runtime_kind: "standalone".into(),
            core_name: String::new(),
        };
        LaunchProfileRow {
            key: key.clone(),
            scope_label: if scope == "global" {
                "All platforms".into()
            } else {
                platform.into()
            },
            platform_name: platform.into(),
            emulator_name: "MAME".into(),
            default_template: if platform == "Arcade" {
                "-rompath %{mame_rompath} %{mame_romset}".into()
            } else {
                "%f".into()
            },
            preview_extra_insert_index: 0,
            profile: customized.then(|| EmulatorLaunchProfile {
                scope_kind: key.scope_kind,
                scope_key: key.scope_key,
                emulator_id: key.emulator_id,
                runtime_kind: key.runtime_kind,
                core_name: key.core_name,
                extra_arguments: "-video bgfx".into(),
                command_template: String::new(),
                updated_at: 1,
            }),
        }
    }

    #[test]
    fn bulk_profile_filter_combines_search_scope_and_customization() {
        let rows = vec![row("global", "", false), row("platform", "Arcade", true)];
        assert_eq!(filter_rows(&rows, "mame", "all", "all").len(), 2);
        assert_eq!(
            filter_rows(&rows, "arcade", "platform", "customized").len(),
            1
        );
        assert!(filter_rows(&rows, "arcade", "global", "all").is_empty());
    }

    #[test]
    fn bulk_profile_template_rejects_contextually_unavailable_placeholders() {
        let arcade = row("platform", "Arcade", false);
        validate_template_for_row(&arcade, "-rompath %{mame_rompath} %{mame_romset}").unwrap();
        assert!(validate_template_for_row(&arcade, "%{core} %f").is_err());

        let global = row("global", "", false);
        validate_template_for_row(&global, "%f").unwrap();
        assert!(validate_template_for_row(&global, "%{mame_romset}").is_err());
    }

    #[test]
    fn platform_preview_applies_independent_global_field_inheritance() {
        let global = row("global", "", true);
        let platform = row("platform", "Arcade", false);
        let fallback = preview_fallback_for_row(&platform, &[global.clone(), platform.clone()]);
        assert_eq!(fallback.extra_arguments, "-video bgfx");
        assert!(fallback.command_template.is_empty());

        let (extra_arguments, command_template) = effective_launch_preview_values(
            "",
            "",
            &fallback.extra_arguments,
            &fallback.command_template,
        );
        let preview = preview_for_row(&platform, &extra_arguments, &command_template).unwrap();
        assert_eq!(
            preview.arguments,
            [
                "-video",
                "bgfx",
                "-rompath",
                "<rom-directory>",
                "<machine-set>",
            ]
        );

        let (extra_arguments, command_template) = effective_launch_preview_values(
            "--window",
            "%{mame_romset}",
            &fallback.extra_arguments,
            &fallback.command_template,
        );
        let preview = preview_for_row(&platform, &extra_arguments, &command_template).unwrap();
        assert_eq!(preview.arguments, ["<machine-set>"]);
        assert!(preview.extra_arguments_ignored);
    }
}
