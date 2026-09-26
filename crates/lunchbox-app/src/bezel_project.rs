//! The Bezel Project console and per-game arcade bezels for RetroArch launches.
//!
//! Packs are plain GitHub repos (`thebezelproject/bezelprojectsa-<Theme>`) of
//! per-ROM RetroArch overlay configs plus PNG artwork. Only the overlay file
//! needed for the launching ROM is fetched, cached under the Lunchbox data
//! directory, and attached through the launch-time private config. The packs
//! carry no explicit redistribution license, so artwork is fetched on the
//! user's machine at runtime and is never vendored into this repository.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

const PACK_RAW_BASE: &str = "https://raw.githubusercontent.com/thebezelproject/";
const PACK_API_BASE: &str = "https://api.github.com/repos/thebezelproject/";
const PACK_BRANCH: &str = "master";
const OVERLAY_ROOT_IN_PACK: &str = "retroarch/overlay/GameBezels";
const INDEX_FILE: &str = ".index.json";
const INDEX_TTL_SECS: u64 = 7 * 24 * 60 * 60;
const MAX_CONFIG_BYTES: usize = 64 * 1024;
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(20);
const ARCADE_RAW_BASE: &str = "https://raw.githubusercontent.com/thebezelproject/bezelproject-MAME/master/retroarch/overlay/ArcadeBezels";

/// Platform names are matched exactly after normalization; no fuzzy title
/// matching decides which artwork a game receives.
const THEME_ALIASES: &[(&str, &str)] = &[
    ("super nintendo entertainment system", "SNES"),
    ("super famicom", "SFC"),
    ("snes", "SNES"),
    ("nintendo entertainment system", "NES"),
    ("famicom", "NES"),
    ("famicom disk system", "FDS"),
    ("nes", "NES"),
    ("nintendo 64", "N64"),
    ("n64", "N64"),
    ("game boy color", "GBC"),
    ("game boy advance", "GBA"),
    ("game boy", "GB"),
    ("nintendo ds", "NDS"),
    ("virtual boy", "Virtualboy"),
    ("neo geo pocket color", "NGPC"),
    ("neo geo pocket", "NGP"),
    ("wonderswan color", "WonderSwanColor"),
    ("wonderswan", "WonderSwan"),
    ("sega pico", "Pico"),
    ("sega master system", "MasterSystem"),
    ("master system", "MasterSystem"),
    ("sega genesis", "MegaDrive"),
    ("mega drive", "MegaDrive"),
    ("genesis mega drive", "MegaDrive"),
    ("genesis", "MegaDrive"),
    ("sega cd", "SegaCD"),
    ("mega cd", "SegaCD"),
    ("sega 32x", "Sega32X"),
    ("32x", "Sega32X"),
    ("sg 1000", "SG-1000"),
    ("game gear", "GameGear"),
    ("sega saturn", "Saturn"),
    ("saturn", "Saturn"),
    ("dreamcast", "Dreamcast"),
    ("pc engine cd", "PCE-CD"),
    ("turbografx cd", "TG-CD"),
    ("pc engine", "PCEngine"),
    ("turbografx 16", "TG16"),
    ("turbografx", "TG16"),
    ("supergrafx", "SuperGrafx"),
    ("sony playstation", "PSX"),
    ("playstation", "PSX"),
    ("atari 2600", "Atari2600"),
    ("atari 5200", "Atari5200"),
    ("atari 7800", "Atari7800"),
    ("atari 800", "Atari800"),
    ("atari lynx", "AtariLynx"),
    ("atari jaguar", "AtariJaguar"),
    ("atari st", "AtariST"),
    ("colecovision", "ColecoVision"),
    ("intellivision", "Intellivision"),
    ("vectrex", "GCEVectrex"),
    ("magnavox odyssey 2", "Videopac"),
    ("videopac", "Videopac"),
    ("commodore 64", "C64"),
    ("c64", "C64"),
    ("amiga cd32", "CD32"),
    ("cdtv", "CDTV"),
    ("amiga", "Amiga"),
    ("msx2", "MSX2"),
    ("msx", "MSX"),
    ("x68000", "X68000"),
    ("zx spectrum", "ZXSpectrum"),
    ("zx81", "ZX81"),
    ("trs 80", "TRS-80"),
];

