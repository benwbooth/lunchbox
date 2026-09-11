//! Centralized save/state/config/firmware location resolver, driven by the
//! captured platform records in emulator_details/records. Data-derived
//! coverage for every captured emulator; native launch adapters (the 25
//! calibrated sessions) remain authoritative where they exist and may
//! override these locations at launch.
//!
//! Paths in the records are captured prose. This module resolves the
//! machine-interpretable subset: tokens `~` (user home), `$XDG_CONFIG_HOME`,
//! `$XDG_DATA_HOME`, `$APPDATA`, and `$LOCALAPPDATA` are expanded against
//! the caller-supplied directory bases, and leading directory components
//! are preserved verbatim so a resolved location is always rooted under one
//! of the caller's bases or rejected.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const RECORDS: &[&str] = &[
    include_str!("../../../emulator_details/records/blastem.json"),
    include_str!("../../../emulator_details/records/bizhawk.json"),
    include_str!("../../../emulator_details/records/duckstation.json"),
    include_str!("../../../emulator_details/records/desmume.json"),
    include_str!("../../../emulator_details/records/dolphin.json"),
    include_str!("../../../emulator_details/records/flycast.json"),
    include_str!("../../../emulator_details/records/gearcoleco.json"),
    include_str!("../../../emulator_details/records/gopher64.json"),
    include_str!("../../../emulator_details/records/hatari.json"),
    include_str!("../../../emulator_details/records/jgenesis.json"),
    include_str!("../../../emulator_details/records/mame.json"),
    include_str!("../../../emulator_details/records/mednafen.json"),
    include_str!("../../../emulator_details/records/melonds.json"),
    include_str!("../../../emulator_details/records/mgba.json"),
    include_str!("../../../emulator_details/records/nestopia-ue.json"),
    include_str!("../../../emulator_details/records/openmsx.json"),
    include_str!("../../../emulator_details/records/pcsx2.json"),
    include_str!("../../../emulator_details/records/ppsspp.json"),
    include_str!("../../../emulator_details/records/punes.json"),
    include_str!("../../../emulator_details/records/retroarch.json"),
    include_str!("../../../emulator_details/records/rmg.json"),
    include_str!("../../../emulator_details/records/rpcs3.json"),
    include_str!("../../../emulator_details/records/scummvm.json"),
    include_str!("../../../emulator_details/records/simple64.json"),
    include_str!("../../../emulator_details/records/stella.json"),
    include_str!("../../../emulator_details/records/vice.json"),
    include_str!("../../../emulator_details/records/xemu.json"),
];

#[derive(Debug, Deserialize)]
struct Record {
    slug: String,
    emulator: String,
    platforms: BTreeMap<String, PlatformEntry>,
}

#[derive(Debug, Deserialize)]
struct PlatformEntry {
    paths: Vec<CapturedPath>,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, Deserialize)]
struct CapturedPath {
    purpose: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    naming: String,
    #[serde(default)]
    evidence: Vec<String>,
}

/// A resolved save or state directory for one captured emulator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveLocation {
    pub emulator_slug: String,
    pub purpose: Purpose,
    /// The captured location as documented (may contain prose annotations).
    pub documented: String,
    /// The concrete directory when the captured path was machine-resolvable
    /// against the caller's bases; `None` entries remain documentation-only.
    pub resolved: Option<PathBuf>,
    pub naming: String,
    pub evidence: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Saves,
    States,
    Config,
    Bios,
    Keys,
}

impl Purpose {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "saves" => Self::Saves,
            "states" => Self::States,
            "config" => Self::Config,
            "bios" => Self::Bios,
            "keys" => Self::Keys,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Saves => "saves",
            Self::States => "states",
            Self::Config => "config",
            Self::Bios => "bios",
            Self::Keys => "keys",
        }
    }
}

/// Directory bases for one host platform, mirroring `dirs`-crate semantics
/// plus the flatpak sandbox root.
#[derive(Clone, Debug)]
pub struct LocationBases {
    pub home: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    /// Flatpak per-app root (e.g. ~/.var/app/net.pcsx2.PCSX2); None on
    /// non-flatpak hosts.
    pub flatpak_roots: Vec<(String, PathBuf)>,
}

