//! Exact metadata-driven dependencies. Metadata provenance must be established
//! by the caller against the selected core; title similarity is never evidence.
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MachineDependencies {
    pub name: String,
    pub rom_parent: Option<String>,
    pub devices: Vec<String>,
    pub has_roms: bool,
    pub disks: Vec<String>,
    /// Native SHA-1 identities for good dumps. Missing values never authorize
    /// differently named disks; legacy explicit metadata may omit this map.
    #[serde(default)]
    pub disk_sha1: BTreeMap<String, String>,
}

pub(super) fn component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value != "."
        && value != ".."
        && !value.chars().any(|c| c.is_control() || "/\\:".contains(c))
}

pub(super) fn validate_roots(roots: &[PathBuf]) -> Result<()> {
    ensure!(
        !roots.is_empty() && roots.len() <= 64,
        "MAME dependency roots exceed limit"
    );
    let mut root_set = BTreeSet::new();
    for root in roots {
        ensure!(
            root.is_absolute()
                && root.is_dir()
                && root.canonicalize()? == *root
                && root_set.insert(root),
            "MAME dependency roots must be distinct resolved directories"
        );
    }
    Ok(())
}

/// Resolve each declared ROM-bearing set and disk through exact local paths.
/// Missing or ambiguous locations stop resolution; no similarly named set is
/// substituted. Split/nonmerged archives are accepted only when every declared
/// ROM-bearing dependency has its own archive. Merged-set member resolution
/// needs a separate archive manifest and is not inferred here.
pub(crate) fn resolve(
    machine: &str,
    metadata: &[MachineDependencies],
    roots: &[PathBuf],
    declared: &[super::InspectionInput],
    cancel: &AtomicBool,
) -> Result<Vec<super::InspectionInput>> {
    ensure!(
        !metadata.is_empty() && metadata.len() <= 100_000,
        "MAME metadata count exceeds limit"
    );
    validate_roots(roots)?;
    ensure!(
        declared.len() <= 4096,
        "Too many declared MAME dependencies"
    );
    let mut destinations = BTreeSet::new();
    for input in declared {
        ensure!(
            destinations.insert(&input.destination),
            "Duplicate declared MAME dependency destination"
        );
    }
    let mut index = BTreeMap::new();
    for entry in metadata {
        ensure!(
            component(&entry.name)
                && entry.rom_parent.as_deref().is_none_or(component)
                && entry.devices.len() <= 4096
                && entry.disks.len() <= 4096
                && entry.disk_sha1.len() <= entry.disks.len()
                && entry
                    .disk_sha1
                    .iter()
                    .all(|(name, hash)| entry.disks.contains(name)
                        && hash.len() == 40
                        && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
                && entry
                    .devices
                    .iter()
                    .chain(&entry.disks)
                    .all(|name| component(name)),
            "Invalid MAME dependency metadata"
        );
        ensure!(
            index.insert(entry.name.as_str(), entry).is_none(),
            "Duplicate MAME machine metadata"
        );
    }
    let mut pending = vec![machine.to_owned()];
    let mut visited = BTreeSet::new();
    let mut result = BTreeMap::<PathBuf, PathBuf>::new();
    while let Some(name) = pending.pop() {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "MAME dependency discovery cancelled"
        );
        if !visited.insert(name.clone()) {
            continue;
        }
        ensure!(
            visited.len() <= 4096,
            "MAME dependency closure exceeds limit"
        );
        let entry = index
            .get(name.as_str())
            .with_context(|| format!("Missing exact MAME metadata for {name}"))?;
        if let Some(parent) = &entry.rom_parent {
            pending.push(parent.clone());
        }
        pending.extend(entry.devices.iter().cloned());
        if entry.has_roms {
            let alternatives = [
                PathBuf::from(format!("{name}.zip")),
                PathBuf::from(format!("{name}.7z")),
            ];
            let (source, relative) = locate(roots, &alternatives, declared)?;
            result.insert(Path::new("roms").join(relative), source);
        }
        for disk in &entry.disks {
            // Native driver search paths include the ROM-parent chain. Accept
            // the exact disk filename in those folders, then stage it under
            // this record's own set name so its native lookup remains explicit.
            let filename = format!("{disk}.chd");
            let relative = Path::new(&name).join(&filename);
            let mut folders = vec![name.as_str()];
            let mut filenames = BTreeSet::from([filename.clone()]);
            let mut ancestors = BTreeSet::from([name.as_str()]);
            let mut parent = entry.rom_parent.as_deref();
            while let Some(ancestor) = parent {
                ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "MAME dependency discovery cancelled"
                );
                ensure!(
                    ancestors.len() < 4096 && ancestors.insert(ancestor),
                    "MAME ROM-parent chain exceeds its limit or contains a cycle"
                );
                folders.push(ancestor);
                let parent_entry = index
                    .get(ancestor)
                    .with_context(|| format!("Missing exact MAME metadata for {ancestor}"))?;
                if let Some(expected) = entry.disk_sha1.get(disk) {
                    for (parent_disk, hash) in &parent_entry.disk_sha1 {
                        if expected.eq_ignore_ascii_case(hash) {
                            filenames.insert(format!("{parent_disk}.chd"));
                        }
                    }
                }
                parent = parent_entry.rom_parent.as_deref();
            }
            ensure!(
                folders.len().saturating_mul(filenames.len()) <= 4096,
                "MAME disk search alternatives exceed limit"
            );
            let alternatives: Vec<_> = folders
                .iter()
                .flat_map(|folder| {
                    filenames
                        .iter()
                        .map(move |filename| Path::new(folder).join(filename))
                })
                .collect();
            let (source, _) = locate(roots, &alternatives, declared)?;
            ensure!(
                result
                    .insert(Path::new("roms").join(relative), source)
                    .is_none(),
                "Duplicate MAME disk destination"
            );
        }
    }
    Ok(result
        .into_iter()
        .map(|(destination, source)| super::InspectionInput {
            source,
            destination,
        })
        .collect())
}