/// Console/handheld theme packs. Arcade artwork has its own ROM-set lookup
/// under ArcadeBezels rather than the console GameBezels directory layout.
pub fn theme_for_platform(platform: &str) -> Option<&'static str> {
    let normalized = normalize_name(platform);
    if normalized.is_empty() {
        return None;
    }
    THEME_ALIASES
        .iter()
        .find(|(alias, _)| *alias == normalized)
        .map(|(_, theme)| *theme)
}

fn normalize_name(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut separate = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if separate && !output.is_empty() {
                output.push(' ');
            }
            output.push(character.to_ascii_lowercase());
            separate = false;
        } else {
            separate = true;
        }
    }
    output
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct PackIndex {
    fetched_at: u64,
    directory: String,
    configs: Vec<String>,
    #[serde(default)]
    system_config: Option<String>,
    #[serde(default)]
    system_config_scanned: bool,
}

#[derive(Clone)]
struct BezelConfig {
    name: String,
    system: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackStyle {
    SystemArt,
    GameArt,
}

impl PackStyle {
    fn repository(self, theme: &str) -> String {
        let prefix = match self {
            Self::SystemArt => "bezelprojectsa-",
            Self::GameArt => "bezelproject-",
        };
        format!("{prefix}{theme}")
    }
}

/// Resolve (downloading on first use) the overlay config for one ROM stem.
/// A miss is a normal outcome - unnamed revisions and obscure regions have
/// no bezel - so `Ok(None)` simply means "no bezel for this game".
pub fn system_bezel_overlay(platform: &str, rom_stem: &str) -> Result<Option<PathBuf>> {
    bezel_overlay(platform, rom_stem, PackStyle::SystemArt)
}

pub fn bezel_overlay(platform: &str, rom_stem: &str, style: PackStyle) -> Result<Option<PathBuf>> {
    if arcade_bezels_supported(platform) {
        return arcade_bezel_overlay(rom_stem);
    }
    let Some(theme) = theme_for_platform(platform) else {
        return Ok(None);
    };
    if rom_stem.trim().is_empty() {
        return Ok(None);
    }
    let storage = match style {
        PackStyle::SystemArt => bezel_storage_directory()?.join(theme),
        PackStyle::GameArt => bezel_storage_directory()?.join("game-art").join(theme),
    };
    fs::create_dir_all(&storage).with_context(|| {
        format!(
            "creating the {} system bezel directory {}",
            theme,
            storage.display()
        )
    })?;

    let selection = match cached_config(&storage, rom_stem) {
        Some(selection) => selection,
        None => match resolve_pack_config_name(theme, &storage, rom_stem, style)? {
            Some(selection) => {
                fetch_config(theme, &storage, &selection, style)?;
                selection
            }
            None => return Ok(None),
        },
    };
    let config_path = storage.join(&selection.name);
    let png_name = ensure_local_png(theme, &storage, &config_path, selection.system, style)?;
    rewrite_overlay_path(&config_path, &png_name)?;
    Ok(Some(config_path))
}

pub fn arcade_bezels_supported(platform: &str) -> bool {
    matches!(
        normalize_name(platform).as_str(),
        "arcade"
            | "arcade pinball"
            | "arcade laserdisc"
            | "mame"
            | "finalburn neo"
            | "fbneo"
            | "snk neo geo mvs"
            | "neo geo mvs"
            | "sega naomi"
            | "sega naomi 2"
            | "sammy atomiswave"
            | "sega model 2"
            | "sega model 3"
    )
}

fn arcade_romset_key(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    .then(|| name.to_ascii_lowercase())
}

pub fn arcade_bezel_overlay(rom_stem: &str) -> Result<Option<PathBuf>> {
    let storage = bezel_storage_directory()?.join("game-art").join("Arcade");
    arcade_bezel_at(&storage, rom_stem, &mut |url, limit| {
        fetch_optional_bytes(url, limit, Duration::from_secs(5))
    })
}

/// Fetch two exact files, never the enormous (and truncated) recursive MAME
/// pack index. A cached miss avoids repeating a network request every launch.
fn arcade_bezel_at(
    storage: &Path,
    rom_stem: &str,
    fetch: &mut impl FnMut(&str, u64) -> Result<Option<Vec<u8>>>,
) -> Result<Option<PathBuf>> {
    let Some(key) = arcade_romset_key(rom_stem) else {
        return Ok(None);
    };
    fs::create_dir_all(storage)?;
    let config_path = storage.join(format!("{key}.cfg"));
    let missing = storage.join(format!(".{key}.missing"));
    let cached = fs::read_to_string(&config_path).ok();
    if cached.is_none()
        && fs::read_to_string(&missing)
            .ok()
            .and_then(|text| text.parse::<u64>().ok())
            .is_some_and(|when| unix_timestamp().saturating_sub(when) < INDEX_TTL_SECS)
    {
        return Ok(None);
    }
    let config = match cached {
        Some(config) => config,
        None => {
            let Some(bytes) = fetch(
                &format!("{ARCADE_RAW_BASE}/{key}.cfg"),
                MAX_CONFIG_BYTES as u64,
            )?
            else {
                fs::write(missing, unix_timestamp().to_string())?;
                return Ok(None);
            };
            String::from_utf8(bytes).context("the arcade bezel config was not UTF-8")?
        }
    };
    let image = png_reference(&config).context("the arcade bezel references no image")?;
    let image_stem = image
        .strip_suffix(".png")
        .context("the arcade bezel image is not a PNG")?;
    anyhow::ensure!(
        arcade_romset_key(image_stem).as_deref() == Some(image_stem),
        "the arcade bezel references an unsafe image path"
    );
    let image_path = storage.join(&image);
    if !image_path.is_file() {
        let bytes = fetch(&format!("{ARCADE_RAW_BASE}/{image}"), MAX_IMAGE_BYTES)?
            .context("the arcade bezel's image is unavailable")?;
        anyhow::ensure!(
            crate::bezel_orionsangel::png_dimensions(&bytes).is_some(),
            "the arcade bezel image has no valid PNG dimensions"
        );
        fs::write(image_path, bytes)?;
    }
    // Import artwork only, not the pack's emulator paths, input bindings or
    // viewport overrides. Clones may legitimately reference a parent's PNG.
    if !config_path.is_file() {
        fs::write(
            &config_path,
            format!(
                "overlays = 1\noverlay0_overlay = \"{image}\"\noverlay0_full_screen = true\noverlay0_descs = 0\n"
            ),
        )?;
    }
    Ok(Some(config_path))
}

fn cached_config(storage: &Path, rom_stem: &str) -> Option<BezelConfig> {
    let exact = format!("{rom_stem}.cfg");
    if storage.join(&exact).is_file() {
        return Some(BezelConfig {
            name: exact,
            system: false,
        });
    }
    let normalized = normalize_name(rom_stem);
    let index = read_index(storage)?;
    let game = index
        .configs
        .iter()
        .find(|config| normalize_name(config.strip_suffix(".cfg").unwrap_or(config)) == normalized)
        .cloned();
    if let Some(name) = game {
        return storage.join(&name).is_file().then_some(BezelConfig {
            name,
            system: false,
        });
    }
    index
        .system_config
        .filter(|name| storage.join(name).is_file())
        .map(|name| BezelConfig { name, system: true })
}

fn resolve_pack_config_name(
    theme: &str,
    storage: &Path,
    rom_stem: &str,
    style: PackStyle,
) -> Result<Option<BezelConfig>> {
    let index = match read_index(storage) {
        Some(index) if !index_expired(&index) && index.system_config_scanned => index,
        _ => fetch_index(theme, style).map_err(|error| {
            anyhow::anyhow!("listing The Bezel Project {} pack: {error:#}", theme)
        })?,
    };
    write_index(storage, &index)?;
    let normalized = normalize_name(rom_stem);
    Ok(index
        .configs
        .into_iter()
        .find(|config| normalize_name(config.strip_suffix(".cfg").unwrap_or(config)) == normalized)
        .map(|name| BezelConfig {
            name,
            system: false,
        })
        .or_else(|| {
            index
                .system_config
                .map(|name| BezelConfig { name, system: true })
        }))
}

fn fetch_index(theme: &str, style: PackStyle) -> Result<PackIndex> {
    let directory = pack_overlay_directory(theme, style)?;
    // The contents API caps directory listings at 1000 entries, which would
    // silently drop every game past the cutoff, so walk the recursive git
    // tree instead.
    let url = format!(
        "{PACK_API_BASE}{}/git/trees/{PACK_BRANCH}?recursive=1",
        style.repository(theme)
    );
    let text = fetch_text(&url, 32 * 1024 * 1024)?;
    let value: serde_json::Value =
        serde_json::from_str(&text).context("parsing the Bezel Project pack tree")?;
    if value
        .get("truncated")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        bail!("the {theme} pack tree listing was truncated");
    }
    let prefix = format!("{OVERLAY_ROOT_IN_PACK}/{directory}/");
    let configs = tree_configs(&value, &prefix);
    if configs.is_empty() {
        bail!("the {} pack lists no per-game bezel configs", theme);
    }
    Ok(PackIndex {
        fetched_at: unix_timestamp(),
        directory,
        configs,
        system_config: tree_system_config(&value),
        system_config_scanned: true,
    })
}

