//! EmuMovies FTP client with archive and video support
//!
//! FTP access to EmuMovies media library.
//! Host: files.emumovies.com (or files2.emumovies.com for Europe)
//! Port: 21
//! Uses forum username/password for authentication.
//!
//! EmuMovies distributes artwork as archive packs (zip files) that must be
//! downloaded whole, then individual images extracted on demand.
//!
//! Videos are distributed as individual mp4 files and can be downloaded directly.
//!
//! FTP Structure:
//!   /Official/Artwork/{Platform}/{Platform} (Type)(Source)(Version).zip
//!   /Official/Video Snaps (HQ)/{Platform} (Video Snaps)(HQ)(...)/game.mp4

use crate::tags;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use suppaftp::FtpStream;

/// EmuMovies FTP configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmuMoviesConfig {
    /// EmuMovies forum username
    pub username: String,
    /// EmuMovies forum password
    pub password: String,
}

/// Media types available from EmuMovies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmuMoviesMediaType {
    BoxFront,
    BoxBack,
    Box3D,
    Screenshot,
    TitleScreen,
    CartFront,
    CartBack,
    Video,
    Manual,
    Fanart,
    ClearLogo,
    Banner,
}

impl EmuMoviesMediaType {
    /// Get the folder name pattern used in archive filenames
    pub fn archive_pattern(&self) -> &'static str {
        match self {
            EmuMoviesMediaType::BoxFront => "Boxes-2D",
            EmuMoviesMediaType::BoxBack => "Boxes-Back",
            EmuMoviesMediaType::Box3D => "Boxes-3D",
            EmuMoviesMediaType::Screenshot => "Snaps",
            EmuMoviesMediaType::TitleScreen => "Titles",
            EmuMoviesMediaType::CartFront => "Carts",
            EmuMoviesMediaType::CartBack => "Carts-Back",
            EmuMoviesMediaType::Video => "Video",
            EmuMoviesMediaType::Manual => "Manuals",
            EmuMoviesMediaType::Fanart => "Fanart",
            EmuMoviesMediaType::ClearLogo => "Logos",
            EmuMoviesMediaType::Banner => "Banners",
        }
    }

    /// Get the normalized filename for local cache
    pub fn cache_filename(&self) -> &'static str {
        match self {
            EmuMoviesMediaType::BoxFront => "box-front",
            EmuMoviesMediaType::BoxBack => "box-back",
            EmuMoviesMediaType::Box3D => "box-3d",
            EmuMoviesMediaType::Screenshot => "screenshot",
            EmuMoviesMediaType::TitleScreen => "title-screen",
            EmuMoviesMediaType::CartFront => "cart-front",
            EmuMoviesMediaType::CartBack => "cart-back",
            EmuMoviesMediaType::Video => "video",
            EmuMoviesMediaType::Manual => "manual",
            EmuMoviesMediaType::Fanart => "fanart",
            EmuMoviesMediaType::ClearLogo => "clear-logo",
            EmuMoviesMediaType::Banner => "banner",
        }
    }

    /// Convert from LaunchBox image type
    pub fn from_launchbox_type(image_type: &str) -> Option<Self> {
        match image_type {
            "Box - Front" => Some(EmuMoviesMediaType::BoxFront),
            "Box - Back" => Some(EmuMoviesMediaType::BoxBack),
            "Box - 3D" => Some(EmuMoviesMediaType::Box3D),
            "Screenshot - Gameplay" | "Screenshot" => Some(EmuMoviesMediaType::Screenshot),
            "Screenshot - Game Title" => Some(EmuMoviesMediaType::TitleScreen),
            "Cart - Front" => Some(EmuMoviesMediaType::CartFront),
            "Cart - Back" => Some(EmuMoviesMediaType::CartBack),
            "Fanart - Background" => Some(EmuMoviesMediaType::Fanart),
            "Clear Logo" => Some(EmuMoviesMediaType::ClearLogo),
            "Banner" => Some(EmuMoviesMediaType::Banner),
            _ => None,
        }
    }

    /// Check if this is a video type
    pub fn is_video(&self) -> bool {
        matches!(self, EmuMoviesMediaType::Video)
    }
}

fn normalize_emumovies_platform_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn tokenize_emumovies_platform_name(name: &str) -> Vec<String> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn dedupe_emumovies_platform_candidates(candidates: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for candidate in candidates {
        let key = normalize_emumovies_platform_key(&candidate);
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        deduped.push(candidate);
    }
    deduped
}

fn emumovies_platform_search_candidates(platform: &str) -> Vec<String> {
    let canonical = crate::arcade::canonicalize_platform_name(platform);
    let mut candidates = vec![platform.to_string()];
    if canonical != platform {
        candidates.push(canonical.to_string());
    }

    let key = normalize_emumovies_platform_key(&canonical);
    let platform_tokens = tokenize_emumovies_platform_name(&canonical);
    match key.as_str() {
        "arcade" | "mame" => candidates.push("MAME".to_string()),
        "nes" | "nintendoentertainmentsystem" => {
            candidates.push("Nintendo Entertainment System".to_string())
        }
        "snes" | "nintendosnes" | "supernintendoentertainmentsystem" => {
            candidates.push("Super Nintendo Entertainment System".to_string());
            candidates.push("Nintendo Super Nintendo Entertainment System".to_string());
            candidates.push("Nintendo Super Nintendo".to_string());
            candidates.push("Nintendo Super NES".to_string());
            candidates.push("Super Nintendo".to_string());
            candidates.push("Super NES".to_string());
            candidates.push("Nintendo Super Famicom".to_string());
            candidates.push("Super Famicom".to_string());
        }
        "nintendosnesmsu1" | "snesmsu1" | "snesmsu" => {
            candidates.push("Nintendo SNES MSU1".to_string())
        }
        "n64" | "nintendo64" => candidates.push("Nintendo 64".to_string()),
        "gba" | "nintendogameboyadvance" => {
            candidates.push("Nintendo Game Boy Advance".to_string())
        }
        "gbc" | "nintendogameboycolor" => {
            candidates.push("Nintendo Game Boy Color".to_string());
            candidates.push("Nintendo Gameboy Color".to_string());
        }
        "nds" | "nintendods" => candidates.push("Nintendo DS".to_string()),
        "3ds" | "nintendo3ds" => candidates.push("Nintendo 3DS".to_string()),
        "genesis" | "segagenesis" | "megadrive" | "segamegadrive" => {
            candidates.push("Sega Genesis - Mega Drive".to_string())
        }
        "segacd" | "megacd" => candidates.push("Sega CD".to_string()),
        "psx" | "ps1" => candidates.push("Sony PlayStation".to_string()),
        "ps2" => candidates.push("Sony PlayStation 2".to_string()),
        "ps3" => candidates.push("Sony PlayStation 3".to_string()),
        "psp" | "sonypsp" | "sonyplaystationportable" => {
            candidates.push("Sony Playstation Portable".to_string());
            candidates.push("Sony PSP".to_string());
        }
        "psvita" | "sonyplaystationvita" | "playstationvita" => {
            candidates.push("Sony PlayStation Vita".to_string())
        }
        "pcengine" | "necpcengine" | "turbografx16" | "tg16" | "necturbografx16" => {
            candidates.push("NEC PC Engine - Turbografx 16".to_string());
            candidates.push("NEC TurboGrafx 16".to_string());
        }
        "pcenginecd" | "necpcenginecd" | "turbografxcd" | "tgcd" | "necturbografxcd" => {
            candidates.push("NEC PC Engine CD - Turbografx CD".to_string());
            candidates.push("NEC TurboGrafx CD".to_string());
        }
        "pcenginesupergrafx" => candidates.push("NEC PC-Engine SuperGrafx".to_string()),
        "3dointeractivemultiplayer" => candidates.push("Panasonic 3DO".to_string()),
        _ => {}
    }

    if key.starts_with("arcade") || platform_tokens.iter().any(|token| token == "arcade") {
        candidates.push("MAME".to_string());
    }

    if key.starts_with("msdos") || platform_tokens.iter().any(|token| token == "dos") {
        candidates.push("Microsoft DOS".to_string());
    }

    dedupe_emumovies_platform_candidates(candidates)
}

