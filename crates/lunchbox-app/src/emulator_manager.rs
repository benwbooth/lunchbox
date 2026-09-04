use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::catalog;
use crate::platform_process::{host_command, host_program_available, is_flatpak};
use crate::settings::{
    EmulatorLifecycleOperation, EmulatorUpdatePin, ManagedEmulatorInstall, SettingsStore,
};

const MAX_MANAGED_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;
const GITHUB_API: &str = "https://api.github.com/repos";
const COMPLETED_ACTIVITY_TTL_SECONDS: i64 = 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedAction {
    Install,
    Update,
    Uninstall,
}

impl ManagedAction {
    fn slug(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Update => "update",
            Self::Uninstall => "uninstall",
        }
    }
}

struct EmulatorOperationLease {
    file: File,
}

impl Drop for EmulatorOperationLease {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedEmulator {
    pub emulator_id: String,
    pub name: String,
    pub host_system_slug: String,
    pub manager: String,
    pub package_id: String,
    pub metadata_json: String,
    pub source_label: String,
    pub installed: bool,
    pub managed: bool,
    pub manager_available: bool,
    pub version: String,
    pub install_path: String,
    pub compatible_platform_keys: BTreeSet<String>,
    pub recommended_platform_keys: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedEmulatorUpdate {
    pub row: ManagedEmulator,
    pub display_name: String,
    pub source_label: String,
    pub current_version: String,
    pub available_version: String,
    pub pin: Option<EmulatorUpdatePin>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EmulatorUpdateInventory {
    pub updates: Vec<ManagedEmulatorUpdate>,
    pub warnings: Vec<String>,
    pub recovered_operations: usize,
}

impl ManagedEmulator {
    pub fn status_label(&self) -> &'static str {
        if self.installed && self.managed {
            "MANAGED"
        } else if self.installed {
            "INSTALLED"
        } else if self.manager_available {
            "AVAILABLE"
        } else {
            "UNAVAILABLE"
        }
    }

    pub fn detail(&self) -> String {
        let source = format!("{} · {}", self.source_label, self.package_id);
        if self.installed {
            let ownership = if self.managed {
                "installed by Lunchbox"
            } else {
                "detected outside Lunchbox"
            };
            if self.version.is_empty() {
                format!("{source} · {ownership}")
            } else {
                format!("{source} · {} · {ownership}", self.version)
            }
        } else if self.manager_available {
            format!("{source} · ready to install")
        } else {
            format!("{source} · {} is not available", self.source_label)
        }
    }

    pub fn can_install(&self) -> bool {
        !self.installed && self.manager_available
    }

    pub fn can_update(&self) -> bool {
        self.installed && self.manager_available
    }

    pub fn can_uninstall(&self) -> bool {
        self.installed
            && self.manager_available
            && (self.managed || self.can_uninstall_external_package())
    }

    fn can_uninstall_external_package(&self) -> bool {
        matches!(self.manager.as_str(), "flatpak" | "winget" | "homebrew")
    }

    pub fn uninstall_confirmation(&self) -> String {
        if self.managed {
            return format!(
                "Lunchbox will remove only its managed {} installation. Your games, saves, and firmware are not touched.",
                self.package_id
            );
        }

        let scope = if self.manager == "flatpak" {
            match self.install_path.as_str() {
                "system" => "system ",
                "user" => "user ",
                _ => "",
            }
        } else {
            ""
        };
        format!(
            "Lunchbox detected this installation outside Lunchbox. {} will be asked to uninstall the exact {scope}package {}. This can affect other games or applications that use the same emulator. Your ROMs, saves, and firmware are not removed.",
            self.source_label, self.package_id
        )
    }

    pub fn is_compatible_with(&self, platform_key: &str) -> bool {
        platform_key.is_empty() || self.compatible_platform_keys.contains(platform_key)
    }

    pub fn is_recommended_for(&self, platform_key: &str) -> bool {
        !platform_key.is_empty() && self.recommended_platform_keys.contains(platform_key)
    }
}

#[derive(Clone, Debug, Default)]
struct EmulatorCompatibility {
    platform_keys: BTreeSet<String>,
    recommended_platform_keys: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct InstallSource {
    emulator_id: String,
    name: String,
    host_system_slug: String,
    manager: String,
    package_id: String,
    metadata_json: String,
}

#[derive(Clone, Debug, Default)]
struct InstallationState {
    installed: bool,
    version: String,
    install_path: String,
}

type InstalledSourceGroup = (InstallSource, InstallationState, bool, BTreeSet<String>);
type InstalledSourceGroups = BTreeMap<(String, String, String), InstalledSourceGroup>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NixProfileList {
    elements: BTreeMap<String, NixProfileElement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NixProfileElement {
    active: bool,
    #[serde(default)]
    attr_path: Option<String>,
    #[serde(default)]
    store_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct SourceMetadata {
    #[serde(default)]
    archive_format: String,
    #[serde(default)]
    archive_payload: String,
    #[serde(default)]
    asset_terms: Vec<String>,
    #[serde(default)]
    executable_candidates: Vec<String>,
    #[serde(default)]
    asset_terms_by_arch: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    supported_arches: Vec<String>,
    #[serde(default)]
    release_api: String,
    #[serde(default)]
    runtime: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManagedProgramManifest {
    emulator_id: String,
    manager: String,
    package_id: String,
    version: String,
    asset_name: String,
    download_url: String,
    executable_path: String,
    update_transport: String,
}

pub fn load_managed_emulators() -> Result<Vec<ManagedEmulator>> {
    let host = current_host_slug()?;
    let database = catalog::requested_database_path()
        .context("No canonical database found. Pass --database PATH or set LUNCHBOX_DATABASE.")?;
    let connection = catalog::open_read_only(&database, "Lunchbox emulator catalog")?;
    let (emulator_compatibility, core_compatibility) = load_emulator_compatibility(&connection)?;
    let mut statement = connection.prepare(
        "SELECT e.id, e.name, p.host_system_slug, p.manager, p.package_id, p.metadata_json
         FROM emulator_packages p
         JOIN emulators e ON e.id=p.emulator_id
         WHERE p.host_system_slug=?1
         ORDER BY e.name COLLATE NOCASE, p.manager, p.package_id",
    )?;
    let sources = statement
        .query_map([host], |row| {
            Ok(InstallSource {
                emulator_id: row.get(0)?,
                name: row.get(1)?,
                host_system_slug: row.get(2)?,
                manager: row.get(3)?,
                package_id: row.get(4)?,
                metadata_json: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let store = SettingsStore::open_default()?;
    let receipts = store
        .managed_emulator_installs()?
        .into_iter()
        .map(|receipt| (receipt_key(&receipt), receipt))
        .collect::<HashMap<_, _>>();
    let snapshot = HostSnapshot::load(host, &sources)?;
    let mut grouped = BTreeMap::<String, Vec<(InstallSource, InstallationState, bool)>>::new();
    for source in sources {
        validate_source(&source)?;
        let state = snapshot.installation_state(&source, receipts.get(&source_key(&source)))?;
        let managed = state.installed && receipts.contains_key(&source_key(&source));
        grouped
            .entry(source.emulator_id.clone())
            .or_default()
            .push((source, state, managed));
    }

    let mut rows = Vec::with_capacity(grouped.len());
    for candidates in grouped.into_values() {
        let (source, state, managed) = candidates
            .into_iter()
            .min_by_key(|(source, state, _)| {
                (
                    if state.installed { 0 } else { 1 },
                    if manager_available(&source.manager, &source.metadata_json) {
                        0
                    } else {
                        1
                    },
                    manager_priority(&source.manager),
                    source.package_id.clone(),
                )
            })
            .context("emulator install source group was unexpectedly empty")?;
        rows.push(ManagedEmulator {
            compatible_platform_keys: emulator_compatibility
                .get(&source.emulator_id)
                .map(|compatibility| compatibility.platform_keys.clone())
                .unwrap_or_default(),
            recommended_platform_keys: emulator_compatibility
                .get(&source.emulator_id)
                .map(|compatibility| compatibility.recommended_platform_keys.clone())
                .unwrap_or_default(),
            emulator_id: source.emulator_id,
            name: source.name,
            host_system_slug: source.host_system_slug,
            source_label: source_label(&source.manager, &source.metadata_json),
            manager_available: manager_available(&source.manager, &source.metadata_json),
            manager: source.manager,
            package_id: source.package_id,
            metadata_json: source.metadata_json,
            installed: state.installed,
            managed,
            version: state.version,
            install_path: state.install_path,
        });
    }
    rows.extend(load_libretro_core_rows(
        &connection,
        host,
        &receipts,
        &core_compatibility,
    )?);
    rows.sort_by(|left, right| {
        right.installed.cmp(&left.installed).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    Ok(rows)
}

fn load_emulator_compatibility(
    connection: &rusqlite::Connection,
) -> Result<(
    HashMap<String, EmulatorCompatibility>,
    HashMap<String, EmulatorCompatibility>,
)> {
    let mut statement = connection.prepare(
        "SELECT ep.emulator_id, p.normalized_name, ep.core_name, ep.recommended
         FROM emulator_platforms ep
         JOIN platforms p ON p.id=ep.platform_id
         ORDER BY ep.emulator_id, p.normalized_name, ep.core_name",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut emulators = HashMap::<String, EmulatorCompatibility>::new();
    let mut cores = HashMap::<String, EmulatorCompatibility>::new();
    for (emulator_id, platform_key, core_names, recommended) in rows {
        add_compatibility(
            emulators.entry(emulator_id).or_default(),
            &platform_key,
            recommended,
        );
        for core_name in core_names
            .split(';')
            .map(str::trim)
            .filter(|core_name| !core_name.is_empty())
        {
            add_compatibility(
                cores.entry(core_name.to_owned()).or_default(),
                &platform_key,
                recommended,
            );
        }
    }

    // Imported collections sometimes retain an established alias instead of
    // the canonical display name. Keep those aliases eligible for the same
    // platform-scoped manager without guessing from game titles.
    let mut alias_statement = connection.prepare(
        "SELECT ep.emulator_id, a.normalized_alias, ep.core_name, ep.recommended
         FROM emulator_platforms ep
         JOIN platform_aliases a ON a.platform_id=ep.platform_id
         ORDER BY ep.emulator_id, a.normalized_alias, ep.core_name",
    )?;
    let alias_rows = alias_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (emulator_id, platform_key, core_names, recommended) in alias_rows {
        add_compatibility(
            emulators.entry(emulator_id).or_default(),
            &platform_key,
            recommended,
        );
        for core_name in core_names
            .split(';')
            .map(str::trim)
            .filter(|core_name| !core_name.is_empty())
        {
            add_compatibility(
                cores.entry(core_name.to_owned()).or_default(),
                &platform_key,
                recommended,
            );
        }
    }
    Ok((emulators, cores))
}

fn add_compatibility(
    compatibility: &mut EmulatorCompatibility,
    platform_key: &str,
    recommended: bool,
) {
    if platform_key.is_empty() {
        return;
    }
    compatibility.platform_keys.insert(platform_key.to_owned());
    if recommended {
        compatibility
            .recommended_platform_keys
            .insert(platform_key.to_owned());
    }
}

pub fn load_available_emulator_updates() -> Result<EmulatorUpdateInventory> {
    let recovered_operations = recover_interrupted_emulator_operations()?.len();
    let host = current_host_slug()?;
    let database = catalog::requested_database_path()
        .context("No canonical database found. Pass --database PATH or set LUNCHBOX_DATABASE.")?;
    let connection = catalog::open_read_only(&database, "Lunchbox emulator catalog")?;
    let mut statement = connection.prepare(
        "SELECT e.id, e.name, p.host_system_slug, p.manager, p.package_id, p.metadata_json
         FROM emulator_packages p
         JOIN emulators e ON e.id=p.emulator_id
         WHERE p.host_system_slug=?1
         ORDER BY p.manager, p.package_id, e.name COLLATE NOCASE, e.id",
    )?;
    let sources = statement
        .query_map([host], |row| {
            Ok(InstallSource {
                emulator_id: row.get(0)?,
                name: row.get(1)?,
                host_system_slug: row.get(2)?,
                manager: row.get(3)?,
                package_id: row.get(4)?,
                metadata_json: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let store = SettingsStore::open_default()?;
    let receipts = store
        .managed_emulator_installs()?
        .into_iter()
        .map(|receipt| (receipt_key(&receipt), receipt))
        .collect::<HashMap<_, _>>();
    let update_pins = store
        .emulator_update_pins()?
        .into_iter()
        .map(|pin| (update_pin_key_for_pin(&pin), pin))
        .collect::<HashMap<_, _>>();
    let snapshot = HostSnapshot::load(host, &sources)?;
    let mut installed = InstalledSourceGroups::new();
    for source in sources {
        validate_source(&source)?;
        for state in
            snapshot.installation_states_for_updates(&source, receipts.get(&source_key(&source)))?
        {
            let managed = receipts.contains_key(&source_key(&source));
            let group_key = (
                source.manager.clone(),
                source.package_id.clone(),
                state.install_path.clone(),
            );
            installed
                .entry(group_key)
                .and_modify(|(_, _, grouped_managed, names)| {
                    *grouped_managed |= managed;
                    names.insert(source.name.clone());
                })
                .or_insert_with(|| {
                    let mut names = BTreeSet::new();
                    names.insert(source.name.clone());
                    (source.clone(), state, managed, names)
                });
        }
    }

    let mut checks = UpdateChecks::load(host, &installed);
    let mut updates = Vec::new();
    for (_, (source, state, managed, names)) in installed {
        let row = ManagedEmulator {
            compatible_platform_keys: BTreeSet::new(),
            recommended_platform_keys: BTreeSet::new(),
            emulator_id: source.emulator_id,
            name: source.name,
            host_system_slug: source.host_system_slug,
            source_label: source_label(&source.manager, &source.metadata_json),
            manager_available: manager_available(&source.manager, &source.metadata_json),
            manager: source.manager,
            package_id: source.package_id,
            metadata_json: source.metadata_json,
            installed: true,
            managed,
            version: state.version.clone(),
            install_path: state.install_path,
        };
        match checks.check(&row) {
            UpdateCheckResult::Available(available_version) => {
                let pin = update_pins.get(&update_pin_key(&row)).cloned();
                updates.push(ManagedEmulatorUpdate {
                    display_name: summarize_emulator_names(&names),
                    source_label: update_source_label(&row),
                    current_version: state.version,
                    available_version,
                    row,
                    pin,
                });
            }
            UpdateCheckResult::Failed(error) => checks.warnings.push(format!(
                "{} via {}: {error}",
                summarize_emulator_names(&names),
                manager_label(&row.manager)
            )),
            UpdateCheckResult::Current | UpdateCheckResult::Unsupported => {}
        }
    }
    updates.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
            .then_with(|| left.source_label.cmp(&right.source_label))
            .then_with(|| left.row.package_id.cmp(&right.row.package_id))
    });
    checks.warnings.sort();
    checks.warnings.dedup();
    Ok(EmulatorUpdateInventory {
        updates,
        warnings: checks.warnings,
        recovered_operations,
    })
}

fn update_pin_key(row: &ManagedEmulator) -> (String, String, String, String) {
    (
        row.host_system_slug.clone(),
        row.manager.clone(),
        row.package_id.clone(),
        row.install_path.clone(),
    )
}

fn update_pin_key_for_pin(pin: &EmulatorUpdatePin) -> (String, String, String, String) {
    (
        pin.host_system_slug.clone(),
        pin.manager.clone(),
        pin.package_id.clone(),
        pin.install_path.clone(),
    )
}

fn load_libretro_core_rows(
    connection: &rusqlite::Connection,
    host: &str,
    receipts: &HashMap<(String, String, String), ManagedEmulatorInstall>,
    core_compatibility: &HashMap<String, EmulatorCompatibility>,
) -> Result<Vec<ManagedEmulator>> {
    let mut statement = connection.prepare(
        "SELECT ep.core_name, e.name
         FROM emulator_platforms ep
         JOIN emulators e ON e.id=ep.emulator_id
         WHERE ep.core_name<>''
         ORDER BY ep.core_name, e.name COLLATE NOCASE",
    )?;
    let pairs = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut cores = BTreeMap::<String, BTreeSet<String>>::new();
    for (core_names, emulator_name) in pairs {
        for core_name in core_names
            .split(';')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            validate_package_id("libretro", core_name)?;
            cores
                .entry(core_name.to_owned())
                .or_default()
                .insert(emulator_name.clone());
        }
    }

    let installed_paths = installed_libretro_cores();
    let available = manager_available("libretro", "{}");
    Ok(cores
        .into_iter()
        .map(|(core_name, names)| {
            let key = (host.to_owned(), "libretro".to_owned(), core_name.clone());
            let receipt = receipts.get(&key);
            let detected_path = installed_paths.get(&core_name).cloned().or_else(|| {
                receipt
                    .filter(|receipt| Path::new(&receipt.install_path).is_file())
                    .map(|receipt| receipt.install_path.clone())
            });
            let installed = detected_path.is_some();
            let managed = installed && receipt.is_some();
            let display_name = names
                .iter()
                .next()
                .filter(|_| names.len() == 1)
                .cloned()
                .unwrap_or_else(|| core_name.replace('_', " "));
            ManagedEmulator {
                compatible_platform_keys: core_compatibility
                    .get(&core_name)
                    .map(|compatibility| compatibility.platform_keys.clone())
                    .unwrap_or_default(),
                recommended_platform_keys: core_compatibility
                    .get(&core_name)
                    .map(|compatibility| compatibility.recommended_platform_keys.clone())
                    .unwrap_or_default(),
                emulator_id: format!("libretro-core-{core_name}"),
                name: format!("RetroArch: {display_name}"),
                host_system_slug: host.to_owned(),
                manager: "libretro".to_owned(),
                package_id: core_name,
                metadata_json: "{}".to_owned(),
                source_label: manager_label("libretro").to_owned(),
                installed,
                managed,
                manager_available: available,
                version: String::new(),
                install_path: detected_path.unwrap_or_default(),
            }
        })
        .collect())
}

fn emulator_operation_lock_path(store: &SettingsStore) -> PathBuf {
    let parent = store.path().parent().unwrap_or_else(|| Path::new("."));
    parent.join("emulator-lifecycle.lock")
}

fn try_acquire_emulator_operation_lease(
    store: &SettingsStore,
) -> Result<Option<EmulatorOperationLease>> {
    let path = emulator_operation_lock_path(store);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "creating emulator lifecycle lock directory {}",
                parent.display()
            )
        })?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("opening emulator lifecycle lock {}", path.display()))?;
    match FileExt::try_lock_exclusive(&file) {
        Ok(()) => Ok(Some(EmulatorOperationLease { file })),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error).context("locking the emulator lifecycle service"),
    }
}

pub fn recover_interrupted_emulator_operations() -> Result<Vec<EmulatorLifecycleOperation>> {
    let store = SettingsStore::open_default()?;
    recover_interrupted_emulator_operations_at(&store)
}

fn recover_interrupted_emulator_operations_at(
    store: &SettingsStore,
) -> Result<Vec<EmulatorLifecycleOperation>> {
    let Some(lease) = try_acquire_emulator_operation_lease(store)? else {
        return Ok(Vec::new());
    };
    let mut recovered = Vec::new();
    for operation in store.running_emulator_lifecycle_operations()? {
        let detail = "Lunchbox closed before the operation reported a result. The exact installation state will be rescanned before another operation is offered; review it before retrying.";
        let interrupted =
            store.finish_emulator_lifecycle_operation(&operation.id, "interrupted", detail)?;
        println!(
            "LUNCHBOX_EMULATOR_OPERATION_RECOVERED action={} manager={} package={}",
            interrupted.action, interrupted.manager, interrupted.package_id
        );
        recovered.push(interrupted);
    }
    drop(lease);
    Ok(recovered)
}

pub fn recent_emulator_lifecycle_operation() -> Result<Option<EmulatorLifecycleOperation>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    Ok(SettingsStore::open_default()?
        .recent_emulator_lifecycle_operations(20)?
        .into_iter()
        .find(|operation| lifecycle_operation_visible_at(operation, now)))
}

fn lifecycle_operation_visible_at(operation: &EmulatorLifecycleOperation, now: i64) -> bool {
    if !matches!(operation.status.as_str(), "succeeded" | "cancelled") {
        return true;
    }
    let finished_at = operation.finished_at.unwrap_or(operation.started_at);
    now.saturating_sub(finished_at) <= COMPLETED_ACTIVITY_TTL_SECONDS
}

pub fn perform_action(row: &ManagedEmulator, action: ManagedAction) -> Result<String> {
    validate_row_for_action(row, action)?;
    let store = SettingsStore::open_default()?;
    let lease = try_acquire_emulator_operation_lease(&store)?.context(
        "another Lunchbox window is already installing, updating, uninstalling, or reconciling an emulator; wait for it to finish and refresh",
    )?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    store.begin_emulator_lifecycle_operation(
        &operation_id,
        &row.emulator_id,
        &row.name,
        &row.host_system_slug,
        &row.manager,
        &row.package_id,
        &row.install_path,
        action.slug(),
    )?;
    let result = perform_action_inner(row, action, &store);
    let (status, detail) = match &result {
        Ok(message) => ("succeeded", bounded_operation_detail(message)),
        Err(error) => ("failed", bounded_operation_detail(&format!("{error:#}"))),
    };
    let journal_result = store.finish_emulator_lifecycle_operation(&operation_id, status, &detail);
    drop(lease);
    match (result, journal_result) {
        (Ok(message), Ok(_)) => Ok(message),
        (Ok(_), Err(error)) => Err(error).context(
            "the emulator operation succeeded, but Lunchbox could not record its final state; refresh before acting again",
        ),
        (Err(error), Ok(_)) => Err(error),
        (Err(error), Err(journal_error)) => Err(error).context(format!(
            "Lunchbox also could not record the failed emulator operation: {journal_error:#}"
        )),
    }
}

fn perform_action_inner(
    row: &ManagedEmulator,
    action: ManagedAction,
    store: &SettingsStore,
) -> Result<String> {
    match action {
        ManagedAction::Install => {
            let install_path = if row.manager == "libretro" {
                let runtime = ensure_retroarch_runtime(&store)?;
                install_libretro_core(row, runtime.manager == "flatpak")?
            } else {
                install(row)?
            };
            store.record_managed_emulator_install(
                &row.emulator_id,
                &row.host_system_slug,
                &row.manager,
                &row.package_id,
                &install_path,
            )?;
            Ok(format!("Installed {} with {}.", row.name, row.source_label))
        }
        ManagedAction::Update => {
            let install_path = if row.manager == "libretro" {
                let runtime = ensure_retroarch_runtime(&store)?;
                install_libretro_core(row, runtime.manager == "flatpak")?
            } else {
                update(row)?
            };
            if row.managed {
                store.record_managed_emulator_install(
                    &row.emulator_id,
                    &row.host_system_slug,
                    &row.manager,
                    &row.package_id,
                    &install_path,
                )?;
            }
            Ok(format!("Updated {} with {}.", row.name, row.source_label))
        }
        ManagedAction::Uninstall => {
            uninstall(row)?;
            if row.managed {
                store.remove_managed_emulator_install(
                    &row.host_system_slug,
                    &row.manager,
                    &row.package_id,
                )?;
            }
            Ok(format!(
                "Uninstalled {} with {}.",
                row.name, row.source_label
            ))
        }
    }
}

fn bounded_operation_detail(detail: &str) -> String {
    const MAX_DETAIL_CHARS: usize = 8192;
    if detail.chars().count() <= MAX_DETAIL_CHARS {
        return detail.to_owned();
    }
    detail.chars().take(MAX_DETAIL_CHARS).collect()
}

fn install(row: &ManagedEmulator) -> Result<String> {
    match row.manager.as_str() {
        "flatpak" => {
            run_checked(
                command_path("flatpak")?,
                [
                    "install",
                    "--user",
                    "--noninteractive",
                    "-y",
                    "flathub",
                    row.package_id.as_str(),
                ],
                "Flatpak install",
            )?;
            Ok("user".to_owned())
        }
        "nix" => install_nix(row),
        "appimage" | "github" => install_github_program(row),
        "direct" => install_direct_program(row),
        "winget" => {
            run_checked(
                command_path("winget")?,
                [
                    "install",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                    "--disable-interactivity",
                    "-e",
                    "--id",
                    row.package_id.as_str(),
                ],
                "winget install",
            )?;
            Ok(row.package_id.clone())
        }
        "homebrew" => {
            run_checked(
                command_path("brew")?,
                ["install", row.package_id.as_str()],
                "Homebrew install",
            )?;
            Ok(row.package_id.clone())
        }
        manager => bail!("unsupported emulator install manager {manager}"),
    }
}

fn update(row: &ManagedEmulator) -> Result<String> {
    match row.manager.as_str() {
        "flatpak" => {
            let scope = if row.install_path == "system" {
                "--system"
            } else {
                "--user"
            };
            run_checked(
                command_path("flatpak")?,
                [
                    "update",
                    scope,
                    "--noninteractive",
                    "-y",
                    row.package_id.as_str(),
                ],
                "Flatpak update",
            )?;
            Ok(scope.trim_start_matches("--").to_owned())
        }
        "nix" => update_nix(row),
        "appimage" => update_appimage(row),
        "github" => install_github_program(row),
        "direct" => install_direct_program(row),
        "winget" => {
            run_checked(
                command_path("winget")?,
                [
                    "upgrade",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                    "--disable-interactivity",
                    "-e",
                    "--id",
                    row.package_id.as_str(),
                ],
                "winget upgrade",
            )?;
            Ok(row.package_id.clone())
        }
        "homebrew" => {
            run_checked(
                command_path("brew")?,
                ["upgrade", row.package_id.as_str()],
                "Homebrew upgrade",
            )?;
            Ok(row.package_id.clone())
        }
        manager => bail!("unsupported emulator update manager {manager}"),
    }
}

fn uninstall(row: &ManagedEmulator) -> Result<()> {
    match row.manager.as_str() {
        "flatpak" => {
            let scope = if row.install_path == "system" {
                "--system"
            } else {
                "--user"
            };
            run_checked(
                command_path("flatpak")?,
                [
                    "uninstall",
                    scope,
                    "--noninteractive",
                    "-y",
                    row.package_id.as_str(),
                ],
                "Flatpak uninstall",
            )?;
        }
        "nix" => uninstall_nix(row)?,
        "appimage" | "github" | "direct" => {
            remove_owned_program(row)?;
        }
        "winget" => {
            run_checked(
                command_path("winget")?,
                [
                    "uninstall",
                    "--disable-interactivity",
                    "-e",
                    "--id",
                    row.package_id.as_str(),
                ],
                "winget uninstall",
            )?;
        }
        "homebrew" => {
            run_checked(
                command_path("brew")?,
                ["uninstall", row.package_id.as_str()],
                "Homebrew uninstall",
            )?;
        }
        "libretro" => remove_owned_libretro_core(row)?,
        manager => bail!("unsupported emulator uninstall manager {manager}"),
    }
    Ok(())
}

fn validate_row_for_action(row: &ManagedEmulator, action: ManagedAction) -> Result<()> {
    validate_package_id(&row.manager, &row.package_id)?;
    if row.host_system_slug != current_host_slug()? {
        bail!("emulator install source does not match this host");
    }
    match action {
        ManagedAction::Install if !row.can_install() => bail!("{} cannot be installed", row.name),
        ManagedAction::Update if !row.can_update() => bail!("{} cannot be updated", row.name),
        ManagedAction::Uninstall if !row.can_uninstall() => {
            bail!(
                "{} cannot be safely uninstalled from Lunchbox; remove it with {} instead",
                row.name,
                row.source_label
            )
        }
        _ => Ok(()),
    }
}

fn validate_source(source: &InstallSource) -> Result<()> {
    if source.emulator_id.trim().is_empty() || source.name.trim().is_empty() {
        bail!("emulator install source is missing exact identity");
    }
    validate_package_id(&source.manager, &source.package_id)?;
    let metadata: serde_json::Value = serde_json::from_str(&source.metadata_json)
        .context("parsing emulator install source metadata")?;
    if !metadata.is_object() {
        bail!("emulator install source metadata must be a JSON object");
    }
    let metadata: SourceMetadata = serde_json::from_value(metadata)
        .context("parsing emulator install source metadata fields")?;
    if !metadata.release_api.is_empty() && !metadata.release_api.starts_with("https://") {
        bail!("managed release APIs must use HTTPS");
    }
    if metadata
        .supported_arches
        .iter()
        .chain(metadata.asset_terms_by_arch.keys())
        .any(|architecture| !matches!(architecture.as_str(), "x86_64" | "aarch64"))
    {
        bail!("managed release metadata contains an unsupported architecture");
    }
    Ok(())
}

fn validate_package_id(manager: &str, package_id: &str) -> Result<()> {
    let value = package_id.trim();
    if value.is_empty()
        || value.starts_with('-')
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        bail!("invalid {manager} package identity {package_id:?}");
    }
    if !matches!(
        manager,
        "flatpak" | "appimage" | "nix" | "github" | "direct" | "winget" | "homebrew" | "libretro"
    ) {
        bail!("unsupported emulator package manager {manager}");
    }
    if manager == "flatpak" && !value.contains('.') {
        bail!("Flatpak identity is not a reverse-DNS application ID");
    }
    if matches!(manager, "flatpak" | "nix" | "winget")
        && !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".+_-".contains(character))
    {
        bail!("invalid characters in {manager} package identity");
    }
    if manager == "homebrew"
        && !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "@/+._-".contains(character))
    {
        bail!("invalid characters in Homebrew package identity");
    }
    if matches!(manager, "appimage" | "github")
        && (value.split('/').count() != 2
            || value.contains("..")
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "/._-".contains(character)))
    {
        bail!("GitHub release identity must be an exact owner/repository pair");
    }
    if manager == "direct" && !value.starts_with("https://") {
        bail!("direct emulator downloads must use HTTPS");
    }
    if manager == "libretro"
        && !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
    {
        bail!("invalid Libretro core identity");
    }
    Ok(())
}

fn current_host_slug() -> Result<&'static str> {
    if cfg!(target_os = "linux") {
        Ok("linux")
    } else if cfg!(target_os = "windows") {
        Ok("windows")
    } else if cfg!(target_os = "macos") {
        Ok("macos")
    } else {
        bail!("emulator management does not support this host")
    }
}

fn manager_priority(manager: &str) -> u8 {
    match manager {
        "flatpak" => 0,
        "appimage" => 1,
        "nix" => 2,
        "github" => 3,
        "direct" => 4,
        "winget" | "homebrew" => 0,
        "libretro" => 0,
        _ => u8::MAX,
    }
}

fn manager_label(manager: &str) -> &'static str {
    match manager {
        "flatpak" => "Flatpak",
        "appimage" => "AppImage",
        "nix" => "Nixpkgs profile",
        "github" => "GitHub release",
        "direct" => "Official download",
        "winget" => "winget",
        "homebrew" => "Homebrew",
        "libretro" => "Libretro buildbot",
        _ => "Package manager",
    }
}

fn source_label(manager: &str, metadata_json: &str) -> String {
    let metadata: SourceMetadata = serde_json::from_str(metadata_json).unwrap_or_default();
    if !metadata.release_api.is_empty() {
        return if manager == "appimage" {
            "Official AppImage".to_owned()
        } else {
            "Official release".to_owned()
        };
    }
    manager_label(manager).to_owned()
}

fn source_supports_current_architecture(metadata: &SourceMetadata) -> bool {
    metadata.supported_arches.is_empty()
        || metadata
            .supported_arches
            .iter()
            .any(|architecture| architecture == env::consts::ARCH)
}

fn summarize_emulator_names(names: &BTreeSet<String>) -> String {
    let Some(first) = names.iter().next() else {
        return "Unknown emulator".to_owned();
    };
    if names.len() == 1 {
        return first.clone();
    }
    let primary = names
        .iter()
        .find(|name| !name.contains(" ("))
        .unwrap_or(first);
    format!("{primary} (+{} variants)", names.len() - 1)
}

fn update_source_label(row: &ManagedEmulator) -> String {
    match row.manager.as_str() {
        "flatpak" if row.install_path == "system" => "Flatpak · system".to_owned(),
        "flatpak" => "Flatpak · user".to_owned(),
        "appimage" | "github" => read_program_manifest(&row.emulator_id, &row.manager)
            .map(|manifest| format!("{} · {}", row.source_label, manifest.update_transport))
            .unwrap_or_else(|| row.source_label.clone()),
        _ => row.source_label.clone(),
    }
}

fn manager_available(manager: &str, metadata_json: &str) -> bool {
    let metadata: SourceMetadata = serde_json::from_str(metadata_json).unwrap_or_default();
    if !source_supports_current_architecture(&metadata) {
        return false;
    }
    match manager {
        "flatpak" => cfg!(target_os = "linux") && command_path("flatpak").is_ok(),
        "appimage" => {
            cfg!(target_os = "linux") && matches!(env::consts::ARCH, "x86_64" | "aarch64")
        }
        "nix" => cfg!(target_os = "linux") && command_path("nix").is_ok(),
        "github" => {
            cfg!(any(
                target_os = "linux",
                target_os = "windows",
                target_os = "macos"
            )) && (metadata.archive_format != "dmg"
                || (cfg!(target_os = "macos") && command_path("hdiutil").is_ok()))
        }
        "direct" => {
            cfg!(target_os = "linux")
                && (metadata.runtime != "wine"
                    || (command_path("wine").is_ok() || command_path("wine64").is_ok()))
        }
        "winget" => cfg!(target_os = "windows") && command_path("winget").is_ok(),
        "homebrew" => cfg!(target_os = "macos") && command_path("brew").is_ok(),
        "libretro" => libretro_buildbot_target().is_some(),
        _ => false,
    }
}

fn libretro_buildbot_target() -> Option<(&'static str, &'static str)> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Some(("linux/x86_64", "so")),
        ("linux", "aarch64") => Some(("linux/aarch64", "so")),
        ("windows", "x86_64") => Some(("windows/x86_64", "dll")),
        ("windows", "x86") => Some(("windows/x86", "dll")),
        ("macos", "x86_64") => Some(("apple/osx/x86_64", "dylib")),
        ("macos", "aarch64") => Some(("apple/osx/arm64", "dylib")),
        _ => None,
    }
}