fn tree_system_config(value: &serde_json::Value) -> Option<String> {
    let mut configs = tree_configs(value, "retroarch/overlay/");
    if configs.len() == 1 {
        configs.pop()
    } else {
        None
    }
}

/// Config file names under one overlay directory in a recursive git-tree
/// listing. Kept separate from the fetch so the cutoff-prone filtering is
/// directly testable.
fn tree_configs(value: &serde_json::Value, prefix: &str) -> Vec<String> {
    let mut configs = Vec::new();
    for entry in value
        .get("tree")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let kind = entry
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let path = entry
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let Some(name) = path.strip_prefix(prefix) else {
            continue;
        };
        if kind == "blob" && name.ends_with(".cfg") && !name.contains('/') && !name.starts_with('.')
        {
            configs.push(name.to_owned());
        }
    }
    configs
}

/// The directory inside the pack is usually the theme name, but the exact
/// casing lives in the repo, so it is resolved through the pack listing.
fn pack_overlay_directory(theme: &str, style: PackStyle) -> Result<String> {
    let url = format!(
        "{PACK_API_BASE}{}/contents/{OVERLAY_ROOT_IN_PACK}?ref={PACK_BRANCH}",
        style.repository(theme)
    );
    let text = fetch_text(&url, 1024 * 1024)?;
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(&text).context("parsing the Bezel Project pack directories")?;
    let theme_key = normalize_name(theme);
    for entry in entries {
        let Some(name) = entry.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if entry
            .get("type")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|kind| kind == "dir")
            && normalize_name(name) == theme_key
        {
            return Ok(name.to_owned());
        }
    }
    bail!("the {theme} pack has no {} directory", OVERLAY_ROOT_IN_PACK)
}