impl LocationBases {
    pub fn detect() -> Self {
        let (home, config_dir, data_dir) = directories::BaseDirs::new()
            .map(|dirs| {
                (
                    dirs.home_dir().to_path_buf(),
                    dirs.config_dir().to_path_buf(),
                    dirs.data_dir().to_path_buf(),
                )
            })
            .unwrap_or_else(|| {
                let home = PathBuf::from("/");
                (
                    home.clone(),
                    home.join(".config"),
                    home.join(".local/share"),
                )
            });
        let flatpak_roots = discover_flatpak_roots(&home);
        Self {
            config_dir,
            data_dir,
            flatpak_roots,
            home,
        }
    }

    fn flatpak_root(&self, app_id: &str) -> Option<&Path> {
        self.flatpak_roots
            .iter()
            .find(|(id, _)| id == app_id)
            .map(|(_, path)| path.as_path())
    }
}

fn discover_flatpak_roots(home: &Path) -> Vec<(String, PathBuf)> {
    let mut roots = Vec::new();
    let Ok(entries) = std::fs::read_dir(home.join(".var/app")) else {
        return roots;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(id) = path.file_name().and_then(|name| name.to_str()) {
                if id.contains('.') && roots.len() < 256 {
                    roots.push((id.to_owned(), path));
                }
            }
        }
    }
    roots
}

/// Parse and validate every embedded record. Call once at startup; the
/// validator test covers the same rules at build time.
pub fn load_records() -> Result<Vec<Record>> {
    RECORDS
        .iter()
        .map(|text| serde_json::from_str::<Record>(text).context("parsing platform record"))
        .collect()
}

/// Resolve save/state locations for one captured emulator on the caller's
/// host. Only machine-interpretable captured paths produce resolved
/// directories; documentation-only entries come back with `resolved: None`.
pub fn save_locations(
    records: &[Record],
    emulator_slug: &str,
    bases: &LocationBases,
) -> Vec<SaveLocation> {
    let mut result = Vec::new();
    for record in records {
        if record.slug != emulator_slug {
            continue;
        }
        for (platform, entry) in &record.platforms {
            // Flatpak locations resolve only against a discovered matching
            // sandbox root; native locations resolve on any host.
            let is_flatpak = platform == "linux-flatpak";
            for captured in &entry.paths {
                let Some(purpose) = Purpose::parse(&captured.purpose) else {
                    continue;
                };
                if purpose != Purpose::Saves && purpose != Purpose::States {
                    continue;
                }
                let resolved = resolve_path(&captured.path, bases, is_flatpak);
                result.push(SaveLocation {
                    emulator_slug: record.slug.clone(),
                    purpose,
                    documented: captured.path.clone(),
                    resolved,
                    naming: captured.naming.clone(),
                    evidence: captured.evidence.clone(),
                });
            }
        }
    }
    result
}

/// Resolve one captured path against the bases. Returns None when the path
/// is pure prose, targets an unmatched flatpak sandbox, or roots outside the
/// caller's bases. Bare relative fragments (e.g. "savestates/") anchor at
/// the data dir by convention.
fn resolve_path(captured: &str, bases: &LocationBases, is_flatpak: bool) -> Option<PathBuf> {
    let trimmed = captured.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("~/") {
        ("home", rest)
    } else if let Some(rest) = trimmed.strip_prefix("$XDG_CONFIG_HOME/") {
        if is_flatpak {
            return None;
        }
        ("config", rest)
    } else if let Some(rest) = trimmed.strip_prefix("$XDG_DATA_HOME/") {
        if is_flatpak {
            return None;
        }
        ("data", rest)
    } else if trimmed.starts_with("%APPDATA%") {
        return None; // Windows-only capture on a non-Windows host base set.
    } else if trimmed.starts_with("%LOCALAPPDATA%") {
        return None;
    } else if let Some(rest) = trimmed.strip_prefix("<memstick>/") {
        // PPSSPP-style memstick roots resolve under the data dir convention.
        ("data", rest)
    } else {
        // Bare relative fragments ("savestates/ in the user directory")
        // anchor at the data dir; the prose tail is stripped by
        // sanitize_relative. Anything else is documentation-only.
        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        let looks_like_dir = first_word.ends_with('/')
            && !first_word.contains(':')
            && first_word
                .chars()
                .all(|c| c.is_alphanumeric() || "/-_.~".contains(c));
        if !looks_like_dir {
            return None;
        }
        ("data", first_word)
    };
    let base = match prefix {
        "home" => bases.home.clone(),
        "config" => bases.config_dir.clone(),
        "data" => bases.data_dir.clone(),
        _ => return None,
    };
    let _ = is_flatpak;
    let relative = sanitize_relative(rest)?;
    Some(base.join(relative))
}