fn libretro_core_file_name(core_name: &str) -> Result<String> {
    validate_package_id("libretro", core_name)?;
    let (_, extension) = libretro_buildbot_target()
        .context("Libretro buildbot does not publish cores for this host architecture")?;
    Ok(format!("{core_name}_libretro.{extension}"))
}

fn libretro_core_url(core_name: &str) -> Result<String> {
    let (buildbot_path, _) = libretro_buildbot_target()
        .context("Libretro buildbot does not publish cores for this host architecture")?;
    let file_name = libretro_core_file_name(core_name)?;
    Ok(format!(
        "https://buildbot.libretro.com/nightly/{buildbot_path}/latest/{file_name}.zip"
    ))
}

fn libretro_core_directories() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(base) = directories::BaseDirs::new() {
        if cfg!(target_os = "linux") {
            paths.push(base.config_dir().join("retroarch/cores"));
            paths.push(
                base.home_dir()
                    .join(".var/app/org.libretro.RetroArch/config/retroarch/cores"),
            );
            paths.push(PathBuf::from("/usr/lib/libretro"));
            paths.push(PathBuf::from("/usr/lib64/libretro"));
        } else if cfg!(target_os = "windows") {
            paths.push(base.data_local_dir().join("RetroArch/cores"));
            if let Some(program_files) = env::var_os("ProgramFiles") {
                paths.push(PathBuf::from(program_files).join("RetroArch/cores"));
            }
        } else if cfg!(target_os = "macos") {
            paths.push(base.data_dir().join("RetroArch/cores"));
            paths.push(PathBuf::from(
                "/Applications/RetroArch.app/Contents/Resources/cores",
            ));
        }
    }
    paths
}

