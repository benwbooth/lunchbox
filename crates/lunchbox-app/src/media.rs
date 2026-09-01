use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use sha2::{Digest, Sha256};

use crate::catalog;

pub(crate) const DEFAULT_PROVIDER_PRIORITY: [&str; 9] = [
    "local",
    "launchbox",
    "libretro",
    "steamgriddb",
    "igdb",
    "minerva",
    "emumovies",
    "screenscraper",
    "websearch",
];

pub(crate) fn provider_display_name(provider: &str) -> &'static str {
    match provider {
        "local" => "Local files",
        "launchbox" => "LaunchBox cache",
        "libretro" => "LibRetro Thumbnails",
        "steamgriddb" => "SteamGridDB",
        "igdb" => "IGDB",
        "minerva" => "Minerva",
        "emumovies" => "EmuMovies",
        "screenscraper" => "ScreenScraper",
        "websearch" => "Web search cache",
        _ => "Unknown provider",
    }
}

pub(crate) fn provider_description(provider: &str) -> &'static str {
    match provider {
        "local" => "User-managed artwork already stored with the game",
        "launchbox" => "Imported LaunchBox artwork already present in the cache",
        "libretro" => "Free retro box art, screenshots, title screens, and logos",
        "steamgriddb" => "Community artwork for modern, PC, and console games",
        "igdb" => "Broad commercial game covers, screenshots, and artwork",
        "minerva" => "Media distributed beside Minerva acquisition records",
        "emumovies" => "Premium retro artwork, videos, and manuals",
        "screenscraper" => "Checksum-oriented community game media",
        "websearch" => "Explicitly reviewed artwork cached from a web search",
        _ => "Provider is not recognized",
    }
}

pub(crate) fn default_provider_priority() -> Vec<String> {
    DEFAULT_PROVIDER_PRIORITY
        .iter()
        .map(|provider| (*provider).to_owned())
        .collect()
}

pub(crate) fn normalize_provider_priority(priority: &[String]) -> Result<Vec<String>> {
    if priority.len() != DEFAULT_PROVIDER_PRIORITY.len() {
        bail!(
            "media provider priority must contain all {} providers exactly once",
            DEFAULT_PROVIDER_PRIORITY.len()
        );
    }
    let mut seen = std::collections::HashSet::new();
    for provider in priority {
        if !DEFAULT_PROVIDER_PRIORITY.contains(&provider.as_str()) {
            bail!("unsupported media provider {provider}");
        }
        if !seen.insert(provider.as_str()) {
            bail!("media provider priority contains duplicate {provider}");
        }
    }
    Ok(priority.to_vec())
}

pub(crate) fn effective_provider_priority(priority: &[String]) -> Vec<String> {
    normalize_provider_priority(priority).unwrap_or_else(|_| default_provider_priority())
}

const LIBRETRO_THUMBNAILS_URL: &str = "https://thumbnails.libretro.com";
const MEDIA_DOWNLOAD_WORKERS: usize = 2;
const MEDIA_DOWNLOAD_QUEUE_CAPACITY: usize = 128;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const PROVIDER_FAILURE_BACKOFF: Duration = Duration::from_secs(2 * 60);
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const AUTOMATIC_DOWNLOAD_PROVIDERS: [&str; 5] = [
    "libretro",
    "steamgriddb",
    "igdb",
    "emumovies",
    "screenscraper",
];
static PROVIDER_FAILURES: std::sync::OnceLock<Mutex<HashMap<String, Instant>>> =
    std::sync::OnceLock::new();

// These aliases are exact mappings between names in the Lunchbox discovery
// catalog and current libretro-thumbnails playlist directories. They are media
// routing hints only; they never create or accept a game identity link.
const LIBRETRO_PLATFORM_ALIASES: &[(&str, &str)] = &[
    ("3DO Interactive Multiplayer", "The 3DO Company - 3DO"),
    ("Amstrad CPC", "Amstrad - CPC"),
    ("Amstrad GX4000", "Amstrad - GX4000"),
    ("Arcade", "MAME"),
    ("Arduboy", "Arduboy Inc - Arduboy"),
    ("Atari - 8-bit Family", "Atari - 8-bit Family"),
    ("Atari 2600", "Atari - 2600"),
    ("Atari 5200", "Atari - 5200"),
    ("Atari 7800", "Atari - 7800"),
    ("Atari 800", "Atari - 8-bit Family"),
    ("Atari Jaguar", "Atari - Jaguar"),
    ("Atari Lynx", "Atari - Lynx"),
    ("Atari ST", "Atari - ST"),
    ("Casio Loopy", "Casio - Loopy"),
    ("Casio PV-1000", "Casio - PV-1000"),
    ("ColecoVision", "Coleco - ColecoVision"),
    ("Commodore - CD32", "Commodore - CD32"),
    ("Commodore - CDTV", "Commodore - CDTV"),
    ("Commodore - Plus-4", "Commodore - Plus-4"),
    ("Commodore 64", "Commodore - 64"),
    ("Commodore Amiga", "Commodore - Amiga"),
    ("Commodore Amiga CD32", "Commodore - CD32"),
    ("Commodore CDTV", "Commodore - CDTV"),
    ("Commodore PET", "Commodore - PET"),
    ("Commodore Plus 4", "Commodore - Plus-4"),
    ("Commodore VIC-20", "Commodore - VIC-20"),
    ("Emerson Arcadia 2001", "Emerson - Arcadia 2001"),
    ("Entex Adventure Vision", "Entex - Adventure Vision"),
    (
        "Epoch Super Cassette Vision",
        "Epoch - Super Cassette Vision",
    ),
    ("Fairchild Channel F", "Fairchild - Channel F"),
    ("Funtech - Super Acan", "Funtech - Super Acan"),
    ("Funtech Super Acan", "Funtech - Super Acan"),
    ("GCE Vectrex", "GCE - Vectrex"),
    ("GamePark GP32", "GamePark - GP32"),
    ("Hartung Game Master", "Hartung - Game Master"),
    ("MS-DOS", "DOS"),
    ("Magnavox Odyssey 2", "Magnavox - Odyssey2"),
    ("Mattel Intellivision", "Mattel - Intellivision"),
    ("Microsoft MSX", "Microsoft - MSX"),
    ("Microsoft MSX2", "Microsoft - MSX2"),
    ("Microsoft Xbox", "Microsoft - Xbox"),
    ("Microsoft Xbox 360", "Microsoft - Xbox 360"),
    ("NEC PC-8801", "NEC - PC-8001 - PC-8801"),
    ("NEC PC-9801", "NEC - PC-98"),
    ("NEC PC-FX", "NEC - PC-FX"),
    ("NEC TurboGrafx-16", "NEC - PC Engine - TurboGrafx 16"),
    ("NEC TurboGrafx-CD", "NEC - PC Engine CD - TurboGrafx-CD"),
    ("PC Engine SuperGrafx", "NEC - PC Engine SuperGrafx"),
    ("Nintendo - Satellaview", "Nintendo - Satellaview"),
    ("Nintendo - Sufami Turbo", "Nintendo - Sufami Turbo"),
    ("Nintendo 3DS", "Nintendo - Nintendo 3DS"),
    ("Nintendo 64", "Nintendo - Nintendo 64"),
    ("Nintendo DS", "Nintendo - Nintendo DS"),
    (
        "Nintendo Entertainment System",
        "Nintendo - Nintendo Entertainment System",
    ),
    (
        "Nintendo Famicom Disk System",
        "Nintendo - Family Computer Disk System",
    ),
    ("Nintendo Game Boy", "Nintendo - Game Boy"),
    ("Nintendo Game Boy Advance", "Nintendo - Game Boy Advance"),
    ("Nintendo Game Boy Color", "Nintendo - Game Boy Color"),
    ("Nintendo GameCube", "Nintendo - GameCube"),
    ("Nintendo Pokemon Mini", "Nintendo - Pokemon Mini"),
    ("Nintendo Satellaview", "Nintendo - Satellaview"),
    ("Nintendo Virtual Boy", "Nintendo - Virtual Boy"),
    ("Nintendo Wii", "Nintendo - Wii"),
    ("Nintendo Wii U", "Nintendo - Wii U"),
    ("Philips - Videopac+", "Philips - Videopac+"),
    ("Philips CD-i", "Philips - CD-i"),
    ("Philips Videopac+", "Philips - Videopac+"),
    ("RCA Studio II", "RCA - Studio II"),
    ("SNK Neo Geo AES", "SNK - Neo Geo"),
    ("SNK Neo Geo CD", "SNK - Neo Geo CD"),
    ("SNK Neo Geo MVS", "SNK - Neo Geo"),
    ("SNK Neo Geo Pocket", "SNK - Neo Geo Pocket"),
    ("SNK Neo Geo Pocket Color", "SNK - Neo Geo Pocket Color"),
    ("ScummVM", "ScummVM"),
    ("Sega - Naomi", "Sega - Naomi"),
    ("Sega - Naomi 2", "Sega - Naomi 2"),
    ("Sega - PICO", "Sega - PICO"),
    ("Sega 32X", "Sega - 32X"),
    ("Sega CD", "Sega - Mega-CD - Sega CD"),
    ("Sega Dreamcast", "Sega - Dreamcast"),
    ("Sega Game Gear", "Sega - Game Gear"),
    ("Sega Genesis", "Sega - Mega Drive - Genesis"),
    ("Sega Master System", "Sega - Master System - Mark III"),
    ("Sega Naomi", "Sega - Naomi"),
    ("Sega Naomi 2", "Sega - Naomi 2"),
    ("Sega Pico", "Sega - PICO"),
    ("Sega SG-1000", "Sega - SG-1000"),
    ("Sega Saturn", "Sega - Saturn"),
    ("Sharp X1", "Sharp - X1"),
    ("Sharp X68000", "Sharp - X68000"),
    ("Sinclair ZX Spectrum", "Sinclair - ZX Spectrum"),
    ("Sinclair ZX-81", "Sinclair - ZX 81"),
    ("Sony PSP", "Sony - PlayStation Portable"),
    ("Sony Playstation", "Sony - PlayStation"),
    ("Sony Playstation 2", "Sony - PlayStation 2"),
    ("Sony Playstation 3", "Sony - PlayStation 3"),
    ("Sony Playstation 4", "Sony - PlayStation 4"),
    ("Sony Playstation Vita", "Sony - PlayStation Vita"),
    ("Spectravideo", "Spectravideo - SVI-318 - SVI-328"),
    (
        "Super Nintendo Entertainment System",
        "Nintendo - Super Nintendo Entertainment System",
    ),
    ("Tiger Game.com", "Tiger - Game.com"),
    ("VTech - V.Smile", "VTech - V.Smile"),
    ("VTech CreatiVision", "VTech - CreatiVision"),
    ("VTech V.Smile", "VTech - V.Smile"),
    ("WASM-4", "WASM-4"),
    ("Watara Supervision", "Watara - Supervision"),
    ("WonderSwan", "Bandai - WonderSwan"),
    ("WonderSwan Color", "Bandai - WonderSwan Color"),
];

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ArtworkKind {
    #[default]
    BoxFront,
    BoxBack,
    Box3d,
    Screenshot,
    TitleScreen,
    Fanart,
    ClearLogo,
}

