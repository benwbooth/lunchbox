use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::database;

const HOSTS: [&str; 4] = ["linux", "linux-flatpak", "macos", "windows"];
const PURPOSES: [&str; 6] = ["saves", "states", "config", "input", "bios", "keys"];
const GAP_STATUSES: [&str; 3] = ["unsupported", "no_verified_package", "unresolved"];
const TEST_STATUSES: [&str; 5] = ["not_tested", "pass", "fail", "blocked", "not_applicable"];
const CONTROLLER_STATUSES: [&str; 3] = ["contracted", "missing_contract", "not_applicable"];
const CORE_FIRMWARE_STATUSES: [&str; 6] = [
    "required",
    "optional",
    "mixed",
    "not_required",
    "content_dependent",
    "unknown",
];
const CORE_FEATURE_STATUSES: [&str; 4] =
    ["supported", "not_supported", "content_dependent", "unknown"];
const CORE_STATE_STATUSES: [&str; 3] = ["supported", "not_supported", "unknown"];
const CHECKSUM_STATUSES: [&str; 4] = [
    "published",
    "not_published",
    "not_applicable",
    "dynamic_or_multi_variant",
];
const CORE_HOST_STATUSES: [&str; 3] = ["available", "unavailable", "unverified"];
const CAPTURE_STATUSES: [&str; 4] = ["captured", "not_supported", "not_required", "unresolved"];

fn default_capture_status() -> String {
    "captured".to_owned()
}

#[derive(Debug, Deserialize)]
struct PlatformRecord {
    slug: String,
    emulator: String,
    #[serde(default)]
    platforms: BTreeMap<String, HostRecord>,
    #[serde(default)]
    platform_gaps: BTreeMap<String, PlatformGap>,
}

#[derive(Debug, Deserialize)]
struct HostRecord {
    paths: Vec<CapturedPath>,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, Deserialize)]