fn installed_libretro_cores() -> HashMap<String, String> {
    let Some((_, extension)) = libretro_buildbot_target() else {
        return HashMap::new();
    };
    let suffix = format!("_libretro.{extension}");
    let mut installed = HashMap::new();
    for directory in libretro_core_directories() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if let Some(core_name) = name.strip_suffix(&suffix) {
                installed
                    .entry(core_name.to_owned())
                    .or_insert_with(|| path.to_string_lossy().into_owned());
            }
        }
    }
    installed
}

fn libretro_core_install_directory(prefer_flatpak: bool) -> Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .context("could not determine a user data directory for RetroArch cores")?;
    if cfg!(target_os = "linux") {
        if prefer_flatpak {
            Ok(base
                .home_dir()
                .join(".var/app/org.libretro.RetroArch/config/retroarch/cores"))
        } else {
            Ok(base.config_dir().join("retroarch/cores"))
        }
    } else if cfg!(target_os = "windows") {
        Ok(base.data_local_dir().join("RetroArch/cores"))
    } else if cfg!(target_os = "macos") {
        Ok(base.data_dir().join("RetroArch/cores"))
    } else {
        bail!("RetroArch cores are not supported on this host")
    }
}

fn ensure_retroarch_runtime(store: &SettingsStore) -> Result<ManagedEmulator> {
    let mut runtime = load_managed_emulators()?
        .into_iter()
        .find(|candidate| candidate.manager != "libretro" && candidate.name == "RetroArch")
        .context("the canonical emulator catalog has no RetroArch package for this host")?;
    if runtime.installed {
        return Ok(runtime);
    }
    if !runtime.can_install() {
        bail!(
            "RetroArch must be installed first, but {} is unavailable",
            runtime.source_label
        );
    }
    let install_path = install(&runtime).context("installing RetroArch before its core")?;
    store.record_managed_emulator_install(
        &runtime.emulator_id,
        &runtime.host_system_slug,
        &runtime.manager,
        &runtime.package_id,
        &install_path,
    )?;
    runtime.installed = true;
    runtime.managed = true;
    runtime.install_path = install_path;
    Ok(runtime)
}