impl ArtworkKind {
    pub const ALL: [Self; 7] = [
        Self::BoxFront,
        Self::BoxBack,
        Self::Box3d,
        Self::Screenshot,
        Self::TitleScreen,
        Self::Fanart,
        Self::ClearLogo,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::BoxFront => "box-front",
            Self::BoxBack => "box-back",
            Self::Box3d => "box-3d",
            Self::Screenshot => "screenshot",
            Self::TitleScreen => "title-screen",
            Self::Fanart => "fanart",
            Self::ClearLogo => "clear-logo",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::BoxFront => "Box front",
            Self::BoxBack => "Box back",
            Self::Box3d => "3D box",
            Self::Screenshot => "Gameplay",
            Self::TitleScreen => "Title screen",
            Self::Fanart => "Fan art",
            Self::ClearLogo => "Clear logo",
        }
    }

    pub fn has_exact_libretro_source(self, platform: &str) -> bool {
        self.libretro_path_segment().is_some() && libretro_platform_name(platform).is_some()
    }

    fn from_file_stem(stem: &str) -> Option<Self> {
        match stem {
            "box-front" => Some(Self::BoxFront),
            "box-back" => Some(Self::BoxBack),
            "box-3d" => Some(Self::Box3d),
            "screenshot-gameplay" => Some(Self::Screenshot),
            "screenshot-game-title" => Some(Self::TitleScreen),
            "fanart-background" => Some(Self::Fanart),
            "clear-logo" => Some(Self::ClearLogo),
            _ => None,
        }
    }

    fn fallbacks(self) -> &'static [Self] {
        match self {
            Self::BoxFront => &[
                Self::BoxFront,
                Self::Box3d,
                Self::Screenshot,
                Self::TitleScreen,
            ],
            Self::BoxBack => &[Self::BoxBack, Self::BoxFront, Self::Box3d],
            Self::Box3d => &[Self::Box3d, Self::BoxFront, Self::Screenshot],
            Self::Screenshot => &[
                Self::Screenshot,
                Self::TitleScreen,
                Self::BoxFront,
                Self::Box3d,
            ],
            Self::TitleScreen => &[
                Self::TitleScreen,
                Self::Screenshot,
                Self::BoxFront,
                Self::Box3d,
            ],
            Self::Fanart => &[
                Self::Fanart,
                Self::Screenshot,
                Self::TitleScreen,
                Self::BoxFront,
            ],
            Self::ClearLogo => &[Self::ClearLogo, Self::BoxFront, Self::Box3d],
        }
    }

    fn libretro_path_segment(self) -> Option<&'static str> {
        match self {
            Self::BoxFront => Some("Named_Boxarts"),
            Self::Screenshot => Some("Named_Snaps"),
            Self::TitleScreen => Some("Named_Titles"),
            Self::ClearLogo => Some("Named_Logos"),
            Self::BoxBack | Self::Box3d | Self::Fanart => None,
        }
    }

    fn retrieval_fallbacks(self) -> &'static [Self] {
        match self {
            Self::BoxFront | Self::BoxBack | Self::Box3d => &[Self::BoxFront],
            Self::Screenshot | Self::Fanart => &[Self::Screenshot],
            Self::TitleScreen => &[Self::TitleScreen],
            Self::ClearLogo => &[Self::ClearLogo, Self::BoxFront],
        }
    }

    fn cache_file_stem(self) -> &'static str {
        match self {
            Self::BoxFront => "box-front",
            Self::BoxBack => "box-back",
            Self::Box3d => "box-3d",
            Self::Screenshot => "screenshot-gameplay",
            Self::TitleScreen => "screenshot-game-title",
            Self::Fanart => "fanart-background",
            Self::ClearLogo => "clear-logo",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaAsset {
    pub path: PathBuf,
    pub source: String,
    source_rank: usize,
    format_rank: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SupplementalMedia {
    pub video: Option<MediaAsset>,
    pub manual: Option<MediaAsset>,
    pub soundtrack: Vec<SoundtrackAsset>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoundtrackAsset {
    pub path: PathBuf,
    pub source: String,
    pub title: String,
    source_rank: usize,
    format_rank: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameMedia {
    assets: HashMap<ArtworkKind, Vec<MediaAsset>>,
}

impl GameMedia {
    pub fn selected(&self, requested: ArtworkKind) -> Option<&MediaAsset> {
        requested
            .fallbacks()
            .iter()
            .find_map(|kind| self.assets.get(kind).and_then(|assets| assets.first()))
    }

    pub fn exact(&self, requested: ArtworkKind) -> Option<&MediaAsset> {
        self.assets
            .get(&requested)
            .and_then(|assets| assets.first())
    }

    pub fn candidate_count(&self, requested: ArtworkKind) -> usize {
        requested
            .fallbacks()
            .iter()
            .filter_map(|kind| self.assets.get(kind))
            .map(Vec::len)
            .sum()
    }

    pub fn candidate(&self, requested: ArtworkKind, mut index: usize) -> Option<&MediaAsset> {
        for kind in requested.fallbacks() {
            let Some(assets) = self.assets.get(kind) else {
                continue;
            };
            if index < assets.len() {
                return assets.get(index);
            }
            index -= assets.len();
        }
        None
    }

    fn asset_count(&self) -> usize {
        self.assets.values().map(Vec::len).sum()
    }

    fn is_empty(&self) -> bool {
        self.assets.values().all(Vec::is_empty)
    }

    fn insert(&mut self, kind: ArtworkKind, asset: MediaAsset) -> bool {
        let assets = self.assets.entry(kind).or_default();
        if let Some(existing) = assets
            .iter_mut()
            .find(|existing| existing.path == asset.path)
        {
            *existing = asset;
            assets.sort_by(|left, right| {
                (left.source_rank, left.format_rank, &left.source, &left.path).cmp(&(
                    right.source_rank,
                    right.format_rank,
                    &right.source,
                    &right.path,
                ))
            });
            return false;
        }
        assets.push(asset);
        assets.sort_by(|left, right| {
            (left.source_rank, left.format_rank, &left.source, &left.path).cmp(&(
                right.source_rank,
                right.format_rank,
                &right.source,
                &right.path,
            ))
        });
        true
    }
}

#[derive(Clone, Debug, Default)]
pub struct MediaIndex {
    pub root: PathBuf,
    pub games: HashMap<i64, GameMedia>,
    pub asset_count: usize,
    pub skipped_entries: usize,
    pub warning: Option<String>,
    provider_priority: Vec<String>,
}

impl MediaIndex {
    #[cfg(test)]
    pub fn scan(root: PathBuf) -> Self {
        Self::scan_with_provider_priority(root, default_provider_priority())
    }

    pub fn scan_with_provider_priority(root: PathBuf, provider_priority: Vec<String>) -> Self {
        let cancelled = AtomicBool::new(false);
        Self::scan_with_control(root, provider_priority, &cancelled, |_, _| {})
    }

    pub fn scan_with_control<F>(
        root: PathBuf,
        provider_priority: Vec<String>,
        cancelled: &AtomicBool,
        mut progress: F,
    ) -> Self
    where
        F: FnMut(usize, usize),
    {
        let provider_priority = effective_provider_priority(&provider_priority);
        let mut index = Self {
            root: root.clone(),
            provider_priority: provider_priority.clone(),
            ..Self::default()
        };
        let directories = match fs::read_dir(&root) {
            Ok(directories) => directories.collect::<Vec<_>>(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return index,
            Err(error) => {
                index.warning = Some(format!("could not read {}: {error}", root.display()));
                return index;
            }
        };

        let total = directories.len();
        for (position, directory) in directories.into_iter().enumerate() {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            progress(position, total);
            let Ok(directory) = directory else {
                index.skipped_entries += 1;
                continue;
            };
            let Ok(file_type) = directory.file_type() else {
                index.skipped_entries += 1;
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let Some(database_id) = parse_launchbox_directory(&directory.file_name()) else {
                continue;
            };
            let media = scan_game_directory(
                &directory.path(),
                &mut index.skipped_entries,
                &provider_priority,
            );
            if !media.is_empty() {
                index.asset_count += media.asset_count();
                index.games.insert(database_id, media);
            }
        }
        progress(total, total);
        index
    }

    pub fn selected(&self, database_id: i64, kind: ArtworkKind) -> Option<&MediaAsset> {
        self.games.get(&database_id)?.selected(kind)
    }

    pub fn exact(&self, database_id: i64, kind: ArtworkKind) -> Option<&MediaAsset> {
        self.games.get(&database_id)?.exact(kind)
    }

    pub fn candidate_count(&self, database_id: i64, kind: ArtworkKind) -> usize {
        self.games
            .get(&database_id)
            .map(|media| media.candidate_count(kind))
            .unwrap_or_default()
    }

    pub fn candidate(
        &self,
        database_id: i64,
        kind: ArtworkKind,
        index: usize,
    ) -> Option<&MediaAsset> {
        self.games.get(&database_id)?.candidate(kind, index)
    }

    pub fn provider_priority(&self) -> &[String] {
        &self.provider_priority
    }

    pub fn insert_asset(
        &mut self,
        database_id: i64,
        kind: ArtworkKind,
        path: PathBuf,
        source: &str,
    ) -> bool {
        if database_id <= 0 {
            return false;
        }
        let game_media = self.games.entry(database_id).or_default();
        let inserted = game_media.insert(
            kind,
            MediaAsset {
                path,
                source: source.to_owned(),
                source_rank: provider_rank(source, &self.provider_priority),
                format_rank: 0,
            },
        );
        if inserted {
            self.asset_count = self.asset_count.saturating_add(1);
        }
        inserted
    }
}

pub fn requested_media_directory() -> PathBuf {
    catalog::requested_path("--media-directory", "LUNCHBOX_MEDIA_DIRECTORY")
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| {
            crate::settings::state_database_path()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("media")))
        })
        .or_else(|| {
            ProjectDirs::from("com", "Lunchbox", "Lunchbox")
                .map(|dirs| dirs.data_local_dir().join("media"))
        })
        .unwrap_or_else(|| PathBuf::from("media"))
}

pub fn supplemental_media(game_id: &str, database_id: i64) -> Result<SupplementalMedia> {
    let root = requested_media_directory();
    let provider_priority = crate::settings::SettingsStore::open_default()
        .and_then(|store| store.load())
        .map(|settings| effective_provider_priority(&settings.media_provider_priority))
        .unwrap_or_else(|_| default_provider_priority());
    let mut directories = Vec::with_capacity(2);
    if database_id > 0 {
        directories.push(game_media_directory_in(&root, game_id, database_id)?);
    }
    if !game_id.trim().is_empty() && safe_game_identity(game_id) {
        directories.push(root.join(format!("game-{game_id}")));
    }

    let mut selected = SupplementalMedia::default();
    for directory in directories {
        let candidate = scan_supplemental_directory(&root, &directory, &provider_priority)?;
        select_supplemental_asset(&mut selected.video, candidate.video);
        select_supplemental_asset(&mut selected.manual, candidate.manual);
        selected.soundtrack.extend(candidate.soundtrack);
    }
    sort_soundtrack_assets(&mut selected.soundtrack);
    Ok(selected)
}

pub fn game_media_directory(game_id: &str, database_id: i64) -> Result<PathBuf> {
    game_media_directory_in(&requested_media_directory(), game_id, database_id)
}

pub fn exact_artwork_exists(
    root: &Path,
    game_id: &str,
    database_id: i64,
    kind: ArtworkKind,
    provider_priority: &[String],
) -> bool {
    let Ok(directory) = game_media_directory_in(root, game_id, database_id) else {
        return false;
    };
    let mut skipped = 0;
    scan_game_directory(
        &directory,
        &mut skipped,
        &effective_provider_priority(provider_priority),
    )
    .exact(kind)
    .is_some()
}

fn game_media_directory_in(root: &Path, game_id: &str, database_id: i64) -> Result<PathBuf> {
    if database_id > 0 {
        return Ok(root.join(format!("lb-{database_id}")));
    }
    if game_id.trim().is_empty() || !safe_game_identity(game_id) {
        bail!("a bounded stable game identity is required for local media");
    }
    Ok(root.join(format!("game-{game_id}")))
}

pub fn media_identity(path: &Path) -> Result<String> {
    let metadata = path
        .metadata()
        .with_context(|| format!("inspecting media file {}", path.display()))?;
    if !metadata.is_file() {
        bail!("media path is not a file: {}", path.display());
    }

    let mut file =
        fs::File::open(path).with_context(|| format!("opening media file {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(metadata.len().to_le_bytes());
    let mut sample = vec![0_u8; usize::try_from(metadata.len().min(64 * 1024)).unwrap_or(0)];
    file.read_exact(&mut sample)
        .with_context(|| format!("reading media identity sample from {}", path.display()))?;
    hasher.update(&sample);

    if metadata.len() > 64 * 1024 {
        let tail_length = metadata.len().min(64 * 1024);
        file.seek(SeekFrom::End(
            -i64::try_from(tail_length).unwrap_or(i64::MAX),
        ))
        .with_context(|| format!("seeking media identity sample in {}", path.display()))?;
        sample.resize(usize::try_from(tail_length).unwrap_or(0), 0);
        file.read_exact(&mut sample)
            .with_context(|| format!("reading media identity tail from {}", path.display()))?;
        hasher.update(&sample);
    }

    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn safe_game_identity(game_id: &str) -> bool {
    game_id.len() <= 512
        && game_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn scan_supplemental_directory(
    root: &Path,
    directory: &Path,
    provider_priority: &[String],
) -> Result<SupplementalMedia> {
    if !directory.is_dir() {
        return Ok(SupplementalMedia::default());
    }
    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("canonicalizing media directory {}", root.display()))?;
    let canonical_directory = directory.canonicalize().with_context(|| {
        format!(
            "canonicalizing game media directory {}",
            directory.display()
        )
    })?;
    if !canonical_directory.starts_with(&canonical_root) {
        bail!(
            "game media directory escapes the configured media root: {}",
            directory.display()
        );
    }

    let mut result = SupplementalMedia::default();
    for source in fs::read_dir(&canonical_directory)
        .with_context(|| format!("reading game media directory {}", directory.display()))?
    {
        let source = source.with_context(|| {
            format!(
                "reading a media source under {}",
                canonical_directory.display()
            )
        })?;
        if !source
            .file_type()
            .with_context(|| format!("inspecting media source {}", source.path().display()))?
            .is_dir()
        {
            continue;
        }
        let source_name = source.file_name().to_string_lossy().to_lowercase();
        let source_rank = provider_rank(&source_name, provider_priority);
        for file in fs::read_dir(source.path())
            .with_context(|| format!("reading media source {}", source.path().display()))?
        {
            let file = file.with_context(|| {
                format!("reading a media file under {}", source.path().display())
            })?;
            if !file
                .file_type()
                .with_context(|| format!("inspecting media file {}", file.path().display()))?
                .is_file()
            {
                continue;
            }
            if file
                .metadata()
                .with_context(|| format!("inspecting media file {}", file.path().display()))?
                .len()
                == 0
            {
                continue;
            }
            let path = file
                .path()
                .canonicalize()
                .with_context(|| format!("canonicalizing media file {}", file.path().display()))?;
            if !path.starts_with(&canonical_root) {
                bail!(
                    "media file escapes the configured media root: {}",
                    path.display()
                );
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let stem = stem.to_ascii_lowercase();
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let extension = extension.to_ascii_lowercase();
            let (target, format_rank) = if stem == "video" {
                let Some(rank) = video_format_rank(&extension) else {
                    continue;
                };
                (&mut result.video, rank)
            } else if stem == "manual" {
                let Some(rank) = manual_format_rank(&extension) else {
                    continue;
                };
                (&mut result.manual, rank)
            } else if stem.starts_with("soundtrack-") {
                let Some(format_rank) = soundtrack_format_rank(&extension) else {
                    continue;
                };
                let title = read_soundtrack_title(&path).unwrap_or_else(|| "Game music".to_owned());
                result.soundtrack.push(SoundtrackAsset {
                    path,
                    source: source_name.to_string(),
                    title,
                    source_rank,
                    format_rank,
                });
                continue;
            } else {
                continue;
            };
            select_supplemental_asset(
                target,
                Some(MediaAsset {
                    path,
                    source: source_name.to_string(),
                    source_rank,
                    format_rank,
                }),
            );
        }
    }
    sort_soundtrack_assets(&mut result.soundtrack);
    Ok(result)
}

fn read_soundtrack_title(path: &Path) -> Option<String> {
    let sidecar = path.with_extension("title");
    let metadata = sidecar.metadata().ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 1024 {
        return None;
    }
    let title = fs::read_to_string(sidecar).ok()?;
    let title = title.trim();
    if title.is_empty() || title.chars().any(char::is_control) {
        return None;
    }
    Some(title.to_owned())
}

fn sort_soundtrack_assets(tracks: &mut Vec<SoundtrackAsset>) {
    tracks.sort_by(|a, b| {
        (a.source_rank, a.format_rank)
            .cmp(&(b.source_rank, b.format_rank))
            .then_with(|| {
                a.title
                    .to_ascii_lowercase()
                    .cmp(&b.title.to_ascii_lowercase())
            })
            .then_with(|| a.path.cmp(&b.path))
    });
    tracks.dedup_by(|a, b| a.path == b.path);
}

fn select_supplemental_asset(target: &mut Option<MediaAsset>, candidate: Option<MediaAsset>) {
    let Some(candidate) = candidate else {
        return;
    };
    if target.as_ref().is_none_or(|existing| {
        (
            candidate.source_rank,
            candidate.format_rank,
            &candidate.source,
            &candidate.path,
        ) < (
            existing.source_rank,
            existing.format_rank,
            &existing.source,
            &existing.path,
        )
    }) {
        *target = Some(candidate);
    }
}

fn video_format_rank(extension: &str) -> Option<usize> {
    ["mp4", "webm", "mkv", "mov", "m4v", "avi"]
        .iter()
        .position(|candidate| *candidate == extension)
}

fn manual_format_rank(extension: &str) -> Option<usize> {
    ["pdf", "cbz", "cbr", "epub", "html", "htm", "txt", "zip"]
        .iter()
        .position(|candidate| *candidate == extension)
}

fn soundtrack_format_rank(extension: &str) -> Option<usize> {
    ["mp3", "flac", "ogg", "opus", "m4a", "aac", "wav"]
        .iter()
        .position(|candidate| *candidate == extension)
}

fn parse_launchbox_directory(value: &std::ffi::OsStr) -> Option<i64> {
    let value = value.to_str()?;
    let database_id = value.strip_prefix("lb-")?.parse::<i64>().ok()?;
    (database_id > 0).then_some(database_id)
}

fn scan_game_directory(
    path: &Path,
    skipped_entries: &mut usize,
    provider_priority: &[String],
) -> GameMedia {
    let mut media = GameMedia::default();
    let Ok(sources) = fs::read_dir(path) else {
        *skipped_entries += 1;
        return media;
    };
    for source in sources {
        let Ok(source) = source else {
            *skipped_entries += 1;
            continue;
        };
        let Ok(file_type) = source.file_type() else {
            *skipped_entries += 1;
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let source_name = source.file_name().to_string_lossy().to_lowercase();
        let source_rank = provider_rank(&source_name, provider_priority);
        let Ok(files) = fs::read_dir(source.path()) else {
            *skipped_entries += 1;
            continue;
        };
        for file in files {
            let Ok(file) = file else {
                *skipped_entries += 1;
                continue;
            };
            let Ok(file_type) = file.file_type() else {
                *skipped_entries += 1;
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let path = file.path();
            let Some(format_rank) = image_format_rank(&path) else {
                continue;
            };
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(kind) = ArtworkKind::from_file_stem(&stem.to_lowercase()) else {
                continue;
            };
            media.insert(
                kind,
                MediaAsset {
                    path,
                    source: source_name.to_string(),
                    source_rank,
                    format_rank,
                },
            );
        }
    }
    media
}

fn provider_rank(source: &str, provider_priority: &[String]) -> usize {
    if provider_priority.is_empty() {
        return DEFAULT_PROVIDER_PRIORITY
            .iter()
            .position(|candidate| *candidate == source)
            .unwrap_or(DEFAULT_PROVIDER_PRIORITY.len());
    }
    provider_priority
        .iter()
        .position(|candidate| candidate == source)
        .unwrap_or(provider_priority.len())
}

fn image_format_rank(path: &Path) -> Option<usize> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| {
            ["png", "jpg", "jpeg", "webp", "gif", "svg"]
                .iter()
                .position(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaFetchRequest {
    pub request_id: String,
    pub database_id: i64,
    pub title: String,
    pub platform: String,
    pub requested_kind: ArtworkKind,
    pub force: bool,
    pub exact_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaFetchOutcome {
    Found {
        request_id: String,
        database_id: i64,
        requested_kind: ArtworkKind,
        fetched_kind: ArtworkKind,
        path: PathBuf,
        provider: String,
        force: bool,
    },
    Missing {
        request_id: String,
        database_id: i64,
        requested_kind: ArtworkKind,
        force: bool,
    },
    Failed {
        request_id: String,
        database_id: i64,
        requested_kind: ArtworkKind,
        error: String,
        force: bool,
    },
}

pub struct MediaFetchQueue {
    shared: Arc<(Mutex<MediaFetchState>, Condvar)>,
}

#[derive(Default)]
struct MediaFetchState {
    foreground_requests: VecDeque<MediaFetchRequest>,
    requests: VecDeque<MediaFetchRequest>,
    shutdown: bool,
}

impl MediaFetchState {
    fn queued_len(&self) -> usize {
        self.foreground_requests
            .len()
            .saturating_add(self.requests.len())
    }

    fn push(&mut self, request: MediaFetchRequest, foreground: bool) {
        if foreground {
            self.foreground_requests.push_back(request);
        } else {
            self.requests.push_back(request);
        }
    }

    fn prioritize(&mut self, database_id: i64, requested_kind: ArtworkKind) -> bool {
        if let Some(position) = self.foreground_requests.iter().position(|request| {
            request.database_id == database_id && request.requested_kind == requested_kind
        }) {
            if let Some(request) = self.foreground_requests.remove(position) {
                self.foreground_requests.push_back(request);
                return true;
            }
        }
        let Some(position) = self.requests.iter().position(|request| {
            request.database_id == database_id && request.requested_kind == requested_kind
        }) else {
            return false;
        };
        let Some(request) = self.requests.remove(position) else {
            return false;
        };
        self.foreground_requests.push_back(request);
        true
    }

    fn pop_next(&mut self) -> Option<MediaFetchRequest> {
        self.foreground_requests
            .pop_back()
            .or_else(|| self.requests.pop_back())
    }
}

impl MediaFetchQueue {
    pub fn start<F>(root: PathBuf, provider_priority: Vec<String>, callback: F) -> Result<Self>
    where
        F: Fn(MediaFetchOutcome) + Send + Sync + 'static,
    {
        let shared = Arc::new((Mutex::new(MediaFetchState::default()), Condvar::new()));
        let callback = Arc::new(callback);
        let base_url = requested_libretro_base_url();
        let provider_priority = Arc::new(effective_provider_priority(&provider_priority));
        for worker_index in 0..MEDIA_DOWNLOAD_WORKERS {
            let shared = Arc::clone(&shared);
            let worker_shared = Arc::clone(&shared);
            let callback = Arc::clone(&callback);
            let root = root.clone();
            let base_url = base_url.clone();
            let provider_priority = Arc::clone(&provider_priority);
            if let Err(error) = std::thread::Builder::new()
                .name(format!("lunchbox-media-fetch-{worker_index}"))
                .spawn(move || {
                    media_fetch_worker(
                        worker_shared,
                        callback,
                        root,
                        base_url,
                        provider_priority,
                        worker_index,
                    )
                })
            {
                let (state, ready) = &*shared;
                if let Ok(mut state) = state.lock() {
                    state.shutdown = true;
                    ready.notify_all();
                }
                return Err(error).context("starting artwork retrieval worker");
            }
        }
        Ok(Self { shared })
    }

    pub fn try_request(&self, request: MediaFetchRequest) -> Result<()> {
        self.try_request_with_priority(request, false)
    }

    pub fn try_priority_request(&self, request: MediaFetchRequest) -> Result<()> {
        self.try_request_with_priority(request, true)
    }

    fn try_request_with_priority(
        &self,
        request: MediaFetchRequest,
        foreground: bool,
    ) -> Result<()> {
        let (state, ready) = &*self.shared;
        let mut state = state
            .lock()
            .map_err(|_| anyhow::anyhow!("artwork retrieval queue is unavailable"))?;
        if state.shutdown {
            bail!("artwork retrieval workers are unavailable");
        }
        if state.queued_len() >= MEDIA_DOWNLOAD_QUEUE_CAPACITY {
            bail!("artwork retrieval queue is full");
        }
        state.push(request, foreground);
        ready.notify_one();
        Ok(())
    }

    pub fn prioritize_request(
        &self,
        database_id: i64,
        requested_kind: ArtworkKind,
    ) -> Result<bool> {
        let (state, ready) = &*self.shared;
        let mut state = state
            .lock()
            .map_err(|_| anyhow::anyhow!("artwork retrieval queue is unavailable"))?;
        if state.shutdown {
            bail!("artwork retrieval workers are unavailable");
        }
        let prioritized = state.prioritize(database_id, requested_kind);
        if prioritized {
            ready.notify_one();
        }
        Ok(prioritized)
    }
}

impl Drop for MediaFetchQueue {
    fn drop(&mut self) {
        let (state, ready) = &*self.shared;
        if let Ok(mut state) = state.lock() {
            state.shutdown = true;
            ready.notify_all();
        }
    }
}

pub fn media_retrieval_enabled() -> bool {
    if std::env::args().any(|argument| argument == "--offline") {
        return false;
    }
    !std::env::var("LUNCHBOX_OFFLINE")
        .ok()
        .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

fn requested_libretro_base_url() -> String {
    std::env::var("LUNCHBOX_LIBRETRO_THUMBNAILS_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| LIBRETRO_THUMBNAILS_URL.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn media_fetch_worker<F>(
    shared: Arc<(Mutex<MediaFetchState>, Condvar)>,
    callback: Arc<F>,
    root: PathBuf,
    base_url: String,
    provider_priority: Arc<Vec<String>>,
    worker_index: usize,
) where
    F: Fn(MediaFetchOutcome) + Send + Sync + 'static,
{
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(4)))
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .build()
        .into();
    loop {
        let request = {
            let (state, ready) = &*shared;
            let mut state = match state.lock() {
                Ok(state) => state,
                Err(_) => return,
            };
            while state.foreground_requests.is_empty()
                && state.requests.is_empty()
                && !state.shutdown
            {
                state = match ready.wait(state) {
                    Ok(state) => state,
                    Err(_) => return,
                };
            }
            if state.shutdown {
                return;
            }
            state.pop_next()
        };
        let Some(request) = request else { continue };
        callback(fetch_media(
            &agent,
            &root,
            &base_url,
            &provider_priority,
            request,
            worker_index,
        ));
    }
}

fn fetch_media(
    agent: &ureq::Agent,
    root: &Path,
    base_url: &str,
    provider_priority: &[String],
    request: MediaFetchRequest,
    worker_index: usize,
) -> MediaFetchOutcome {
    let mut errors = Vec::new();
    for provider in automatic_provider_order(provider_priority) {
        if !request.force && provider_failure_backoff_is_active(&provider) {
            continue;
        }
        let result = match provider.as_str() {
            "libretro" => fetch_libretro(agent, root, base_url, &request, worker_index),
            "steamgriddb" => fetch_steamgriddb(root, &request),
            "igdb" => fetch_igdb(root, &request),
            "emumovies" => fetch_emumovies(root, &request),
            "screenscraper" => fetch_screenscraper(root, &request),
            _ => Ok(None),
        };
        match result {
            Ok(Some((fetched_kind, path))) => {
                clear_provider_failure(&provider);
                return MediaFetchOutcome::Found {
                    request_id: request.request_id,
                    database_id: request.database_id,
                    requested_kind: request.requested_kind,
                    fetched_kind,
                    path,
                    provider,
                    force: request.force,
                };
            }
            Ok(None) => clear_provider_failure(&provider),
            Err(error) => {
                remember_provider_failure(&provider);
                errors.push(format!(
                    "{}: {}",
                    provider_display_name(&provider),
                    concise_provider_error(&error.to_string())
                ));
            }
        }
    }

    if errors.is_empty() {
        missing_outcome(request)
    } else {
        MediaFetchOutcome::Failed {
            request_id: request.request_id,
            database_id: request.database_id,
            requested_kind: request.requested_kind,
            error: errors.join("; "),
            force: request.force,
        }
    }
}

fn provider_failure_backoff_is_active(provider: &str) -> bool {
    PROVIDER_FAILURES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|failures| failures.get(provider).copied())
        .is_some_and(|failed_at| failed_at.elapsed() < PROVIDER_FAILURE_BACKOFF)
}

fn remember_provider_failure(provider: &str) {
    if let Ok(mut failures) = PROVIDER_FAILURES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        failures.insert(provider.to_owned(), Instant::now());
    }
}

fn clear_provider_failure(provider: &str) {
    if let Ok(mut failures) = PROVIDER_FAILURES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        failures.remove(provider);
    }
}

fn automatic_provider_order(provider_priority: &[String]) -> Vec<String> {
    effective_provider_priority(provider_priority)
        .into_iter()
        .filter(|provider| AUTOMATIC_DOWNLOAD_PROVIDERS.contains(&provider.as_str()))
        .collect()
}

fn fetch_libretro(
    agent: &ureq::Agent,
    root: &Path,
    base_url: &str,
    request: &MediaFetchRequest,
    worker_index: usize,
) -> Result<Option<(ArtworkKind, PathBuf)>> {
    if !request.force && negative_cache_is_fresh(root, request.database_id, request.requested_kind)
    {
        return Ok(None);
    }

    let Some(platform) = libretro_platform_name(&request.platform) else {
        let _ = write_negative_cache(root, request, "platform is not provided by LibRetro");
        return Ok(None);
    };

    for fetched_kind in retrieval_kinds(request.requested_kind, request.exact_only) {
        let Some(type_directory) = fetched_kind.libretro_path_segment() else {
            continue;
        };
        let output = media_output_path(root, request.database_id, fetched_kind);
        if output.is_file() && !request.force {
            return Ok(Some((fetched_kind, output)));
        }

        for candidate in libretro_title_candidates(&request.title) {
            let url = format!(
                "{}/{}/{}/{}.png",
                base_url,
                percent_encode_path_segment(platform),
                type_directory,
                percent_encode_path_segment(&candidate)
            );
            if let Some(bytes) = download_png(agent, &url)? {
                write_download_atomically(&output, &bytes, worker_index, request.force)?;
                let _ = fs::remove_file(negative_cache_path(
                    root,
                    request.database_id,
                    request.requested_kind,
                ));
                return Ok(Some((fetched_kind, output)));
            }
        }
    }

    let _ = write_negative_cache(root, request, "no matching LibRetro thumbnail");
    Ok(None)
}

fn fetch_steamgriddb(
    root: &Path,
    request: &MediaFetchRequest,
) -> Result<Option<(ArtworkKind, PathBuf)>> {
    let Some(api_key) = configured_steamgriddb_api_key()? else {
        return Ok(None);
    };
    let link = crate::settings::SettingsStore::open_default()?
        .media_provider_game_link("steamgriddb", request.database_id)?;
    if link.is_none()
        && !request.force
        && provider_negative_cache_is_fresh(
            root,
            "steamgriddb",
            request.database_id,
            request.requested_kind,
            request.exact_only,
        )
    {
        return Ok(None);
    }
    let client = crate::steamgriddb::Client::new(api_key)?;
    let game_id = if let Some(link) = link {
        positive_provider_game_id(&link.provider_game_id, "SteamGridDB")?
    } else {
        let candidates = client.search_games(&request.title)?;
        let Some(game_id) = unique_exact_provider_game_id(
            candidates
                .iter()
                .map(|candidate| (candidate.id, candidate.name.as_str())),
            &request.title,
        ) else {
            write_provider_negative_cache(
                root,
                "steamgriddb",
                request,
                "no unique exact-title SteamGridDB game",
            )?;
            return Ok(None);
        };
        game_id
    };
    for fetched_kind in retrieval_kinds(request.requested_kind, request.exact_only) {
        if !matches!(
            fetched_kind,
            ArtworkKind::BoxFront | ArtworkKind::Fanart | ArtworkKind::ClearLogo
        ) {
            continue;
        }
        let Some(candidate) = client
            .artwork_for_game(game_id, fetched_kind)?
            .into_iter()
            .next()
        else {
            continue;
        };
        let fetched = client
            .download_and_publish(&candidate, root, request.database_id, fetched_kind)
            .map(|path| Some((fetched_kind, path)))?;
        remove_provider_negative_cache(root, "steamgriddb", request);
        return Ok(fetched);
    }
    write_provider_negative_cache(
        root,
        "steamgriddb",
        request,
        "the exact SteamGridDB game has no requested artwork",
    )?;
    Ok(None)
}

fn fetch_igdb(root: &Path, request: &MediaFetchRequest) -> Result<Option<(ArtworkKind, PathBuf)>> {
    let Some((client_id, client_secret)) = configured_igdb_credentials()? else {
        return Ok(None);
    };
    let link = crate::settings::SettingsStore::open_default()?
        .media_provider_game_link("igdb", request.database_id)?;
    if link.is_none()
        && !request.force
        && provider_negative_cache_is_fresh(
            root,
            "igdb",
            request.database_id,
            request.requested_kind,
            request.exact_only,
        )
    {
        return Ok(None);
    }
    let client = crate::igdb::Client::new(client_id, client_secret)?;
    let game_id = if let Some(link) = link {
        positive_provider_game_id(&link.provider_game_id, "IGDB")?
    } else {
        let candidates = client.search_games(&request.title)?;
        let exact = candidates
            .iter()
            .filter(|candidate| {
                strict_provider_title_matches(&candidate.name, &request.title)
                    && igdb_platform_matches(candidate, &request.platform)
            })
            .map(|candidate| (candidate.id, candidate.name.as_str()));
        let Some(game_id) = unique_exact_provider_game_id(exact, &request.title) else {
            write_provider_negative_cache(
                root,
                "igdb",
                request,
                "no unique exact-title and exact-platform IGDB game",
            )?;
            return Ok(None);
        };
        game_id
    };
    for fetched_kind in retrieval_kinds(request.requested_kind, request.exact_only) {
        if !matches!(
            fetched_kind,
            ArtworkKind::BoxFront | ArtworkKind::Screenshot | ArtworkKind::Fanart
        ) {
            continue;
        }
        let Some(candidate) = client
            .artwork_for_game(game_id, fetched_kind)?
            .into_iter()
            .next()
        else {
            continue;
        };
        let fetched = client
            .download_and_publish(&candidate, root, request.database_id, fetched_kind)
            .map(|path| Some((fetched_kind, path)))?;
        remove_provider_negative_cache(root, "igdb", request);
        return Ok(fetched);
    }
    write_provider_negative_cache(
        root,
        "igdb",
        request,
        "the exact IGDB game has no requested artwork",
    )?;
    Ok(None)
}

fn fetch_emumovies(
    root: &Path,
    request: &MediaFetchRequest,
) -> Result<Option<(ArtworkKind, PathBuf)>> {
    let Some((username, password)) = configured_emumovies_credentials()? else {
        return Ok(None);
    };
    let client = crate::emumovies::EmuMoviesClient::new(
        crate::emumovies::EmuMoviesConfig { username, password },
        root.to_path_buf(),
    );
    let game_directory = root.join(format!("lb-{}", request.database_id));
    let lookup_title = crate::emumovies::resolve_arcade_download_lookup_name(
        &request.platform,
        &request.title,
        Some(request.database_id),
    );
    for fetched_kind in retrieval_kinds(request.requested_kind, request.exact_only) {
        let media_type = match fetched_kind {
            ArtworkKind::BoxFront => crate::emumovies::EmuMoviesMediaType::BoxFront,
            ArtworkKind::BoxBack => crate::emumovies::EmuMoviesMediaType::BoxBack,
            ArtworkKind::Box3d => crate::emumovies::EmuMoviesMediaType::Box3D,
            ArtworkKind::Screenshot => crate::emumovies::EmuMoviesMediaType::Screenshot,
            ArtworkKind::TitleScreen => crate::emumovies::EmuMoviesMediaType::TitleScreen,
            ArtworkKind::Fanart => crate::emumovies::EmuMoviesMediaType::Fanart,
            ArtworkKind::ClearLogo => crate::emumovies::EmuMoviesMediaType::ClearLogo,
        };
        if let Some(path) = client.try_get_media_from_archive(
            &request.platform,
            media_type,
            lookup_title.as_ref(),
            &game_directory,
            None,
        )? {
            return Ok(Some((fetched_kind, path)));
        }
    }
    Ok(None)
}

fn positive_provider_game_id(value: &str, provider: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|game_id| *game_id > 0)
        .with_context(|| format!("reviewed {provider} game ID is invalid"))
}

fn strict_provider_title_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

fn strict_provider_title_matches(candidate: &str, requested: &str) -> bool {
    let requested = strict_provider_title_key(requested);
    !requested.is_empty() && strict_provider_title_key(candidate) == requested
}

fn unique_exact_provider_game_id<'a>(
    candidates: impl IntoIterator<Item = (i64, &'a str)>,
    requested_title: &str,
) -> Option<i64> {
    let mut exact_id = None;
    for (id, name) in candidates {
        if id <= 0 || !strict_provider_title_matches(name, requested_title) {
            continue;
        }
        if exact_id.is_some_and(|existing| existing != id) {
            return None;
        }
        exact_id = Some(id);
    }
    exact_id
}

fn exact_platform_keys(platform: &str) -> HashSet<String> {
    let mut keys = HashSet::from([catalog::normalize_platform_key(platform)]);
    if let Some(aliases) = catalog::legacy_platform_search_aliases(platform) {
        keys.extend(aliases.split(',').map(catalog::normalize_platform_key));
    }
    keys.retain(|key| !key.is_empty());
    keys
}

fn igdb_platform_matches(candidate: &crate::igdb::GameCandidate, requested_platform: &str) -> bool {
    let requested = exact_platform_keys(requested_platform);
    candidate.platforms.iter().any(|platform| {
        requested.contains(&catalog::normalize_platform_key(&platform.name))
            || requested.contains(&catalog::normalize_platform_key(&platform.abbreviation))
    })
}

fn configured_steamgriddb_api_key() -> Result<Option<String>> {
    if let Some(api_key) = std::env::var("LUNCHBOX_STEAMGRIDDB_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(Some(api_key));
    }
    crate::settings::load_steamgriddb_api_key()
}

fn configured_igdb_credentials() -> Result<Option<(String, String)>> {
    configured_credential_pair(
        "LUNCHBOX_IGDB_CLIENT_ID",
        "LUNCHBOX_IGDB_CLIENT_SECRET",
        crate::settings::load_igdb_credentials,
    )
}

fn configured_emumovies_credentials() -> Result<Option<(String, String)>> {
    configured_credential_pair(
        "LUNCHBOX_EMUMOVIES_USERNAME",
        "LUNCHBOX_EMUMOVIES_PASSWORD",
        crate::settings::load_emumovies_credentials,
    )
}

fn configured_credential_pair<F>(
    first_name: &str,
    second_name: &str,
    load_saved: F,
) -> Result<Option<(String, String)>>
where
    F: FnOnce() -> Result<Option<(String, String)>>,
{
    let first = std::env::var(first_name)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let second = std::env::var(second_name)
        .ok()
        .filter(|value| !value.trim().is_empty());
    match (first, second) {
        (Some(first), Some(second)) => Ok(Some((first, second))),
        (None, None) => load_saved(),
        _ => bail!("both {first_name} and {second_name} must be set"),
    }
}

fn concise_provider_error(error: &str) -> String {
    const LIMIT: usize = 240;
    let error = error.trim();
    if error.chars().count() <= LIMIT {
        error.to_owned()
    } else {
        format!("{}…", error.chars().take(LIMIT).collect::<String>())
    }
}

fn missing_outcome(request: MediaFetchRequest) -> MediaFetchOutcome {
    MediaFetchOutcome::Missing {
        request_id: request.request_id,
        database_id: request.database_id,
        requested_kind: request.requested_kind,
        force: request.force,
    }
}

fn fetch_screenscraper(
    root: &Path,
    request: &MediaFetchRequest,
) -> Result<Option<(ArtworkKind, PathBuf)>> {
    let link = crate::settings::SettingsStore::open_default()?
        .media_provider_game_link("screenscraper", request.database_id)?;
    if link.is_none()
        && !request.force
        && screenscraper_negative_cache_is_fresh(
            root,
            request.database_id,
            request.requested_kind,
            request.exact_only,
        )
    {
        return Ok(None);
    }
    let Some(credentials) = crate::screenscraper::configured_credentials()? else {
        return Ok(None);
    };
    let client = crate::screenscraper::Client::new(credentials)?;
    let game_id = if let Some(link) = link {
        positive_provider_game_id(&link.provider_game_id, "ScreenScraper")?
    } else {
        if crate::screenscraper::platform_id(&request.platform).is_none() {
            return Ok(None);
        }
        let candidates = client.search_games(&request.title, &request.platform)?;
        let Some(game_id) = unique_exact_provider_game_id(
            candidates
                .iter()
                .map(|candidate| (candidate.id, candidate.name.as_str())),
            &request.title,
        ) else {
            write_screenscraper_negative_cache(root, request)?;
            return Ok(None);
        };
        game_id
    };
    let fetched = client.download_linked_artwork(
        game_id,
        root,
        request.database_id,
        request.requested_kind,
        request.exact_only,
    )?;
    if fetched.is_some() {
        let _ = fs::remove_file(screenscraper_negative_cache_path(
            root,
            request.database_id,
            request.requested_kind,
            request.exact_only,
        ));
    } else {
        write_screenscraper_negative_cache(root, request)?;
    }
    Ok(fetched)
}

pub(crate) fn retrieval_kinds(requested: ArtworkKind, exact_only: bool) -> Vec<ArtworkKind> {
    if exact_only {
        vec![requested]
    } else {
        requested.retrieval_fallbacks().to_vec()
    }
}

fn download_png(agent: &ureq::Agent, url: &str) -> Result<Option<Vec<u8>>> {
    let mut response = agent
        .get(url)
        .header("User-Agent", "Lunchbox/0.1 artwork cache")
        .call()
        .with_context(|| format!("requesting LibRetro artwork from {url}"))?;
    let status = response.status().as_u16();
    if status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        bail!("LibRetro artwork request failed with HTTP {status}");
    }
    if response
        .headers()
        .get(ureq::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_IMAGE_BYTES)
    {
        bail!("LibRetro artwork exceeds the 8 MiB safety limit");
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("reading LibRetro artwork")?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        bail!("LibRetro artwork exceeds the 8 MiB safety limit");
    }
    if !bytes.starts_with(PNG_SIGNATURE) {
        bail!("LibRetro artwork response is not a PNG image");
    }
    Ok(Some(bytes))
}

fn write_download_atomically(
    path: &Path,
    bytes: &[u8],
    worker_index: usize,
    replace_existing: bool,
) -> Result<()> {
    let parent = path
        .parent()
        .context("artwork cache path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating artwork cache directory {}", parent.display()))?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("artwork");
    let temporary = parent.join(format!(
        ".{stem}.{}.{worker_index}.download",
        std::process::id()
    ));
    let mut file = fs::File::create(&temporary)
        .with_context(|| format!("creating temporary artwork {}", temporary.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("writing temporary artwork {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing temporary artwork {}", temporary.display()))?;
    drop(file);
    if path.is_file() && !replace_existing {
        fs::remove_file(&temporary).ok();
        return Ok(());
    }
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) if path.is_file() && !replace_existing => {
            fs::remove_file(&temporary).ok();
            Ok(())
        }
        Err(rename_error) if path.is_file() => {
            let backup = parent.join(format!(
                ".{stem}.{}.{worker_index}.previous",
                std::process::id()
            ));
            fs::remove_file(&backup).ok();
            fs::rename(path, &backup).with_context(|| {
                format!(
                    "preparing to replace artwork {} after rename failed: {rename_error}",
                    path.display()
                )
            })?;
            match fs::rename(&temporary, path) {
                Ok(()) => {
                    fs::remove_file(&backup).ok();
                    Ok(())
                }
                Err(error) => {
                    let _ = fs::rename(&backup, path);
                    Err(error).with_context(|| {
                        format!(
                            "publishing replacement artwork {} as {}",
                            temporary.display(),
                            path.display()
                        )
                    })
                }
            }
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "publishing artwork {} as {}",
                temporary.display(),
                path.display()
            )
        }),
    }
}

fn media_output_path(root: &Path, database_id: i64, kind: ArtworkKind) -> PathBuf {
    root.join(format!("lb-{database_id}"))
        .join("libretro")
        .join(format!("{}.png", kind.cache_file_stem()))
}

fn negative_cache_path(root: &Path, database_id: i64, kind: ArtworkKind) -> PathBuf {
    root.join(format!("lb-{database_id}"))
        .join("libretro")
        .join(format!(".missing-{}", kind.key()))
}

fn screenscraper_negative_cache_path(
    root: &Path,
    database_id: i64,
    kind: ArtworkKind,
    exact_only: bool,
) -> PathBuf {
    root.join(format!("lb-{database_id}"))
        .join("screenscraper")
        .join(format!(
            ".missing-{}-{}",
            kind.key(),
            if exact_only { "exact" } else { "fallback" }
        ))
}

fn provider_negative_cache_path(
    root: &Path,
    provider: &str,
    database_id: i64,
    kind: ArtworkKind,
    exact_only: bool,
) -> PathBuf {
    root.join(format!("lb-{database_id}"))
        .join(provider)
        .join(format!(
            ".missing-{}-{}",
            kind.key(),
            if exact_only { "exact" } else { "fallback" }
        ))
}

fn negative_cache_is_fresh(root: &Path, database_id: i64, kind: ArtworkKind) -> bool {
    cache_marker_is_fresh(&negative_cache_path(root, database_id, kind))
}

fn screenscraper_negative_cache_is_fresh(
    root: &Path,
    database_id: i64,
    kind: ArtworkKind,
    exact_only: bool,
) -> bool {
    cache_marker_is_fresh(&screenscraper_negative_cache_path(
        root,
        database_id,
        kind,
        exact_only,
    ))
}

fn provider_negative_cache_is_fresh(
    root: &Path,
    provider: &str,
    database_id: i64,
    kind: ArtworkKind,
    exact_only: bool,
) -> bool {
    cache_marker_is_fresh(&provider_negative_cache_path(
        root,
        provider,
        database_id,
        kind,
        exact_only,
    ))
}

fn cache_marker_is_fresh(path: &Path) -> bool {
    let Some(modified) = fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
    else {
        return false;
    };
    modified.elapsed().unwrap_or(Duration::ZERO) < NEGATIVE_CACHE_TTL
}

fn write_screenscraper_negative_cache(root: &Path, request: &MediaFetchRequest) -> Result<()> {
    let path = screenscraper_negative_cache_path(
        root,
        request.database_id,
        request.requested_kind,
        request.exact_only,
    );
    let parent = path
        .parent()
        .context("ScreenScraper negative artwork cache path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "creating ScreenScraper negative artwork cache directory {}",
            parent.display()
        )
    })?;
    fs::write(&path, b"no unique exact ScreenScraper media match\n").with_context(|| {
        format!(
            "writing ScreenScraper negative artwork cache marker {}",
            path.display()
        )
    })
}

fn write_provider_negative_cache(
    root: &Path,
    provider: &str,
    request: &MediaFetchRequest,
    reason: &str,
) -> Result<()> {
    let path = provider_negative_cache_path(
        root,
        provider,
        request.database_id,
        request.requested_kind,
        request.exact_only,
    );
    let parent = path
        .parent()
        .context("provider negative artwork cache path has no parent directory")?;
    fs::create_dir_all(parent).with_context(|| {
        format!(
            "creating provider negative artwork cache directory {}",
            parent.display()
        )
    })?;
    fs::write(
        &path,
        format!(
            "provider={provider}\nplatform={}\ntitle={}\nreason={reason}\n",
            request.platform, request.title
        ),
    )
    .with_context(|| format!("writing provider negative artwork cache {}", path.display()))
}

fn remove_provider_negative_cache(root: &Path, provider: &str, request: &MediaFetchRequest) {
    let _ = fs::remove_file(provider_negative_cache_path(
        root,
        provider,
        request.database_id,
        request.requested_kind,
        request.exact_only,
    ));
}

fn write_negative_cache(root: &Path, request: &MediaFetchRequest, reason: &str) -> Result<()> {
    let path = negative_cache_path(root, request.database_id, request.requested_kind);
    let parent = path
        .parent()
        .context("negative artwork cache path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating artwork cache directory {}", parent.display()))?;
    fs::write(
        &path,
        format!(
            "provider=libretro\nplatform={}\ntitle={}\nreason={reason}\n",
            request.platform, request.title
        ),
    )
    .with_context(|| format!("writing negative artwork cache {}", path.display()))
}