fn fetch_config(
    theme: &str,
    storage: &Path,
    selection: &BezelConfig,
    style: PackStyle,
) -> Result<()> {
    let directory = read_index(storage)
        .map(|index| index.directory)
        .unwrap_or_else(|| theme.to_owned());
    let prefix = if selection.system {
        "retroarch/overlay".to_owned()
    } else {
        format!("{OVERLAY_ROOT_IN_PACK}/{directory}")
    };
    let url = format!(
        "{PACK_RAW_BASE}{}/{PACK_BRANCH}/{prefix}/{}",
        style.repository(theme),
        percent_encode(&selection.name)
    );
    let text = fetch_text(&url, MAX_CONFIG_BYTES as u64)?;
    if !text.contains("overlay0_overlay") {
        bail!(
            "the {} bezel config has no overlay image reference",
            selection.name
        );
    }
    fs::write(storage.join(&selection.name), text)
        .with_context(|| format!("saving the {} bezel config", selection.name))?;
    Ok(())
}

/// Download the PNG the config references (if it is not cached yet) and
/// return its plain file name for the rewritten config.
fn ensure_local_png(
    theme: &str,
    storage: &Path,
    config_path: &Path,
    system: bool,
    style: PackStyle,
) -> Result<String> {
    let config = fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let remote = png_reference(&config).context("the bezel config references no overlay image")?;
    let png_name = remote
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(remote.as_str())
        .to_owned();
    if png_name.is_empty()
        || png_name.starts_with('.')
        || png_name.contains('/')
        || png_name.contains('\\')
    {
        bail!("the bezel config references an unusable image name");
    }
    let local = storage.join(&png_name);
    if !local.is_file() {
        let directory = read_index(storage)
            .map(|index| index.directory)
            .unwrap_or_else(|| theme.to_owned());
        let prefix = if system {
            "retroarch/overlay".to_owned()
        } else {
            format!("{OVERLAY_ROOT_IN_PACK}/{directory}")
        };
        let url = format!(
            "{PACK_RAW_BASE}{}/{PACK_BRANCH}/{prefix}/{}",
            style.repository(theme),
            percent_encode(&remote)
        );
        let bytes = fetch_bytes(&url, MAX_IMAGE_BYTES)?;
        if bytes.len() < 8 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
            bail!("the bezel image for {png_name} is not a PNG file");
        }
        fs::write(&local, bytes)
            .with_context(|| format!("saving the {} bezel image", local.display()))?;
    }
    Ok(png_name)
}