fn install_libretro_core(row: &ManagedEmulator, prefer_flatpak: bool) -> Result<String> {
    let file_name = libretro_core_file_name(&row.package_id)?;
    let directory = libretro_core_install_directory(prefer_flatpak)?;
    fs::create_dir_all(&directory)?;
    let archive = directory.join(format!(".{file_name}.download-{}.zip", std::process::id()));
    let staged = directory.join(format!(".{file_name}.part-{}", std::process::id()));
    let target = directory.join(&file_name);
    let url = libretro_core_url(&row.package_id)?;
    let result = (|| {
        download_url(&url, &archive, None, None)?;
        extract_exact_zip_member(&archive, &file_name, &staged)?;
        replace_file_safely(&staged, &target)?;
        Ok(target.to_string_lossy().into_owned())
    })();
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_file(&staged);
    result
}

fn remove_owned_libretro_core(row: &ManagedEmulator) -> Result<()> {
    let expected_name = libretro_core_file_name(&row.package_id)?;
    let path = PathBuf::from(&row.install_path);
    if path.file_name().and_then(|name| name.to_str()) != Some(expected_name.as_str()) {
        bail!("managed Libretro receipt does not identify the expected core file");
    }
    if !libretro_core_directories()
        .into_iter()
        .any(|directory| path.starts_with(directory))
    {
        bail!("managed Libretro receipt points outside a RetroArch core directory");
    }
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("removing managed Libretro core {}", path.display()))?;
    }
    Ok(())
}

fn extract_exact_zip_member(archive_path: &Path, member_name: &str, target: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).context("opening Libretro core ZIP")?;
    let mut matched = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            bail!("Libretro core ZIP contains an unsafe path");
        };
        if relative.file_name().and_then(|name| name.to_str()) == Some(member_name)
            && matched.replace(index).is_some()
        {
            bail!("Libretro core ZIP contains the expected core more than once");
        }
    }
    let index = matched.context("Libretro core ZIP does not contain the expected core")?;
    let mut entry = archive.by_index(index)?;
    let mut output = File::create(target)?;
    std::io::copy(&mut entry, &mut output)?;
    output.sync_all()?;
    Ok(())
}

fn replace_file_safely(staged: &Path, target: &Path) -> Result<()> {
    let backup = target.with_extension(format!("backup-{}", std::process::id()));
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if target.exists() {
        fs::rename(target, &backup)?;
    }
    if let Err(error) = fs::rename(staged, target) {
        if backup.exists() {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).context("publishing updated Libretro core");
    }
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
}

fn source_key(source: &InstallSource) -> (String, String, String) {
    (
        source.host_system_slug.clone(),
        source.manager.clone(),
        source.package_id.clone(),
    )
}