fn libretro_platform_name(platform: &str) -> Option<&'static str> {
    let platform = platform.trim();
    LIBRETRO_PLATFORM_ALIASES
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(platform))
        .map(|(_, directory)| *directory)
}

fn libretro_title_candidates(title: &str) -> Vec<String> {
    const SUFFIXES: [&str; 6] = [
        "",
        " (USA)",
        " (World)",
        " (USA, Europe)",
        " (Europe)",
        " (Japan)",
    ];
    let mut bases = Vec::new();
    push_title_variants(&mut bases, title);
    if let Some(moved) = move_leading_article(title) {
        push_title_variants(&mut bases, &moved);
    }
    let mut candidates = Vec::with_capacity(bases.len() * SUFFIXES.len());
    for base in bases {
        for suffix in SUFFIXES {
            push_unique(&mut candidates, format!("{base}{suffix}"));
        }
    }
    candidates
}

fn push_title_variants(values: &mut Vec<String>, title: &str) {
    let title = strip_known_extension(title.trim());
    let official = sanitize_libretro_title(title, false);
    if !official.is_empty() {
        push_unique(values, official);
    }
    let colon_as_hyphen = sanitize_libretro_title(title, true);
    if !colon_as_hyphen.is_empty() {
        push_unique(values, colon_as_hyphen);
    }
}

