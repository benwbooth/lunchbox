use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const DEFAULT_THEME_ID: &str = "lunchbox-default";
const UI_PROBE_THEME_ID: &str = "ultraviolet-circuit";
const THEME_SCHEMA_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "theme.json";
const RECEIPT_FILE: &str = ".lunchbox-theme-owned.json";
const MAX_PACKAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_ENTRIES: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouchTheme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub background: String,
    pub panel: String,
    pub panel_raised: String,
    pub ink: String,
    pub muted: String,
    pub accent: String,
    pub accent_cool: String,
    pub danger: String,
    pub hero_scrim_percent: u8,
    pub card_radius: u8,
    pub background_path: Option<PathBuf>,
    pub installed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeCatalog {
    pub themes: Vec<CouchTheme>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeInstallResult {
    pub theme: CouchTheme,
    pub reused: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThemeManifest {
    schema_version: u32,
    id: String,
    name: String,
    author: String,
    description: String,
    palette: ThemePalette,
    #[serde(default)]
    background_image: Option<String>,
    #[serde(default = "default_hero_scrim")]
    hero_scrim_percent: u8,
    #[serde(default = "default_card_radius")]
    card_radius: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThemePalette {
    background: String,
    panel: String,
    panel_raised: String,
    ink: String,
    muted: String,
    accent: String,
    accent_cool: String,
    danger: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThemeReceipt {
    schema_version: u32,
    id: String,
    package_sha256: String,
    manifest_sha256: String,
    background_sha256: Option<String>,
}

struct ParsedPackage {
    manifest: ThemeManifest,
    manifest_bytes: Vec<u8>,
    background: Option<(PathBuf, Vec<u8>)>,
    package_sha256: String,
}

fn default_hero_scrim() -> u8 {
    62
}

fn default_card_radius() -> u8 {
    16
}

pub fn validate_theme_id(id: &str) -> Result<()> {
    if !(3..=64).contains(&id.len())
        || id.starts_with('-')
        || id.ends_with('-')
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("theme ID must be 3-64 lowercase letters, digits, or interior hyphens");
    }
    Ok(())
}

pub fn catalog_for_state(state_database: &Path) -> ThemeCatalog {
    let mut themes = built_in_themes();
    let mut warnings = Vec::new();
    let root = match theme_root(state_database) {
        Ok(root) => root,
        Err(error) => {
            warnings.push(error.to_string());
            return ThemeCatalog { themes, warnings };
        }
    };
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ThemeCatalog { themes, warnings };
        }
        Err(error) => {
            warnings.push(format!(
                "Could not inspect installed Couch Mode themes at {}: {error}",
                root.display()
            ));
            return ThemeCatalog { themes, warnings };
        }
    };
    let built_in_ids = themes
        .iter()
        .map(|theme| theme.id.clone())
        .collect::<HashSet<_>>();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("Could not inspect one installed theme: {error}"));
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                warnings.push(format!(
                    "Could not inspect theme path {}: {error}",
                    entry.path().display()
                ));
                continue;
            }
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        match load_installed_theme(&entry.path()) {
            Ok((theme, _)) if built_in_ids.contains(&theme.id) => warnings.push(format!(
                "Ignored installed theme {} because its ID is reserved by Lunchbox",
                theme.id
            )),
            Ok((theme, _)) => themes.push(theme),
            Err(error) => warnings.push(format!(
                "Ignored invalid installed theme at {}: {error:#}",
                entry.path().display()
            )),
        }
    }
    themes[3..].sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    ThemeCatalog { themes, warnings }
}

pub fn built_in_catalog() -> ThemeCatalog {
    ThemeCatalog {
        themes: built_in_themes(),
        warnings: Vec::new(),
    }
}

pub fn default_theme() -> CouchTheme {
    built_in_themes()
        .into_iter()
        .next()
        .expect("Lunchbox always has a built-in default theme")
}

pub(crate) fn seed_ui_probe() -> Result<()> {
    let state_database =
        crate::catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
            .filter(|path| !path.as_os_str().is_empty())
            .context(
                "the Couch Mode theme UI probe requires an explicit --state-database or LUNCHBOX_STATE_DATABASE",
            )?;
    seed_ui_probe_at(&state_database)
}