fn receipt_key(receipt: &ManagedEmulatorInstall) -> (String, String, String) {
    (
        receipt.host_system_slug.clone(),
        receipt.manager.clone(),
        receipt.package_id.clone(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum UpdateCheckResult {
    Current,
    Available(String),
    Unsupported,
    Failed(String),
}

#[derive(Default)]
struct UpdateChecks {
    flatpak: HashMap<(String, String), String>,
    homebrew: HashMap<String, String>,
    github: HashMap<String, Result<String, String>>,
    warnings: Vec<String>,
}

impl UpdateChecks {
    fn load(host: &str, installed: &InstalledSourceGroups) -> Self {
        let mut checks = Self::default();
        if host == "linux" {
            for (scope, flag) in [("user", "--user"), ("system", "--system")] {
                let needed = installed.values().any(|(source, state, _, _)| {
                    source.manager == "flatpak" && state.install_path == scope
                });
                if !needed {
                    continue;
                }
                match load_flatpak_updates(scope, flag) {
                    Ok(updates) => checks.flatpak.extend(updates),
                    Err(error) => checks
                        .warnings
                        .push(format!("Flatpak {scope} update check: {error:#}")),
                }
            }
        }
        if host == "macos"
            && installed
                .values()
                .any(|(source, _, _, _)| source.manager == "homebrew")
        {
            match load_homebrew_updates() {
                Ok(updates) => checks.homebrew = updates,
                Err(error) => checks
                    .warnings
                    .push(format!("Homebrew update check: {error:#}")),
            }
        }
        checks
    }

    fn check(&mut self, row: &ManagedEmulator) -> UpdateCheckResult {
        if !row.manager_available {
            return UpdateCheckResult::Unsupported;
        }
        match row.manager.as_str() {
            "flatpak" => self
                .flatpak
                .get(&(row.install_path.clone(), row.package_id.clone()))
                .cloned()
                .map(UpdateCheckResult::Available)
                .unwrap_or(UpdateCheckResult::Current),
            "nix" => match latest_nixpkgs_version(&row.package_id) {
                Ok(available) if versions_match(&row.version, &available) => {
                    UpdateCheckResult::Current
                }
                Ok(_) if row.version.trim().is_empty() => UpdateCheckResult::Failed(
                    "the installed Nix profile version could not be determined".to_owned(),
                ),
                Ok(available) => UpdateCheckResult::Available(available),
                Err(error) => UpdateCheckResult::Failed(format!("{error:#}")),
            },
            "appimage" | "github" => {
                if row.version.trim().is_empty() {
                    return UpdateCheckResult::Failed(
                        "the installed release version could not be determined".to_owned(),
                    );
                }
                let metadata: SourceMetadata =
                    serde_json::from_str(&row.metadata_json).unwrap_or_default();
                let release_key = format!("{}|{}", row.package_id, metadata.release_api);
                let result = self.github.entry(release_key).or_insert_with(|| {
                    managed_latest_release(&row.package_id, &metadata)
                        .map(|release| normalize_version(&release.tag_name))
                        .map_err(|error| format!("{error:#}"))
                });
                match result {
                    Ok(available) if versions_match(&row.version, available) => {
                        UpdateCheckResult::Current
                    }
                    Ok(available) => UpdateCheckResult::Available(available.clone()),
                    Err(error) => UpdateCheckResult::Failed(error.clone()),
                }
            }
            "winget" => check_winget_update(&row.package_id),
            "homebrew" => self
                .homebrew
                .get(&row.package_id)
                .cloned()
                .map(UpdateCheckResult::Available)
                .unwrap_or(UpdateCheckResult::Current),
            "direct" | "libretro" => UpdateCheckResult::Unsupported,
            _ => UpdateCheckResult::Unsupported,
        }
    }
}

fn versions_match(installed: &str, available: &str) -> bool {
    installed
        .trim()
        .trim_start_matches(['v', 'V'])
        .eq_ignore_ascii_case(available.trim().trim_start_matches(['v', 'V']))
}

fn latest_nixpkgs_version(package_id: &str) -> Result<String> {
    validate_package_id("nix", package_id)?;
    let output = run_checked(
        command_path("nix")?,
        ["eval", "--raw", &format!("nixpkgs#{package_id}.version")],
        "Nixpkgs version check",
    )?;
    let version = String::from_utf8(output.stdout)
        .context("Nixpkgs returned a non-UTF-8 version")?
        .trim()
        .to_owned();
    if version.is_empty() {
        bail!("Nixpkgs returned an empty version for {package_id}");
    }
    Ok(version)
}

fn load_flatpak_updates(scope: &str, flag: &str) -> Result<HashMap<(String, String), String>> {
    let output = run_checked(
        command_path("flatpak")?,
        [
            "remote-ls",
            flag,
            "--updates",
            "--app",
            "--columns=application,version",
        ],
        &format!("Flatpak {scope} update inventory"),
    )?;
    Ok(parse_flatpak_update_output(scope, &output.stdout))
}

fn parse_flatpak_update_output(scope: &str, output: &[u8]) -> HashMap<(String, String), String> {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t').map(str::trim);
            let package_id = fields.next()?.to_owned();
            (!package_id.is_empty()).then(|| {
                (
                    (scope.to_owned(), package_id),
                    fields.next().unwrap_or_default().to_owned(),
                )
            })
        })
        .collect()
}

fn load_homebrew_updates() -> Result<HashMap<String, String>> {
    let output = run_checked(
        command_path("brew")?,
        ["outdated", "--json=v2"],
        "Homebrew update inventory",
    )?;
    parse_homebrew_update_output(&output.stdout)
}

fn parse_homebrew_update_output(output: &[u8]) -> Result<HashMap<String, String>> {
    let value: serde_json::Value =
        serde_json::from_slice(output).context("parsing Homebrew outdated JSON")?;
    let mut updates = HashMap::new();
    for kind in ["formulae", "casks"] {
        let rows = value
            .get(kind)
            .and_then(serde_json::Value::as_array)
            .with_context(|| format!("Homebrew outdated JSON has no {kind} array"))?;
        for row in rows {
            let Some(name) = row.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let available = row
                .get("current_version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            updates.insert(name.to_owned(), available.to_owned());
        }
    }
    Ok(updates)
}

fn check_winget_update(package_id: &str) -> UpdateCheckResult {
    if !cfg!(target_os = "windows") {
        return UpdateCheckResult::Unsupported;
    }
    let Ok(winget) = command_path("winget") else {
        return UpdateCheckResult::Unsupported;
    };
    let output = match host_command(winget)
        .args([
            "upgrade",
            "--disable-interactivity",
            "--accept-source-agreements",
            "-e",
            "--id",
            package_id,
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) => return UpdateCheckResult::Failed(error.to_string()),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() && output_has_exact_package_id(&stdout, package_id) {
        UpdateCheckResult::Available(String::new())
    } else if output.status.success() {
        UpdateCheckResult::Current
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let detail = if stderr.is_empty() {
            stdout.trim().to_owned()
        } else {
            stderr
        };
        UpdateCheckResult::Failed(format!("winget update check failed: {detail}"))
    }
}

fn output_has_exact_package_id(output: &str, package_id: &str) -> bool {
    output
        .lines()
        .any(|line| line.split_whitespace().any(|field| field == package_id))
}

struct HostSnapshot {
    flatpaks: HashMap<(String, String), String>,
    nix_profile: Option<NixProfileList>,
    homebrew: HashMap<String, String>,
}

impl HostSnapshot {
    fn load(host: &str, sources: &[InstallSource]) -> Result<Self> {
        let flatpaks =
            if host == "linux" && sources.iter().any(|source| source.manager == "flatpak") {
                installed_flatpaks()
            } else {
                HashMap::new()
            };
        let nix_profile = if host == "linux"
            && sources.iter().any(|source| source.manager == "nix")
            && command_path("nix").is_ok()
        {
            Some(load_nix_profile()?)
        } else {
            None
        };
        let homebrew = if host == "macos" {
            installed_homebrew_packages()
        } else {
            HashMap::new()
        };
        Ok(Self {
            flatpaks,
            nix_profile,
            homebrew,
        })
    }

    fn installation_state(
        &self,
        source: &InstallSource,
        receipt: Option<&ManagedEmulatorInstall>,
    ) -> Result<InstallationState> {
        match source.manager.as_str() {
            "flatpak" => Ok(["user", "system"]
                .into_iter()
                .find_map(|scope| {
                    self.flatpaks
                        .get(&(scope.to_owned(), source.package_id.clone()))
                        .map(|version| InstallationState {
                            installed: true,
                            version: version.clone(),
                            install_path: scope.to_owned(),
                        })
                })
                .unwrap_or_default()),
            "nix" => Ok(self
                .nix_profile
                .as_ref()
                .and_then(|profile| nix_profile_element(profile, &source.package_id))
                .map(|(name, element)| InstallationState {
                    installed: element.active,
                    version: nix_store_version(&element.store_paths, &source.package_id)
                        .unwrap_or_default(),
                    install_path: name,
                })
                .unwrap_or_default()),
            "homebrew" => Ok(self
                .homebrew
                .get(&source.package_id)
                .map(|version| InstallationState {
                    installed: true,
                    version: version.clone(),
                    install_path: source.package_id.clone(),
                })
                .unwrap_or_default()),
            "winget" => installed_winget(&source.package_id),
            "appimage" | "github" | "direct" => Ok(receipt
                .filter(|receipt| Path::new(&receipt.install_path).exists())
                .map(|receipt| {
                    let manifest = read_program_manifest(&source.emulator_id, &source.manager);
                    InstallationState {
                        installed: true,
                        version: manifest
                            .map(|manifest| manifest.version)
                            .unwrap_or_default(),
                        install_path: receipt.install_path.clone(),
                    }
                })
                .unwrap_or_default()),
            _ => Ok(InstallationState::default()),
        }
    }

    fn installation_states_for_updates(
        &self,
        source: &InstallSource,
        receipt: Option<&ManagedEmulatorInstall>,
    ) -> Result<Vec<InstallationState>> {
        if source.manager == "flatpak" {
            return Ok(["user", "system"]
                .into_iter()
                .filter_map(|scope| {
                    self.flatpaks
                        .get(&(scope.to_owned(), source.package_id.clone()))
                        .map(|version| InstallationState {
                            installed: true,
                            version: version.clone(),
                            install_path: scope.to_owned(),
                        })
                })
                .collect());
        }
        let state = self.installation_state(source, receipt)?;
        Ok(state.installed.then_some(state).into_iter().collect())
    }
}

fn command_path(name: &str) -> Result<PathBuf> {
    if is_flatpak() && host_program_available(name) {
        return Ok(PathBuf::from(name));
    }
    let mut names = vec![name.to_owned()];
    if cfg!(target_os = "windows") && !name.to_ascii_lowercase().ends_with(".exe") {
        names.push(format!("{name}.exe"));
    }
    let mut directories = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    directories.extend([
        PathBuf::from("/run/current-system/sw/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ]);
    directories
        .into_iter()
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|path| path.is_file())
        .with_context(|| format!("{name} is not installed or not on PATH"))
}

fn run_checked<I, S>(program: PathBuf, arguments: I, operation: &str) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = host_command(&program)
        .args(arguments)
        .output()
        .with_context(|| format!("starting {operation} with {}", program.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        bail!("{operation} failed: {detail}");
    }
    Ok(output)
}

fn installed_flatpaks() -> HashMap<(String, String), String> {
    let Ok(flatpak) = command_path("flatpak") else {
        return HashMap::new();
    };
    let mut installed = HashMap::new();
    for (scope, flag) in [("user", "--user"), ("system", "--system")] {
        let Ok(output) = host_command(&flatpak)
            .args(["list", flag, "--app", "--columns=application,version"])
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut fields = line.split('\t');
            let Some(app_id) = fields
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            installed.insert(
                (scope.to_owned(), app_id.to_owned()),
                fields.next().unwrap_or_default().trim().to_owned(),
            );
        }
    }
    installed
}

fn installed_homebrew_packages() -> HashMap<String, String> {
    let Ok(brew) = command_path("brew") else {
        return HashMap::new();
    };
    let mut installed = HashMap::new();
    for kind in ["--formula", "--cask"] {
        let Ok(output) = host_command(&brew)
            .args(["list", "--versions", kind])
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut fields = line.split_whitespace();
            if let Some(package) = fields.next() {
                installed.insert(package.to_owned(), fields.collect::<Vec<_>>().join(", "));
            }
        }
    }
    installed
}

fn installed_winget(package_id: &str) -> Result<InstallationState> {
    if !cfg!(target_os = "windows") {
        return Ok(InstallationState::default());
    }
    let output = host_command(command_path("winget")?)
        .args(["list", "--disable-interactivity", "-e", "--id", package_id])
        .output()
        .context("checking winget installation")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(InstallationState {
        installed: output.status.success() && stdout.contains(package_id),
        install_path: package_id.to_owned(),
        ..InstallationState::default()
    })
}

fn nix_profile_path() -> Result<PathBuf> {
    let state = directories::BaseDirs::new()
        .map(|dirs| {
            dirs.state_dir()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| dirs.home_dir().join(".local/state"))
        })
        .context("could not determine home directory for the Lunchbox Nix profile")?;
    Ok(state.join("nix/profiles/lunchbox"))
}

fn load_nix_profile() -> Result<NixProfileList> {
    let profile = nix_profile_path()?;
    let output = host_command(command_path("nix")?)
        .args(["profile", "list", "--profile"])
        .arg(&profile)
        .arg("--json")
        .output()
        .context("listing the Lunchbox Nix profile")?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        if error.contains("does not exist") || error.contains("No such file") {
            return Ok(NixProfileList {
                elements: BTreeMap::new(),
            });
        }
        bail!("Nix profile list failed: {}", error.trim());
    }
    serde_json::from_slice(&output.stdout).context("parsing the Lunchbox Nix profile")
}

fn nix_profile_element<'a>(
    profile: &'a NixProfileList,
    package: &str,
) -> Option<(String, &'a NixProfileElement)> {
    profile.elements.iter().find_map(|(name, element)| {
        let attribute_matches = element.attr_path.as_deref().is_some_and(|attribute| {
            attribute == package || attribute.ends_with(&format!(".{package}"))
        });
        (name == package || attribute_matches).then(|| (name.clone(), element))
    })
}