fn png_reference(config: &str) -> Option<String> {
    for line in config.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("overlay0_overlay") {
            let value = value
                .trim()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}

/// Point the overlay at the cached image next to the config file.
fn rewrite_overlay_path(config_path: &Path, png_name: &str) -> Result<()> {
    let config = fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let mut rewritten = String::with_capacity(config.len());
    let mut changed = false;
    for line in config.lines() {
        if line.trim_start().starts_with("overlay0_overlay") {
            rewritten.push_str(&format!("overlay0_overlay = \"{png_name}\""));
            changed = true;
        } else {
            rewritten.push_str(line);
        }
        rewritten.push('\n');
    }
    if changed {
        fs::write(config_path, rewritten)
            .with_context(|| format!("updating {}", config_path.display()))?;
    }
    Ok(())
}

fn read_index(storage: &Path) -> Option<PackIndex> {
    let bytes = fs::read(storage.join(INDEX_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_index(storage: &Path, index: &PackIndex) -> Result<()> {
    fs::write(storage.join(INDEX_FILE), serde_json::to_vec(index)?)
        .context("saving the bezel pack index")?;
    Ok(())
}

fn index_expired(index: &PackIndex) -> bool {
    unix_timestamp().saturating_sub(index.fetched_at) > INDEX_TTL_SECS
}

fn bezel_storage_directory() -> Result<PathBuf> {
    directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("bezels"))
        .context("could not determine the Lunchbox data directory")
}

fn http_agent(timeout: Duration) -> Result<ureq::Agent> {
    Ok(ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .user_agent("Lunchbox/0.1 system bezel client")
        .build()
        .into())
}

fn fetch_text(url: &str, limit: u64) -> Result<String> {
    let bytes = fetch_bytes(url, limit)?;
    String::from_utf8(bytes).context("the bezel download was not UTF-8 text")
}

fn fetch_bytes(url: &str, limit: u64) -> Result<Vec<u8>> {
    fetch_optional_bytes(url, limit, DOWNLOAD_TIMEOUT)?.context("download returned HTTP 404")
}

fn fetch_optional_bytes(url: &str, limit: u64, timeout: Duration) -> Result<Option<Vec<u8>>> {
    let agent = http_agent(timeout)?;
    let mut response = agent
        .get(url)
        .call()
        .with_context(|| format!("downloading {url}"))?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    if !response.status().is_success() {
        bail!("download returned HTTP {}", response.status());
    }
    let mut reader = response.body_mut().as_reader();
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .context("reading the bezel download")?;
    if bytes.len() as u64 > limit {
        bail!("download exceeded the size limit");
    }
    Ok(Some(bytes))
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_png() -> Vec<u8> {
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        header.extend_from_slice(&1920_u32.to_be_bytes());
        header.extend_from_slice(&1080_u32.to_be_bytes());
        header
    }

    #[test]
    fn arcade_bezels_use_exact_romsets_and_cache_their_art() {
        let temporary = tempfile::tempdir().unwrap();
        let mut requests = Vec::new();
        let mut fetch = |url: &str, _: u64| {
            requests.push(url.to_owned());
            Ok(Some(if url.ends_with(".cfg") {
                b"overlay0_overlay = sf2.png\nvideo_scale = 99\n".to_vec()
            } else {
                test_png()
            }))
        };
        let path = arcade_bezel_at(temporary.path(), "SF2CEUA", &mut fetch)
            .unwrap()
            .unwrap();
        assert_eq!(
            requests,
            [
                format!("{ARCADE_RAW_BASE}/sf2ceua.cfg"),
                format!("{ARCADE_RAW_BASE}/sf2.png")
            ]
        );
        let config = fs::read_to_string(&path).unwrap();
        assert!(config.contains("overlay0_overlay = \"sf2.png\""));
        assert!(!config.contains("video_scale"));
        assert_eq!(
            arcade_bezel_at(temporary.path(), "sf2ceua", &mut |_, _| panic!(
                "cache hit must not use network"
            ))
            .unwrap(),
            Some(path)
        );
    }

    #[test]
    fn arcade_bezel_misses_are_cached_but_network_errors_are_not() {
        let temporary = tempfile::tempdir().unwrap();
        let mut calls = 0;
        for _ in 0..2 {
            assert!(
                arcade_bezel_at(temporary.path(), "missinggame", &mut |_, _| {
                    calls += 1;
                    Ok(None)
                })
                .unwrap()
                .is_none()
            );
        }
        assert_eq!(calls, 1);
        fs::write(temporary.path().join(".missinggame.missing"), "0").unwrap();
        assert!(
            arcade_bezel_at(temporary.path(), "missinggame", &mut |_, _| {
                calls += 1;
                Ok(None)
            })
            .unwrap()
            .is_none()
        );
        assert_eq!(calls, 2);
        for _ in 0..2 {
            assert!(
                arcade_bezel_at(temporary.path(), "offlinegame", &mut |_, _| {
                    calls += 1;
                    bail!("offline")
                })
                .is_err()
            );
        }
        assert_eq!(calls, 4);
    }

    #[test]
    fn arcade_bezel_lookup_never_uses_display_titles_or_unsafe_paths() {
        let temporary = tempfile::tempdir().unwrap();
        for name in ["", "../sf2", "/sf2", "sf2\\child", "Street Fighter II"] {
            assert!(
                arcade_bezel_at(temporary.path(), name, &mut |_, _| panic!(
                    "invalid ROM-set name"
                ))
                .unwrap()
                .is_none()
            );
        }
        assert!(
            arcade_bezel_at(temporary.path(), "sf2", &mut |_, _| {
                Ok(Some(b"overlay0_overlay = ../outside.png".to_vec()))
            })
            .is_err()
        );
        for platform in [
            "Arcade",
            "MAME",
            "FinalBurn Neo",
            "Arcade Laserdisc",
            "SNK Neo Geo MVS",
        ] {
            assert!(arcade_bezels_supported(platform));
        }
        assert!(!arcade_bezels_supported("SNK Neo Geo Pocket"));
        assert!(!arcade_bezels_supported("Nintendo Entertainment System"));
    }

    #[test]
    #[ignore = "downloads only Bezel Project artwork, never games"]
    fn arcade_bezel_live_horizontal_and_vertical_game_art() {
        for romset in ["mslug", "pacman"] {
            let path = arcade_bezel_overlay(romset).unwrap().unwrap();
            let image = png_reference(&fs::read_to_string(&path).unwrap()).unwrap();
            let image = image::open(path.parent().unwrap().join(image)).unwrap();
            assert_eq!((image.width(), image.height()), (1920, 1080));
            println!("ARCADE_BEZEL romset={romset} config={}", path.display());
        }
    }

    #[test]
    fn platform_themes_match_on_exact_normalized_aliases() {
        assert_eq!(
            theme_for_platform("Super Nintendo Entertainment System"),
            Some("SNES")
        );
        assert_eq!(theme_for_platform("Genesis/Mega Drive"), Some("MegaDrive"));
        assert_eq!(theme_for_platform("game boy color"), Some("GBC"));
        assert_eq!(theme_for_platform("TurboGrafx-CD"), Some("TG-CD"));
        // Longer aliases win over their prefixes, and arcade platforms have
        // no system bezel pack.
        assert_eq!(theme_for_platform("Game Boy"), Some("GB"));
        assert_eq!(theme_for_platform("MAME"), None);
        assert_eq!(theme_for_platform(""), None);
    }

    #[test]
    fn config_references_are_reduced_to_local_file_names() {
        let config = "overlays = 1\noverlay0_overlay = \"GameBezels/SNES/Zelda.png\"\noverlay0_full_screen = true\noverlay0_descs = 0\n";
        assert_eq!(
            png_reference(config).as_deref(),
            Some("GameBezels/SNES/Zelda.png")
        );
    }

    #[test]
    fn percent_encoding_covers_reserved_characters() {
        assert_eq!(
            percent_encode("Zelda, The (USA).cfg"),
            "Zelda%2C%20The%20%28USA%29.cfg"
        );
    }

    #[test]
    fn tree_configs_keeps_late_alphabet_games_past_the_contents_cutoff() {
        let value = serde_json::json!({
            "truncated": false,
            "tree": [
                {"path": "retroarch/overlay/GameBezels/SNES/ActRaiser (USA).cfg", "type": "blob"},
                {"path": "retroarch/overlay/GameBezels/SNES/Super Metroid (USA).cfg", "type": "blob"},
                {"path": "retroarch/overlay/GameBezels/SNES/Super Metroid (USA).png", "type": "blob"},
                {"path": "retroarch/overlay/GameBezels/SNES/nested/Extra.cfg", "type": "blob"},
                {"path": "retroarch/overlay/GameBezels/SNES", "type": "tree"},
                {"path": "retroarch/overlay/GameBezels/NES/Zelda.cfg", "type": "blob"},
            ],
        });
        assert_eq!(
            tree_configs(&value, "retroarch/overlay/GameBezels/SNES/"),
            vec!["ActRaiser (USA).cfg", "Super Metroid (USA).cfg"],
        );
    }

    #[test]
    fn pack_without_a_game_bezel_uses_its_system_overlay() {
        let directory = tempfile::tempdir().unwrap();
        let index = PackIndex {
            fetched_at: unix_timestamp(),
            directory: "SNES".to_owned(),
            configs: vec!["ActRaiser (USA).cfg".to_owned()],
            system_config: Some("Super-Nintendo-Entertainment-System.cfg".to_owned()),
            system_config_scanned: true,
        };
        write_index(directory.path(), &index).unwrap();
        let selected = resolve_pack_config_name(
            "SNES",
            directory.path(),
            "Super Metroid (USA)",
            PackStyle::SystemArt,
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.name, "Super-Nintendo-Entertainment-System.cfg");
        assert!(selected.system);
    }

    #[test]
    fn tree_system_config_uses_only_a_unique_overlay_root_config() {
        let value = serde_json::json!({"tree": [
            {"path": "retroarch/overlay/Super-Nintendo-Entertainment-System.cfg", "type": "blob"},
            {"path": "retroarch/overlay/GameBezels/SNES/ActRaiser (USA).cfg", "type": "blob"}
        ]});
        assert_eq!(
            tree_system_config(&value).as_deref(),
            Some("Super-Nintendo-Entertainment-System.cfg")
        );
    }
}
