//! Unified per-runtime, per-host adapter descriptors for the three adapter
//! dimensions: controller configuration/input syntax and location,
//! BIOS/firmware/keys location, and save-state/save-game location and naming.
//!
//! The descriptors are purely data-driven from the embedded platform records
//! and contain no host syscalls, so they resolve identically on Linux, macOS,
//! and Windows for any named host. Callers that touch the local filesystem
//! must select the compile host's own platform; everything else (dry-run UI,
//! save-sync planning, the cross-platform test suite) may reason about any
//! host from any build.
//!
//! Native launch adapters remain authoritative where they exist and may
//! override these locations at launch time.
use anyhow::{Context, Result};

use crate::platform_locations::{
    LocationBases, Purpose, Record, SaveLocation, SaveSyncModel, adapter_locations_for_platform,
    load_records, platform_gap_status, slug_for_emulator_name, sync_model,
};

include!(concat!(env!("OUT_DIR"), "/retroarch_core_records.rs"));

/// Matrix host variants, matching `host_os` in the feature matrix and the
/// runtime-test-results ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterHost {
    Linux,
    LinuxFlatpak,
    Macos,
    Windows,
}

impl AdapterHost {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::LinuxFlatpak => "linux-flatpak",
            Self::Macos => "macos",
            Self::Windows => "windows",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "linux" => Self::Linux,
            "linux-flatpak" => Self::LinuxFlatpak,
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            _ => return None,
        })
    }

    /// The host this binary was compiled for. Flatpak detection is a runtime
    /// property of the launch target, not the compile host, so Linux builds
    /// report [`AdapterHost::Linux`].
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Linux, Self::LinuxFlatpak, Self::Macos, Self::Windows]
    }
}

/// One runtime on one host: every adapter dimension with its documented
/// capture, machine-resolvable location (when the capture permits one),
/// naming convention, and evidence.
#[derive(Clone, Debug)]
pub struct RuntimeAdapter {
    /// Record slug; the stable sync namespace, never a display label.
    pub slug: String,
    pub emulator: String,
    pub host: AdapterHost,
    /// Source-capture disposition for this host: `captured`, `partial`, or a
    /// platform-gap status (`unsupported`, `no_verified_package`,
    /// `unresolved`, `missing_record`).
    pub record_status: String,
    /// Gap reason when the host has no platform record entry.
    pub gap_reason: Option<String>,
    /// Controller mapping: `config` + `input` purposes (syntax + location).
    pub controller: Vec<SaveLocation>,
    /// Firmware: `bios` + `keys` purposes (location + file identity).
    pub firmware: Vec<SaveLocation>,
    /// Future save sync: `saves` + `states` purposes (location + naming).
    pub saves: Vec<SaveLocation>,
    pub sync_model: SaveSyncModel,
}

impl RuntimeAdapter {
    /// Dimensions with a machine-resolvable location on this host.
    pub fn resolved<'a>(&'a self, purpose: Purpose) -> impl Iterator<Item = &'a SaveLocation> {
        self.controller
            .iter()
            .chain(&self.firmware)
            .chain(&self.saves)
            .filter(move |location| location.purpose == purpose && location.resolved.is_some())
    }

    /// Whether every captured dimension on this host machine-resolves.
    /// Documentation-only captures stay visible in the descriptor; they just
    /// cannot drive file access.
    pub fn fully_resolved(&self) -> bool {
        self.controller
            .iter()
            .chain(&self.firmware)
            .chain(&self.saves)
            .filter(|location| location.status == "captured")
            .all(|location| location.resolved.is_some())
    }
}