fn nix_store_version(paths: &[String], package: &str) -> Option<String> {
    paths.iter().find_map(|path| {
        let name = Path::new(path).file_name()?.to_str()?;
        let derivation = name.split_once('-')?.1;
        derivation
            .match_indices('-')
            .map(|(index, _)| &derivation[index + 1..])
            .find(|candidate| {
                candidate.starts_with(|character: char| character.is_ascii_digit())
                    || candidate.starts_with("unstable-")
                    || candidate.starts_with("git-")
            })
            .map(ToOwned::to_owned)
            .or_else(|| {
                derivation
                    .strip_prefix(&format!("{package}-"))
                    .map(ToOwned::to_owned)
            })
    })
}

fn install_nix(row: &ManagedEmulator) -> Result<String> {
    let profile = nix_profile_path()?;
    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating Nix profile directory {}", parent.display()))?;
    }
    let output = host_command(command_path("nix")?)
        .args(["profile", "install", "--profile"])
        .arg(&profile)
        .arg(format!("nixpkgs#{}", row.package_id))
        .output()
        .context("starting Nix profile install")?;
    require_output(output, "Nix profile install")?;
    Ok(profile.join("bin").to_string_lossy().into_owned())
}

fn update_nix(row: &ManagedEmulator) -> Result<String> {
    let profile = nix_profile_path()?;
    let profile_list = load_nix_profile()?;
    let (name, _) = nix_profile_element(&profile_list, &row.package_id)
        .with_context(|| format!("{} is not in the Lunchbox Nix profile", row.package_id))?;
    let output = host_command(command_path("nix")?)
        .args(["profile", "upgrade", "--profile"])
        .arg(&profile)
        .arg(&name)
        .output()
        .context("starting Nix profile upgrade")?;
    require_output(output, "Nix profile upgrade")?;
    Ok(name)
}

fn uninstall_nix(row: &ManagedEmulator) -> Result<()> {
    let profile = nix_profile_path()?;
    let profile_list = load_nix_profile()?;
    let (name, _) = nix_profile_element(&profile_list, &row.package_id)
        .with_context(|| format!("{} is not in the Lunchbox Nix profile", row.package_id))?;
    let output = host_command(command_path("nix")?)
        .args(["profile", "remove", "--profile"])
        .arg(&profile)
        .arg(&name)
        .output()
        .context("starting Nix profile removal")?;
    require_output(output, "Nix profile removal")?;
    Ok(())
}

fn require_output(output: Output, operation: &str) -> Result<Output> {
    if output.status.success() {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    bail!(
        "{operation} failed: {}",
        if stderr.is_empty() { stdout } else { stderr }
    )
}

fn programs_root() -> Result<PathBuf> {
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("programs"))
        .context("could not determine the managed emulator data directory")
}

fn program_root(emulator_id: &str, manager: &str) -> Result<PathBuf> {
    if emulator_id.is_empty()
        || !emulator_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        bail!("invalid stable emulator identity");
    }
    Ok(programs_root()?.join(manager).join(emulator_id))
}

fn manifest_path(emulator_id: &str, manager: &str) -> Result<PathBuf> {
    Ok(program_root(emulator_id, manager)?.join("install.json"))
}

fn read_program_manifest(emulator_id: &str, manager: &str) -> Option<ManagedProgramManifest> {
    let bytes = fs::read(manifest_path(emulator_id, manager).ok()?).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_program_manifest(manifest: &ManagedProgramManifest) -> Result<()> {
    let path = manifest_path(&manifest.emulator_id, &manifest.manager)?;
    let parent = path
        .parent()
        .context("managed program manifest has no parent")?;
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec_pretty(manifest)?;
    let temporary = path.with_extension(format!("json.part-{}", std::process::id()));
    fs::write(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&temporary, &path)?;
    Ok(())
}

fn install_github_program(row: &ManagedEmulator) -> Result<String> {
    let metadata: SourceMetadata = serde_json::from_str(&row.metadata_json)?;
    let release = managed_latest_release(&row.package_id, &metadata)?;
    let asset = select_release_asset(&release, &metadata)?;
    let version = normalize_version(&release.tag_name);
    let root = program_root(&row.emulator_id, &row.manager)?;
    let version_root = root.join(&version);
    let staging = root.join(format!(".staging-{}", std::process::id()));
    remove_exact_tree_if_present(&staging)?;
    fs::create_dir_all(&staging)?;
    let archive = staging.join(&asset.name);
    download_asset(asset, &archive)?;

    let payload_root = staging.join("payload");
    fs::create_dir_all(&payload_root)?;
    if asset.name.to_ascii_lowercase().ends_with(".appimage") {
        let target = payload_root.join(&asset.name);
        fs::rename(&archive, &target)?;
    } else if asset.name.to_ascii_lowercase().ends_with(".zip")
        || metadata.archive_format.eq_ignore_ascii_case("zip")
    {
        extract_zip(&archive, &payload_root)?;
    } else if asset.name.to_ascii_lowercase().ends_with(".tar.gz") {
        extract_tar_gz(&archive, &payload_root)?;
    } else if asset.name.to_ascii_lowercase().ends_with(".dmg")
        || metadata.archive_format.eq_ignore_ascii_case("dmg")
    {
        extract_dmg(&archive, &payload_root)?;
    } else {
        bail!("unsupported managed release asset {}", asset.name);
    }

    let executable = find_managed_executable(&payload_root, &metadata)?;
    mark_executable(&executable)?;
    if version_root.exists() {
        remove_exact_tree_if_present(&version_root)?;
    }
    fs::rename(&payload_root, &version_root)?;
    remove_exact_tree_if_present(&staging)?;
    let final_executable = version_root.join(executable.strip_prefix(&payload_root)?);
    let update_transport = if row.manager == "appimage" {
        appimage_update_transport(&final_executable).unwrap_or_else(|| {
            if metadata.release_api.is_empty() {
                "github-release".to_owned()
            } else {
                "official-release".to_owned()
            }
        })
    } else if !metadata.release_api.is_empty() {
        "official-release".to_owned()
    } else {
        "github-release".to_owned()
    };
    write_program_manifest(&ManagedProgramManifest {
        emulator_id: row.emulator_id.clone(),
        manager: row.manager.clone(),
        package_id: row.package_id.clone(),
        version,
        asset_name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
        executable_path: final_executable.to_string_lossy().into_owned(),
        update_transport,
    })?;
    Ok(final_executable.to_string_lossy().into_owned())
}

fn update_appimage(row: &ManagedEmulator) -> Result<String> {
    let Some(mut manifest) = read_program_manifest(&row.emulator_id, &row.manager) else {
        return install_github_program(row);
    };
    let executable = PathBuf::from(&manifest.executable_path);
    if executable.is_file()
        && appimage_update_transport(&executable).is_some()
        && let Ok(updated) = run_appimage_update(&executable)
    {
        let metadata: SourceMetadata = serde_json::from_str(&row.metadata_json)?;
        if let Ok(release) = managed_latest_release(&row.package_id, &metadata) {
            manifest.version = normalize_version(&release.tag_name);
        }
        manifest.executable_path = updated.to_string_lossy().into_owned();
        write_program_manifest(&manifest)?;
        return Ok(updated.to_string_lossy().into_owned());
    }
    install_github_program(row)
}

fn run_appimage_update(executable: &Path) -> Result<PathBuf> {
    let updater = ensure_appimage_updater()?;
    let work_root = programs_root()?.join("appimage-tools/update-work");
    fs::create_dir_all(&work_root)?;
    let work = work_root.join(format!(
        "{}-{}",
        std::process::id(),
        crate::settings::unix_timestamp()
    ));
    fs::create_dir_all(&work)?;
    let staged = work.join(
        executable
            .file_name()
            .context("installed AppImage has no file name")?,
    );
    fs::copy(executable, &staged)?;
    mark_executable(&staged)?;
    let output = host_command(&updater)
        .arg(&staged)
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .current_dir(&work)
        .output()
        .context("starting AppImageUpdate")?;
    require_output(output, "AppImageUpdate")?;
    let updated = find_files(&work, |path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("appimage"))
            && path != staged
    })
    .into_iter()
    .next()
    .unwrap_or(staged);
    let replacement = executable.with_extension("AppImage.updated");
    fs::copy(&updated, &replacement)?;
    mark_executable(&replacement)?;
    replace_file_safely(&replacement, executable)?;
    remove_exact_tree_if_present(&work)?;
    Ok(executable.to_path_buf())
}

fn ensure_appimage_updater() -> Result<PathBuf> {
    let repository = "AppImageCommunity/AppImageUpdate";
    let release = github_latest_release(repository)?;
    let architecture = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    let asset = release
        .assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.starts_with("appimageupdatetool-")
                && name.contains(architecture)
                && name.ends_with(".appimage")
        })
        .max_by_key(|asset| asset.name.clone())
        .context("official AppImageUpdate release has no updater for this architecture")?;
    let root = programs_root()?
        .join("appimage-tools")
        .join(normalize_version(&release.tag_name));
    let path = root.join(&asset.name);
    if !path.is_file() {
        fs::create_dir_all(&root)?;
        download_asset(asset, &path)?;
        mark_executable(&path)?;
    }
    Ok(path)
}

fn appimage_update_transport(path: &Path) -> Option<String> {
    for flag in ["--appimage-updateinformation", "--appimage-updateinfo"] {
        let output = host_command(path)
            .arg(flag)
            .env("APPIMAGE_EXTRACT_AND_RUN", "1")
            .output()
            .ok()?;
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !value.is_empty() {
                return Some(value.split('|').next().unwrap_or("appimage").to_owned());
            }
        }
    }
    None
}

fn install_direct_program(row: &ManagedEmulator) -> Result<String> {
    let metadata: SourceMetadata = serde_json::from_str(&row.metadata_json)?;
    if metadata.runtime != "wine" {
        bail!("unsupported direct emulator runtime {}", metadata.runtime);
    }
    let page = download_text(&row.package_id, 2 * 1024 * 1024)?;
    let href = extract_altirra_href(&page)
        .context("official Altirra page did not contain the expected x86/x64 ZIP")?;
    let url = resolve_https_url(&row.package_id, &href)?;
    let root = program_root(&row.emulator_id, &row.manager)?;
    let staging = root.join(format!(".staging-{}", std::process::id()));
    remove_exact_tree_if_present(&staging)?;
    fs::create_dir_all(&staging)?;
    let archive = staging.join("download.zip");
    download_url(&url, &archive, None, None)?;
    let payload = staging.join("payload");
    fs::create_dir_all(&payload)?;
    extract_zip(&archive, &payload)?;
    let executable = find_managed_executable(&payload, &metadata)?;
    let version = href
        .split("Altirra-")
        .nth(1)
        .and_then(|value| value.strip_suffix(".zip"))
        .unwrap_or("latest")
        .to_owned();
    let version_root = root.join(&version);
    if version_root.exists() {
        remove_exact_tree_if_present(&version_root)?;
    }
    fs::rename(&payload, &version_root)?;
    remove_exact_tree_if_present(&staging)?;
    let final_executable = version_root.join(executable.strip_prefix(&payload)?);
    write_program_manifest(&ManagedProgramManifest {
        emulator_id: row.emulator_id.clone(),
        manager: row.manager.clone(),
        package_id: row.package_id.clone(),
        version,
        asset_name: href,
        download_url: url,
        executable_path: final_executable.to_string_lossy().into_owned(),
        update_transport: "official-page".to_owned(),
    })?;
    Ok(final_executable.to_string_lossy().into_owned())
}