fn seed_ui_probe_at(state_database: &Path) -> Result<()> {
    let parent = state_database
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .context("state database path has no parent for the Couch Mode theme probe")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating theme probe directory {}", parent.display()))?;
    let package = parent.join(".couch-theme-ui-probe.lunchbox-theme");
    let seed_result = (|| -> Result<()> {
        write_ui_probe_package(&package)?;
        let installed = install_package(&package, state_database)?;
        if installed.theme.id != UI_PROBE_THEME_ID {
            bail!("theme probe installed an unexpected package identity");
        }
        let store = crate::settings::SettingsStore::at(state_database)?;
        let mut preferences = store.load_couch_mode_preferences()?;
        preferences.theme_id = UI_PROBE_THEME_ID.to_owned();
        store.save_couch_mode_preferences(&preferences)?;
        Ok(())
    })();
    let cleanup_result = match fs::remove_file(&package) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("removing temporary Couch Mode theme probe package"),
    };
    seed_result?;
    cleanup_result
}

fn write_ui_probe_package(path: &Path) -> Result<()> {
    let manifest = ThemeManifest {
        schema_version: THEME_SCHEMA_VERSION,
        id: UI_PROBE_THEME_ID.to_owned(),
        name: "Ultraviolet Circuit".to_owned(),
        author: "Lunchbox Probe".to_owned(),
        description: "A deterministic package that proves installed Couch Mode themes reach the real Qt surface.".to_owned(),
        palette: ThemePalette {
            background: "#060714".to_owned(),
            panel: "#11142b".to_owned(),
            panel_raised: "#1b2142".to_owned(),
            ink: "#f7f5ff".to_owned(),
            muted: "#aaa7ca".to_owned(),
            accent: "#8c7bff".to_owned(),
            accent_cool: "#51dfca".to_owned(),
            danger: "#ff6f91".to_owned(),
        },
        background_image: None,
        hero_scrim_percent: 58,
        card_radius: 24,
    };
    let file = fs::File::create(path)
        .with_context(|| format!("creating theme probe package {}", path.display()))?;
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file(MANIFEST_FILE, zip::write::SimpleFileOptions::default())
        .context("starting theme probe manifest")?;
    writer
        .write_all(&serde_json::to_vec_pretty(&manifest)?)
        .context("writing theme probe manifest")?;
    writer.finish().context("finishing theme probe package")?;
    Ok(())
}

pub fn install_package(package: &Path, state_database: &Path) -> Result<ThemeInstallResult> {
    let parsed = parse_package(package)?;
    if built_in_themes()
        .iter()
        .any(|theme| theme.id == parsed.manifest.id)
    {
        bail!(
            "theme ID {} is reserved by a built-in Lunchbox theme",
            parsed.manifest.id
        );
    }
    let root = theme_root(state_database)?;
    fs::create_dir_all(&root)
        .with_context(|| format!("creating Couch Mode theme directory {}", root.display()))?;
    let target = root.join(&parsed.manifest.id);
    if target.exists() {
        let (_, receipt) = load_installed_theme(&target).with_context(|| {
            format!(
                "refusing to replace unowned or invalid theme directory {}",
                target.display()
            )
        })?;
        if receipt.package_sha256 == parsed.package_sha256 {
            return Ok(ThemeInstallResult {
                theme: manifest_theme(&parsed.manifest, Some(&target), true)?,
                reused: true,
            });
        }
    }

    let staging = root.join(format!(".install-{}", Uuid::new_v4().simple()));
    fs::create_dir(&staging)
        .with_context(|| format!("creating theme staging directory {}", staging.display()))?;
    let install_result = (|| -> Result<()> {
        fs::write(staging.join(MANIFEST_FILE), &parsed.manifest_bytes)
            .context("writing validated theme manifest")?;
        let background_sha256 = if let Some((relative, bytes)) = &parsed.background {
            let output = staging.join(relative);
            fs::create_dir_all(
                output
                    .parent()
                    .context("theme background destination has no parent")?,
            )?;
            fs::write(&output, bytes).context("writing validated theme background")?;
            Some(sha256(bytes))
        } else {
            None
        };
        let receipt = ThemeReceipt {
            schema_version: THEME_SCHEMA_VERSION,
            id: parsed.manifest.id.clone(),
            package_sha256: parsed.package_sha256.clone(),
            manifest_sha256: sha256(&parsed.manifest_bytes),
            background_sha256,
        };
        let mut receipt_bytes = serde_json::to_vec_pretty(&receipt)?;
        receipt_bytes.push(b'\n');
        fs::write(staging.join(RECEIPT_FILE), receipt_bytes)
            .context("writing theme ownership receipt")?;
        publish_theme_directory(&root, &staging, &target)
    })();
    if install_result.is_err() {
        let _ = remove_exact_directory(&root, &staging, false);
    }
    install_result?;
    let (theme, _) = load_installed_theme(&target)?;
    Ok(ThemeInstallResult {
        theme,
        reused: false,
    })
}