/// Build the adapter descriptor for one record slug on one host.
pub fn adapter_for_slug(
    records: &[Record],
    slug: &str,
    host: AdapterHost,
    bases: &LocationBases,
) -> Result<RuntimeAdapter> {
    let record = records
        .iter()
        .find(|record| record.slug() == slug)
        .context("no captured platform record for this emulator")?;
    let platform = host.as_str();
    let gap = platform_gap_status(records, slug, platform);
    let mut controller = adapter_locations_for_platform(
        records,
        slug,
        &[Purpose::Config, Purpose::Input],
        platform,
        bases,
    );
    let mut firmware = adapter_locations_for_platform(
        records,
        slug,
        &[Purpose::Bios, Purpose::Keys],
        platform,
        bases,
    );
    let mut saves = adapter_locations_for_platform(
        records,
        slug,
        &[Purpose::Saves, Purpose::States],
        platform,
        bases,
    );
    controller.sort_by(|a, b| a.purpose.as_str().cmp(b.purpose.as_str()));
    firmware.sort_by(|a, b| a.purpose.as_str().cmp(b.purpose.as_str()));
    saves.sort_by(|a, b| a.purpose.as_str().cmp(b.purpose.as_str()));
    // Record status mirrors the matrix generator: a present host entry is
    // `captured` unless some dimension is explicitly `unresolved`; a missing
    // entry reports the platform-gap status (or `missing_record`).
    let (record_status, gap_reason) = if record.has_platform(platform) {
        let unresolved = controller
            .iter()
            .chain(&firmware)
            .chain(&saves)
            .any(|location| location.status == "unresolved");
        if unresolved {
            ("partial".to_owned(), None)
        } else {
            ("captured".to_owned(), None)
        }
    } else if let Some((status, reason)) = gap {
        (status, Some(reason))
    } else {
        ("missing_record".to_owned(), None)
    };
    Ok(RuntimeAdapter {
        slug: record.slug().to_owned(),
        emulator: record.emulator().to_owned(),
        host,
        record_status,
        gap_reason,
        controller,
        firmware,
        saves,
        sync_model: sync_model(slug).0,
    })
}

/// Build the adapter descriptor addressing the emulator by display name.
pub fn adapter_for_emulator(
    records: &[Record],
    emulator_name: &str,
    host: AdapterHost,
    bases: &LocationBases,
) -> Result<RuntimeAdapter> {
    let slug = slug_for_emulator_name(records, emulator_name)?;
    adapter_for_slug(records, &slug, host, bases)
}

/// Every record slug crossed with every host that has a platform entry,
/// matching the standalone rows of the feature matrix (250 runtimes x the
/// hosts each one captures or gaps).
pub fn all_standalone_adapters(bases: &LocationBases) -> Result<Vec<RuntimeAdapter>> {
    let records = load_records()?;
    let mut adapters = Vec::new();
    for record in &records {
        // Every record crosses every host: present entries resolve their
        // dimensions while gap-only hosts stay explicit instead of vanishing.
        for host in AdapterHost::all() {
            adapters.push(adapter_for_slug(&records, record.slug(), host, bases)?);
        }
    }
    Ok(adapters)
}

/// Count descriptors with at least one machine-resolved location per
/// dimension, for progress reporting against the 1,380-row matrix.
pub fn adapter_coverage(adapters: &[RuntimeAdapter]) -> (usize, usize, usize) {
    let controller = adapters
        .iter()
        .filter(|adapter| {
            adapter
                .controller
                .iter()
                .any(|location| location.resolved.is_some())
        })
        .count();
    let firmware = adapters
        .iter()
        .filter(|adapter| {
            adapter
                .firmware
                .iter()
                .any(|location| location.resolved.is_some())
        })
        .count();
    let saves = adapters
        .iter()
        .filter(|adapter| {
            adapter
                .saves
                .iter()
                .any(|location| location.resolved.is_some())
        })
        .count();
    (controller, firmware, saves)
}

/// One firmware file a RetroArch core expects, with published checksums
/// when the core record carries them.
#[derive(Clone, Debug)]
pub struct CoreFirmwareFile {
    pub path: String,
    pub description: String,
    pub required: bool,
    pub checksum_status: String,
    pub checksums: Vec<(String, String)>,
}

/// One RetroArch core on one host: the shared frontend descriptor plus the
/// core's own controller contract, firmware files, and save/state naming.
#[derive(Clone, Debug)]
pub struct CoreAdapter {
    pub core: String,
    pub display_name: String,
    pub host: AdapterHost,
    /// Core availability on this host: `available`, `unavailable`, or
    /// `unverified`, with the record's reason.
    pub host_status: String,
    pub host_reason: String,
    pub controller_status: String,
    pub profile_ids: Vec<String>,
    pub mapping_summary: String,
    pub firmware_status: String,
    pub firmware_files: Vec<CoreFirmwareFile>,
    pub saves_status: String,
    pub save_extensions: Vec<String>,
    pub save_naming: String,
    pub states_status: String,
    pub state_serialization: String,
    pub state_naming: String,
    /// Shared RetroArch frontend paths (config, saves/, states/, system/)
    /// for this host; core naming deltas live in the fields above.
    pub frontend: RuntimeAdapter,
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CoreRecord {
    core: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    hosts: std::collections::BTreeMap<String, CoreHost>,
    #[serde(default)]
    controller: CoreController,
    #[serde(default)]
    firmware: CoreFirmware,
    #[serde(default)]
    saves: CoreSaves,
    #[serde(default)]
    states: CoreStates,
}

#[derive(Debug, Default, Deserialize)]
struct CoreHost {
    #[serde(default)]
    status: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Default, Deserialize)]