fn remove_owned_program(row: &ManagedEmulator) -> Result<()> {
    let root = program_root(&row.emulator_id, &row.manager)?;
    if !row.install_path.is_empty() && !Path::new(&row.install_path).starts_with(&root) {
        bail!("managed emulator receipt points outside the Lunchbox program directory");
    }
    remove_exact_tree_if_present(&root)
}

fn github_latest_release(repository: &str) -> Result<GitHubRelease> {
    validate_package_id("github", repository)?;
    let url = format!("{GITHUB_API}/{repository}/releases/latest");
    let text = download_text(&url, 4 * 1024 * 1024)?;
    serde_json::from_str(&text).context("parsing GitHub release metadata")
}

fn managed_latest_release(repository: &str, metadata: &SourceMetadata) -> Result<GitHubRelease> {
    if metadata.release_api.is_empty() {
        return github_latest_release(repository);
    }
    if !metadata.release_api.starts_with("https://") {
        bail!("managed release APIs must use HTTPS");
    }
    let text = download_text(&metadata.release_api, 4 * 1024 * 1024)?;
    serde_json::from_str(&text).context("parsing official release metadata")
}

fn select_release_asset<'a>(
    release: &'a GitHubRelease,
    metadata: &SourceMetadata,
) -> Result<&'a GitHubAsset> {
    if !source_supports_current_architecture(metadata) {
        bail!(
            "the reviewed release source does not support host architecture {}",
            env::consts::ARCH
        );
    }
    let mut reviewed_terms = metadata.asset_terms.clone();
    if let Some(architecture_terms) = metadata.asset_terms_by_arch.get(env::consts::ARCH) {
        reviewed_terms.extend(architecture_terms.iter().cloned());
    }
    release
        .assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            let format_matches =
                if metadata.archive_payload == "appimage" && metadata.archive_format == "tar.gz" {
                    name.ends_with(".tar.gz") && name.contains("appimage")
                } else if metadata.archive_payload == "appimage" {
                    name.ends_with(".appimage")
                } else if metadata.archive_format == "zip" {
                    name.ends_with(".zip")
                } else if metadata.archive_format == "dmg" {
                    name.ends_with(".dmg")
                } else {
                    true
                };
            format_matches
                && reviewed_terms
                    .iter()
                    .all(|term| name.contains(&term.to_ascii_lowercase()))
        })
        .min_by_key(|asset| asset.name.to_ascii_lowercase())
        .context("latest GitHub release has no asset matching the reviewed catalog rules")
}

fn normalize_version(tag: &str) -> String {
    let value = tag.trim().trim_start_matches('v');
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".+_-".contains(character))
    {
        "latest".to_owned()
    } else {
        value.to_owned()
    }
}

fn download_text(url: &str, limit: u64) -> Result<String> {
    if !url.starts_with("https://") {
        bail!("managed downloads require HTTPS");
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(60)))
        .http_status_as_error(false)
        .user_agent("Lunchbox/0.1 managed emulator installer")
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .with_context(|| format!("requesting {url}"))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        bail!("managed download request failed with HTTP {status}");
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("managed download response exceeds its safety limit");
    }
    String::from_utf8(bytes).context("managed download response is not UTF-8")
}

fn download_asset(asset: &GitHubAsset, path: &Path) -> Result<()> {
    let digest = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"));
    download_url(
        &asset.browser_download_url,
        path,
        (asset.size > 0).then_some(asset.size),
        digest,
    )
}

fn download_url(
    url: &str,
    path: &Path,
    expected_size: Option<u64>,
    expected_sha256: Option<&str>,
) -> Result<()> {
    if !url.starts_with("https://") {
        bail!("managed downloads require HTTPS");
    }
    if expected_size.is_some_and(|size| size > MAX_MANAGED_DOWNLOAD_BYTES) {
        bail!("managed emulator asset exceeds the 1 GiB safety limit");
    }
    let parent = path
        .parent()
        .context("managed download path has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("part-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(1800)))
        .http_status_as_error(false)
        .user_agent("Lunchbox/0.1 managed emulator installer")
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .with_context(|| format!("downloading {url}"))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        bail!("managed emulator download failed with HTTP {status}");
    }
    let mut input = response
        .body_mut()
        .as_reader()
        .take(MAX_MANAGED_DOWNLOAD_BYTES + 1);
    let mut output = File::create(&temporary)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    let mut written = 0_u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output.write_all(&buffer[..count])?;
        digest.update(&buffer[..count]);
        written += count as u64;
        if written > MAX_MANAGED_DOWNLOAD_BYTES {
            let _ = fs::remove_file(&temporary);
            bail!("managed emulator download exceeds the 1 GiB safety limit");
        }
    }
    output.sync_all()?;
    if expected_size.is_some_and(|size| size != written) {
        let _ = fs::remove_file(&temporary);
        bail!("managed emulator asset size does not match GitHub release metadata");
    }
    if expected_sha256.is_some_and(|expected| hex::encode(digest.finalize()) != expected) {
        let _ = fs::remove_file(&temporary);
        bail!("managed emulator asset SHA-256 does not match GitHub release metadata");
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path)?;
    Ok(())
}

fn extract_zip(archive_path: &Path, output_root: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).context("opening managed emulator ZIP")?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .context("managed emulator ZIP contains an unsafe path")?
            .to_path_buf();
        let target = output_root.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&target)?;
        std::io::copy(&mut entry, &mut output)?;
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, output_root: &Path) -> Result<()> {
    let decoder = GzDecoder::new(File::open(archive_path)?);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        entry
            .unpack_in(output_root)
            .context("extracting managed emulator tar archive")?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn extract_dmg(archive_path: &Path, output_root: &Path) -> Result<()> {
    let mountpoint = archive_path.with_extension(format!("mount-{}", std::process::id()));
    remove_exact_tree_if_present(&mountpoint)?;
    fs::create_dir_all(&mountpoint)?;
    let attach = host_command(command_path("hdiutil")?)
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mountpoint)
        .arg(archive_path)
        .output()
        .context("mounting managed emulator DMG")?;
    if let Err(error) = require_output(attach, "DMG mount") {
        let _ = remove_exact_tree_if_present(&mountpoint);
        return Err(error);
    }

    let copied = (|| -> Result<()> {
        let applications = fs::read_dir(&mountpoint)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("Eden.app"))
            })
            .collect::<Vec<_>>();
        let [application] = applications.as_slice() else {
            bail!("official Eden DMG must contain exactly one Eden.app bundle");
        };
        let target = output_root.join("Eden.app");
        let output = host_command(command_path("ditto")?)
            .arg(application)
            .arg(&target)
            .output()
            .context("copying Eden application bundle")?;
        require_output(output, "Eden application copy")?;
        Ok(())
    })();

    let detach = host_command(command_path("hdiutil")?)
        .args(["detach", "-force"])
        .arg(&mountpoint)
        .output()
        .context("unmounting managed emulator DMG")
        .and_then(|output| require_output(output, "DMG unmount").map(|_| ()));
    let _ = remove_exact_tree_if_present(&mountpoint);
    copied.and(detach)
}

#[cfg(not(target_os = "macos"))]
fn extract_dmg(_archive_path: &Path, _output_root: &Path) -> Result<()> {
    bail!("managed DMG installation is available only on macOS")
}

fn find_managed_executable(root: &Path, metadata: &SourceMetadata) -> Result<PathBuf> {
    let candidates = metadata
        .executable_candidates
        .iter()
        .map(|candidate| candidate.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut files = find_files(root, |_| true);
    files.sort();
    files
        .into_iter()
        .find(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if metadata.archive_payload == "appimage" {
                name.ends_with(".appimage")
            } else {
                candidates.contains(&name)
                    || candidates
                        .iter()
                        .any(|candidate| name == format!("{candidate}.exe"))
            }
        })
        .context("managed emulator archive did not contain the reviewed executable")
}

fn find_files<F>(root: &Path, predicate: F) -> Vec<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() && predicate(&path) {
                files.push(path);
            }
        }
    }
    files
}

fn mark_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions)?;
    }
    let _ = path;
    Ok(())
}