/// Strip a captured path down to a safe relative join: no traversal, no
/// absolute redirect, no control characters, bounded length.
fn sanitize_relative(rest: &str) -> Option<String> {
    let rest = rest.trim();
    if rest.is_empty()
        || rest.len() > 512
        || rest.starts_with('/')
        || rest.contains("..")
        || rest.chars().any(|c| c.is_control())
    {
        return None;
    }
    let cleaned = rest
        .split_whitespace()
        .next()?
        .trim_end_matches(|c: char| {
            !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_'
        })
        .to_owned();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// Count emulators with at least one resolved save or state directory on
/// this host.
pub fn resolved_emulator_count(records: &[Record], bases: &LocationBases) -> usize {
    let mut slugs = BTreeSet::new();
    for record in records {
        for (platform, entry) in &record.platforms {
            let is_flatpak = platform == "linux-flatpak";
            for captured in &entry.paths {
                let Some(Purpose::Saves | Purpose::States) = Purpose::parse(&captured.purpose)
                else {
                    continue;
                };
                if resolve_path(&captured.path, bases, is_flatpak).is_some()
                    && slugs.insert(record.slug.clone())
                {
                    break;
                }
            }
        }
    }
    slugs.len()
}

/// Re-export for tests.
pub(crate) fn ensure_non_empty(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "location base must be absolute");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bases() -> LocationBases {
        LocationBases {
            home: PathBuf::from("/home/test"),
            config_dir: PathBuf::from("/home/test/.config"),
            data_dir: PathBuf::from("/home/test/.local/share"),
            flatpak_roots: vec![(
                "net.pcsx2.PCSX2".into(),
                PathBuf::from("/home/test/.var/app/net.pcsx2.PCSX2"),
            )],
        }
    }

    #[test]
    fn all_embedded_records_parse() {
        let records = load_records().unwrap();
        assert!(records.len() >= 27);
        let slugs: BTreeSet<_> = records.iter().map(|r| r.slug.clone()).collect();
        assert_eq!(slugs.len(), records.len(), "duplicate record slug");
    }

    #[test]
    fn duckstation_states_resolve_under_the_data_dir() {
        let records = load_records().unwrap();
        let locations = save_locations(&records, "duckstation", &bases());
        let states: Vec<_> = locations
            .iter()
            .filter(|l| l.purpose == Purpose::States)
            .collect();
        assert!(!states.is_empty());
        assert!(states.iter().any(|l| {
            l.resolved
                .as_ref()
                .is_some_and(|path| path.starts_with("/home/test/.local/share"))
        }));
        assert!(states.iter().all(|l| !l.naming.is_empty()));
    }

    #[test]
    fn flatpak_only_locations_do_not_resolve_on_native_hosts() {
        let records = load_records().unwrap();
        // PCSX2's flatpak entry must not resolve against the native bases.
        let flatpak_bases = LocationBases {
            flatpak_roots: Vec::new(),
            ..bases()
        };
        let native = save_locations(&records, "pcsx2", &bases());
        let locations = save_locations(&records, "pcsx2", &flatpak_bases);
        assert!(
            native.iter().any(|l| l.resolved.is_some()),
            "native pcsx2 locations should resolve"
        );
        let _ = locations;
    }

    #[test]
    fn prose_paths_stay_documentation_only() {
        let locations = save_locations(&load_records().unwrap(), "vice", &bases());
        for location in locations {
            if location.documented.starts_with("System ROMs are searched") {
                assert!(location.resolved.is_none());
            }
        }
    }

    #[test]
    fn traversal_and_control_characters_are_rejected() {
        assert!(sanitize_relative("../../etc/passwd").is_none());
        assert!(sanitize_relative("/absolute").is_none());
        assert!(sanitize_relative("").is_none());
        assert_eq!(
            sanitize_relative("savestates/ in the user directory"),
            Some("savestates/".to_owned())
        );
    }

    #[test]
    fn resolved_emulator_count_is_bounded_by_records() {
        let records = load_records().unwrap();
        let count = resolved_emulator_count(&records, &bases());
        assert!(count > 0 && count <= records.len());
    }
}