fn locate(
    roots: &[PathBuf],
    alternatives: &[PathBuf],
    declared: &[super::InspectionInput],
) -> Result<(PathBuf, PathBuf)> {
    // An explicit staging identity selects its source without inferring identity
    // from the source filename or requiring it to live in a discovery folder.
    let mut selected = None;
    for relative in alternatives {
        let destination = Path::new("roms").join(relative);
        if let Some(input) = declared
            .iter()
            .find(|input| input.destination == destination)
        {
            ensure!(
                selected.is_none(),
                "Multiple explicitly declared MAME dependency alternatives; select one archive/disk"
            );
            let metadata = std::fs::symlink_metadata(&input.source).with_context(|| {
                format!(
                    "Reading explicitly selected MAME dependency {}",
                    input.source.display()
                )
            })?;
            ensure!(
                input.source.is_absolute()
                    && metadata.is_file()
                    && !metadata.file_type().is_symlink(),
                "Explicit MAME discovery dependency must name an absolute regular file: {}",
                input.source.display()
            );
            selected = Some((input.source.clone(), relative.clone()));
        }
    }
    if let Some(selected) = selected {
        return Ok(selected);
    }
    let mut found = None;
    for root in roots {
        for relative in alternatives {
            let path = root.join(relative);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) => {
                    ensure!(
                        metadata.is_file()
                            && !metadata.file_type().is_symlink()
                            && path.canonicalize()?.starts_with(root),
                        "MAME dependency requires an explicit regular-file location: {}",
                        path.display()
                    );
                    ensure!(
                        found.is_none(),
                        "Multiple exact MAME dependency candidates; choose one root/archive explicitly"
                    );
                    found = Some((path, relative.clone()));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    found.context("Required MAME ROM archive or disk is absent from the declared roots")
}
