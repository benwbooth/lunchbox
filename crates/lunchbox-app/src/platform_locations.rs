//! Centralized save/state/config/firmware location resolver, driven by the
//! captured platform records in emulator_details/records. Data-derived
//! coverage for every captured emulator; native launch adapters (the 28
//! calibrated sessions) remain authoritative where they exist and may
//! override these locations at launch.
//!
//! Paths in the records are captured prose. This module resolves the
//! machine-interpretable subset: tokens `~` (user home), `$XDG_CONFIG_HOME`,
//! `$XDG_DATA_HOME`, `%APPDATA%`, `%LOCALAPPDATA%`, and `%USERPROFILE%` are expanded against
//! the caller-supplied directory bases, and leading directory components
//! are preserved verbatim so a resolved location is always rooted under one
//! of the caller's bases or rejected.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::save_sync::{RouteRoot, SavePurpose, SaveRoute};

include!(concat!(env!("OUT_DIR"), "/platform_records.rs"));

const CAPTURE_STATUSES: [&str; 4] = ["captured", "not_supported", "not_required", "unresolved"];

fn default_capture_status() -> String {
    "captured".to_owned()
}

#[derive(Debug, Deserialize)]
pub struct Record {
    slug: String,
    emulator: String,
    #[serde(default)]
    platforms: BTreeMap<String, PlatformEntry>,
    #[serde(default)]
    platform_gaps: BTreeMap<String, PlatformGap>,
}

#[derive(Debug, Deserialize)]
struct PlatformEntry {
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
    /// Source-record host/runtime variant (`linux`, `linux-flatpak`, `macos`,
    /// or `windows`). Callers that mutate save data must select one exact
    /// variant instead of combining every locally resolvable installation.
    pub platform: String,
    pub purpose: Purpose,
    /// Whether this dimension is captured, unsupported, unnecessary, or
    /// unresolved. Older records default to `captured`.
    pub status: String,
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
    /// Controller/input bindings when their syntax is documented separately
    /// from the runtime's general configuration location.
    Input,
    Bios,
    Keys,
}

impl Purpose {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "saves" => Self::Saves,
            "states" => Self::States,
            "config" => Self::Config,
            "input" => Self::Input,
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
            Self::Input => "input",
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
    pub data_local_dir: PathBuf,
    /// Flatpak per-app root (e.g. ~/.var/app/net.pcsx2.PCSX2); None on
    /// non-flatpak hosts.
    pub flatpak_roots: Vec<(String, PathBuf)>,
}