/// Map common shorthand platform names to a primary EmuMovies search name.
/// Exact platform resolution for artwork and video happens against the live FTP
/// directory names using normalized aliases from `emumovies_platform_search_candidates`.
pub fn get_emumovies_system_folder(platform: &str) -> Option<&'static str> {
    let canonical = crate::arcade::canonicalize_platform_name(platform);
    let key = normalize_emumovies_platform_key(&canonical);
    let platform_tokens = tokenize_emumovies_platform_name(&canonical);

    match key.as_str() {
        "arcade" | "mame" => Some("MAME"),
        "nes" | "nintendoentertainmentsystem" => Some("Nintendo Entertainment System"),
        "snes" | "nintendosnes" | "supernintendoentertainmentsystem" => {
            Some("Super Nintendo Entertainment System")
        }
        "nintendosnesmsu1" | "snesmsu1" | "snesmsu" => Some("Nintendo SNES MSU1"),
        "n64" | "nintendo64" => Some("Nintendo 64"),
        "gba" | "nintendogameboyadvance" => Some("Nintendo Game Boy Advance"),
        "gbc" | "nintendogameboycolor" => Some("Nintendo Game Boy Color"),
        "nds" | "nintendods" => Some("Nintendo DS"),
        "3ds" | "nintendo3ds" => Some("Nintendo 3DS"),
        "genesis" | "segagenesis" | "megadrive" | "segamegadrive" => {
            Some("Sega Genesis - Mega Drive")
        }
        "segacd" | "megacd" => Some("Sega CD"),
        "psx" | "ps1" | "sonyplaystation" => Some("Sony PlayStation"),
        "ps2" | "sonyplaystation2" => Some("Sony PlayStation 2"),
        "ps3" | "sonyplaystation3" => Some("Sony PlayStation 3"),
        "psp" | "sonypsp" | "sonyplaystationportable" => Some("Sony Playstation Portable"),
        "psvita" | "sonyplaystationvita" | "playstationvita" => Some("Sony PlayStation Vita"),
        "pcengine" | "necpcengine" | "turbografx16" | "tg16" | "necturbografx16" => {
            Some("NEC PC Engine - Turbografx 16")
        }
        "pcenginecd" | "necpcenginecd" | "turbografxcd" | "tgcd" | "necturbografxcd" => {
            Some("NEC PC Engine CD - Turbografx CD")
        }
        "pcenginesupergrafx" => Some("NEC PC-Engine SuperGrafx"),
        key if key.starts_with("arcade")
            || platform_tokens.iter().any(|token| token == "arcade") =>
        {
            Some("MAME")
        }
        key if key.starts_with("msdos") || platform_tokens.iter().any(|token| token == "dos") => {
            Some("Microsoft DOS")
        }
        "3dointeractivemultiplayer" => Some("3DO Interactive Multiplayer"),
        _ => None,
    }
}

pub fn resolve_arcade_download_lookup_name<'a>(
    platform: &str,
    game_name: &'a str,
    launchbox_db_id: Option<i64>,
) -> Cow<'a, str> {
    let Some(launchbox_db_id) = launchbox_db_id else {
        return Cow::Borrowed(game_name);
    };

    let Some(system_folder) = get_emumovies_system_folder(platform) else {
        return Cow::Borrowed(game_name);
    };

    if system_folder != "MAME" {
        return Cow::Borrowed(game_name);
    }

    let lookup =
        crate::arcade::resolve_download_lookup_name(game_name, Some(launchbox_db_id), false);
    if lookup.as_ref() != game_name {
        tracing::info!(
            "Resolved arcade download lookup '{}' -> '{}' for LaunchBox DB id {}",
            game_name,
            lookup.as_ref(),
            launchbox_db_id
        );
    }
    lookup
}

pub fn resolve_arcade_download_lookup_name_for_torrent<'a>(
    platform: &str,
    game_name: &'a str,
    launchbox_db_id: Option<i64>,
    torrent_url: &str,
) -> Cow<'a, str> {
    let Some(launchbox_db_id) = launchbox_db_id else {
        return Cow::Borrowed(game_name);
    };

    let Some(system_folder) = get_emumovies_system_folder(platform) else {
        return Cow::Borrowed(game_name);
    };

    if system_folder != "MAME" {
        return Cow::Borrowed(game_name);
    }

    let use_parent_lookup = torrent_url.contains("MAME - ROMs (merged).torrent");
    let lookup = crate::arcade::resolve_download_lookup_name(
        game_name,
        Some(launchbox_db_id),
        use_parent_lookup,
    );
    if lookup.as_ref() != game_name {
        tracing::info!(
            "Resolved arcade download lookup '{}' -> '{}' for LaunchBox DB id {} using torrent {}",
            game_name,
            lookup.as_ref(),
            launchbox_db_id,
            torrent_url
        );
    }
    lookup
}

pub fn resolve_video_lookup_name<'a>(
    platform: &str,
    game_name: &'a str,
    launchbox_db_id: Option<i64>,
) -> Cow<'a, str> {
    let Some(launchbox_db_id) = launchbox_db_id else {
        return Cow::Borrowed(game_name);
    };

    let Some(system_folder) = get_emumovies_system_folder(platform) else {
        return Cow::Borrowed(game_name);
    };

    if system_folder != "MAME" {
        return Cow::Borrowed(game_name);
    }

    let lookup = crate::arcade::resolve_video_lookup_name(game_name, Some(launchbox_db_id));
    if lookup.as_ref() != game_name {
        tracing::info!(
            "Resolved arcade video lookup '{}' -> '{}' for LaunchBox DB id {}",
            game_name,
            lookup.as_ref(),
            launchbox_db_id
        );
    }
    lookup
}

// Prevent multiple threads from downloading/building the same archive at once.
static ARCHIVE_LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
static VIDEO_DOWNLOAD_LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
static ARTWORK_FOLDER_CACHE: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();
// Cache discovered video folders per normalized EmuMovies platform folder.
static VIDEO_FOLDER_CACHE: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
// Cache video indices per remote FTP folder path.
static VIDEO_INDEX_CACHE: OnceLock<Mutex<HashMap<String, Arc<Vec<VideoIndexEntry>>>>> =
    OnceLock::new();
static VIDEO_DOWNLOAD_PROGRESS: OnceLock<
    std::sync::RwLock<HashMap<String, VideoDownloadProgressState>>,
> = OnceLock::new();

const VIDEO_MATCH_CACHE_VERSION: &str = "5";
const VIDEO_INDEX_CACHE_VERSION: &str = "1";
const FTP_CONTROL_STALL_TIMEOUT: Duration = Duration::from_secs(45);
const FTP_DATA_STALL_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress: Option<f32>,
    pub stage: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
struct VideoDownloadProgressState {
    progress: VideoDownloadProgress,
    last_updated: Instant,
}

fn video_download_progress_map(
) -> &'static std::sync::RwLock<HashMap<String, VideoDownloadProgressState>> {
    VIDEO_DOWNLOAD_PROGRESS.get_or_init(|| std::sync::RwLock::new(HashMap::new()))
}

fn video_progress_key(game_cache_dir: &Path) -> Option<String> {
    game_cache_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
}

pub fn get_video_download_progress(game_cache_dir: &Path) -> Option<VideoDownloadProgress> {
    let key = video_progress_key(game_cache_dir)?;
    video_download_progress_map()
        .read()
        .ok()?
        .get(&key)
        .map(|state| state.progress.clone())
}

pub fn clear_video_download_progress(game_cache_dir: &Path) {
    let Some(key) = video_progress_key(game_cache_dir) else {
        return;
    };
    if let Ok(mut progress_map) = video_download_progress_map().write() {
        progress_map.remove(&key);
    }
}

fn update_video_download_progress(
    game_cache_dir: &Path,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) {
    let progress = total_bytes.map(|total| {
        if total == 0 {
            0.0
        } else {
            (downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0)
        }
    });
    set_video_download_progress(
        game_cache_dir,
        VideoDownloadProgress {
            downloaded_bytes,
            total_bytes,
            progress,
            stage: Some("downloading".to_string()),
            status: Some("Downloading video...".to_string()),
        },
    );
}