pub fn remove_installed_theme(id: &str, state_database: &Path) -> Result<()> {
    validate_theme_id(id)?;
    if built_in_themes().iter().any(|theme| theme.id == id) {
        bail!("built-in Couch Mode themes cannot be removed");
    }
    let root = theme_root(state_database)?;
    let target = root.join(id);
    let (_, receipt) = load_installed_theme(&target).with_context(|| {
        format!(
            "refusing to remove unowned or invalid theme directory {}",
            target.display()
        )
    })?;
    if receipt.id != id {
        bail!("theme ownership receipt does not match the requested ID");
    }
    let tombstone = root.join(format!(".remove-{}", Uuid::new_v4().simple()));
    fs::rename(&target, &tombstone)
        .with_context(|| format!("retiring installed theme {}", target.display()))?;
    remove_exact_directory(&root, &tombstone, true)
}

fn parse_package(path: &Path) -> Result<ParsedPackage> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("inspecting theme package {}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_PACKAGE_BYTES {
        bail!("theme package must be a non-empty file no larger than 16 MiB");
    }
    let package_bytes =
        fs::read(path).with_context(|| format!("reading theme package {}", path.display()))?;
    let package_sha256 = sha256(&package_bytes);
    let mut archive = zip::ZipArchive::new(Cursor::new(&package_bytes))
        .context("opening theme package as ZIP")?;
    if archive.is_empty() || archive.len() > MAX_ENTRIES {
        bail!("theme package must contain between 1 and {MAX_ENTRIES} entries");
    }
    let mut entries = BTreeMap::<String, Vec<u8>>::new();
    let mut names = HashSet::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = safe_archive_name(entry.name())?;
        let folded = name.to_ascii_lowercase();
        if !names.insert(folded) {
            bail!("theme package contains duplicate or case-ambiguous paths");
        }
        if entry.is_dir() {
            continue;
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("theme package cannot contain symbolic links");
        }
        total = total
            .checked_add(entry.size())
            .context("theme package uncompressed size overflow")?;
        if total > MAX_UNCOMPRESSED_BYTES {
            bail!("theme package expands beyond the 32 MiB safety limit");
        }
        let entry_size = entry.size();
        let limit = usize::try_from(entry_size)
            .context("theme package entry is too large for this host")?;
        let mut bytes = Vec::with_capacity(limit.min(1024 * 1024));
        entry
            .by_ref()
            .take(entry_size.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() != limit {
            bail!("theme package entry length does not match its ZIP metadata");
        }
        entries.insert(name, bytes);
    }
    let manifest_bytes = entries
        .remove(MANIFEST_FILE)
        .context("theme package must contain a top-level theme.json")?;
    if manifest_bytes.is_empty() || manifest_bytes.len() > MAX_MANIFEST_BYTES {
        bail!("theme manifest must be a non-empty JSON file no larger than 64 KiB");
    }
    let manifest: ThemeManifest =
        serde_json::from_slice(&manifest_bytes).context("decoding theme.json")?;
    validate_manifest(&manifest)?;
    let background = if let Some(relative) = manifest.background_image.as_deref() {
        let path = safe_theme_asset_path(relative)?;
        let bytes = entries
            .remove(relative)
            .with_context(|| format!("theme background {relative} is missing from the package"))?;
        validate_background(&path, &bytes)?;
        Some((path, bytes))
    } else {
        None
    };
    if let Some((name, _)) = entries.into_iter().next() {
        bail!(
            "theme package contains unsupported file {name}; only theme.json and its declared background image are allowed"
        );
    }
    Ok(ParsedPackage {
        manifest,
        manifest_bytes,
        background,
        package_sha256,
    })
}