fn remove_exact_tree_if_present(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?
        }
        Ok(_) => fs::remove_file(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn extract_altirra_href(html: &str) -> Option<String> {
    let start = html.find("downloads/Altirra-")?;
    let suffix = &html[start..];
    let end = suffix.find(".zip")?;
    Some(suffix[..end + 4].to_owned())
}

fn resolve_https_url(base: &str, relative: &str) -> Result<String> {
    let base = base
        .strip_prefix("https://")
        .context("managed direct source must use HTTPS")?;
    let (authority, base_path) = base.split_once('/').unwrap_or((base, ""));
    if authority.is_empty() || authority.contains(['?', '#']) {
        bail!("managed direct source has an invalid HTTPS authority");
    }
    if relative.starts_with("https://") {
        return Ok(relative.to_owned());
    }
    if relative.starts_with("http://") || relative.contains("..") {
        bail!("managed direct source resolved an unsafe download URL");
    }
    let directory = base_path
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    let path = if relative.starts_with('/') {
        relative.trim_start_matches('/').to_owned()
    } else if directory.is_empty() {
        relative.to_owned()
    } else {
        format!("{directory}/{relative}")
    };
    Ok(format!("https://{authority}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_id_validation_rejects_option_and_shell_shaped_values() {
        assert!(validate_package_id("flatpak", "org.libretro.RetroArch").is_ok());
        assert!(validate_package_id("github", "SourMesen/Mesen2").is_ok());
        assert!(validate_package_id("direct", "https://example.com/downloads").is_ok());
        assert!(validate_package_id("libretro", "beetle_psx_hw").is_ok());
        assert!(validate_package_id("nix", "--option").is_err());
        assert!(validate_package_id("homebrew", "name;rm").is_err());
        assert!(validate_package_id("winget", "bad value").is_err());
        assert!(validate_package_id("libretro", "mesen;rm").is_err());
    }

    #[test]
    fn flatpak_update_parser_preserves_scope_and_available_version() {
        let updates = parse_flatpak_update_output(
            "system",
            b"org.libretro.RetroArch\t1.22.2\norg.mamedev.MAME\t0.290\n",
        );
        assert_eq!(
            updates.get(&("system".to_owned(), "org.libretro.RetroArch".to_owned())),
            Some(&"1.22.2".to_owned())
        );
        assert_eq!(
            updates.get(&("system".to_owned(), "org.mamedev.MAME".to_owned())),
            Some(&"0.290".to_owned())
        );
        assert!(!updates.contains_key(&("user".to_owned(), "org.mamedev.MAME".to_owned())));
    }

    #[test]
    fn homebrew_update_parser_combines_formulae_and_casks() {
        let updates = parse_homebrew_update_output(
            br#"{
              "formulae": [{"name":"mame","current_version":"0.290"}],
              "casks": [{"name":"retroarch","current_version":"1.22.2"}]
            }"#,
        )
        .unwrap();
        assert_eq!(updates.get("mame"), Some(&"0.290".to_owned()));
        assert_eq!(updates.get("retroarch"), Some(&"1.22.2".to_owned()));
    }

    #[test]
    fn update_identity_and_version_comparisons_are_exact() {
        assert!(output_has_exact_package_id(
            "RetroArch  Libretro.RetroArch  1.21.0  1.22.2",
            "Libretro.RetroArch"
        ));
        assert!(!output_has_exact_package_id(
            "RetroArch  Libretro.RetroArch.Beta  1.21.0  1.22.2",
            "Libretro.RetroArch"
        ));
        assert!(versions_match("v1.22.2", "1.22.2"));
        assert!(!versions_match("1.21.0", "1.22.2"));
    }

    #[test]
    fn emulator_update_pin_identity_includes_the_exact_install_scope() {
        let row = ManagedEmulator {
            compatible_platform_keys: BTreeSet::new(),
            recommended_platform_keys: BTreeSet::new(),
            emulator_id: "retroarch".to_owned(),
            name: "RetroArch".to_owned(),
            host_system_slug: "linux".to_owned(),
            source_label: "Flatpak".to_owned(),
            manager_available: true,
            manager: "flatpak".to_owned(),
            package_id: "org.libretro.RetroArch".to_owned(),
            metadata_json: String::new(),
            installed: true,
            managed: false,
            version: "1.21.0".to_owned(),
            install_path: "user".to_owned(),
        };
        let mut pin = EmulatorUpdatePin {
            host_system_slug: "linux".to_owned(),
            manager: "flatpak".to_owned(),
            package_id: "org.libretro.RetroArch".to_owned(),
            install_path: "user".to_owned(),
            pinned_version: "1.21.0".to_owned(),
            pinned_at: 10,
        };

        assert_eq!(update_pin_key(&row), update_pin_key_for_pin(&pin));
        pin.install_path = "system".to_owned();
        assert_ne!(update_pin_key(&row), update_pin_key_for_pin(&pin));
    }

    #[test]
    fn operation_recovery_respects_a_live_cross_process_lease() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let lease = try_acquire_emulator_operation_lease(&store)
            .unwrap()
            .unwrap();
        store
            .begin_emulator_lifecycle_operation(
                &operation_id,
                "retroarch",
                "RetroArch",
                "linux",
                "nix",
                "retroarch",
                "retroarch",
                "update",
            )
            .unwrap();

        assert!(
            recover_interrupted_emulator_operations_at(&store)
                .unwrap()
                .is_empty()
        );
        drop(lease);
        let recovered = recover_interrupted_emulator_operations_at(&store).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].id, operation_id);
        assert_eq!(recovered[0].status, "interrupted");
        assert!(
            recover_interrupted_emulator_operations_at(&store)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn duplicate_package_names_have_a_stable_summary() {
        let names = BTreeSet::from([
            "MAME (Arcade)".to_owned(),
            "MAME".to_owned(),
            "MAME (Software Lists)".to_owned(),
        ]);
        assert_eq!(summarize_emulator_names(&names), "MAME (+2 variants)");
    }

    #[test]
    fn nix_store_versions_handle_wrapped_package_names() {
        assert_eq!(
            nix_store_version(
                &["/nix/store/hash-retroarch-with-cores-1.21.0".to_owned()],
                "retroarch"
            ),
            Some("1.21.0".to_owned())
        );
        assert_eq!(
            nix_store_version(
                &["/nix/store/hash-dolphin-emu-unstable-2026-08-01".to_owned()],
                "dolphin-emu"
            ),
            Some("unstable-2026-08-01".to_owned())
        );
    }

    #[test]
    fn exact_altirra_link_and_https_resolution_are_bounded() {
        let html = r#"<a href="downloads/Altirra-4.31.zip">download</a>"#;
        let href = extract_altirra_href(html).unwrap();
        assert_eq!(href, "downloads/Altirra-4.31.zip");
        assert_eq!(
            resolve_https_url("https://www.virtualdub.org/altirra.html", &href).unwrap(),
            "https://www.virtualdub.org/downloads/Altirra-4.31.zip"
        );
        assert!(resolve_https_url("http://example.com/page", &href).is_err());
        assert!(resolve_https_url("https://example.com/page", "../bad.zip").is_err());
    }

    #[test]
    fn release_asset_selection_uses_reviewed_terms_deterministically() {
        let release = GitHubRelease {
            tag_name: "v2.1.1".into(),
            assets: vec![
                GitHubAsset {
                    name: "Mesen_2.1.1_Windows.zip".into(),
                    browser_download_url: "https://example.com/windows".into(),
                    size: 1,
                    digest: None,
                },
                GitHubAsset {
                    name: "Mesen_2.1.1_Linux_x64.zip".into(),
                    browser_download_url: "https://example.com/linux".into(),
                    size: 1,
                    digest: None,
                },
            ],
        };
        let metadata = SourceMetadata {
            archive_format: "zip".into(),
            asset_terms: vec!["mesen".into(), "linux".into(), "x64".into()],
            executable_candidates: vec!["Mesen".into()],
            ..SourceMetadata::default()
        };
        assert_eq!(
            select_release_asset(&release, &metadata).unwrap().name,
            "Mesen_2.1.1_Linux_x64.zip"
        );

        let wrong_platform_only = GitHubRelease {
            tag_name: "v2.1.1".into(),
            assets: vec![release.assets[0].clone()],
        };
        assert!(select_release_asset(&wrong_platform_only, &metadata).is_err());
    }

    #[test]
    fn appimage_payload_can_be_selected_from_a_reviewed_tarball() {
        let release = GitHubRelease {
            tag_name: "v3.0.1".into(),
            assets: vec![
                GitHubAsset {
                    name: "Hypseus.Singe-v3.0.1-win64.zip".into(),
                    browser_download_url: "https://example.com/windows".into(),
                    size: 1,
                    digest: None,
                },
                GitHubAsset {
                    name: "hypseus-singe_v3.0.1_AppImage.tar.gz".into(),
                    browser_download_url: "https://example.com/linux".into(),
                    size: 1,
                    digest: None,
                },
            ],
        };
        let metadata = SourceMetadata {
            archive_format: "tar.gz".into(),
            archive_payload: "appimage".into(),
            asset_terms: vec!["hypseus-singe".into(), "appimage".into()],
            ..SourceMetadata::default()
        };

        assert_eq!(
            select_release_asset(&release, &metadata).unwrap().name,
            "hypseus-singe_v3.0.1_AppImage.tar.gz"
        );
    }

    #[test]
    fn official_eden_release_selects_the_host_appimage_not_its_zsync() {
        let release = GitHubRelease {
            tag_name: "v0.2.1".into(),
            assets: vec![
                GitHubAsset {
                    name: "Eden-Linux-amd64-clang-pgo.AppImage.zsync".into(),
                    browser_download_url: "https://stable.eden-emu.dev/update.zsync".into(),
                    size: 0,
                    digest: None,
                },
                GitHubAsset {
                    name: "Eden-Linux-v0.2.1-amd64-clang-pgo.AppImage".into(),
                    browser_download_url: "https://stable.eden-emu.dev/eden.AppImage".into(),
                    size: 0,
                    digest: None,
                },
                GitHubAsset {
                    name: "Eden-Linux-v0.2.1-aarch64-clang-pgo.AppImage".into(),
                    browser_download_url: "https://stable.eden-emu.dev/eden-arm.AppImage".into(),
                    size: 0,
                    digest: None,
                },
            ],
        };
        let metadata = SourceMetadata {
            archive_payload: "appimage".into(),
            asset_terms: vec!["eden-linux".into(), "clang".into(), "pgo".into()],
            asset_terms_by_arch: BTreeMap::from([(
                env::consts::ARCH.to_owned(),
                vec!["amd64".into()],
            )]),
            supported_arches: vec![env::consts::ARCH.to_owned()],
            release_api: "https://git.eden-emu.dev/api/v1/releases/latest".into(),
            ..SourceMetadata::default()
        };

        let selected = select_release_asset(&release, &metadata).unwrap();
        assert_eq!(selected.name, "Eden-Linux-v0.2.1-amd64-clang-pgo.AppImage");
        assert_eq!(
            source_label("appimage", &serde_json::to_string(&metadata).unwrap()),
            "Official AppImage"
        );
    }

    #[test]
    fn managed_archive_extraction_cannot_escape_the_owned_root() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("payload.zip");
        {
            let file = File::create(&archive_path).unwrap();
            let mut archive = zip::ZipWriter::new(file);
            archive
                .start_file("../escape", zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(b"bad").unwrap();
            archive.finish().unwrap();
        }
        assert!(extract_zip(&archive_path, &temp.path().join("output")).is_err());
        assert!(!temp.path().join("escape").exists());
    }

    #[test]
    fn libretro_core_download_is_host_specific_and_exact() {
        let url = libretro_core_url("mesen").unwrap();
        let (_, extension) = libretro_buildbot_target().unwrap();
        assert!(url.starts_with("https://buildbot.libretro.com/nightly/"));
        assert!(url.ends_with(&format!("/latest/mesen_libretro.{extension}.zip")));
        assert!(libretro_core_url("../mesen").is_err());
    }

    #[test]
    fn libretro_zip_extracts_only_the_expected_core() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("core.zip");
        {
            let file = File::create(&archive_path).unwrap();
            let mut archive = zip::ZipWriter::new(file);
            archive
                .start_file(
                    "nested/mesen_libretro.so",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(b"core").unwrap();
            archive
                .start_file("unrelated.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(b"ignore").unwrap();
            archive.finish().unwrap();
        }
        let target = temp.path().join("mesen_libretro.so.part");
        extract_exact_zip_member(&archive_path, "mesen_libretro.so", &target).unwrap();
        assert_eq!(fs::read(target).unwrap(), b"core");
        assert!(!temp.path().join("unrelated.txt").exists());
    }

    #[test]
    fn completed_activity_expires_but_attention_states_remain_visible() {
        let operation = EmulatorLifecycleOperation {
            id: uuid::Uuid::new_v4().to_string(),
            emulator_id: "dosbox-x".into(),
            display_name: "DOSBox-X".into(),
            host_system_slug: "linux".into(),
            manager: "flatpak".into(),
            package_id: "com.dosbox_x.DOSBox-X".into(),
            install_path: "user".into(),
            action: "update".into(),
            status: "succeeded".into(),
            detail: "Updated DOSBox-X".into(),
            started_at: 100,
            finished_at: Some(110),
        };
        assert!(lifecycle_operation_visible_at(&operation, 140));
        assert!(!lifecycle_operation_visible_at(&operation, 141));

        for status in ["failed", "interrupted", "running"] {
            let mut attention = operation.clone();
            attention.status = status.into();
            assert!(lifecycle_operation_visible_at(&attention, 10_000));
        }
    }
}