pub fn get_video_download_progress_age(game_cache_dir: &Path) -> Option<Duration> {
    let Some(key) = video_progress_key(game_cache_dir) else {
        return None;
    };
    let progress_map = video_download_progress_map().read().ok()?;
    let state = progress_map.get(&key)?;
    Some(state.last_updated.elapsed())
}

fn set_video_download_progress(game_cache_dir: &Path, progress: VideoDownloadProgress) {
    let Some(key) = video_progress_key(game_cache_dir) else {
        return;
    };
    if let Ok(mut progress_map) = video_download_progress_map().write() {
        progress_map.insert(
            key,
            VideoDownloadProgressState {
                progress,
                last_updated: Instant::now(),
            },
        );
    }
}

fn update_video_download_status(
    game_cache_dir: &Path,
    stage: impl Into<String>,
    status: impl Into<String>,
) {
    set_video_download_progress(
        game_cache_dir,
        VideoDownloadProgress {
            downloaded_bytes: 0,
            total_bytes: None,
            progress: None,
            stage: Some(stage.into()),
            status: Some(status.into()),
        },
    );
}

fn video_cache_version_path(game_cache_dir: &Path) -> PathBuf {
    game_cache_dir.join("emumovies").join("video.match-version")
}

pub fn is_video_cache_current(game_cache_dir: &Path) -> bool {
    std::fs::read_to_string(video_cache_version_path(game_cache_dir))
        .map(|v| v.trim() == VIDEO_MATCH_CACHE_VERSION)
        .unwrap_or(false)
}