fn load_installed_theme(path: &Path) -> Result<(CouchTheme, ThemeReceipt)> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting installed theme {}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("installed theme path is not an owned directory");
    }
    let manifest_bytes = read_bounded(&path.join(MANIFEST_FILE), MAX_MANIFEST_BYTES)?;
    let manifest: ThemeManifest =
        serde_json::from_slice(&manifest_bytes).context("decoding installed theme manifest")?;
    validate_manifest(&manifest)?;
    if path.file_name().and_then(|name| name.to_str()) != Some(manifest.id.as_str()) {
        bail!("installed theme directory does not match its manifest ID");
    }
    let receipt_bytes = read_bounded(&path.join(RECEIPT_FILE), MAX_MANIFEST_BYTES)?;
    let receipt: ThemeReceipt =
        serde_json::from_slice(&receipt_bytes).context("decoding theme ownership receipt")?;
    if receipt.schema_version != THEME_SCHEMA_VERSION
        || receipt.id != manifest.id
        || receipt.manifest_sha256 != sha256(&manifest_bytes)
    {
        bail!("theme ownership receipt does not match the installed manifest");
    }
    if let Some(relative) = manifest.background_image.as_deref() {
        let relative = safe_theme_asset_path(relative)?;
        let bytes = read_bounded(&path.join(&relative), MAX_PACKAGE_BYTES as usize)?;
        validate_background(&relative, &bytes)?;
        if receipt.background_sha256.as_deref() != Some(sha256(&bytes).as_str()) {
            bail!("installed theme background does not match its ownership receipt");
        }
    } else if receipt.background_sha256.is_some() {
        bail!("theme receipt records an undeclared background image");
    }
    Ok((manifest_theme(&manifest, Some(path), true)?, receipt))
}

fn validate_manifest(manifest: &ThemeManifest) -> Result<()> {
    if manifest.schema_version != THEME_SCHEMA_VERSION {
        bail!(
            "unsupported theme schema {}; expected {}",
            manifest.schema_version,
            THEME_SCHEMA_VERSION
        );
    }
    validate_theme_id(&manifest.id)?;
    validate_text("theme name", &manifest.name, 1, 80)?;
    validate_text("theme author", &manifest.author, 1, 80)?;
    validate_text("theme description", &manifest.description, 1, 240)?;
    for (label, color) in [
        ("background", &manifest.palette.background),
        ("panel", &manifest.palette.panel),
        ("panel_raised", &manifest.palette.panel_raised),
        ("ink", &manifest.palette.ink),
        ("muted", &manifest.palette.muted),
        ("accent", &manifest.palette.accent),
        ("accent_cool", &manifest.palette.accent_cool),
        ("danger", &manifest.palette.danger),
    ] {
        validate_color(label, color)?;
    }
    if !(20..=90).contains(&manifest.hero_scrim_percent) {
        bail!("theme hero scrim must be between 20 and 90 percent");
    }
    if !(4..=32).contains(&manifest.card_radius) {
        bail!("theme card radius must be between 4 and 32 pixels");
    }
    if let Some(path) = manifest.background_image.as_deref() {
        safe_theme_asset_path(path)?;
    }
    Ok(())
}

fn validate_text(label: &str, value: &str, minimum: usize, maximum: usize) -> Result<()> {
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    if !(minimum..=maximum).contains(&length)
        || value.chars().any(|character| character.is_control())
    {
        bail!("{label} must contain {minimum}-{maximum} printable characters");
    }
    Ok(())
}

fn validate_color(label: &str, value: &str) -> Result<()> {
    let digits = value.strip_prefix('#').unwrap_or_default();
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("theme {label} color must be #RRGGBB or #AARRGGBB");
    }
    Ok(())
}