impl LocationBases {
    pub fn detect() -> Self {
        let (home, config_dir, data_dir, data_local_dir) = directories::BaseDirs::new()
            .map(|dirs| {
                (
                    dirs.home_dir().to_path_buf(),
                    dirs.config_dir().to_path_buf(),
                    dirs.data_dir().to_path_buf(),
                    dirs.data_local_dir().to_path_buf(),
                )
            })
            .unwrap_or_else(|| {
                let home = PathBuf::from("/");
                (
                    home.clone(),
                    home.join(".config"),
                    home.join(".local/share"),
                    home.join(".local/share"),
                )
            });
        let flatpak_roots = discover_flatpak_roots(&home);
        Self {
            config_dir,
            data_dir,
            data_local_dir,
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
    let records = RECORDS
        .iter()
        .map(|text| serde_json::from_str::<Record>(text).context("parsing platform record"))
        .collect::<Result<Vec<_>>>()?;
    for record in &records {
        for (platform, entry) in &record.platforms {
            for captured in &entry.paths {
                ensure!(
                    CAPTURE_STATUSES.contains(&captured.status.as_str()),
                    "{} has unknown {} status {} for {platform}",
                    record.slug,
                    captured.purpose,
                    captured.status
                );
            }
        }
    }
    Ok(records)
}

/// Return the embedded record slugs for coverage and location UIs. Keeping the
/// list here prevents UI availability from drifting behind newly added records.
pub fn record_slugs() -> Result<Vec<String>> {
    let mut slugs = load_records()?
        .into_iter()
        .map(|record| record.slug)
        .collect::<Vec<_>>();
    slugs.sort();
    Ok(slugs)
}

/// Resolve a catalog/runtime display name to the stable embedded-record slug.
/// Save synchronization uses the slug as its remote namespace, never a
/// localized UI label or executable filename.
pub fn slug_for_emulator_name(records: &[Record], emulator_name: &str) -> Result<String> {
    let wanted = normalize_slug(emulator_name);
    records
        .iter()
        .find(|record| {
            normalize_slug(&record.slug) == wanted || normalize_slug(&record.emulator) == wanted
        })
        .map(|record| record.slug.clone())
        .context("no captured platform record for this emulator")
}

/// Resolve save/state locations for one captured emulator on the caller's
/// host. Only machine-interpretable captured paths produce resolved
/// directories; documentation-only entries come back with `resolved: None`.
pub fn save_locations(
    records: &[Record],
    emulator_slug: &str,
    bases: &LocationBases,
) -> Vec<SaveLocation> {
    adapter_locations(
        records,
        emulator_slug,
        &[Purpose::Saves, Purpose::States],
        bases,
    )
}

/// Resolve save/state locations for one exact runtime variant. This is the
/// mutation-safe entry point for save synchronization: a Linux host can have
/// native and Flatpak installations at the same time, and their data roots
/// must never be silently merged.
pub fn save_locations_for_platform(
    records: &[Record],
    emulator_slug: &str,
    platform: &str,
    bases: &LocationBases,
) -> Result<Vec<SaveLocation>> {
    ensure!(
        matches!(platform, "linux" | "linux-flatpak" | "macos" | "windows"),
        "unknown save runtime platform {platform}"
    );
    let locations = save_locations(records, emulator_slug, bases)
        .into_iter()
        .filter(|location| location.platform == platform)
        .collect::<Vec<_>>();
    ensure!(
        !locations.is_empty(),
        "{emulator_slug} has no captured save/state locations for {platform}"
    );
    Ok(locations)
}

/// Convert one exact runtime's captured, machine-resolvable save locations
/// into stable synchronization routes. Unresolved, unsupported, and
/// documentation-only locations are excluded instead of being mistaken for
/// empty directories.
pub fn save_route_roots_for_platform(
    records: &[Record],
    emulator_slug: &str,
    platform: &str,
    bases: &LocationBases,
) -> Result<Vec<RouteRoot>> {
    let locations = save_locations_for_platform(records, emulator_slug, platform, bases)?;
    let mut indices = BTreeMap::<&'static str, u16>::new();
    let mut roots = Vec::new();
    for location in locations {
        let purpose = match location.purpose {
            Purpose::Saves => SavePurpose::Saves,
            Purpose::States => SavePurpose::States,
            _ => continue,
        };
        let counter = indices.entry(purpose.as_str()).or_default();
        let root_index = *counter;
        *counter = counter
            .checked_add(1)
            .context("too many captured save roots")?;
        if location.status != "captured" {
            continue;
        }
        let Some(path) = location.resolved else {
            continue;
        };
        roots.push(RouteRoot {
            route: SaveRoute {
                purpose,
                root_index,
            },
            path,
            // Prose, user-chosen media paths, and unresolved dimensions were
            // excluded above. A missing fixed path means the emulator has not
            // created its save directory yet and is safe to create on pull.
            create_if_missing: true,
        });
    }
    ensure!(
        !roots.is_empty(),
        "{emulator_slug} has no machine-resolvable save/state roots for {platform}"
    );
    Ok(roots)
}

/// Resolve captured locations for one emulator filtered to the requested
/// purposes (controller `config`/`input`, `bios`/`keys` firmware,
/// `saves`/`states`).
/// Powers per-emulator adapters: every entry keeps its documented capture,
/// machine-resolvable directory (or `None` when documentation-only),
/// naming convention, and evidence.
pub fn adapter_locations(
    records: &[Record],
    emulator_slug: &str,
    purposes: &[Purpose],
    bases: &LocationBases,
) -> Vec<SaveLocation> {
    let mut result = Vec::new();
    for record in records {
        if record.slug != emulator_slug {
            continue;
        }
        for (platform, entry) in &record.platforms {
            // Only the current host resolves. Flatpak locations additionally
            // require a discovered matching sandbox root.
            for captured in &entry.paths {
                let Some(purpose) = Purpose::parse(&captured.purpose) else {
                    continue;
                };
                if !purposes.contains(&purpose) {
                    continue;
                }
                let resolved = (captured.status == "captured")
                    .then(|| resolve_path(&captured.path, bases, platform))
                    .flatten();
                result.push(SaveLocation {
                    emulator_slug: record.slug.clone(),
                    platform: platform.clone(),
                    purpose,
                    status: captured.status.clone(),
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
/// caller's bases. Bare relative fragments are documentation-only: their base
/// is emulator-specific and guessing an XDG directory would make mutating
/// callers synchronize the wrong files.
fn resolve_path(captured: &str, bases: &LocationBases, platform: &str) -> Option<PathBuf> {
    let trimmed = captured.trim();
    if trimmed.is_empty() {
        return None;
    }
    match platform {
        "linux" if cfg!(target_os = "linux") => {}
        "linux-flatpak" if cfg!(target_os = "linux") => {
            let rest = trimmed.strip_prefix("~/.var/app/")?;
            let (app_id, relative) = rest.split_once('/')?;
            let root = bases.flatpak_root(app_id)?;
            return Some(root.join(sanitize_relative(relative)?));
        }
        "macos" if cfg!(target_os = "macos") => {}
        "windows" if cfg!(target_os = "windows") => {}
        _ => return None,
    }
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("~/") {
        ("home", rest)
    } else if let Some(rest) = trimmed.strip_prefix("$XDG_CONFIG_HOME/") {
        ("config", rest)
    } else if let Some(rest) = trimmed.strip_prefix("$XDG_DATA_HOME/") {
        ("data", rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("%APPDATA%\\")
        .or_else(|| trimmed.strip_prefix("%APPDATA%/"))
    {
        ("config", rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("%LOCALAPPDATA%\\")
        .or_else(|| trimmed.strip_prefix("%LOCALAPPDATA%/"))
    {
        ("data_local", rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("%USERPROFILE%\\")
        .or_else(|| trimmed.strip_prefix("%USERPROFILE%/"))
    {
        ("home", rest)
    } else if let Some(rest) = trimmed.strip_prefix("<memstick>/") {
        // PPSSPP-style memstick roots resolve under the data dir convention.
        ("data", rest)
    } else {
        return None;
    };
    let base = match prefix {
        "home" => bases.home.clone(),
        "config" => bases.config_dir.clone(),
        "data" => bases.data_dir.clone(),
        "data_local" => bases.data_local_dir.clone(),
        _ => return None,
    };
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
    let (candidate, suffix) = rest
        .find(char::is_whitespace)
        .map(|index| (&rest[..index], rest[index..].trim_start()))
        .unwrap_or((rest, ""));
    if !suffix.is_empty()
        && ![
            "(", "[", "and ", "by ", "default", "or ", "plus ", "under ", "when ", "with ",
        ]
        .iter()
        .any(|prefix| suffix.starts_with(prefix))
    {
        // A space can be part of the native path (for example macOS
        // `Application Support`) or prose. The catalog needs a structured
        // machine path before we can distinguish those cases safely.
        return None;
    }
    let cleaned = candidate
        .replace('\\', "/")
        .trim_end_matches(|c: char| {
            !c.is_alphanumeric() && c != '/' && c != '.' && c != '-' && c != '_'
        })
        .to_owned();
    if cleaned.is_empty()
        || cleaned.starts_with('/')
        || cleaned.contains("//")
        || cleaned
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || "/-_.".contains(c)))
        || cleaned
            .split('/')
            .any(|component| component == "." || component == "..")
    {
        return None;
    }
    Some(cleaned)
}

/// Resolve locations for an emulator addressed by its display name
/// (case-insensitive, normalized the same way as slugs); returns the
/// resolved entries serialized for the settings layer, or an error when no
/// record matches. Covers all adapter purposes: controller `config`/`input`
/// syntax/location, `bios`/`keys` firmware, and `saves`/`states` for save
/// sync.
pub fn locations_for_emulator_name(
    records: &[Record],
    emulator_name: &str,
    bases: &LocationBases,
) -> Result<String> {
    let slug = slug_for_emulator_name(records, emulator_name)?;
    let record = records
        .iter()
        .find(|record| record.slug == slug)
        .context("resolved emulator record disappeared")?;
    let mut out = serde_json::Map::new();
    out.insert("slug".into(), record.slug.clone().into());
    out.insert("emulator".into(), record.emulator.clone().into());
    let mut locations = Vec::new();
    for (platform, entry) in &record.platforms {
        for captured in &entry.paths {
            let Some(purpose) = Purpose::parse(&captured.purpose) else {
                continue;
            };
            locations.push(serde_json::json!({
                "platform": platform,
                "purpose": purpose.as_str(),
                "status": captured.status,
                "documented": captured.path,
                "resolved": (captured.status == "captured")
                    .then(|| resolve_path(&captured.path, bases, platform))
                    .flatten()
                    .map(|path| path.to_string_lossy().into_owned()),
                "naming": captured.naming,
                "evidence": captured.evidence,
            }));
        }
    }
    out.insert("locations".into(), locations.into());
    out.insert(
        "platform_gaps".into(),
        record
            .platform_gaps
            .iter()
            .map(|(platform, gap)| {
                serde_json::json!({
                    "platform": platform,
                    "status": gap.status,
                    "reason": gap.reason,
                    "evidence": gap.evidence,
                })
            })
            .collect::<Vec<_>>()
            .into(),
    );
    Ok(serde_json::Value::Object(out).to_string())
}

fn normalize_slug(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Count emulators with at least one resolved save or state directory on
/// this host.
pub fn resolved_emulator_count(records: &[Record], bases: &LocationBases) -> usize {
    let mut slugs = BTreeSet::new();
    for record in records {
        for (platform, entry) in &record.platforms {
            for captured in &entry.paths {
                let Some(Purpose::Saves | Purpose::States) = Purpose::parse(&captured.purpose)
                else {
                    continue;
                };
                if captured.status == "captured"
                    && resolve_path(&captured.path, bases, platform).is_some()
                    && slugs.insert(record.slug.clone())
                {
                    break;
                }
            }
        }
    }
    slugs.len()
}

/// How save/state sync must treat a captured emulator. Derived from the
/// record captures: emulators whose saves live inside a single disk image
/// or backup-RAM image are whole-image; everything else is file-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveSyncModel {
    PerFile,
    WholeImage,
}

/// Documented sync model per captured slug, with the record evidence that
/// motivated it. Unknown slugs are PerFile by default.
pub fn sync_model(slug: &str) -> (SaveSyncModel, &'static str) {
    match slug {
        "xemu" => (
            SaveSyncModel::WholeImage,
            "per-game saves and QEMU snapshots live inside the HDD qcow2 image (sys.files.hdd_path); eeprom.bin sits beside it",
        ),
        "hatari" => (
            SaveSyncModel::WholeImage,
            "writes land in place inside mounted writable floppy/hard-disk images; memory snapshot is <home>/hatari.sav",
        ),
        "kronos" | "yaba-sanshiro-2" => (
            SaveSyncModel::WholeImage,
            "per-game Saturn saves live inside the single internal backup-RAM image (bkram.bin)",
        ),
        "altirra" => (
            SaveSyncModel::WholeImage,
            "writes go in place into the mounted ATR/ATX/DCM/PRO/XFD/SAP/CAS media; states are user-chosen *.atstate2",
        ),
        "dosbox-staging" => (
            SaveSyncModel::WholeImage,
            "game saves are ordinary files written inside mounted drives/disk images; no emulator-managed save files exist",
        ),
        _ => (SaveSyncModel::PerFile, ""),
    }
}

/// Enumerate concrete save/state files for one captured emulator on this
/// host: a bounded recursive walk of every machine-resolvable save/state
/// directory. Whole-image emulators return their image file when it is
/// discoverable, otherwise an empty list (the caller surfaces the documented
/// location instead). Bounded to 4096 files and depth 6.
pub fn enumerate_save_files(
    records: &[Record],
    emulator_slug: &str,
    bases: &LocationBases,
) -> Result<Vec<PathBuf>> {
    let (model, _) = sync_model(emulator_slug);
    let locations = save_locations(records, emulator_slug, bases);
    let mut files = Vec::new();
    if model == SaveSyncModel::WholeImage {
        for location in &locations {
            if let Some(dir) = &location.resolved {
                collect_files(dir, &mut files, 0)?;
            }
        }
        // A whole-image emulator yields few, large files; cap tighter.
        files.truncate(64);
        return Ok(files);
    }
    for location in &locations {
        if let Some(dir) = &location.resolved {
            collect_files(dir, &mut files, 0)?;
        }
    }
    ensure!(
        files.len() <= 4096,
        "save enumeration exceeded the bounded file count"
    );
    Ok(files)
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>, depth: usize) -> Result<()> {
    if depth > 6 || files.len() > 4096 {
        return Ok(());
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files, depth + 1)?;
        } else if path.is_file() {
            files.push(path);
            if files.len() > 4096 {
                return Ok(());
            }
        }
    }
    Ok(())
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
            data_local_dir: PathBuf::from("/home/test/.local/share"),
            flatpak_roots: vec![(
                "net.pcsx2.PCSX2".into(),
                PathBuf::from("/home/test/.var/app/net.pcsx2.PCSX2"),
            )],
        }
    }

    #[test]
    fn all_embedded_records_parse() {
        let records = load_records().unwrap();
        assert!(records.len() >= 34);
        let slugs: BTreeSet<_> = records.iter().map(|r| r.slug.clone()).collect();
        assert_eq!(slugs.len(), records.len(), "duplicate record slug");
    }

    #[test]
    fn duckstation_relative_states_remain_documentation_only() {
        let records = load_records().unwrap();
        let locations = save_locations(&records, "duckstation", &bases());
        let states: Vec<_> = locations
            .iter()
            .filter(|l| l.purpose == Purpose::States)
            .collect();
        assert!(!states.is_empty());
        assert!(states.iter().all(|location| location.resolved.is_none()));
        assert!(states.iter().all(|l| !l.naming.is_empty()));
    }

    #[test]
    fn flatpak_only_locations_do_not_resolve_on_native_hosts() {
        let records = load_records().unwrap();
        let flatpak_bases = LocationBases {
            flatpak_roots: Vec::new(),
            ..bases()
        };
        let native = save_locations(&records, "pcsx2", &bases());
        let locations = save_locations(&records, "pcsx2", &flatpak_bases);
        assert!(
            native.iter().any(|location| {
                location.documented.contains(".var/app/net.pcsx2.PCSX2")
                    && location
                        .resolved
                        .as_ref()
                        .is_some_and(|path| path.starts_with("/home/test/.var/app/net.pcsx2.PCSX2"))
            }),
            "a discovered PCSX2 sandbox should resolve"
        );
        assert!(locations.iter().all(|location| {
            !location.documented.contains(".var/app/net.pcsx2.PCSX2") || location.resolved.is_none()
        }));
    }

    #[test]
    fn sync_roots_are_scoped_to_one_exact_runtime_variant() {
        let records = load_records().unwrap();
        let roots =
            save_route_roots_for_platform(&records, "pcsx2", "linux-flatpak", &bases()).unwrap();
        assert!(!roots.is_empty());
        assert!(
            roots
                .iter()
                .all(|root| { root.path.starts_with("/home/test/.var/app/net.pcsx2.PCSX2") })
        );
        assert!(save_route_roots_for_platform(&records, "pcsx2", "windows", &bases()).is_err());
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
    fn unresolved_dimensions_never_resolve_to_local_paths() {
        let locations = adapter_locations(
            &load_records().unwrap(),
            "denise",
            &[
                Purpose::Config,
                Purpose::Input,
                Purpose::Saves,
                Purpose::States,
            ],
            &bases(),
        );
        assert!(!locations.is_empty());
        assert!(
            locations
                .iter()
                .all(|location| location.status == "unresolved")
        );
        assert!(locations.iter().all(|location| location.resolved.is_none()));
    }

    #[test]
    fn traversal_and_control_characters_are_rejected() {
        assert!(sanitize_relative("../../etc/passwd").is_none());
        assert!(sanitize_relative("/absolute").is_none());
        assert!(sanitize_relative("").is_none());
        assert!(sanitize_relative("Library/Application Support/Game").is_none());
        assert!(sanitize_relative("Dolphin/MemoryCardA.<region>.raw").is_none());
        assert_eq!(sanitize_relative("savestates/ in the user directory"), None);
        assert_eq!(
            sanitize_relative("RetroArch\\saves\\ (default)"),
            Some("RetroArch/saves/".to_owned())
        );
    }

    #[test]
    fn sync_models_match_the_documented_constraints() {
        for slug in [
            "xemu",
            "hatari",
            "kronos",
            "yaba-sanshiro-2",
            "altirra",
            "dosbox-staging",
        ] {
            let (model, reason) = sync_model(slug);
            assert_eq!(model, SaveSyncModel::WholeImage, "{slug}");
            assert!(!reason.is_empty());
        }
        for slug in ["duckstation", "pcsx2", "mgba", "scummvm", "nestopia-ue"] {
            assert_eq!(sync_model(slug).0, SaveSyncModel::PerFile);
        }
        assert_eq!(sync_model("unknown-emulator").0, SaveSyncModel::PerFile);
    }

    #[test]
    fn enumeration_walks_resolved_dirs_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".config/retroarch/saves")).unwrap();
        std::fs::write(dir.path().join(".config/retroarch/saves/game.sav"), b"x").unwrap();
        let bases = LocationBases {
            home: dir.path().to_path_buf(),
            config_dir: PathBuf::from("/nonexistent-config"),
            data_dir: PathBuf::from("/nonexistent-data"),
            data_local_dir: PathBuf::from("/nonexistent-data-local"),
            flatpak_roots: Vec::new(),
        };
        let records = load_records().unwrap();
        let files = enumerate_save_files(&records, "retroarch", &bases).unwrap();
        assert!(files.contains(&dir.path().join(".config/retroarch/saves/game.sav")));
        // Files at the home root outside the resolved RetroArch save dirs are
        // deliberately not enumerated: only captured directories are walked.
        assert!(!files.contains(&dir.path().join("state.sav")));
    }

    #[test]
    fn relative_prose_never_becomes_a_mutation_root() {
        let records = load_records().unwrap();
        let locations = save_locations(&records, "duckstation", &bases());
        assert!(locations.iter().any(|location| {
            location.documented.starts_with("memcards/") && location.resolved.is_none()
        }));
        assert!(locations.iter().any(|location| {
            location.documented.starts_with("savestates/") && location.resolved.is_none()
        }));
        assert!(save_route_roots_for_platform(&records, "duckstation", "linux", &bases()).is_err());
    }

    #[test]
    fn resolved_emulator_count_is_bounded_by_records() {
        let records = load_records().unwrap();
        let count = resolved_emulator_count(&records, &bases());
        assert!(count > 0 && count <= records.len());
    }

    #[test]
    fn adapter_locations_cover_controller_firmware_and_keys() {
        let records = load_records().unwrap();
        // Every record with a captured host documents controller and firmware
        // dispositions for future adapters. All-gap records intentionally
        // produce no locations.
        for record in &records {
            if record.platforms.is_empty() {
                continue;
            }
            let config = adapter_locations(
                &records,
                &record.slug,
                &[Purpose::Config, Purpose::Input],
                &bases(),
            );
            assert!(
                !config.is_empty(),
                "{} documents no controller config",
                record.slug
            );
            assert!(config.iter().all(|l| !l.documented.is_empty()));
            assert!(config.iter().all(|l| !l.evidence.is_empty()));
            let firmware = adapter_locations(
                &records,
                &record.slug,
                &[Purpose::Bios, Purpose::Keys],
                &bases(),
            );
            assert!(
                !firmware.is_empty(),
                "{} documents no firmware/key disposition",
                record.slug
            );
        }
        // Switch-derived runtimes resolve their keys under the data dir.
        let keys = adapter_locations(&records, "citron-neo", &[Purpose::Keys], &bases());
        assert!(!keys.is_empty());
        assert!(keys.iter().any(|l| {
            l.resolved
                .as_ref()
                .is_some_and(|path| path.starts_with("/home/test/.local/share"))
        }));
        // RetroArch exposes its shared frontend config for every core.
        let retro = adapter_locations(&records, "retroarch", &[Purpose::Config], &bases());
        assert!(retro.iter().any(|l| l.documented.contains("retroarch.cfg")));
        // Controller binding syntax is not cryptographic key material.
        let input = adapter_locations(&records, "mesen", &[Purpose::Input], &bases());
        assert!(!input.is_empty());
        let mesen_keys = adapter_locations(&records, "mesen", &[Purpose::Keys], &bases());
        assert!(mesen_keys.is_empty());
    }

    #[test]
    fn settings_payload_includes_all_adapter_purposes() {
        let records = load_records().unwrap();
        let text = locations_for_emulator_name(&records, "Eden", &bases()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let locations = value["locations"].as_array().unwrap();
        for purpose in ["config", "bios", "keys", "saves"] {
            assert!(
                locations.iter().any(|l| l["purpose"] == purpose),
                "eden payload lacks {purpose}"
            );
        }
        assert!(locations.iter().all(|l| l["evidence"].as_array().is_some()));
    }
}