fn write_video_cache_version(game_cache_dir: &Path) -> Result<()> {
    let version_path = video_cache_version_path(game_cache_dir);
    if let Some(parent) = version_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(version_path, VIDEO_MATCH_CACHE_VERSION)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideoIndexEntry {
    path: String,
    normalized: String,
    no_region: String,
    tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideoIndexCache {
    version: String,
    entries: Vec<VideoIndexEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoMatchKind {
    Exact,
    Regionless,
    Fuzzy,
}

fn video_match_kind_rank(kind: VideoMatchKind) -> u8 {
    match kind {
        VideoMatchKind::Exact => 3,
        VideoMatchKind::Regionless => 2,
        VideoMatchKind::Fuzzy => 1,
    }
}

fn compare_video_candidates(
    a_kind: VideoMatchKind,
    a_folder_rank: u8,
    a_score: f32,
    a_source_order: usize,
    b_kind: VideoMatchKind,
    b_folder_rank: u8,
    b_score: f32,
    b_source_order: usize,
) -> Ordering {
    video_match_kind_rank(a_kind)
        .cmp(&video_match_kind_rank(b_kind))
        // Lower folder rank is better (exact platform folder first, then variants).
        .then_with(|| b_folder_rank.cmp(&a_folder_rank))
        .then_with(|| a_score.partial_cmp(&b_score).unwrap_or(Ordering::Equal))
        // Lower source order is better (HQ before SQ).
        .then_with(|| b_source_order.cmp(&a_source_order))
}

fn video_folder_name(folder_path: &str) -> &str {
    folder_path.rsplit('/').next().unwrap_or(folder_path)
}

fn video_folder_platform_stem(folder_path: &str) -> &str {
    let folder_name = video_folder_name(folder_path);
    folder_name.split(" (").next().unwrap_or(folder_name).trim()
}

fn artwork_folder_name(folder_path: &str) -> &str {
    folder_path.rsplit('/').next().unwrap_or(folder_path)
}

fn exact_platform_key_match(name_a: &str, name_b: &str) -> bool {
    normalize_emumovies_platform_key(name_a) == normalize_emumovies_platform_key(name_b)
}

fn tokens_start_with(haystack: &[String], needle: &[String]) -> bool {
    haystack.len() >= needle.len() && haystack.iter().zip(needle.iter()).all(|(a, b)| a == b)
}

fn video_folder_match_rank(folder_path: &str, candidate: &str) -> Option<u8> {
    let stem = video_folder_platform_stem(folder_path);
    if exact_platform_key_match(stem, candidate) {
        return Some(0);
    }

    let stem_tokens = tokenize_emumovies_platform_name(stem);
    let candidate_tokens = tokenize_emumovies_platform_name(candidate);
    if candidate_tokens.is_empty() || stem_tokens.len() <= candidate_tokens.len() {
        return None;
    }

    if tokens_start_with(&stem_tokens, &candidate_tokens) {
        Some(1)
    } else {
        None
    }
}

fn video_folder_match_rank_for_platform(folder_path: &str, platform: &str) -> Option<u8> {
    emumovies_platform_search_candidates(platform)
        .iter()
        .enumerate()
        .find_map(|(idx, candidate)| {
            video_folder_match_rank(folder_path, candidate).and_then(|match_rank| {
                let base = idx.saturating_mul(2);
                if base > u8::MAX as usize {
                    None
                } else {
                    Some((base as u8).saturating_add(match_rank))
                }
            })
        })
}

fn select_artwork_folder_from_list<'a>(folders: &'a [String], platform: &str) -> Option<&'a str> {
    for candidate in emumovies_platform_search_candidates(platform) {
        if let Some(folder) = folders
            .iter()
            .find(|folder| exact_platform_key_match(artwork_folder_name(folder), &candidate))
        {
            return Some(folder.as_str());
        }
    }
    None
}

#[derive(Debug, Clone)]
struct VideoFolderCandidate {
    path: String,
    source_order: usize,
    match_rank: u8,
}

fn tokenize_for_match(normalized_name: &str) -> Vec<String> {
    normalized_name
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(canonicalize_match_token)
        .collect()
}

fn parse_roman_numeral(token: &str) -> Option<u32> {
    let upper = token.to_ascii_uppercase();
    if upper.is_empty() {
        return None;
    }
    if !upper
        .chars()
        .all(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'))
    {
        return None;
    }

    let value_of = |c| match c {
        'I' => 1,
        'V' => 5,
        'X' => 10,
        'L' => 50,
        'C' => 100,
        'D' => 500,
        'M' => 1000,
        _ => 0,
    };

    let chars: Vec<char> = upper.chars().collect();
    let mut total = 0u32;
    for i in 0..chars.len() {
        let cur = value_of(chars[i]);
        let next = chars.get(i + 1).map(|c| value_of(*c)).unwrap_or(0);
        if cur < next {
            total = total.saturating_sub(cur);
        } else {
            total = total.saturating_add(cur);
        }
    }

    // Keep roman parsing focused on sequel numerals to avoid false positives.
    if (1..=30).contains(&total) {
        Some(total)
    } else {
        None
    }
}

fn parse_sequence_number(token: &str) -> Option<u32> {
    if token.chars().all(|c| c.is_ascii_digit()) && token.len() <= 4 {
        return token.parse::<u32>().ok();
    }
    parse_roman_numeral(token)
}

fn canonicalize_match_token(token: &str) -> String {
    parse_sequence_number(token)
        .map(|n| format!("#{}", n))
        .unwrap_or_else(|| token.to_string())
}

fn extract_sequence_numbers(tokens: &[String]) -> HashSet<u32> {
    tokens
        .iter()
        .filter_map(|token| token.strip_prefix('#'))
        .filter_map(|num| num.parse::<u32>().ok())
        .collect()
}

fn dice_similarity(tokens_a: &[String], tokens_b: &[String]) -> f32 {
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<&str> = tokens_a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = tokens_b.iter().map(|s| s.as_str()).collect();
    if set_a.is_empty() || set_b.is_empty() {
        return 0.0;
    }

    let overlap = set_a.intersection(&set_b).count() as f32;
    if overlap == 0.0 {
        return 0.0;
    }

    (2.0 * overlap) / (set_a.len() as f32 + set_b.len() as f32)
}

fn find_best_video_match(
    entries: &[VideoIndexEntry],
    game_name: &str,
) -> Option<(String, VideoMatchKind, f32)> {
    const MIN_FUZZY_MATCH_SCORE: f32 = 0.70;
    const PREFIX_BOOST: f32 = 0.05;
    const MIN_FUZZY_MARGIN: f32 = 0.06;

    let game_normalized = normalize_game_name(game_name);
    let game_no_region = remove_region_codes(&game_normalized);
    let game_tokens = tokenize_for_match(&game_no_region);
    let game_sequence_numbers = extract_sequence_numbers(&game_tokens);

    for entry in entries {
        if entry.normalized == game_normalized {
            return Some((entry.path.clone(), VideoMatchKind::Exact, 1.0));
        }
    }

    for entry in entries {
        if entry.no_region == game_no_region {
            return Some((entry.path.clone(), VideoMatchKind::Regionless, 0.99));
        }
    }

    if game_tokens.is_empty() {
        return None;
    }

    let mut best: Option<(f32, &VideoIndexEntry)> = None;
    let mut second_best = 0.0f32;

    for entry in entries {
        let entry_sequence_numbers = extract_sequence_numbers(&entry.tokens);
        if !game_sequence_numbers.is_empty() && game_sequence_numbers != entry_sequence_numbers {
            continue;
        }

        let mut score = dice_similarity(&game_tokens, &entry.tokens);
        if score <= 0.0 {
            continue;
        }

        if entry.no_region.starts_with(&game_no_region)
            || game_no_region.starts_with(&entry.no_region)
        {
            score = (score + PREFIX_BOOST).min(1.0);
        }

        if let Some((best_score, _)) = best {
            if score > best_score {
                second_best = best_score;
                best = Some((score, entry));
            } else if score > second_best {
                second_best = score;
            }
        } else {
            best = Some((score, entry));
        }
    }

    best.and_then(|(score, entry)| {
        if score < MIN_FUZZY_MATCH_SCORE {
            return None;
        }
        if second_best > 0.0 && (score - second_best) < MIN_FUZZY_MARGIN {
            tracing::info!(
                "Rejecting ambiguous fuzzy match for '{}': best {:.3}, second {:.3}",
                game_name,
                score,
                second_best
            );
            return None;
        }
        Some((entry.path.clone(), VideoMatchKind::Fuzzy, score))
    })
}

fn get_archive_lock(archive_path: &Path) -> Arc<Mutex<()>> {
    let key = archive_path.to_string_lossy().to_string();
    let locks = ARCHIVE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = locks.lock().expect("archive lock map poisoned");
    map.entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn get_video_download_lock(game_cache_dir: &Path) -> Arc<Mutex<()>> {
    let key = game_cache_dir.to_string_lossy().to_string();
    let locks = VIDEO_DOWNLOAD_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = locks.lock().expect("video download lock map poisoned");
    map.entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Archive index entry - maps filenames in archive to their paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveIndex {
    /// Archive filename (e.g., "nes-boxes-2d.zip")
    pub archive_name: String,
    /// Map of normalized game names to archive entry paths
    pub entries: HashMap<String, String>,
    /// When the index was created
    pub created_at: String,
}

impl ArchiveIndex {
    /// Create a new archive index
    pub fn new(archive_name: String) -> Self {
        Self {
            archive_name,
            entries: HashMap::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Find a matching entry for a game name
    pub fn find_entry(&self, game_name: &str) -> Option<&String> {
        let normalized = normalize_game_name(game_name);

        // Try exact normalized match first
        if let Some(entry) = self.entries.get(&normalized) {
            return Some(entry);
        }

        // Try without region codes
        let no_region = remove_region_codes(&normalized);
        if no_region != normalized {
            if let Some(entry) = self.entries.get(&no_region) {
                return Some(entry);
            }
        }

        // Try fuzzy matching
        for (key, entry) in &self.entries {
            let key_no_region = remove_region_codes(key);
            if key_no_region == no_region {
                return Some(entry);
            }
        }

        None
    }
}

/// Progress callback type for downloads
pub type ProgressCallback = Box<dyn Fn(f32) + Send + Sync>;

/// EmuMovies FTP client
#[derive(Clone)]
pub struct EmuMoviesClient {
    config: EmuMoviesConfig,
    cache_dir: PathBuf,
}

const FTP_HOST: &str = "files.emumovies.com";
const FTP_PORT: u16 = 21;

impl EmuMoviesClient {
    /// Create a new EmuMovies client
    pub fn new(config: EmuMoviesConfig, cache_dir: PathBuf) -> Self {
        Self { config, cache_dir }
    }

    /// Check if the client has valid credentials
    pub fn has_credentials(&self) -> bool {
        !self.config.username.is_empty() && !self.config.password.is_empty()
    }

    /// Get the archives directory
    fn archives_dir(&self) -> PathBuf {
        self.cache_dir.join("emumovies-archives")
    }

    /// Get the archive path for a platform and media type
    fn get_archive_path(&self, platform: &str, media_type: EmuMoviesMediaType) -> PathBuf {
        let safe_platform = platform
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();

        let filename = format!(
            "{}-{}.zip",
            safe_platform,
            media_type.archive_pattern().to_lowercase()
        );
        self.archives_dir().join(filename)
    }

    /// Get the index path for an archive
    fn get_index_path(&self, archive_path: &Path) -> PathBuf {
        archive_path.with_extension("json")
    }

    fn video_index_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("emumovies-video-index")
    }

    fn video_index_cache_path(&self, video_folder: &str) -> PathBuf {
        let folder_name = video_folder_name(video_folder)
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let mut hasher = DefaultHasher::new();
        video_folder.hash(&mut hasher);
        let hash = hasher.finish();
        self.video_index_cache_dir()
            .join(format!("{folder_name}-{hash:016x}.json"))
    }

    /// Connect to FTP server
    fn connect(&self) -> Result<FtpStream> {
        let addr = format!("{}:{}", FTP_HOST, FTP_PORT);
        let mut ftp =
            FtpStream::connect(&addr).context("Failed to connect to EmuMovies FTP server")?;
        ftp.get_ref()
            .set_read_timeout(Some(FTP_CONTROL_STALL_TIMEOUT))
            .context("Failed to configure EmuMovies FTP read timeout")?;
        ftp.get_ref()
            .set_write_timeout(Some(FTP_CONTROL_STALL_TIMEOUT))
            .context("Failed to configure EmuMovies FTP write timeout")?;

        ftp.login(&self.config.username, &self.config.password)
            .context("FTP login failed - check username/password")?;

        Ok(ftp)
    }

    /// List files in a directory
    pub fn list_files(&self, path: &str) -> Result<Vec<String>> {
        self.list_files_with_progress(path, |_count| {})
    }

    fn list_files_with_progress<F>(&self, path: &str, mut on_progress: F) -> Result<Vec<String>>
    where
        F: FnMut(usize),
    {
        let mut ftp = self.connect()?;
        let (_response, data_stream) = ftp
            .custom_data_command(
                format!("NLST {}", path),
                &[suppaftp::Status::AboutToSend, suppaftp::Status::AlreadyOpen],
            )
            .context("Failed to list directory")?;
        data_stream
            .get_ref()
            .set_read_timeout(Some(FTP_DATA_STALL_TIMEOUT))
            .context("Failed to configure EmuMovies FTP data read timeout")?;
        data_stream
            .get_ref()
            .set_write_timeout(Some(FTP_DATA_STALL_TIMEOUT))
            .context("Failed to configure EmuMovies FTP data write timeout")?;

        let mut data_stream = BufReader::new(data_stream);
        let mut files = Vec::new();
        loop {
            let mut line_buf = vec![];
            match data_stream.read_until(b'\n', &mut line_buf) {
                Ok(0) => break,
                Ok(len) => {
                    let mut line = String::from_utf8_lossy(&line_buf[..len]).to_string();
                    if line.ends_with('\n') {
                        line.pop();
                    }
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    if line.is_empty() {
                        continue;
                    }
                    files.push(line);
                    on_progress(files.len());
                }
                Err(err) => {
                    let _ = ftp.close_data_connection(data_stream);
                    let _ = ftp.quit();
                    return Err(anyhow::anyhow!("Failed to list directory: {}", err));
                }
            }
        }
        ftp.close_data_connection(data_stream)
            .context("Failed to finalize directory listing")?;
        let _ = ftp.quit();
        Ok(files)
    }

    fn get_artwork_folders(&self) -> Result<Vec<String>> {
        if let Some(cached) = ARTWORK_FOLDER_CACHE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("artwork folder cache lock poisoned")
            .clone()
        {
            return Ok(cached);
        }

        let folders = self.list_files("/Official/Artwork")?;
        ARTWORK_FOLDER_CACHE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("artwork folder cache lock poisoned")
            .replace(folders.clone());
        Ok(folders)
    }

    fn find_artwork_folder(&self, platform: &str) -> Result<Option<String>> {
        let folders = self.get_artwork_folders()?;
        Ok(select_artwork_folder_from_list(&folders, platform).map(str::to_string))
    }

    /// Find the archive file for a platform and media type on the FTP server
    pub fn find_archive(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
    ) -> Result<Option<String>> {
        let Some(system_folder) = self.find_artwork_folder(platform)? else {
            tracing::info!("No EmuMovies artwork folder found for {}", platform);
            return Ok(None);
        };

        let artwork_path = format!("/Official/Artwork/{}", system_folder);
        let pattern = media_type.archive_pattern();

        tracing::info!("Searching for {} archives in {}", pattern, artwork_path);

        let files = self.list_files(&artwork_path)?;

        // Find an archive containing the pattern
        for file in &files {
            let filename = file.rsplit('/').next().unwrap_or(file);
            if filename.contains(pattern) && filename.ends_with(".zip") {
                tracing::info!("Found archive: {}", file);
                return Ok(Some(file.clone()));
            }
        }

        tracing::info!("No archive found matching pattern {}", pattern);
        Ok(None)
    }

    /// Download an archive from FTP with progress callback
    pub fn download_archive(
        &self,
        remote_path: &str,
        local_path: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        if local_path.exists() {
            tracing::info!("Archive already exists: {}", local_path.display());
            return Ok(());
        }

        // Create parent directories
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        tracing::info!(
            "Downloading archive: {} -> {}",
            remote_path,
            local_path.display()
        );

        let mut ftp = self.connect()?;
        ftp.transfer_type(suppaftp::types::FileType::Binary)?;

        // Get file size for progress reporting
        let _file_size = ftp.size(remote_path).ok();

        // Create a temp file path
        let temp_path = local_path.with_extension("tmp");

        // Download with progress tracking
        let data = ftp
            .retr_as_buffer(remote_path)
            .context(format!("Failed to download: {}", remote_path))?;

        let _ = ftp.quit();

        let bytes = data.into_inner();

        if let Some(progress_fn) = progress {
            progress_fn(1.0);
        }

        tracing::info!("Downloaded {} bytes", bytes.len());

        // Write to temp file then rename
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, local_path)?;

        Ok(())
    }

    /// Build or load an archive index
    pub fn get_or_build_index(&self, archive_path: &Path) -> Result<ArchiveIndex> {
        let index_path = self.get_index_path(archive_path);

        // Try to load existing index
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            if let Ok(index) = serde_json::from_str::<ArchiveIndex>(&content) {
                return Ok(index);
            }
        }

        // Build new index from archive
        tracing::info!("Building index for archive: {}", archive_path.display());

        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let archive_name = archive_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut index = ArchiveIndex::new(archive_name);

        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let entry_name = entry.name().to_string();

            // Skip directories
            if entry_name.ends_with('/') {
                continue;
            }

            // Extract just the filename for matching
            let filename = entry_name.rsplit('/').next().unwrap_or(&entry_name);

            // Remove extension and normalize
            let base_name = filename
                .rsplit_once('.')
                .map(|(name, _)| name)
                .unwrap_or(filename);

            let normalized = normalize_game_name(base_name);
            index.entries.insert(normalized, entry_name);
        }

        // Save index
        let json = serde_json::to_string_pretty(&index)?;
        std::fs::write(&index_path, json)?;

        tracing::info!("Built index with {} entries", index.entries.len());

        Ok(index)
    }

    /// Extract a specific file from an archive
    pub fn extract_from_archive(
        &self,
        archive_path: &Path,
        entry_path: &str,
        output_path: &Path,
    ) -> Result<()> {
        if output_path.exists() {
            return Ok(());
        }

        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entry = archive
            .by_name(entry_path)
            .context(format!("Entry not found in archive: {}", entry_path))?;

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut output = std::fs::File::create(output_path)?;
        std::io::copy(&mut entry, &mut output)?;

        tracing::info!("Extracted {} to {}", entry_path, output_path.display());

        Ok(())
    }

    /// Get media from archives - downloads archive if needed, extracts requested file
    pub fn get_media_from_archive(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
        game_cache_dir: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<PathBuf> {
        // Don't use archives for video
        if media_type.is_video() {
            anyhow::bail!("Use get_video() for video content");
        }

        let archive_path = self.get_archive_path(platform, media_type);
        let index = {
            let lock = get_archive_lock(&archive_path);
            let _guard = match lock.try_lock() {
                Ok(g) => g,
                Err(std::sync::TryLockError::WouldBlock) => {
                    anyhow::bail!(
                        "Archive setup already in progress for {}",
                        archive_path.display()
                    );
                }
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    anyhow::bail!("Archive setup lock poisoned for {}", archive_path.display());
                }
            };

            // Check if we need to download the archive
            if !archive_path.exists() {
                // Find the archive on FTP
                let remote_path = self.find_archive(platform, media_type)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "No {} archive found for platform {}",
                        media_type.archive_pattern(),
                        platform
                    )
                })?;

                // Download it
                self.download_archive(&remote_path, &archive_path, progress)?;
            }

            // Build/load index once while holding archive setup lock
            self.get_or_build_index(&archive_path)?
        };

        // Find the entry for this game
        let entry_path = index.find_entry(game_name).ok_or_else(|| {
            anyhow::anyhow!(
                "No entry found for game '{}' in {} archive",
                game_name,
                media_type.archive_pattern()
            )
        })?;

        // Determine output path
        let ext = entry_path.rsplit('.').next().unwrap_or("png");
        let output_path = game_cache_dir.join("emumovies").join(format!(
            "{}.{}",
            media_type.cache_filename(),
            ext
        ));

        // Extract the file
        self.extract_from_archive(&archive_path, entry_path, &output_path)?;

        Ok(output_path)
    }

    /// Find candidate video folders for a platform, in priority order.
    /// We prefer HQ, then fall back to SQ when HQ doesn't contain a title.
    pub fn find_video_folders(
        &self,
        platform: &str,
        game_cache_dir: Option<&Path>,
    ) -> Result<Vec<String>> {
        let search_candidates = emumovies_platform_search_candidates(platform);
        let cache_key = search_candidates
            .iter()
            .map(|candidate| normalize_emumovies_platform_key(candidate))
            .collect::<Vec<_>>()
            .join("|");

        if let Some(cached) = VIDEO_FOLDER_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("video folder cache lock poisoned")
            .get(&cache_key)
            .cloned()
        {
            return Ok(cached);
        }

        let bases = ["/Official/Video Snaps (HQ)", "/Official/Video Snaps (SQ)"];
        let mut matches: Vec<VideoFolderCandidate> = Vec::new();

        for (source_order, video_base) in bases.iter().enumerate() {
            let base_label = if video_base.contains("(HQ)") {
                "HQ video folders"
            } else {
                "SQ video folders"
            };
            if let Some(game_cache_dir) = game_cache_dir {
                update_video_download_status(
                    game_cache_dir,
                    "finding-folder",
                    format!("Scanning {} for {} videos...", base_label, platform),
                );
            }
            tracing::info!(
                "Searching for video folder for {} in {} using candidates {:?}",
                platform,
                video_base,
                search_candidates
            );
            let mut last_progress_update = Instant::now()
                .checked_sub(Duration::from_secs(1))
                .unwrap_or_else(Instant::now);
            let folders = match self.list_files_with_progress(video_base, |count| {
                if count == 1
                    || count % 250 == 0
                    || last_progress_update.elapsed() >= Duration::from_millis(500)
                {
                    if let Some(game_cache_dir) = game_cache_dir {
                        update_video_download_status(
                            game_cache_dir,
                            "finding-folder",
                            format!(
                                "Scanning {} for {} videos... {} entries",
                                base_label, platform, count
                            ),
                        );
                    }
                    last_progress_update = Instant::now();
                }
            }) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Failed to list {}: {}", video_base, e);
                    continue;
                }
            };

            for folder in &folders {
                if let Some(match_rank) = video_folder_match_rank_for_platform(&folder, platform) {
                    tracing::info!("Found video folder: {} (match_rank={})", folder, match_rank);
                    matches.push(VideoFolderCandidate {
                        path: folder.clone(),
                        source_order,
                        match_rank,
                    });
                }
            }
        }

        matches.sort_by(|a, b| {
            a.match_rank
                .cmp(&b.match_rank)
                .then_with(|| a.source_order.cmp(&b.source_order))
                .then_with(|| a.path.cmp(&b.path))
        });
        matches.dedup_by(|a, b| a.path == b.path);

        let ordered_paths: Vec<String> = matches.into_iter().map(|m| m.path).collect();

        if ordered_paths.is_empty() {
            tracing::info!("No video folder found for {}", platform);
        }

        if !ordered_paths.is_empty() {
            VIDEO_FOLDER_CACHE
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("video folder cache lock poisoned")
                .insert(cache_key, ordered_paths.clone());
        }

        Ok(ordered_paths)
    }

    /// Build or load a cached video index for a specific FTP folder.
    fn get_video_index(
        &self,
        video_folder: &str,
        game_cache_dir: Option<&Path>,
    ) -> Result<Arc<Vec<VideoIndexEntry>>> {
        if let Some(index) = VIDEO_INDEX_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("video index cache lock poisoned")
            .get(video_folder)
            .cloned()
        {
            return Ok(index);
        }

        let cache_path = self.video_index_cache_path(video_folder);
        if cache_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cache_path) {
                if let Ok(cache) = serde_json::from_str::<VideoIndexCache>(&content) {
                    if cache.version == VIDEO_INDEX_CACHE_VERSION {
                        if let Some(game_cache_dir) = game_cache_dir {
                            update_video_download_status(
                                game_cache_dir,
                                "index-ready",
                                "Using cached video index...",
                            );
                        }
                        let index = Arc::new(cache.entries);
                        VIDEO_INDEX_CACHE
                            .get_or_init(|| Mutex::new(HashMap::new()))
                            .lock()
                            .expect("video index cache lock poisoned")
                            .insert(video_folder.to_string(), index.clone());
                        tracing::info!("Loaded cached video index for {}", video_folder);
                        return Ok(index);
                    }
                }
            }
        }

        if let Some(game_cache_dir) = game_cache_dir {
            update_video_download_status(
                game_cache_dir,
                "listing-folder",
                format!(
                    "Listing {}...",
                    video_folder.rsplit('/').next().unwrap_or(video_folder)
                ),
            );
        }
        let mut last_listing_update = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);
        let videos = self.list_files_with_progress(video_folder, |count| {
            if count == 1
                || count % 250 == 0
                || last_listing_update.elapsed() >= Duration::from_millis(500)
            {
                if let Some(game_cache_dir) = game_cache_dir {
                    update_video_download_status(
                        game_cache_dir,
                        "listing-folder",
                        format!(
                            "Listing {}... {} files found",
                            video_folder.rsplit('/').next().unwrap_or(video_folder),
                            count
                        ),
                    );
                }
                last_listing_update = Instant::now();
            }
        })?;
        if let Some(game_cache_dir) = game_cache_dir {
            update_video_download_status(
                game_cache_dir,
                "indexing-folder",
                format!("Indexing {} video entries...", videos.len()),
            );
        }
        let entries: Vec<VideoIndexEntry> = videos
            .into_iter()
            .enumerate()
            .filter_map(|video| {
                let index_position = video.0 + 1;
                let video = video.1;
                if (index_position == 1 || index_position % 500 == 0) && game_cache_dir.is_some() {
                    if let Some(game_cache_dir) = game_cache_dir {
                        update_video_download_status(
                            game_cache_dir,
                            "indexing-folder",
                            format!("Indexing video entries... {} processed", index_position),
                        );
                    }
                }
                let filename = video.rsplit('/').next().unwrap_or(&video);
                if !filename.ends_with(".mp4") {
                    return None;
                }

                let video_name = filename.strip_suffix(".mp4").unwrap_or(filename);
                let normalized = normalize_game_name(video_name);
                let no_region = remove_region_codes(&normalized);
                let tokens = tokenize_for_match(&no_region);

                Some(VideoIndexEntry {
                    path: video,
                    normalized,
                    no_region,
                    tokens,
                })
            })
            .collect();

        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let cache = VideoIndexCache {
            version: VIDEO_INDEX_CACHE_VERSION.to_string(),
            entries: entries.clone(),
        };
        if let Ok(json) = serde_json::to_string(&cache) {
            let _ = std::fs::write(&cache_path, json);
        }

        let index = Arc::new(entries);
        VIDEO_INDEX_CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("video index cache lock poisoned")
            .insert(video_folder.to_string(), index.clone());

        Ok(index)
    }

    /// Download a video for a game
    pub fn get_video(
        &self,
        platform: &str,
        game_name: &str,
        game_cache_dir: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<PathBuf> {
        let output_path = game_cache_dir.join("emumovies").join("video.mp4");

        // Check cache first
        if output_path.exists() && is_video_cache_current(game_cache_dir) {
            return Ok(output_path);
        }

        let download_lock = get_video_download_lock(game_cache_dir);
        let _download_guard = download_lock.lock().map_err(|_| {
            anyhow::anyhow!(
                "Video download lock poisoned for {}",
                game_cache_dir.display()
            )
        })?;

        if output_path.exists() && is_video_cache_current(game_cache_dir) {
            return Ok(output_path);
        }

        // Find candidate video folders (HQ first, then SQ fallback)
        update_video_download_status(
            game_cache_dir,
            "finding-folder",
            "Finding matching video folder...",
        );
        let video_folders = self.find_video_folders(platform, Some(game_cache_dir))?;
        if video_folders.is_empty() {
            anyhow::bail!("No video folder found for platform {}", platform);
        }

        // Evaluate matches across all candidate folders before selecting.
        let mut selected_video: Option<(String, String, VideoMatchKind, f32, u8, usize)> = None;

        for (source_order, video_folder) in video_folders.iter().enumerate() {
            let index = match self.get_video_index(video_folder, Some(game_cache_dir)) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Failed to build video index for {}: {}", video_folder, e);
                    continue;
                }
            };

            if let Some((path, kind, score)) = find_best_video_match(index.as_slice(), game_name) {
                let folder_rank =
                    video_folder_match_rank_for_platform(video_folder, platform).unwrap_or(255);
                let replace = match selected_video.as_ref() {
                    Some((_, _, cur_kind, cur_score, cur_rank, cur_order)) => {
                        compare_video_candidates(
                            kind,
                            folder_rank,
                            score,
                            source_order,
                            *cur_kind,
                            *cur_rank,
                            *cur_score,
                            *cur_order,
                        ) == Ordering::Greater
                    }
                    None => true,
                };

                if replace {
                    selected_video = Some((
                        path,
                        video_folder.clone(),
                        kind,
                        score,
                        folder_rank,
                        source_order,
                    ));
                }
            }
        }

        let (video_path, selected_folder, selected_kind, selected_score, folder_rank, _) =
            selected_video
                .ok_or_else(|| anyhow::anyhow!("No video found for game '{}'", game_name))?;

        tracing::info!(
            "Selected video for '{}' from {} using {:?} match (score {:.2}, folder_rank={})",
            game_name,
            selected_folder,
            selected_kind,
            selected_score,
            folder_rank
        );

        tracing::info!("Downloading video: {}", video_path);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Download the video
        let mut ftp = self.connect()?;
        ftp.transfer_type(suppaftp::types::FileType::Binary)?;

        let file_size = ftp.size(&video_path).ok().map(|size| size as u64);
        tracing::info!("Video size: {:?} bytes", file_size);

        // Write to file
        let temp_path = output_path.with_extension("tmp");
        let temp_file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(temp_file);

        update_video_download_progress(game_cache_dir, 0, file_size);

        let mut stream = ftp
            .retr_as_stream(&video_path)
            .context(format!("Failed to download: {}", video_path))?;
        stream
            .get_ref()
            .set_read_timeout(Some(FTP_DATA_STALL_TIMEOUT))
            .context("Failed to configure EmuMovies video data read timeout")?;
        stream
            .get_ref()
            .set_write_timeout(Some(FTP_DATA_STALL_TIMEOUT))
            .context("Failed to configure EmuMovies video data write timeout")?;
        let mut buffer = [0u8; 64 * 1024];
        let mut downloaded_bytes = 0u64;
        loop {
            let bytes_read = stream
                .read(&mut buffer)
                .context(format!("Failed while reading: {}", video_path))?;
            if bytes_read == 0 {
                break;
            }

            writer.write_all(&buffer[..bytes_read])?;
            downloaded_bytes += bytes_read as u64;
            update_video_download_progress(game_cache_dir, downloaded_bytes, file_size);

            if let Some(progress_fn) = progress {
                if let Some(total_bytes) = file_size {
                    if total_bytes > 0 {
                        progress_fn((downloaded_bytes as f32 / total_bytes as f32).clamp(0.0, 1.0));
                    }
                }
            }
        }

        writer.flush()?;
        ftp.finalize_retr_stream(stream)
            .context(format!("Failed to finalize download: {}", video_path))?;
        let _ = ftp.quit();

        if let Some(progress_fn) = progress {
            progress_fn(1.0);
        }

        std::fs::rename(&temp_path, &output_path)?;
        write_video_cache_version(game_cache_dir)?;
        clear_video_download_progress(game_cache_dir);

        tracing::info!("Downloaded video to {}", output_path.display());

        Ok(output_path)
    }

    /// Check whether a matching video exists for a game without downloading it.
    pub fn has_video_match(&self, platform: &str, game_name: &str) -> Result<bool> {
        let video_folders = self.find_video_folders(platform, None)?;
        if video_folders.is_empty() {
            return Ok(false);
        }

        for (source_order, video_folder) in video_folders.iter().enumerate() {
            let index = match self.get_video_index(video_folder, None) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        "Failed to build video index for {} during availability probe: {}",
                        video_folder,
                        e
                    );
                    continue;
                }
            };

            if let Some((_, kind, score)) = find_best_video_match(index.as_slice(), game_name) {
                let folder_rank =
                    video_folder_match_rank_for_platform(video_folder, platform).unwrap_or(255);
                tracing::info!(
                    "Video availability probe matched '{}' in {} using {:?} match (score {:.2}, folder_rank={}, source_order={})",
                    game_name,
                    video_folder,
                    kind,
                    score,
                    folder_rank,
                    source_order
                );
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Download media to the unified cache structure
    pub fn download_to_path(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
        cache_dir: &Path,
        game_id: &str,
        _image_type: &str,
    ) -> Result<String> {
        let game_cache_dir = cache_dir.join(game_id);

        // Check cache first
        let expected_ext = if media_type.is_video() { "mp4" } else { "png" };
        let cache_path = game_cache_dir.join("emumovies").join(format!(
            "{}.{}",
            media_type.cache_filename(),
            expected_ext
        ));

        if cache_path.exists() {
            return Ok(cache_path.to_string_lossy().to_string());
        }

        // Route to appropriate download method
        let result_path = if media_type.is_video() {
            self.get_video(platform, game_name, &game_cache_dir, None)?
        } else {
            self.get_media_from_archive(platform, media_type, game_name, &game_cache_dir, None)?
        };

        Ok(result_path.to_string_lossy().to_string())
    }

    /// Test connection with credentials
    pub fn test_connection(&self) -> Result<()> {
        if !self.has_credentials() {
            anyhow::bail!("EmuMovies credentials not configured");
        }

        let mut ftp = self.connect()?;

        // Try to list root directory to verify access
        let _ = ftp
            .nlst(Some("/"))
            .context("Failed to list directory - access denied")?;

        let _ = ftp.quit();

        Ok(())
    }
}