struct PlatformGap {
    status: String,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CapturedPath {
    purpose: String,
    #[serde(default = "default_capture_status")]
    status: String,
    path: String,
    #[serde(default)]
    naming: String,
    #[serde(default)]
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FirmwareCatalog {
    rules: Vec<FirmwareRule>,
}

#[derive(Debug, Deserialize)]
struct FirmwareRule {
    runtime_kind: String,
    runtime_name: String,
    source_package_name: String,
}

#[derive(Debug, Deserialize)]
struct ControllerCatalog {
    emulator_profiles: Vec<ControllerProfile>,
}

#[derive(Clone, Debug, Deserialize)]
struct ControllerProfile {
    id: String,
    core: String,
    transport: String,
    status: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct RetroarchCoreRecord {
    schema_version: u32,
    core: String,
    display_name: String,
    reviewed_at: String,
    core_info_file: Option<String>,
    hosts: BTreeMap<String, CoreHost>,
    controller: CoreController,
    firmware: CoreFirmware,
    saves: CoreSaveFeature,
    states: CoreStateFeature,
}

#[derive(Debug, Deserialize)]
struct CoreHost {
    status: String,
    reason: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoreController {
    status: String,
    profile_ids: Vec<String>,
    mapping_summary: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoreFirmware {
    status: String,
    files: Vec<CoreFirmwareFile>,
    notes: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoreFirmwareFile {
    path: String,
    description: String,
    required: bool,
    checksum_status: String,
    checksums: Vec<CoreChecksum>,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoreChecksum {
    algorithm: String,
    value: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct CoreSaveFeature {
    status: String,
    extensions: Vec<String>,
    naming: String,
    notes: String,
    evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoreStateFeature {
    status: String,
    serialization: String,
    naming: String,
    notes: String,
    evidence: Vec<String>,
}

#[derive(Debug)]
struct Relationship {
    emulator_id: String,
    emulator_name: String,
    platform: String,
    cores: String,
}

#[derive(Debug, Default)]
struct RuntimeAccumulator {
    id: String,
    name: String,
    platforms: BTreeSet<String>,
    owners: BTreeSet<String>,
}

#[derive(Debug)]
struct Runtime {
    kind: &'static str,
    id: String,
    name: String,
    platforms: String,
}

#[derive(Debug, Serialize)]
struct FeatureMatrixRow {
    runtime_kind: String,
    runtime_id: String,
    runtime_name: String,
    emulated_platforms: String,
    host_os: String,
    record_slug: String,
    record_status: String,
    controller_config_status: String,
    controller_syntax_status: String,
    controller_config_path_and_syntax: String,
    controller_config_evidence: String,
    retroarch_core_record_status: String,
    retroarch_core_display_name: String,
    retroarch_core_info_file: String,
    retroarch_core_reviewed_at: String,
    retroarch_core_host_status: String,
    retroarch_core_host_reason: String,
    retroarch_core_host_evidence: String,
    controller_contract_status: String,
    controller_profile_count: usize,
    controller_profile_ids: String,
    controller_mapping_summary: String,
    controller_contract_evidence: String,
    firmware_status: String,
    firmware_locations: String,
    firmware_identity_status: String,
    firmware_checksum_status: String,
    firmware_names_and_checksums: String,
    firmware_rule_count: usize,
    firmware_rule_packages: String,
    firmware_evidence: String,
    core_firmware_status: String,
    core_firmware_requirements: String,
    core_firmware_evidence: String,
    save_status: String,
    save_locations: String,
    save_naming: String,
    core_save_status: String,
    core_save_extensions: String,
    core_save_naming: String,
    core_save_evidence: String,
    state_status: String,
    state_locations: String,
    state_naming: String,
    core_state_status: String,
    core_state_serialization: String,
    core_state_naming: String,
    core_state_evidence: String,
    host_notes: String,
    platform_evidence: String,
    controller_test_status: String,
    firmware_test_status: String,
    save_test_status: String,
    test_notes: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PriorTestRow {
    runtime_kind: String,
    runtime_id: String,
    host_os: String,
    controller_test_status: String,
    firmware_test_status: String,
    save_test_status: String,
    test_notes: String,
}

#[derive(Clone, Debug)]
struct TestStatus {
    controller: String,
    firmware: String,
    save: String,
    notes: String,
}

#[derive(Debug, Serialize)]
pub struct FeatureMatrixStats {
    pub standalone_runtimes: usize,
    pub record_only_runtimes: usize,
    pub retroarch_cores: usize,
    pub host_platforms: usize,
    pub rows: usize,
    pub captured_rows: usize,
    pub partial_rows: usize,
    pub gap_rows: usize,
    pub missing_record_rows: usize,
    pub platform_records: usize,
    pub retroarch_core_records: usize,
    pub missing_retroarch_core_records: usize,
    pub contracted_retroarch_cores: usize,
    pub missing_retroarch_controller_contracts: usize,
    pub available_retroarch_core_host_cells: usize,
    pub unavailable_retroarch_core_host_cells: usize,
    pub unverified_retroarch_core_host_cells: usize,
    pub missing_retroarch_core_host_cells: usize,
    pub captured_record_host_cells: usize,
    pub gap_record_host_cells: usize,
    pub output: PathBuf,
}

pub fn generate(
    database_path: &Path,
    records_dir: &Path,
    retroarch_core_records_dir: &Path,
    controller_catalog_path: &Path,
    firmware_rules_path: &Path,
    output: &Path,
) -> Result<FeatureMatrixStats> {
    let connection = database::open_read_only(database_path)?;
    let mut runtimes = load_runtimes(&connection)?;
    let records = load_records(records_dir)?;
    let controller_catalog: ControllerCatalog = serde_json::from_slice(
        &fs::read(controller_catalog_path)
            .with_context(|| format!("reading {}", controller_catalog_path.display()))?,
    )
    .with_context(|| format!("parsing {}", controller_catalog_path.display()))?;
    for record in records.values() {
        if record.slug != "retroarch"
            && !runtimes.iter().any(|runtime| {
                runtime.kind == "standalone" && runtime.name.eq_ignore_ascii_case(&record.emulator)
            })
        {
            runtimes.push(Runtime {
                kind: "record_only",
                id: format!("record:{}", record.slug),
                name: record.emulator.clone(),
                platforms: "not present in canonical database".to_owned(),
            });
        }
    }
    runtimes.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    let firmware: FirmwareCatalog = serde_json::from_slice(
        &fs::read(firmware_rules_path)
            .with_context(|| format!("reading {}", firmware_rules_path.display()))?,
    )
    .with_context(|| format!("parsing {}", firmware_rules_path.display()))?;
    let retroarch_runtime_ids = runtimes
        .iter()
        .filter(|runtime| runtime.kind == "retroarch")
        .map(|runtime| runtime.name.as_str())
        .collect::<BTreeSet<_>>();
    let controller_profiles = load_controller_profiles(&controller_catalog)?;
    let retroarch_core_records = load_retroarch_core_records(
        retroarch_core_records_dir,
        &retroarch_runtime_ids,
        &controller_profiles,
    )?;

    let standalone_runtimes = runtimes
        .iter()
        .filter(|runtime| runtime.kind == "standalone")
        .count();
    let record_only_runtimes = runtimes
        .iter()
        .filter(|runtime| runtime.kind == "record_only")
        .count();
    let retroarch_cores = runtimes
        .iter()
        .filter(|runtime| runtime.kind == "retroarch")
        .count();
    let expected_rows = runtimes.len() * HOSTS.len();
    let prior_tests = load_prior_tests(output)?;
    let mut rows = Vec::with_capacity(expected_rows);
    for runtime in &runtimes {
        let record = if runtime.kind == "retroarch" {
            records.get("retroarch")
        } else {
            records
                .values()
                .find(|record| record.emulator.eq_ignore_ascii_case(&runtime.name))
        };
        for host in HOSTS {
            let key = (runtime.kind.to_owned(), runtime.id.clone(), host.to_owned());
            let core_record = (runtime.kind == "retroarch")
                .then(|| retroarch_core_records.get(runtime.name.as_str()))
                .flatten();
            let core_profiles = (runtime.kind == "retroarch")
                .then(|| controller_profiles.get(runtime.name.as_str()))
                .flatten();
            rows.push(matrix_row(
                runtime,
                host,
                record,
                core_record,
                core_profiles,
                &firmware.rules,
                prior_tests.get(&key),
            ));
        }
    }
    ensure!(
        rows.len() == expected_rows,
        "feature matrix row count drifted"
    );

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let temporary = output.with_file_name(format!(
        ".{}.building",
        output
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("emulator-feature-matrix.csv")
    ));
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_path(&temporary)
        .with_context(|| format!("creating {}", temporary.display()))?;
    for row in &rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    if output.exists() {
        fs::remove_file(output).with_context(|| format!("replacing {}", output.display()))?;
    }
    fs::rename(&temporary, output).with_context(|| format!("installing {}", output.display()))?;

    let captured_rows = rows
        .iter()
        .filter(|row| row.record_status == "captured")
        .count();
    let partial_rows = rows
        .iter()
        .filter(|row| row.record_status == "partial")
        .count();
    let gap_rows = rows
        .iter()
        .filter(|row| {
            matches!(
                row.record_status.as_str(),
                "unsupported" | "no_verified_package" | "unresolved"
            )
        })
        .count();
    let missing_record_rows = rows
        .iter()
        .filter(|row| row.record_status == "missing_record")
        .count();
    let captured_record_host_cells = records.values().map(|record| record.platforms.len()).sum();
    let gap_record_host_cells = records
        .values()
        .map(|record| record.platform_gaps.len())
        .sum();
    let contracted_retroarch_cores = retroarch_runtime_ids
        .iter()
        .filter(|core| {
            controller_profiles
                .get(**core)
                .is_some_and(|profiles| !profiles.is_empty())
        })
        .count();
    let available_retroarch_core_host_cells = rows
        .iter()
        .filter(|row| row.retroarch_core_host_status == "available")
        .count();
    let unavailable_retroarch_core_host_cells = rows
        .iter()
        .filter(|row| row.retroarch_core_host_status == "unavailable")
        .count();
    let unverified_retroarch_core_host_cells = rows
        .iter()
        .filter(|row| row.retroarch_core_host_status == "unverified")
        .count();
    let missing_retroarch_core_host_cells = rows
        .iter()
        .filter(|row| row.retroarch_core_host_status == "missing_core_record")
        .count();
    Ok(FeatureMatrixStats {
        standalone_runtimes,
        record_only_runtimes,
        retroarch_cores,
        host_platforms: HOSTS.len(),
        rows: rows.len(),
        captured_rows,
        partial_rows,
        gap_rows,
        missing_record_rows,
        platform_records: records.len(),
        retroarch_core_records: retroarch_core_records.len(),
        missing_retroarch_core_records: retroarch_cores - retroarch_core_records.len(),
        contracted_retroarch_cores,
        missing_retroarch_controller_contracts: retroarch_cores - contracted_retroarch_cores,
        available_retroarch_core_host_cells,
        unavailable_retroarch_core_host_cells,
        unverified_retroarch_core_host_cells,
        missing_retroarch_core_host_cells,
        captured_record_host_cells,
        gap_record_host_cells,
        output: output.to_path_buf(),
    })
}

fn load_prior_tests(output: &Path) -> Result<BTreeMap<(String, String, String), TestStatus>> {
    if !output.is_file() {
        return Ok(BTreeMap::new());
    }
    let mut reader = csv::Reader::from_path(output)
        .with_context(|| format!("reading prior test statuses from {}", output.display()))?;
    let mut statuses = BTreeMap::new();
    for row in reader.deserialize::<PriorTestRow>() {
        let row = row?;
        for (feature, status) in [
            ("controller", row.controller_test_status.as_str()),
            ("firmware", row.firmware_test_status.as_str()),
            ("save", row.save_test_status.as_str()),
        ] {
            ensure!(
                TEST_STATUSES.contains(&status),
                "prior feature matrix has unknown {feature} test status {status}"
            );
        }
        let key = (row.runtime_kind, row.runtime_id, row.host_os);
        ensure!(
            statuses
                .insert(
                    key,
                    TestStatus {
                        controller: row.controller_test_status,
                        firmware: row.firmware_test_status,
                        save: row.save_test_status,
                        notes: row.test_notes,
                    },
                )
                .is_none(),
            "prior feature matrix contains a duplicate runtime/host row"
        );
    }
    Ok(statuses)
}

fn load_records(directory: &Path) -> Result<BTreeMap<String, PlatformRecord>> {
    let mut paths = fs::read_dir(directory)
        .with_context(|| format!("reading {}", directory.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut records = BTreeMap::new();
    let mut emulator_names = BTreeSet::new();
    for path in paths {
        let record: PlatformRecord = serde_json::from_slice(
            &fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
        )
        .with_context(|| format!("parsing {}", path.display()))?;
        let filename = path.file_stem().and_then(|name| name.to_str());
        ensure!(
            filename == Some(record.slug.as_str()),
            "{} has a mismatched slug",
            path.display()
        );
        ensure!(
            emulator_names.insert(record.emulator.to_lowercase()),
            "{} duplicates emulator identity {}",
            path.display(),
            record.emulator
        );
        for host in record.platforms.keys() {
            ensure!(
                HOSTS.contains(&host.as_str()),
                "{} has unknown host {host}",
                path.display()
            );
            ensure!(
                !record.platform_gaps.contains_key(host),
                "{} marks {host} as both captured and a gap",
                path.display()
            );
        }
        for (host, entry) in &record.platforms {
            ensure!(
                !entry.paths.is_empty(),
                "{} has no paths for {host}",
                path.display()
            );
            let mut purposes = BTreeSet::new();
            for captured in &entry.paths {
                ensure!(
                    PURPOSES.contains(&captured.purpose.as_str()),
                    "{} has unknown purpose {} for {host}",
                    path.display(),
                    captured.purpose
                );
                ensure!(
                    CAPTURE_STATUSES.contains(&captured.status.as_str()),
                    "{} has unknown {} status {} for {host}",
                    path.display(),
                    captured.purpose,
                    captured.status
                );
                ensure!(
                    purposes.insert(captured.purpose.as_str()),
                    "{} duplicates purpose {} for {host}",
                    path.display(),
                    captured.purpose
                );
                ensure!(
                    !captured.path.trim().is_empty() && !captured.evidence.is_empty(),
                    "{} has an incomplete {} capture for {host}",
                    path.display(),
                    captured.purpose
                );
            }
            for (feature, alternatives) in [
                ("controller configuration", &["config", "input"][..]),
                ("firmware/keys", &["bios", "keys"][..]),
                ("persistent saves", &["saves"][..]),
                ("save states", &["states"][..]),
            ] {
                ensure!(
                    alternatives
                        .iter()
                        .any(|purpose| purposes.contains(purpose)),
                    "{} has no {feature} disposition for captured host {host}",
                    path.display()
                );
            }
        }
        for (host, gap) in &record.platform_gaps {
            ensure!(
                HOSTS.contains(&host.as_str()),
                "{} has unknown gap host {host}",
                path.display()
            );
            ensure!(
                GAP_STATUSES.contains(&gap.status.as_str()),
                "{} has unknown gap status {} for {host}",
                path.display(),
                gap.status
            );
            ensure!(
                !gap.reason.trim().is_empty() && !gap.evidence.is_empty(),
                "{} has an incomplete gap disposition for {host}",
                path.display()
            );
        }
        for host in HOSTS {
            ensure!(
                record.platforms.contains_key(host) || record.platform_gaps.contains_key(host),
                "{} has neither a capture nor gap disposition for {host}",
                path.display()
            );
        }
        ensure!(
            records.insert(record.slug.clone(), record).is_none(),
            "duplicate platform-record slug"
        );
    }
    Ok(records)
}

fn load_controller_profiles(
    catalog: &ControllerCatalog,
) -> Result<BTreeMap<String, Vec<ControllerProfile>>> {
    let mut profiles = BTreeMap::<String, Vec<ControllerProfile>>::new();
    let mut ids = BTreeSet::new();
    for profile in &catalog.emulator_profiles {
        ensure!(
            ids.insert(profile.id.as_str()),
            "duplicate controller profile {}",
            profile.id
        );
        if profile.transport != "retropad" {
            continue;
        }
        ensure!(
            !profile.status.trim().is_empty() && !profile.source.trim().is_empty(),
            "RetroArch controller profile {} lacks status/source evidence",
            profile.id
        );
        profiles
            .entry(canonical_core(&profile.core).to_owned())
            .or_default()
            .push(profile.clone());
    }
    for entries in profiles.values_mut() {
        entries.sort_by(|left, right| left.id.cmp(&right.id));
    }
    Ok(profiles)
}

fn load_retroarch_core_records(
    directory: &Path,
    expected_cores: &BTreeSet<&str>,
    controller_profiles: &BTreeMap<String, Vec<ControllerProfile>>,
) -> Result<BTreeMap<String, RetroarchCoreRecord>> {
    let mut paths = fs::read_dir(directory)
        .with_context(|| format!("reading {}", directory.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut records = BTreeMap::new();
    for path in paths {
        let record: RetroarchCoreRecord = serde_json::from_slice(
            &fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
        )
        .with_context(|| format!("parsing {}", path.display()))?;
        ensure!(
            record.schema_version == 1,
            "{} has unsupported schema_version",
            path.display()
        );
        ensure!(
            path.file_stem().and_then(|name| name.to_str()) == Some(record.core.as_str()),
            "{} filename does not match core {}",
            path.display(),
            record.core
        );
        ensure!(
            expected_cores.contains(record.core.as_str()),
            "{} is not a canonical RetroArch core in the database",
            record.core
        );
        ensure!(
            !record.display_name.trim().is_empty()
                && !record.reviewed_at.trim().is_empty()
                && record
                    .core_info_file
                    .as_deref()
                    .is_none_or(|file| file.ends_with("_libretro.info")),
            "{} lacks display/review/core-info identity",
            path.display()
        );
        ensure!(
            record
                .hosts
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == HOSTS.into_iter().collect::<BTreeSet<_>>(),
            "{} must disposition exactly the four matrix hosts",
            path.display()
        );
        for (host, availability) in &record.hosts {
            ensure!(
                CORE_HOST_STATUSES.contains(&availability.status.as_str())
                    && !availability.reason.trim().is_empty()
                    && !availability.evidence.is_empty(),
                "{} has incomplete availability evidence for {host}",
                path.display()
            );
        }
        validate_core_controller(&record, controller_profiles, &path)?;
        validate_core_firmware(&record, &path)?;
        validate_core_features(&record, &path)?;
        ensure!(
            records.insert(record.core.clone(), record).is_none(),
            "duplicate RetroArch core record"
        );
    }
    Ok(records)
}

fn validate_core_controller(
    record: &RetroarchCoreRecord,
    profiles: &BTreeMap<String, Vec<ControllerProfile>>,
    path: &Path,
) -> Result<()> {
    ensure!(
        CONTROLLER_STATUSES.contains(&record.controller.status.as_str()),
        "{} has unknown controller status {}",
        path.display(),
        record.controller.status
    );
    ensure!(
        !record.controller.mapping_summary.trim().is_empty()
            && !record.controller.evidence.is_empty(),
        "{} has incomplete controller evidence",
        path.display()
    );
    let expected = profiles
        .get(record.core.as_str())
        .into_iter()
        .flat_map(|profiles| profiles.iter().map(|profile| profile.id.as_str()))
        .collect::<BTreeSet<_>>();
    let captured = record
        .controller
        .profile_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    ensure!(
        captured.len() == record.controller.profile_ids.len(),
        "{} duplicates a controller profile id",
        path.display()
    );
    ensure!(
        captured == expected,
        "{} controller profile ids do not match the controller catalog",
        path.display()
    );
    let expected_status = if expected.is_empty() {
        "missing_contract"
    } else {
        "contracted"
    };
    ensure!(
        record.controller.status == expected_status,
        "{} controller status must be {expected_status}",
        path.display()
    );
    Ok(())
}

fn validate_core_firmware(record: &RetroarchCoreRecord, path: &Path) -> Result<()> {
    ensure!(
        CORE_FIRMWARE_STATUSES.contains(&record.firmware.status.as_str()),
        "{} has unknown firmware status {}",
        path.display(),
        record.firmware.status
    );
    ensure!(
        !record.firmware.notes.trim().is_empty() && !record.firmware.evidence.is_empty(),
        "{} has incomplete firmware evidence",
        path.display()
    );
    if matches!(
        record.firmware.status.as_str(),
        "required" | "optional" | "mixed" | "content_dependent"
    ) {
        ensure!(
            !record.firmware.files.is_empty(),
            "{} firmware status requires at least one file",
            path.display()
        );
    }
    if record.firmware.status == "not_required" {
        ensure!(
            record.firmware.files.is_empty(),
            "{} marks firmware not_required but lists files",
            path.display()
        );
    }
    let mut firmware_paths = BTreeSet::new();
    for file in &record.firmware.files {
        ensure!(
            firmware_paths.insert(file.path.as_str()),
            "{} duplicates firmware path {}",
            path.display(),
            file.path
        );
        ensure!(
            !file.path.trim().is_empty()
                && !file.description.trim().is_empty()
                && !file.evidence.is_empty()
                && CHECKSUM_STATUSES.contains(&file.checksum_status.as_str()),
            "{} has an incomplete firmware file {}",
            path.display(),
            file.path
        );
        if file.checksum_status == "published" {
            ensure!(
                !file.checksums.is_empty(),
                "{} marks {} checksum published without a digest",
                path.display(),
                file.path
            );
        }
        if matches!(
            file.checksum_status.as_str(),
            "not_published" | "not_applicable"
        ) {
            ensure!(
                file.checksums.is_empty(),
                "{} gives {} a digest despite checksum status {}",
                path.display(),
                file.path,
                file.checksum_status
            );
        }
        for checksum in &file.checksums {
            let expected_length = match checksum.algorithm.as_str() {
                "crc32" => 8,
                "md5" => 32,
                "sha1" => 40,
                "sha256" => 64,
                other => bail!("{} has unknown checksum algorithm {other}", path.display()),
            };
            ensure!(
                checksum.value.len() == expected_length
                    && checksum
                        .value
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
                    && !checksum.source.trim().is_empty(),
                "{} has invalid {} digest for {}",
                path.display(),
                checksum.algorithm,
                file.path
            );
        }
    }
    Ok(())
}

fn validate_core_features(record: &RetroarchCoreRecord, path: &Path) -> Result<()> {
    ensure!(
        CORE_FEATURE_STATUSES.contains(&record.saves.status.as_str())
            && !record.saves.naming.trim().is_empty()
            && !record.saves.notes.trim().is_empty()
            && !record.saves.evidence.is_empty(),
        "{} has incomplete save semantics",
        path.display()
    );
    ensure!(
        record
            .saves
            .extensions
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            == record.saves.extensions.len(),
        "{} duplicates a save extension",
        path.display()
    );
    if record.saves.status == "supported" {
        ensure!(
            !record.saves.extensions.is_empty(),
            "{} marks persistent saves supported without an artifact extension",
            path.display()
        );
    }
    if matches!(record.saves.status.as_str(), "not_supported" | "unknown") {
        ensure!(
            record.saves.extensions.is_empty(),
            "{} lists save extensions despite {} save status",
            path.display(),
            record.saves.status
        );
    }
    ensure!(
        CORE_STATE_STATUSES.contains(&record.states.status.as_str())
            && !record.states.serialization.trim().is_empty()
            && !record.states.naming.trim().is_empty()
            && !record.states.notes.trim().is_empty()
            && !record.states.evidence.is_empty(),
        "{} has incomplete state semantics",
        path.display()
    );
    Ok(())
}

fn load_runtimes(connection: &Connection) -> Result<Vec<Runtime>> {
    let mut statement = connection.prepare(
        "SELECT e.id, e.name, p.canonical_name, ep.core_name
         FROM emulator_platforms ep
         JOIN emulators e ON e.id=ep.emulator_id
         JOIN platforms p ON p.id=ep.platform_id
         ORDER BY e.name COLLATE NOCASE, p.canonical_name COLLATE NOCASE",
    )?;
    let relationships = statement
        .query_map([], |row| {
            Ok(Relationship {
                emulator_id: row.get(0)?,
                emulator_name: row.get(1)?,
                platform: row.get(2)?,
                cores: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut standalone = BTreeMap::<String, RuntimeAccumulator>::new();
    let mut cores = BTreeMap::<String, RuntimeAccumulator>::new();
    for relationship in relationships {
        let runtime = standalone
            .entry(relationship.emulator_id.clone())
            .or_insert_with(|| RuntimeAccumulator {
                id: relationship.emulator_id.clone(),
                name: relationship.emulator_name.clone(),
                ..RuntimeAccumulator::default()
            });
        runtime.platforms.insert(relationship.platform.clone());
        for core in relationship
            .cores
            .split(';')
            .map(str::trim)
            .filter(|core| !core.is_empty())
        {
            let core = canonical_core(core).to_owned();
            let runtime = cores
                .entry(core.clone())
                .or_insert_with(|| RuntimeAccumulator {
                    id: format!("retroarch:{core}"),
                    name: core,
                    ..RuntimeAccumulator::default()
                });
            runtime.platforms.insert(relationship.platform.clone());
            runtime.owners.insert(relationship.emulator_name.clone());
        }
    }

    let mut runtimes = standalone
        .into_values()
        .map(|runtime| Runtime {
            kind: "standalone",
            id: runtime.id,
            name: runtime.name,
            platforms: runtime
                .platforms
                .into_iter()
                .collect::<Vec<_>>()
                .join(" | "),
        })
        .collect::<Vec<_>>();
    runtimes.extend(
        cores
            .into_values()
            .filter(|runtime| {
                runtime
                    .owners
                    .iter()
                    .any(|owner| !owner.eq_ignore_ascii_case("BizHawk"))
            })
            .map(|runtime| Runtime {
                kind: "retroarch",
                id: runtime.id,
                name: runtime.name,
                platforms: runtime
                    .platforms
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" | "),
            }),
    );
    runtimes.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(runtimes)
}

fn matrix_row(
    runtime: &Runtime,
    host: &str,
    record: Option<&PlatformRecord>,
    core_record: Option<&RetroarchCoreRecord>,
    core_profiles: Option<&Vec<ControllerProfile>>,
    firmware_rules: &[FirmwareRule],
    prior_test: Option<&TestStatus>,
) -> FeatureMatrixRow {
    let host_record = record.and_then(|record| record.platforms.get(host));
    let platform_gap = record.and_then(|record| record.platform_gaps.get(host));
    let config = captured(host_record, &["config", "input"]);
    let firmware = captured(host_record, &["bios", "keys"]);
    let saves = captured(host_record, &["saves"]);
    let states = captured(host_record, &["states"]);
    let matching_rules = firmware_rules
        .iter()
        .filter(|rule| firmware_rule_matches(runtime, rule))
        .collect::<Vec<_>>();
    let packages = matching_rules
        .iter()
        .map(|rule| rule.source_package_name.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" | ");
    let names = join_field(&firmware, |path| path.naming.as_str());
    let firmware_names_and_checksums = if firmware.is_empty() {
        String::new()
    } else if names.is_empty() {
        "identity/checksum disposition is absent; inspect location and evidence".to_owned()
    } else {
        format!("{names} | record identity/checksum disposition is prose, not machine-structured")
    };
    let record_status = match (record, host_record, platform_gap) {
        (None, _, _) => "missing_record",
        (Some(_), Some(host), _) if host.paths.iter().any(|path| path.status == "unresolved") => {
            "partial"
        }
        (Some(_), Some(_), _) => "captured",
        (Some(_), None, Some(gap)) => gap.status.as_str(),
        (Some(_), None, None) => "missing_host_disposition",
    };
    let controller_status = feature_status(record, host_record, platform_gap, &config);
    let firmware_status = feature_status(record, host_record, platform_gap, &firmware);
    let core_record_status = if runtime.kind != "retroarch" {
        "not_applicable"
    } else if core_record.is_some() {
        "captured"
    } else {
        "missing_core_record"
    };
    let controller_contract_status = if runtime.kind != "retroarch" {
        "not_applicable"
    } else if core_profiles.is_some_and(|profiles| !profiles.is_empty()) {
        "contracted"
    } else {
        "missing_contract"
    };
    let core_host = core_record.and_then(|record| record.hosts.get(host));
    FeatureMatrixRow {
        runtime_kind: runtime.kind.to_owned(),
        runtime_id: runtime.id.clone(),
        runtime_name: runtime.name.clone(),
        emulated_platforms: runtime.platforms.clone(),
        host_os: host.to_owned(),
        record_slug: record.map(|record| record.slug.clone()).unwrap_or_default(),
        record_status: record_status.to_owned(),
        controller_config_status: controller_status.clone(),
        controller_syntax_status: if controller_status == "captured" {
            "captured_in_prose"
        } else {
            controller_status.as_str()
        }
        .to_owned(),
        controller_config_path_and_syntax: join_path_and_naming(&config),
        controller_config_evidence: join_evidence(&config),
        retroarch_core_record_status: core_record_status.to_owned(),
        retroarch_core_display_name: core_record
            .map(|record| record.display_name.clone())
            .unwrap_or_default(),
        retroarch_core_info_file: core_record
            .and_then(|record| record.core_info_file.clone())
            .unwrap_or_default(),
        retroarch_core_reviewed_at: core_record
            .map(|record| record.reviewed_at.clone())
            .unwrap_or_default(),
        retroarch_core_host_status: core_host
            .map(|availability| availability.status.clone())
            .unwrap_or_else(|| core_dimension_status(runtime)),
        retroarch_core_host_reason: core_host
            .map(|availability| availability.reason.clone())
            .unwrap_or_default(),
        retroarch_core_host_evidence: core_host
            .map(|availability| availability.evidence.join(" | "))
            .unwrap_or_default(),
        controller_contract_status: controller_contract_status.to_owned(),
        controller_profile_count: core_profiles.map_or(0, Vec::len),
        controller_profile_ids: core_profiles
            .into_iter()
            .flat_map(|profiles| profiles.iter().map(|profile| profile.id.as_str()))
            .collect::<Vec<_>>()
            .join(" | "),
        controller_mapping_summary: core_record
            .map(|record| record.controller.mapping_summary.clone())
            .unwrap_or_default(),
        controller_contract_evidence: core_controller_evidence(core_record, core_profiles),
        firmware_status: firmware_status.clone(),
        firmware_locations: join_field(&firmware, |path| path.path.as_str()),
        firmware_identity_status: if firmware_status == "captured" {
            if names.is_empty() {
                "prose_only"
            } else {
                "captured_in_naming"
            }
        } else {
            firmware_status.as_str()
        }
        .to_owned(),
        firmware_checksum_status: if firmware_status == "captured" {
            "captured_in_prose_unstructured"
        } else {
            firmware_status.as_str()
        }
        .to_owned(),
        firmware_names_and_checksums,
        firmware_rule_count: matching_rules.len(),
        firmware_rule_packages: packages,
        firmware_evidence: join_evidence(&firmware),
        core_firmware_status: core_record
            .map(|record| record.firmware.status.clone())
            .unwrap_or_else(|| core_dimension_status(runtime)),
        core_firmware_requirements: core_record
            .map(format_core_firmware_requirements)
            .unwrap_or_default(),
        core_firmware_evidence: core_record.map(core_firmware_evidence).unwrap_or_default(),
        save_status: feature_status(record, host_record, platform_gap, &saves),
        save_locations: join_field(&saves, |path| path.path.as_str()),
        save_naming: join_field(&saves, |path| path.naming.as_str()),
        core_save_status: core_record
            .map(|record| record.saves.status.clone())
            .unwrap_or_else(|| core_dimension_status(runtime)),
        core_save_extensions: core_record
            .map(|record| record.saves.extensions.join(" | "))
            .unwrap_or_default(),
        core_save_naming: core_record
            .map(|record| record.saves.naming.clone())
            .unwrap_or_default(),
        core_save_evidence: core_record
            .map(|record| record.saves.evidence.join(" | "))
            .unwrap_or_default(),
        state_status: feature_status(record, host_record, platform_gap, &states),
        state_locations: join_field(&states, |path| path.path.as_str()),
        state_naming: join_field(&states, |path| path.naming.as_str()),
        core_state_status: core_record
            .map(|record| record.states.status.clone())
            .unwrap_or_else(|| core_dimension_status(runtime)),
        core_state_serialization: core_record
            .map(|record| record.states.serialization.clone())
            .unwrap_or_default(),
        core_state_naming: core_record
            .map(|record| record.states.naming.clone())
            .unwrap_or_default(),
        core_state_evidence: core_record
            .map(|record| record.states.evidence.join(" | "))
            .unwrap_or_default(),
        host_notes: host_record
            .map(|entry| entry.notes.clone())
            .or_else(|| platform_gap.map(|gap| gap.reason.clone()))
            .unwrap_or_default(),
        platform_evidence: platform_gap
            .map(|gap| gap.evidence.join(" | "))
            .unwrap_or_default(),
        controller_test_status: prior_test
            .map(|status| status.controller.clone())
            .unwrap_or_else(|| "not_tested".to_owned()),
        firmware_test_status: prior_test
            .map(|status| status.firmware.clone())
            .unwrap_or_else(|| "not_tested".to_owned()),
        save_test_status: prior_test
            .map(|status| status.save.clone())
            .unwrap_or_else(|| "not_tested".to_owned()),
        test_notes: prior_test
            .map(|status| status.notes.clone())
            .unwrap_or_default(),
    }
}

fn core_dimension_status(runtime: &Runtime) -> String {
    if runtime.kind == "retroarch" {
        "missing_core_record"
    } else {
        "not_applicable"
    }
    .to_owned()
}

fn core_controller_evidence(
    record: Option<&RetroarchCoreRecord>,
    profiles: Option<&Vec<ControllerProfile>>,
) -> String {
    let mut evidence = BTreeSet::new();
    if let Some(record) = record {
        evidence.extend(record.controller.evidence.iter().map(String::as_str));
    }
    if let Some(profiles) = profiles {
        evidence.extend(profiles.iter().map(|profile| profile.source.as_str()));
    }
    evidence.into_iter().collect::<Vec<_>>().join(" | ")
}

fn format_core_firmware_requirements(record: &RetroarchCoreRecord) -> String {
    if record.firmware.files.is_empty() {
        return record.firmware.notes.clone();
    }
    record
        .firmware
        .files
        .iter()
        .map(|file| {
            let requirement = if file.required {
                "required"
            } else {
                "optional"
            };
            let checksums = if file.checksums.is_empty() {
                file.checksum_status.clone()
            } else {
                file.checksums
                    .iter()
                    .map(|checksum| format!("{}={}", checksum.algorithm, checksum.value))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            format!(
                "{} [{requirement}; {checksums}; {}]",
                file.path, file.description
            )
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

fn core_firmware_evidence(record: &RetroarchCoreRecord) -> String {
    let mut evidence = record
        .firmware
        .evidence
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for file in &record.firmware.files {
        evidence.extend(file.evidence.iter().map(String::as_str));
        evidence.extend(
            file.checksums
                .iter()
                .map(|checksum| checksum.source.as_str()),
        );
    }
    evidence.into_iter().collect::<Vec<_>>().join(" | ")
}

fn captured<'a>(host: Option<&'a HostRecord>, purposes: &[&str]) -> Vec<&'a CapturedPath> {
    host.into_iter()
        .flat_map(|entry| &entry.paths)
        .filter(|path| purposes.contains(&path.purpose.as_str()))
        .collect()
}

fn feature_status(
    record: Option<&PlatformRecord>,
    host: Option<&HostRecord>,
    gap: Option<&PlatformGap>,
    paths: &[&CapturedPath],
) -> String {
    if record.is_none() {
        "missing_record"
    } else if let Some(gap) = gap {
        gap.status.as_str()
    } else if host.is_none() {
        "missing_host_disposition"
    } else if paths.is_empty() {
        "not_supported_or_not_captured"
    } else if paths.iter().any(|path| path.status == "unresolved") {
        "unresolved"
    } else if paths.iter().any(|path| path.status == "captured") {
        "captured"
    } else if paths.iter().any(|path| path.status == "not_supported") {
        "not_supported"
    } else if paths.iter().any(|path| path.status == "not_required") {
        "not_required"
    } else {
        "unresolved"
    }
    .to_owned()
}

fn join_field(paths: &[&CapturedPath], field: impl Fn(&CapturedPath) -> &str) -> String {
    paths
        .iter()
        .map(|path| field(path).trim())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" || ")
}

fn join_evidence(paths: &[&CapturedPath]) -> String {
    paths
        .iter()
        .flat_map(|path| &path.evidence)
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" | ")
}

fn join_path_and_naming(paths: &[&CapturedPath]) -> String {
    paths
        .iter()
        .map(|path| {
            let location = path.path.trim();
            let syntax = path.naming.trim();
            if syntax.is_empty() {
                location.to_owned()
            } else {
                format!("{location} [syntax/naming: {syntax}]")
            }
        })
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(" || ")
}

fn firmware_rule_matches(runtime: &Runtime, rule: &FirmwareRule) -> bool {
    if runtime.kind == "retroarch" {
        rule.runtime_kind == "retroarch"
            && canonical_core(&rule.runtime_name).eq_ignore_ascii_case(&runtime.name)
    } else {
        rule.runtime_kind != "retroarch" && rule.runtime_name.eq_ignore_ascii_case(&runtime.name)
    }
}

fn canonical_core(core: &str) -> &str {
    match core {
        "beetle_cygne" => "mednafen_wswan",
        "beetle_lynx" => "mednafen_lynx",
        "beetle_ngp" => "mednafen_ngp",
        "beetle_pce_fast" => "mednafen_pce_fast",
        "beetle_supergrafx" => "mednafen_supergrafx",
        "beetle_psx" => "mednafen_psx",
        "beetle_psx_hw" => "mednafen_psx_hw",
        "beetle_vb" => "mednafen_vb",
        _ => core,
    }
}
