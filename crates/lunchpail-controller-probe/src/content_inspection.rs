//! Explicit, bounded requests for real-content controller inspection.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub core: PathBuf,
    pub core_sha256: String,
    pub core_name: String,
    pub content: PathBuf,
    pub content_dependencies: Vec<PathBuf>,
    pub system_files: Vec<PathBuf>,
    // Lists preserve duplicate entries so they can be rejected, not silently
    // overwritten by JSON map deserialization.
    pub options: Vec<OptionOverride>,
    pub devices: Vec<DeviceSelection>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionOverride {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceSelection {
    pub port: u32,
    pub device: u32,
}

impl Request {
    /// Check the bounded helper response against this request and the current
    /// source files. This detects stale/mismatched results, not a dishonest
    /// native core; executable trust must be established independently.
    pub fn parse_report(
        &self,
        bytes: &[u8],
    ) -> Result<crate::libretro_input::ContentControllerReport> {
        self.validate()?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "Inspection report exceeds size limit"
        );
        let report: crate::libretro_input::ContentControllerReport = serde_json::from_slice(bytes)?;
        ensure!(
            report.schema_version == 1
                && report.refresh_frames == 1
                && report.input_descriptor_updates > report.input_descriptor_updates_before_refresh,
            "Inspection report has no fresh descriptor observation"
        );
        ensure!(
            report.core.core_name == self.core_name
                && report.core.need_fullpath
                && report
                    .core
                    .core_sha256
                    .eq_ignore_ascii_case(&self.core_sha256)
                && crate::file_hash(&self.core)?.eq_ignore_ascii_case(&self.core_sha256),
            "Inspection core provenance changed or does not match request"
        );
        ensure!(
            self.content
                .canonicalize()?
                .file_name()
                .and_then(|name| name.to_str())
                == Some(report.content_filename.as_str()),
            "Inspection primary content differs"
        );
        let mut expected_files = BTreeMap::new();
        for (group, path) in std::iter::once(("content", &self.content))
            .chain(
                self.content_dependencies
                    .iter()
                    .map(|path| ("content", path)),
            )
            .chain(self.system_files.iter().map(|path| ("system", path)))
        {
            // Match the staging function's canonical basename, including
            // symlink targets; never infer identity from a display title.
            let path = path.canonicalize()?;
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Inspection dependency has no UTF-8 basename"))?;
            let metadata = std::fs::metadata(&path)?;
            ensure!(
                metadata.is_file(),
                "Inspection dependency is not a regular file"
            );
            ensure!(
                expected_files
                    .insert(
                        format!("{group}/{filename}"),
                        (metadata.len(), crate::file_hash(&path)?)
                    )
                    .is_none(),
                "Duplicate staged inspection filename"
            );
        }
        ensure!(
            report.staged_files.len() == expected_files.len(),
            "Inspection dependency count differs"
        );
        for file in &report.staged_files {
            let (bytes, hash) = expected_files
                .remove(&file.filename)
                .ok_or_else(|| anyhow::anyhow!("Unexpected or duplicate inspection dependency"))?;
            ensure!(
                file.bytes == bytes && file.sha256.eq_ignore_ascii_case(&hash),
                "Inspection dependency changed or does not match request: {}",
                file.filename
            );
        }
        let requested: BTreeMap<_, _> = self.devices.iter().map(|d| (d.port, d.device)).collect();
        ensure!(
            report.requested_devices == requested,
            "Inspection device selections differ"
        );
        ensure!(
            report.controller_choices.len() <= 16,
            "Inspection has too many controller ports"
        );
        for choices in &report.controller_choices {
            ensure!(
                choices.len() <= 64,
                "Inspection has too many controller choices"
            );
            let mut identities = std::collections::BTreeSet::new();
            for choice in choices {
                ensure!(
                    identities.insert(choice.id) && valid_label(&choice.description),
                    "Inspection controller choice has duplicate identity or invalid label"
                );
            }
        }
        for (port, device) in requested {
            ensure!(
                report
                    .controller_choices
                    .get(port as usize)
                    .is_some_and(|choices| choices.iter().any(|choice| choice.id == device)),
                "Inspection selection is not advertised"
            );
        }
        crate::libretro_options::OptionRegistry::new(report.effective_options.clone())?;
        for option in &self.options {
            ensure!(
                report.effective_options.get(&option.key) == Some(&option.value),
                "Inspection option differs: {}",
                option.key
            );
        }
        ensure!(
            !report.input_descriptors.is_empty() && report.input_descriptors.len() <= 4096,
            "Inspection descriptor count is invalid"
        );
        for descriptor in &report.input_descriptors {
            ensure!(
                descriptor.port < 16 && valid_label(&descriptor.description),
                "Inspection descriptor address or label is invalid"
            );
        }
        ensure!(
            !report.input_queries.is_empty()
                && report.input_queries.len() <= 4096
                && report.input_query_calls >= report.input_queries.len() as u64
                && report.input_query_calls <= 65536,
            "Inspection input-query capture count is invalid"
        );
        let mut unique = std::collections::BTreeSet::new();
        for query in &report.input_queries {
            ensure!(
                query.port < 16 && unique.insert(query),
                "Inspection input-query capture has duplicate or invalid addresses"
            );
        }
        Ok(report)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "Unsupported content-inspection request schema"
        );
        ensure!(
            self.core_sha256.len() == 64 && self.core_sha256.bytes().all(|b| b.is_ascii_hexdigit()),
            "Expected a complete core SHA256"
        );
        ensure!(
            !self.core_name.is_empty()
                && self.core_name.len() <= 1024
                && !self.core_name.chars().any(char::is_control),
            "Invalid expected core name"
        );
        ensure!(
            self.content_dependencies.len() <= 256 && self.system_files.len() <= 256,
            "Too many inspection dependencies"
        );
        for path in std::iter::once(&self.core)
            .chain(std::iter::once(&self.content))
            .chain(&self.content_dependencies)
            .chain(&self.system_files)
        {
            ensure!(
                path.is_absolute()
                    && path
                        .to_str()
                        .is_some_and(|p| p.len() <= 32768 && !p.chars().any(char::is_control)),
                "Inspection paths must be absolute UTF-8 paths without control characters"
            );
        }
        ensure!(
            !self.devices.is_empty() && self.devices.len() <= 16,
            "Invalid inspection device count"
        );
        let mut devices = BTreeMap::new();
        for selection in &self.devices {
            ensure!(
                selection.port < 16 && devices.insert(selection.port, selection.device).is_none(),
                "Duplicate or out-of-range inspection port"
            );
        }
        ensure!(
            self.options.len() <= 4096,
            "Too many inspection option overrides"
        );
        let mut options = BTreeMap::new();
        for option in &self.options {
            ensure!(
                options
                    .insert(option.key.clone(), option.value.clone())
                    .is_none(),
                "Duplicate inspection option override"
            );
        }
        crate::libretro_options::OptionEnvironment::new(options)?;
        Ok(())
    }

    /// Must be called only by the dedicated supervised helper process.
    pub fn inspect(&self) -> Result<crate::libretro_input::ContentControllerReport> {
        self.validate()?;
        crate::libretro_input::inspect_content_controllers(
            &self.core,
            &self.core_sha256,
            &self.core_name,
            &self.content,
            &self.content_dependencies,
            &self.system_files,
            self.options
                .iter()
                .map(|o| (o.key.clone(), o.value.clone()))
                .collect(),
            self.devices.iter().map(|d| (d.port, d.device)).collect(),
        )
    }
}

fn valid_label(label: &str) -> bool {
    !label.is_empty() && label.len() <= 1024 && !label.chars().any(char::is_control)
}