fn safe_archive_name(value: &str) -> Result<String> {
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        bail!("theme package contains an unsafe archive path");
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            !matches!(component, Component::Normal(_))
                && !(matches!(component, Component::CurDir) && value.ends_with('/'))
        })
    {
        bail!("theme package contains an unsafe archive path {value}");
    }
    let normalized = value.trim_end_matches('/');
    if normalized.is_empty() {
        bail!("theme package contains an empty archive path");
    }
    Ok(normalized.to_owned())
}

fn safe_theme_asset_path(value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        bail!("theme background path is unsafe");
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || path.components().count() > 4
    {
        bail!("theme background must be a short portable relative path");
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        bail!("theme background must be PNG, JPEG, or WebP");
    }
    Ok(path.to_path_buf())
}

fn validate_background(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PACKAGE_BYTES as usize {
        bail!("theme background must be non-empty and no larger than 16 MiB");
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let signature_matches = match extension.as_str() {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => bytes.starts_with(b"\xff\xd8\xff"),
        "webp" => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        _ => false,
    };
    if !signature_matches {
        bail!("theme background bytes do not match the declared image format");
    }
    Ok(())
}

fn manifest_theme(
    manifest: &ThemeManifest,
    installed_root: Option<&Path>,
    installed: bool,
) -> Result<CouchTheme> {
    validate_manifest(manifest)?;
    Ok(CouchTheme {
        id: manifest.id.clone(),
        name: manifest.name.trim().to_owned(),
        author: manifest.author.trim().to_owned(),
        description: manifest.description.trim().to_owned(),
        background: manifest.palette.background.to_ascii_lowercase(),
        panel: manifest.palette.panel.to_ascii_lowercase(),
        panel_raised: manifest.palette.panel_raised.to_ascii_lowercase(),
        ink: manifest.palette.ink.to_ascii_lowercase(),
        muted: manifest.palette.muted.to_ascii_lowercase(),
        accent: manifest.palette.accent.to_ascii_lowercase(),
        accent_cool: manifest.palette.accent_cool.to_ascii_lowercase(),
        danger: manifest.palette.danger.to_ascii_lowercase(),
        hero_scrim_percent: manifest.hero_scrim_percent,
        card_radius: manifest.card_radius,
        background_path: installed_root
            .zip(manifest.background_image.as_deref())
            .map(|(root, relative)| root.join(relative)),
        installed,
    })
}

fn built_in_themes() -> Vec<CouchTheme> {
    [
        ThemeManifest {
            schema_version: THEME_SCHEMA_VERSION,
            id: DEFAULT_THEME_ID.to_owned(),
            name: "Lunchbox".to_owned(),
            author: "Lunchbox".to_owned(),
            description: "Warm amber highlights, deep navy surfaces, and restrained living-room contrast.".to_owned(),
            palette: ThemePalette {
                background: "#080c12".to_owned(),
                panel: "#101823".to_owned(),
                panel_raised: "#182230".to_owned(),
                ink: "#f8fafc".to_owned(),
                muted: "#a4adbb".to_owned(),
                accent: "#ffab52".to_owned(),
                accent_cool: "#64d8c8".to_owned(),
                danger: "#ff7b8a".to_owned(),
            },
            background_image: None,
            hero_scrim_percent: 62,
            card_radius: 16,
        },
        ThemeManifest {
            schema_version: THEME_SCHEMA_VERSION,
            id: "midnight-blue".to_owned(),
            name: "Midnight Blue".to_owned(),
            author: "Lunchbox".to_owned(),
            description: "A cool cinema palette with crisp blue focus and quieter secondary chrome.".to_owned(),
            palette: ThemePalette {
                background: "#050914".to_owned(),
                panel: "#0c1426".to_owned(),
                panel_raised: "#14213b".to_owned(),
                ink: "#f4f8ff".to_owned(),
                muted: "#96a8c4".to_owned(),
                accent: "#6ca8ff".to_owned(),
                accent_cool: "#70e1d1".to_owned(),
                danger: "#ff8193".to_owned(),
            },
            background_image: None,
            hero_scrim_percent: 66,
            card_radius: 14,
        },
        ThemeManifest {
            schema_version: THEME_SCHEMA_VERSION,
            id: "arcade-afterglow".to_owned(),
            name: "Arcade Afterglow".to_owned(),
            author: "Lunchbox".to_owned(),
            description: "Magenta and cyan accents inspired by late-night arcade marquees without sacrificing readability.".to_owned(),
            palette: ThemePalette {
                background: "#100817".to_owned(),
                panel: "#1a1024".to_owned(),
                panel_raised: "#281635".to_owned(),
                ink: "#fff7ff".to_owned(),
                muted: "#c0a4c8".to_owned(),
                accent: "#ff6fc8".to_owned(),
                accent_cool: "#59e6e0".to_owned(),
                danger: "#ff7d72".to_owned(),
            },
            background_image: None,
            hero_scrim_percent: 68,
            card_radius: 20,
        },
    ]
    .into_iter()
    .map(|manifest| manifest_theme(&manifest, None, false).expect("built-in theme is valid"))
    .collect()
}

fn theme_root(state_database: &Path) -> Result<PathBuf> {
    let parent = state_database
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .context("state database path has no parent for Couch Mode themes")?;
    Ok(parent.join("themes"))
}

fn publish_theme_directory(root: &Path, staging: &Path, target: &Path) -> Result<()> {
    if staging.parent() != Some(root)
        || target.parent() != Some(root)
        || staging == root
        || target == root
    {
        bail!("refusing to publish a theme outside the exact theme root");
    }
    let backup = root.join(format!(".backup-{}", Uuid::new_v4().simple()));
    let had_target = target.exists();
    if had_target {
        fs::rename(target, &backup)
            .with_context(|| format!("backing up installed theme {}", target.display()))?;
    }
    if let Err(error) = fs::rename(staging, target) {
        if had_target {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).with_context(|| format!("publishing theme {}", target.display()));
    }
    if had_target {
        remove_exact_directory(root, &backup, true)?;
    }
    Ok(())
}

fn remove_exact_directory(root: &Path, path: &Path, must_exist: bool) -> Result<()> {
    if path.parent() != Some(root) || path == root {
        bail!("refusing to remove a path outside the exact theme root");
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?;
            Ok(())
        }
        Ok(_) => bail!("refusing to remove a non-directory theme path"),
        Err(error) if !must_exist && error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    let metadata =
        fs::metadata(path).with_context(|| format!("inspecting theme file {}", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum as u64 {
        bail!("theme file {} has an invalid size", path.display());
    }
    fs::read(path).with_context(|| format!("reading theme file {}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;

    fn manifest(id: &str, background: Option<&str>) -> ThemeManifest {
        ThemeManifest {
            schema_version: 1,
            id: id.to_owned(),
            name: "Fixture Theme".to_owned(),
            author: "Lunchbox Tests".to_owned(),
            description: "A deterministic declarative Couch Mode fixture.".to_owned(),
            palette: ThemePalette {
                background: "#080b16".to_owned(),
                panel: "#12182b".to_owned(),
                panel_raised: "#1b2440".to_owned(),
                ink: "#f7fbff".to_owned(),
                muted: "#9aa9c5".to_owned(),
                accent: "#7e8cff".to_owned(),
                accent_cool: "#55e0c2".to_owned(),
                danger: "#ff7188".to_owned(),
            },
            background_image: background.map(str::to_owned),
            hero_scrim_percent: 60,
            card_radius: 18,
        }
    }

    fn write_package(path: &Path, manifest: &ThemeManifest, extras: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file(MANIFEST_FILE, SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(&serde_json::to_vec_pretty(manifest).unwrap())
            .unwrap();
        for (name, bytes) in extras {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn installs_lists_reuses_updates_and_removes_owned_packages() {
        let directory = TempDir::new().unwrap();
        let state = directory.path().join("state.db");
        let package = directory.path().join("fixture.lunchbox-theme");
        write_package(&package, &manifest("fixture-theme", None), &[]);

        let installed = install_package(&package, &state).unwrap();
        assert!(!installed.reused);
        assert_eq!(installed.theme.id, "fixture-theme");
        assert!(installed.theme.installed);
        let reused = install_package(&package, &state).unwrap();
        assert!(reused.reused);

        let catalog = catalog_for_state(&state);
        assert!(catalog.warnings.is_empty());
        assert_eq!(catalog.themes.len(), 4);
        assert_eq!(catalog.themes[3].name, "Fixture Theme");

        let mut updated = manifest("fixture-theme", None);
        updated.name = "Updated Fixture".to_owned();
        write_package(&package, &updated, &[]);
        let replaced = install_package(&package, &state).unwrap();
        assert!(!replaced.reused);
        assert_eq!(replaced.theme.name, "Updated Fixture");

        remove_installed_theme("fixture-theme", &state).unwrap();
        assert_eq!(catalog_for_state(&state).themes.len(), 3);
        assert!(remove_installed_theme(DEFAULT_THEME_ID, &state).is_err());
    }

    #[test]
    fn ui_probe_seed_is_idempotent_and_selects_the_real_installed_theme() {
        let directory = TempDir::new().unwrap();
        let state = directory.path().join("state.db");

        seed_ui_probe_at(&state).unwrap();
        seed_ui_probe_at(&state).unwrap();

        let catalog = catalog_for_state(&state);
        assert!(catalog.warnings.is_empty());
        assert_eq!(catalog.themes.len(), 4);
        let theme = catalog
            .themes
            .iter()
            .find(|theme| theme.id == UI_PROBE_THEME_ID)
            .unwrap();
        assert_eq!(theme.accent, "#8c7bff");
        assert_eq!(theme.card_radius, 24);
        assert!(theme.installed);
        assert_eq!(
            crate::settings::SettingsStore::at(&state)
                .unwrap()
                .load_couch_mode_preferences()
                .unwrap()
                .theme_id,
            UI_PROBE_THEME_ID
        );
        assert!(
            !directory
                .path()
                .join(".couch-theme-ui-probe.lunchbox-theme")
                .exists()
        );
    }

    #[test]
    fn validates_background_signatures_and_rejects_executable_or_unsafe_content() {
        let directory = TempDir::new().unwrap();
        let state = directory.path().join("state.db");
        let package = directory.path().join("fixture.lunchbox-theme");
        write_package(
            &package,
            &manifest("background-theme", Some("assets/background.png")),
            &[("assets/background.png", b"not a png")],
        );
        assert!(install_package(&package, &state).is_err());

        write_package(
            &package,
            &manifest("script-theme", None),
            &[("Theme.qml", b"import QtQuick")],
        );
        let error = install_package(&package, &state).unwrap_err().to_string();
        assert!(error.contains("unsupported file"));

        let file = fs::File::create(&package).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("../theme.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap();
        assert!(install_package(&package, &state).is_err());
    }

    #[test]
    fn corrupt_or_unowned_directories_are_reported_and_never_replaced_or_removed() {
        let directory = TempDir::new().unwrap();
        let state = directory.path().join("state.db");
        let root = theme_root(&state).unwrap();
        let target = root.join("foreign-theme");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("notes.txt"), b"user data").unwrap();

        let package = directory.path().join("fixture.lunchbox-theme");
        write_package(&package, &manifest("foreign-theme", None), &[]);
        assert!(install_package(&package, &state).is_err());
        assert!(remove_installed_theme("foreign-theme", &state).is_err());
        assert_eq!(fs::read(target.join("notes.txt")).unwrap(), b"user data");
        assert_eq!(catalog_for_state(&state).warnings.len(), 1);
    }

    #[test]
    fn manifest_contract_rejects_bad_ids_colors_and_bounds() {
        let mut invalid = manifest("Bad ID", None);
        assert!(validate_manifest(&invalid).is_err());
        invalid = manifest("valid-id", None);
        invalid.palette.accent = "orange".to_owned();
        assert!(validate_manifest(&invalid).is_err());
        invalid = manifest("valid-id", None);
        invalid.hero_scrim_percent = 91;
        assert!(validate_manifest(&invalid).is_err());
        invalid = manifest("valid-id", None);
        invalid.card_radius = 3;
        assert!(validate_manifest(&invalid).is_err());
    }
}