struct CoreController {
    #[serde(default)]
    status: String,
    #[serde(default)]
    profile_ids: Vec<String>,
    #[serde(default)]
    mapping_summary: String,
}

#[derive(Debug, Default, Deserialize)]
struct CoreFirmware {
    #[serde(default)]
    status: String,
    #[serde(default)]
    files: Vec<CoreFirmwareFileRecord>,
}

#[derive(Debug, Default, Deserialize)]
struct CoreFirmwareFileRecord {
    #[serde(default)]
    path: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    checksum_status: String,
    #[serde(default)]
    checksums: Vec<CoreChecksum>,
}

#[derive(Debug, Default, Deserialize)]
struct CoreChecksum {
    #[serde(default)]
    algorithm: String,
    #[serde(default)]
    value: String,
}

#[derive(Debug, Default, Deserialize)]
struct CoreSaves {
    #[serde(default)]
    status: String,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    naming: String,
}

#[derive(Debug, Default, Deserialize)]
struct CoreStates {
    #[serde(default)]
    status: String,
    #[serde(default)]
    serialization: String,
    #[serde(default)]
    naming: String,
}

/// Parse every embedded RetroArch core record.
pub fn load_core_records() -> Result<Vec<CoreRecord>> {
    CORE_RECORDS
        .iter()
        .map(|text| serde_json::from_str::<CoreRecord>(text).context("parsing core record"))
        .collect()
}

/// Build the adapter descriptor for one canonical core on one host.
pub fn core_adapter_for(
    core_records: &[CoreRecord],
    records: &[Record],
    core: &str,
    host: AdapterHost,
    bases: &LocationBases,
) -> Result<CoreAdapter> {
    let record = core_records
        .iter()
        .find(|record| record.core == core)
        .context("no core record for this RetroArch core")?;
    let platform = host.as_str();
    let host_entry = record.hosts.get(platform);
    let frontend = adapter_for_slug(records, "retroarch", host, bases)?;
    Ok(CoreAdapter {
        core: record.core.clone(),
        display_name: record.display_name.clone(),
        host,
        host_status: host_entry
            .map(|entry| entry.status.clone())
            .unwrap_or_default(),
        host_reason: host_entry
            .map(|entry| entry.reason.clone())
            .unwrap_or_default(),
        controller_status: record.controller.status.clone(),
        profile_ids: record.controller.profile_ids.clone(),
        mapping_summary: record.controller.mapping_summary.clone(),
        firmware_status: record.firmware.status.clone(),
        firmware_files: record
            .firmware
            .files
            .iter()
            .map(|file| CoreFirmwareFile {
                path: file.path.clone(),
                description: file.description.clone(),
                required: file.required,
                checksum_status: file.checksum_status.clone(),
                checksums: file
                    .checksums
                    .iter()
                    .map(|checksum| (checksum.algorithm.clone(), checksum.value.clone()))
                    .collect(),
            })
            .collect(),
        saves_status: record.saves.status.clone(),
        save_extensions: record.saves.extensions.clone(),
        save_naming: record.saves.naming.clone(),
        states_status: record.states.status.clone(),
        state_serialization: record.states.serialization.clone(),
        state_naming: record.states.naming.clone(),
        frontend,
    })
}

/// Every canonical core crossed with every host, matching the RetroArch rows
/// of the feature matrix (94 cores x 4 hosts).
pub fn all_core_adapters(bases: &LocationBases) -> Result<Vec<CoreAdapter>> {
    let records = load_records()?;
    let core_records = load_core_records()?;
    let mut adapters = Vec::new();
    for record in &core_records {
        for host in AdapterHost::all() {
            adapters.push(core_adapter_for(
                &core_records,
                &records,
                &record.core,
                host,
                bases,
            )?);
        }
    }
    Ok(adapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn bases() -> LocationBases {
        LocationBases {
            home: PathBuf::from("/home/test"),
            config_dir: PathBuf::from("/home/test/.config"),
            data_dir: PathBuf::from("/home/test/.local/share"),
            data_local_dir: PathBuf::from("/home/test/.local/share"),
            flatpak_roots: Vec::new(),
        }
    }

    #[test]
    fn host_round_trips_through_matrix_strings() {
        for host in AdapterHost::all() {
            assert_eq!(AdapterHost::parse(host.as_str()), Some(host));
        }
        assert_eq!(AdapterHost::parse("plan9"), None);
    }

    #[test]
    fn every_record_builds_four_descriptors() {
        let records = load_records().unwrap();
        let adapters = all_standalone_adapters(&bases()).unwrap();
        assert_eq!(adapters.len(), records.len() * 4);
    }
}