/// Normalize a game name for matching (uses centralized tags module)
fn normalize_game_name(name: &str) -> String {
    tags::normalize_title_for_matching(name)
}

/// Remove region codes like (USA), (Europe), etc. (uses centralized tags module)
fn remove_region_codes(name: &str) -> String {
    tags::strip_region_and_language_tags(name)
}

/// Move leading articles to end: "The Legend of Zelda" -> "Legend of Zelda, The"
#[allow(dead_code)]
fn move_article_to_end(name: &str) -> Option<String> {
    let articles = ["The ", "A ", "An "];

    for article in articles {
        if name.starts_with(article) {
            let rest = &name[article.len()..];
            return Some(format!("{}, {}", rest, article.trim()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_mapping() {
        assert_eq!(
            get_emumovies_system_folder("Nintendo Entertainment System"),
            Some("Nintendo Entertainment System")
        );
        assert_eq!(
            get_emumovies_system_folder("NES"),
            Some("Nintendo Entertainment System")
        );
        assert_eq!(
            get_emumovies_system_folder("Super Nintendo Entertainment System"),
            Some("Super Nintendo Entertainment System")
        );
        assert_eq!(
            get_emumovies_system_folder("SNES"),
            Some("Super Nintendo Entertainment System")
        );
        assert_eq!(
            get_emumovies_system_folder("Sega Genesis"),
            Some("Sega Genesis - Mega Drive")
        );
        assert_eq!(
            get_emumovies_system_folder("Genesis"),
            Some("Sega Genesis - Mega Drive")
        );
        assert_ne!(
            get_emumovies_system_folder("Sega Genesis"),
            Some("Nintendo Entertainment System")
        );
        assert_eq!(get_emumovies_system_folder("Sony Playstation 4"), None);
        assert_eq!(get_emumovies_system_folder("Vitalize"), None);
        assert_eq!(get_emumovies_system_folder("Nintendo Sufami Turbo"), None);
    }

    #[test]
    fn test_media_type_from_launchbox() {
        assert_eq!(
            EmuMoviesMediaType::from_launchbox_type("Box - Front"),
            Some(EmuMoviesMediaType::BoxFront)
        );
        assert_eq!(
            EmuMoviesMediaType::from_launchbox_type("Screenshot - Gameplay"),
            Some(EmuMoviesMediaType::Screenshot)
        );
    }

    #[test]
    fn test_normalize_game_name() {
        assert_eq!(normalize_game_name("Super Mario Bros."), "super mario bros");
        assert_eq!(
            normalize_game_name("The Legend of Zelda"),
            "the legend of zelda"
        );
    }

    #[test]
    fn test_remove_region_codes() {
        assert_eq!(
            remove_region_codes("super mario bros (usa)"),
            "super mario bros"
        );
        assert_eq!(remove_region_codes("zelda (japan, usa)"), "zelda");
    }

    #[test]
    fn test_find_best_video_match_prefers_exact() {
        let entries = vec![
            VideoIndexEntry {
                path: "/videos/example-game-deluxe.mp4".to_string(),
                normalized: "example game deluxe".to_string(),
                no_region: "example game deluxe".to_string(),
                tokens: tokenize_for_match("example game deluxe"),
            },
            VideoIndexEntry {
                path: "/videos/example-game-deluxe-usa.mp4".to_string(),
                normalized: "example game deluxe usa".to_string(),
                no_region: "example game deluxe".to_string(),
                tokens: tokenize_for_match("example game deluxe"),
            },
        ];

        let exact =
            find_best_video_match(&entries, "Example Game Deluxe").expect("expected exact match");
        assert_eq!(exact.0, "/videos/example-game-deluxe.mp4");
        assert_eq!(exact.1, VideoMatchKind::Exact);
    }

    #[test]
    fn test_find_best_video_match_fuzzy_variant_title() {
        let entries = vec![
            VideoIndexEntry {
                path: "/videos/example-game-deluxe.mp4".to_string(),
                normalized: "example game deluxe".to_string(),
                no_region: "example game deluxe".to_string(),
                tokens: tokenize_for_match("example game deluxe"),
            },
            VideoIndexEntry {
                path: "/videos/completely-different-title.mp4".to_string(),
                normalized: "completely different title".to_string(),
                no_region: "completely different title".to_string(),
                tokens: tokenize_for_match("completely different title"),
            },
        ];

        let matched = find_best_video_match(&entries, "Example Game Deluxe Extended Edition")
            .expect("expected fuzzy match");
        assert_eq!(matched.0, "/videos/example-game-deluxe.mp4");
        assert_eq!(matched.1, VideoMatchKind::Fuzzy);
        assert!(matched.2 >= 0.72);
    }

    #[test]
    fn test_find_best_video_match_rejects_wrong_sequel_number() {
        let entries = vec![VideoIndexEntry {
            path: "/videos/super-mario-bros-3.mp4".to_string(),
            normalized: "super mario bros 3".to_string(),
            no_region: "super mario bros 3".to_string(),
            tokens: tokenize_for_match("super mario bros 3"),
        }];

        let matched = find_best_video_match(&entries, "Super Mario Bros. 2");
        assert!(matched.is_none(), "should reject wrong sequel number");
    }

    #[test]
    fn test_find_best_video_match_treats_roman_and_arabic_numbers_as_equivalent() {
        let entries = vec![VideoIndexEntry {
            path: "/videos/street-fighter-ii.mp4".to_string(),
            normalized: "street fighter ii".to_string(),
            no_region: "street fighter ii".to_string(),
            tokens: tokenize_for_match("street fighter ii"),
        }];

        let matched = find_best_video_match(&entries, "Street Fighter 2")
            .expect("expected numeric-equivalent match");
        assert_eq!(matched.0, "/videos/street-fighter-ii.mp4");
    }

    #[test]
    fn test_resolve_arcade_video_lookup_uses_generated_parent_shortname() {
        assert_eq!(
            resolve_video_lookup_name(
                "Arcade",
                "Dungeons & Dragons: Shadow Over Mystara",
                Some(8727)
            ),
            "ddsom"
        );
        assert_eq!(
            resolve_video_lookup_name("Arcade", "Dungeons & Dragons: Tower of Doom", Some(8729)),
            "ddtod"
        );
    }

    #[test]
    fn test_resolve_arcade_download_lookup_uses_exact_romset() {
        assert_eq!(
            resolve_arcade_download_lookup_name(
                "Arcade",
                "Dungeons & Dragons: Shadow Over Mystara",
                Some(8727)
            ),
            "ddsomu"
        );
        assert_eq!(
            resolve_arcade_download_lookup_name(
                "Arcade",
                "Dungeons & Dragons: Tower of Doom",
                Some(8729)
            ),
            "ddtodu"
        );
    }

    #[test]
    fn test_resolve_arcade_download_lookup_for_merged_mame_uses_parent_set() {
        assert_eq!(
            resolve_arcade_download_lookup_name_for_torrent(
                "Arcade",
                "Dungeons & Dragons: Shadow Over Mystara",
                Some(8727),
                "https://minerva-archive.org/assets/Minerva_Myrient_v0.3/Minerva_Myrient - MAME - ROMs (merged).torrent"
            ),
            "ddsom"
        );
        assert_eq!(
            resolve_arcade_download_lookup_name_for_torrent(
                "Arcade",
                "Dungeons & Dragons: Shadow Over Mystara",
                Some(8727),
                "https://minerva-archive.org/assets/Minerva_Myrient_v0.3/Minerva_Myrient - MAME - ROMs (non-merged).torrent"
            ),
            "ddsomu"
        );
    }

    #[test]
    fn test_video_folder_match_rank_is_strict_for_platform_stem() {
        let nes = "Nintendo Entertainment System";

        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Nintendo Entertainment System (Video Snaps)(HQ)(No-Intro)(EM 2.5)",
                nes
            ),
            Some(0)
        );
        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Super Nintendo Entertainment System (Video Snaps)(HQ)(No-Intro)(EM 2.5)",
                nes
            ),
            None
        );
    }

    #[test]
    fn test_video_folder_match_rank_allows_platform_variants_after_primary() {
        let nes = "Nintendo Entertainment System";
        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Nintendo Entertainment System-Hacks (Video Snaps)(HQ)(EM 1.7)",
                nes
            ),
            Some(1)
        );
    }

    #[test]
    fn test_video_folder_match_rank_is_punctuation_insensitive() {
        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Nintendo Game Boy Color (Video Snaps)(HQ)(EM 2.5)",
                "Nintendo Gameboy Color"
            ),
            Some(0)
        );
        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Colecovision (Video Snaps)(HQ)(EM 1.3)",
                "ColecoVision"
            ),
            Some(0)
        );
    }

    #[test]
    fn test_video_folder_match_rank_handles_token_prefixes() {
        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Sony Playstation 3 - Retail (Video Snaps)(HQ)(ReDump)(EM 0.9)",
                "Sony PlayStation 3"
            ),
            Some(1)
        );
        assert_eq!(
            video_folder_match_rank(
                "/Official/Video Snaps (HQ)/Sega Genesis - USA (Video Snaps)(HQ)(No-Intro)(EM 2.6)",
                "Sega Genesis"
            ),
            Some(1)
        );
    }

    #[test]
    fn test_platform_search_candidates_cover_cross_media_aliases() {
        let snes = emumovies_platform_search_candidates("Super Nintendo Entertainment System");
        assert!(snes.contains(&"Nintendo Super Nintendo".to_string()));
        assert!(snes.contains(&"Nintendo Super Famicom".to_string()));
        assert_eq!(
            video_folder_match_rank_for_platform(
                "/Official/Video Snaps (HQ)/Nintendo Super Nintendo (Video Snaps)(HQ)(No-Intro 20210322)(EM 2.2)",
                "Super Nintendo Entertainment System"
            ),
            Some(4)
        );

        let tg16 = emumovies_platform_search_candidates("NEC TurboGrafx-16");
        assert!(tg16.contains(&"NEC PC Engine - Turbografx 16".to_string()));
        assert_eq!(
            video_folder_match_rank_for_platform(
                "/Official/Video Snaps (HQ)/NEC TurboGrafx 16 (Video Snaps)(HQ)(EM 1.6)",
                "NEC TurboGrafx-16"
            ),
            Some(0)
        );

        let psp = emumovies_platform_search_candidates("Sony PSP");
        assert!(psp.contains(&"Sony Playstation Portable".to_string()));
        assert!(psp.contains(&"Sony PSP".to_string()));

        let three_do = emumovies_platform_search_candidates("3DO Interactive Multiplayer");
        assert!(three_do.contains(&"Panasonic 3DO".to_string()));
    }

    #[test]
    fn test_select_artwork_folder_from_list_uses_normalized_aliases() {
        let folders = vec![
            "/Official/Artwork/NEC PC Engine - Turbografx 16".to_string(),
            "/Official/Artwork/Sony PlayStation 3".to_string(),
        ];
        assert_eq!(
            select_artwork_folder_from_list(&folders, "NEC TurboGrafx-16"),
            Some("/Official/Artwork/NEC PC Engine - Turbografx 16")
        );
        assert_eq!(
            select_artwork_folder_from_list(&folders, "Sony Playstation 3"),
            Some("/Official/Artwork/Sony PlayStation 3")
        );
    }

    #[test]
    fn test_video_folder_match_rank_for_platform_rejects_false_positives() {
        assert_eq!(
            video_folder_match_rank_for_platform(
                "/Official/Video Snaps (HQ)/Sony Playstation (Video Snaps)(HQ480p)(ReDump)(EM 2.3)",
                "Sony Playstation 4"
            ),
            None
        );
        assert_eq!(
            video_folder_match_rank_for_platform(
                "/Official/Video Snaps (HQ)/Sony PlayStation Vita (Video Snaps)(HQ)(EM 1.0)",
                "Vitalize"
            ),
            None
        );
    }
}