fn sanitize_libretro_title(title: &str, colon_as_hyphen: bool) -> String {
    let mut output = String::with_capacity(title.len());
    for character in title.chars() {
        match character {
            ':' if colon_as_hyphen => output.push_str(" -"),
            '&' | '*' | '/' | '`' | ':' | '<' | '>' | '?' | '\\' | '|' | '"' => output.push('_'),
            _ => output.push(character),
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_known_extension(title: &str) -> &str {
    const EXTENSIONS: [&str; 9] = [
        ".zip", ".7z", ".rar", ".rom", ".bin", ".iso", ".cue", ".chd", ".nes",
    ];
    let lower = title.to_ascii_lowercase();
    EXTENSIONS
        .iter()
        .find_map(|extension| {
            lower
                .ends_with(extension)
                .then(|| &title[..title.len() - extension.len()])
        })
        .unwrap_or(title)
}

fn move_leading_article(title: &str) -> Option<String> {
    for article in ["The", "A", "An"] {
        if let Some(remainder) = title
            .strip_prefix(article)
            .and_then(|value| value.strip_prefix(' '))
            && !remainder.trim().is_empty()
        {
            return Some(format!("{}, {article}", remainder.trim()));
        }
    }
    None
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetch_request(database_id: i64, kind: ArtworkKind) -> MediaFetchRequest {
        MediaFetchRequest {
            request_id: format!("game-{database_id}"),
            database_id,
            title: format!("Game {database_id}"),
            platform: "Test platform".to_owned(),
            requested_kind: kind,
            force: false,
            exact_only: false,
        }
    }

    #[test]
    fn foreground_artwork_always_precedes_newer_background_work() {
        let mut state = MediaFetchState::default();
        state.push(fetch_request(1, ArtworkKind::BoxFront), false);
        state.push(fetch_request(2, ArtworkKind::BoxFront), false);
        state.push(fetch_request(3, ArtworkKind::Fanart), true);
        state.push(fetch_request(4, ArtworkKind::BoxFront), false);

        assert_eq!(state.pop_next().unwrap().database_id, 3);
        assert_eq!(state.pop_next().unwrap().database_id, 4);
        assert_eq!(state.pop_next().unwrap().database_id, 2);
    }

    #[test]
    fn queued_artwork_can_be_promoted_and_reasserted_without_duplication() {
        let mut state = MediaFetchState::default();
        state.push(fetch_request(1, ArtworkKind::BoxFront), false);
        state.push(fetch_request(2, ArtworkKind::BoxFront), true);
        state.push(fetch_request(3, ArtworkKind::Fanart), true);

        assert!(state.prioritize(1, ArtworkKind::BoxFront));
        assert!(state.prioritize(2, ArtworkKind::BoxFront));
        assert!(!state.prioritize(9, ArtworkKind::BoxFront));
        assert_eq!(state.queued_len(), 3);
        assert_eq!(state.pop_next().unwrap().database_id, 2);
        assert_eq!(state.pop_next().unwrap().database_id, 1);
        assert_eq!(state.pop_next().unwrap().database_id, 3);
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"image fixture").unwrap();
    }

    #[test]
    fn indexes_launchbox_directories_and_applies_provider_priority() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("lb-1019/steamgriddb/box-front.png"));
        touch(&directory.path().join("lb-1019/launchbox/box-front.jpg"));
        touch(
            &directory
                .path()
                .join("lb-1019/libretro/screenshot-gameplay.png"),
        );
        touch(&directory.path().join("hash-orphan/local/box-front.png"));

        let index = MediaIndex::scan(directory.path().to_owned());
        assert_eq!(index.games.len(), 1);
        assert_eq!(index.asset_count, 3);
        assert_eq!(index.candidate_count(1019, ArtworkKind::BoxFront), 3);
        assert!(
            index
                .candidate(1019, ArtworkKind::BoxFront, 1)
                .unwrap()
                .path
                .ends_with("steamgriddb/box-front.png")
        );
        assert!(
            index
                .candidate(1019, ArtworkKind::BoxFront, 2)
                .unwrap()
                .path
                .ends_with("libretro/screenshot-gameplay.png")
        );
        let cover = index.exact(1019, ArtworkKind::BoxFront).unwrap();
        assert_eq!(cover.source, "launchbox");
        assert!(cover.path.ends_with("launchbox/box-front.jpg"));
        assert_eq!(
            index.exact(1019, ArtworkKind::Screenshot).unwrap().source,
            "libretro"
        );
    }

    #[test]
    fn configured_provider_priority_changes_the_selected_candidate() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("lb-1019/steamgriddb/box-front.png"));
        touch(&directory.path().join("lb-1019/launchbox/box-front.jpg"));
        let mut priority = default_provider_priority();
        let steam_grid_index = priority
            .iter()
            .position(|provider| provider == "steamgriddb")
            .unwrap();
        let steam_grid = priority.remove(steam_grid_index);
        priority.insert(0, steam_grid);

        let index = MediaIndex::scan_with_provider_priority(directory.path().to_owned(), priority);
        let cover = index.exact(1019, ArtworkKind::BoxFront).unwrap();
        assert_eq!(cover.source, "steamgriddb");
        assert!(cover.path.ends_with("steamgriddb/box-front.png"));
    }

    #[test]
    fn cancellable_media_scan_stops_before_reading_game_directories() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("lb-1019/local/box-front.png"));
        let cancelled = AtomicBool::new(true);
        let index = MediaIndex::scan_with_control(
            directory.path().to_owned(),
            default_provider_priority(),
            &cancelled,
            |_, _| {},
        );
        assert!(index.games.is_empty());
        assert_eq!(index.asset_count, 0);
    }

    #[test]
    fn provider_priority_requires_the_complete_unique_provider_set() {
        let mut duplicate = default_provider_priority();
        duplicate[1] = "local".into();
        assert!(normalize_provider_priority(&duplicate).is_err());

        let mut unknown = default_provider_priority();
        unknown[1] = "untrusted".into();
        assert!(normalize_provider_priority(&unknown).is_err());

        let mut missing = default_provider_priority();
        missing.pop();
        assert!(normalize_provider_priority(&missing).is_err());
        assert_eq!(
            effective_provider_priority(&missing),
            default_provider_priority()
        );
    }

    #[test]
    fn automatic_downloads_follow_configured_network_provider_order() {
        let mut priority = default_provider_priority();
        let emumovies_index = priority
            .iter()
            .position(|provider| provider == "emumovies")
            .unwrap();
        let emumovies = priority.remove(emumovies_index);
        priority.insert(0, emumovies);

        assert_eq!(
            automatic_provider_order(&priority),
            vec![
                "emumovies",
                "libretro",
                "steamgriddb",
                "igdb",
                "screenscraper"
            ]
        );
        assert!(!automatic_provider_order(&priority).contains(&"websearch".to_owned()));
    }

    #[test]
    fn automatic_provider_search_accepts_only_one_strict_exact_title() {
        assert_eq!(
            unique_exact_provider_game_id(
                [(42, "Super Mario Odyssey"), (43, "Super Mario Odyssey 2")],
                "  super   mario odyssey ",
            ),
            Some(42)
        );
        assert_eq!(
            unique_exact_provider_game_id(
                [(42, "Super Mario Odyssey"), (44, "super mario odyssey")],
                "Super Mario Odyssey",
            ),
            None
        );
        assert_eq!(
            unique_exact_provider_game_id([(42, "Super Mario Odyssey™")], "Super Mario Odyssey",),
            None
        );
    }

    #[test]
    fn igdb_automatic_search_requires_an_exact_platform_identity() {
        let switch_game = crate::igdb::GameCandidate {
            id: 42,
            name: "Super Mario Odyssey".to_owned(),
            first_release_date: None,
            platforms: vec![crate::igdb::Platform {
                name: "Nintendo Switch".to_owned(),
                abbreviation: "Switch".to_owned(),
            }],
            cover: None,
            screenshots: Vec::new(),
            artworks: Vec::new(),
            ..crate::igdb::GameCandidate::default()
        };

        assert!(igdb_platform_matches(&switch_game, "Nintendo Switch"));
        assert!(!igdb_platform_matches(&switch_game, "Nintendo Wii U"));
    }

    #[test]
    fn failed_provider_is_temporarily_bypassed_without_blocking_the_chain() {
        let provider = "test-provider-failure-backoff";
        clear_provider_failure(provider);
        assert!(!provider_failure_backoff_is_active(provider));
        remember_provider_failure(provider);
        assert!(provider_failure_backoff_is_active(provider));
        clear_provider_failure(provider);
        assert!(!provider_failure_backoff_is_active(provider));
    }

    #[test]
    fn selected_artwork_uses_explicit_kind_fallbacks() {
        let mut media = GameMedia::default();
        media.insert(
            ArtworkKind::BoxFront,
            MediaAsset {
                path: "cover.png".into(),
                source: "local".into(),
                source_rank: 0,
                format_rank: 0,
            },
        );
        assert_eq!(
            media.selected(ArtworkKind::ClearLogo).unwrap().path,
            PathBuf::from("cover.png")
        );
        assert!(media.exact(ArtworkKind::ClearLogo).is_none());
    }

    #[test]
    fn bulk_repair_can_require_the_exact_requested_media_category() {
        assert_eq!(
            retrieval_kinds(ArtworkKind::ClearLogo, true),
            vec![ArtworkKind::ClearLogo]
        );
        assert_eq!(
            retrieval_kinds(ArtworkKind::ClearLogo, false),
            vec![ArtworkKind::ClearLogo, ArtworkKind::BoxFront]
        );
    }

    #[test]
    fn exact_repair_fetch_preserves_request_identity_and_publishes_atomically() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            assert!(
                String::from_utf8_lossy(&request[..read])
                    .starts_with("GET /Nintendo%20-%20Nintendo%20Entertainment%20System/Named_Boxarts/Exact%20Game.png ")
            );
            let bytes = b"\x89PNG\r\n\x1a\nexact-repair-fixture";
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(bytes).unwrap();
        });
        let root = tempfile::tempdir().unwrap();
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(3)))
            .http_status_as_error(false)
            .build()
            .into();
        let outcome = fetch_media(
            &agent,
            root.path(),
            &format!("http://{address}"),
            &default_provider_priority(),
            MediaFetchRequest {
                request_id: "stable-game-uid".to_owned(),
                database_id: 42,
                title: "Exact Game".to_owned(),
                platform: "Nintendo Entertainment System".to_owned(),
                requested_kind: ArtworkKind::BoxFront,
                force: true,
                exact_only: true,
            },
            0,
        );
        server.join().unwrap();
        match outcome {
            MediaFetchOutcome::Found {
                request_id,
                requested_kind,
                fetched_kind,
                path,
                ..
            } => {
                assert_eq!(request_id, "stable-game-uid");
                assert_eq!(requested_kind, ArtworkKind::BoxFront);
                assert_eq!(fetched_kind, ArtworkKind::BoxFront);
                assert_eq!(
                    fs::read(path).unwrap(),
                    b"\x89PNG\r\n\x1a\nexact-repair-fixture"
                );
            }
            outcome => panic!("unexpected repair outcome: {outcome:?}"),
        }
    }

    #[test]
    fn ignores_unrelated_files_and_non_positive_database_ids() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("lb-0/local/box-front.png"));
        touch(&directory.path().join("lb-nope/local/box-front.png"));
        touch(&directory.path().join("lb-2/local/notes.txt"));
        touch(&directory.path().join("lb-2/local/unknown.png"));

        let index = MediaIndex::scan(directory.path().to_owned());
        assert!(index.games.is_empty());
        assert_eq!(index.asset_count, 0);
    }

    #[test]
    fn missing_media_directory_is_an_empty_cache_not_an_error() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("does-not-exist");
        let index = MediaIndex::scan(path.clone());
        assert_eq!(index.root, path);
        assert!(index.games.is_empty());
        assert!(index.warning.is_none());
    }

    #[test]
    fn supplemental_media_prefers_provider_before_format() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("lb-140/emumovies/video.mp4"));
        touch(&directory.path().join("lb-140/emumovies/manual.pdf"));
        touch(&directory.path().join("lb-140/minerva/manual.zip"));
        touch(&directory.path().join("lb-140/local/video.webm"));

        let media = scan_supplemental_directory(
            directory.path(),
            &directory.path().join("lb-140"),
            &default_provider_priority(),
        )
        .unwrap();
        assert_eq!(media.video.unwrap().source, "local");
        let manual = media.manual.unwrap();
        assert_eq!(manual.source, "minerva");
        assert!(manual.path.ends_with("minerva/manual.zip"));
    }

    #[test]
    fn supplemental_media_uses_the_configured_provider_priority() {
        let directory = tempfile::tempdir().unwrap();
        touch(&directory.path().join("lb-140/local/video.webm"));
        touch(&directory.path().join("lb-140/emumovies/video.mp4"));
        let mut priority = default_provider_priority();
        let emumovies_index = priority
            .iter()
            .position(|provider| provider == "emumovies")
            .unwrap();
        let emumovies = priority.remove(emumovies_index);
        priority.insert(0, emumovies);

        let media = scan_supplemental_directory(
            directory.path(),
            &directory.path().join("lb-140"),
            &priority,
        )
        .unwrap();
        let video = media.video.unwrap();
        assert_eq!(video.source, "emumovies");
        assert!(video.path.ends_with("emumovies/video.mp4"));
    }

    #[test]
    fn supplemental_media_indexes_emumovies_soundtrack_sidecars() {
        let directory = tempfile::tempdir().unwrap();
        let audio = directory
            .path()
            .join("lb-140/emumovies/soundtrack-0123456789abcdef.mp3");
        touch(&audio);
        fs::write(audio.with_extension("title"), "Overworld Theme").unwrap();

        let media = scan_supplemental_directory(
            directory.path(),
            &directory.path().join("lb-140"),
            &default_provider_priority(),
        )
        .unwrap();

        assert_eq!(media.soundtrack.len(), 1);
        assert_eq!(media.soundtrack[0].source, "emumovies");
        assert_eq!(media.soundtrack[0].title, "Overworld Theme");
        assert_eq!(media.soundtrack[0].path, audio);
    }

    #[test]
    fn media_identity_changes_when_the_file_content_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("video.mp4");
        fs::write(&path, vec![1_u8; 130_000]).unwrap();
        let first = media_identity(&path).unwrap();
        fs::write(&path, vec![2_u8; 130_000]).unwrap();
        let second = media_identity(&path).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), "sha256:".len() + 64);
    }

    #[test]
    fn libretro_candidates_are_bounded_and_cover_article_and_region_variants() {
        let candidates = libretro_title_candidates("A Nightmare on Elm Street");
        assert_eq!(candidates[0], "A Nightmare on Elm Street");
        assert!(candidates.contains(&"Nightmare on Elm Street, A (USA)".to_owned()));
        assert!(candidates.len() <= 24);
        assert_eq!(
            percent_encode_path_segment("Nightmare on Elm Street, A (USA)"),
            "Nightmare%20on%20Elm%20Street%2C%20A%20%28USA%29"
        );
    }

    #[test]
    fn libretro_platform_aliases_are_unique_and_map_common_catalog_names() {
        let mut aliases = std::collections::HashSet::new();
        for (alias, directory) in LIBRETRO_PLATFORM_ALIASES {
            assert!(
                aliases.insert(alias.to_ascii_lowercase()),
                "duplicate {alias}"
            );
            assert!(!directory.trim().is_empty());
        }
        assert_eq!(
            libretro_platform_name("Nintendo Entertainment System"),
            Some("Nintendo - Nintendo Entertainment System")
        );
        assert_eq!(libretro_platform_name("Nintendo Switch"), None);
    }

    #[test]
    fn inserting_downloaded_assets_preserves_ordered_alternates_without_double_counting_paths() {
        let mut index = MediaIndex::default();
        assert!(index.insert_asset(42, ArtworkKind::BoxFront, "first.png".into(), "libretro"));
        assert_eq!(index.games.len(), 1);
        assert_eq!(index.asset_count, 1);
        assert!(index.insert_asset(42, ArtworkKind::BoxFront, "later.png".into(), "websearch"));
        assert_eq!(index.asset_count, 2);
        assert_eq!(
            index.exact(42, ArtworkKind::BoxFront).unwrap().path,
            PathBuf::from("first.png")
        );
        assert_eq!(index.candidate_count(42, ArtworkKind::BoxFront), 2);
        assert_eq!(
            index.candidate(42, ArtworkKind::BoxFront, 1).unwrap().path,
            PathBuf::from("later.png")
        );
        assert!(!index.insert_asset(42, ArtworkKind::BoxFront, "first.png".into(), "libretro"));
        assert_eq!(index.asset_count, 2);
    }

    #[test]
    fn negative_cache_is_scoped_by_game_and_requested_artwork_kind() {
        let directory = tempfile::tempdir().unwrap();
        let request = MediaFetchRequest {
            request_id: "probe".to_owned(),
            database_id: 42,
            title: "Missing Game".to_owned(),
            platform: "Nintendo Entertainment System".to_owned(),
            requested_kind: ArtworkKind::BoxFront,
            force: false,
            exact_only: false,
        };
        write_negative_cache(directory.path(), &request, "test miss").unwrap();
        assert!(negative_cache_is_fresh(
            directory.path(),
            42,
            ArtworkKind::BoxFront
        ));
        assert!(!negative_cache_is_fresh(
            directory.path(),
            42,
            ArtworkKind::Screenshot
        ));
    }

    #[test]
    fn screenscraper_negative_cache_separates_exact_and_fallback_requests() {
        let directory = tempfile::tempdir().unwrap();
        let request = MediaFetchRequest {
            request_id: "probe".to_owned(),
            database_id: 42,
            title: "Probe".to_owned(),
            platform: "Nintendo Entertainment System".to_owned(),
            requested_kind: ArtworkKind::BoxFront,
            force: false,
            exact_only: true,
        };
        write_screenscraper_negative_cache(directory.path(), &request).unwrap();
        assert!(screenscraper_negative_cache_is_fresh(
            directory.path(),
            42,
            ArtworkKind::BoxFront,
            true,
        ));
        assert!(!screenscraper_negative_cache_is_fresh(
            directory.path(),
            42,
            ArtworkKind::BoxFront,
            false,
        ));
        assert!(!screenscraper_negative_cache_is_fresh(
            directory.path(),
            42,
            ArtworkKind::Screenshot,
            true,
        ));
    }

    #[test]
    fn explicit_refresh_replaces_only_the_owned_target_and_keeps_a_recovery_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lb-42/libretro/box-front.png");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"old artwork").unwrap();

        write_download_atomically(&path, b"new artwork", 0, true).unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new artwork");
        assert_eq!(
            fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .count(),
            1
        );
    }
}
