use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use sha2::{Digest, Sha256};

use crate::catalog;

const PROVIDER_PRIORITY: [&str; 9] = [
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

const LIBRETRO_THUMBNAILS_URL: &str = "https://thumbnails.libretro.com";
const MEDIA_DOWNLOAD_WORKERS: usize = 2;
const MEDIA_DOWNLOAD_QUEUE_CAPACITY: usize = 128;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

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
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameMedia {
    assets: HashMap<ArtworkKind, MediaAsset>,
}

impl GameMedia {
    pub fn selected(&self, requested: ArtworkKind) -> Option<&MediaAsset> {
        requested
            .fallbacks()
            .iter()
            .find_map(|kind| self.assets.get(kind))
    }

    pub fn exact(&self, requested: ArtworkKind) -> Option<&MediaAsset> {
        self.assets.get(&requested)
    }

    fn insert(&mut self, kind: ArtworkKind, asset: MediaAsset) -> bool {
        if self.assets.get(&kind).is_none_or(|existing| {
            (
                asset.source_rank,
                asset.format_rank,
                &asset.source,
                &asset.path,
            ) < (
                existing.source_rank,
                existing.format_rank,
                &existing.source,
                &existing.path,
            )
        }) {
            self.assets.insert(kind, asset);
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MediaIndex {
    pub root: PathBuf,
    pub games: HashMap<i64, GameMedia>,
    pub asset_count: usize,
    pub skipped_entries: usize,
    pub warning: Option<String>,
}

impl MediaIndex {
    pub fn scan(root: PathBuf) -> Self {
        let mut index = Self {
            root: root.clone(),
            ..Self::default()
        };
        let directories = match fs::read_dir(&root) {
            Ok(directories) => directories,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return index,
            Err(error) => {
                index.warning = Some(format!("could not read {}: {error}", root.display()));
                return index;
            }
        };

        for directory in directories {
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
            let media = scan_game_directory(&directory.path(), &mut index.skipped_entries);
            if !media.assets.is_empty() {
                index.asset_count += media.assets.len();
                index.games.insert(database_id, media);
            }
        }
        index
    }

    pub fn selected(&self, database_id: i64, kind: ArtworkKind) -> Option<&MediaAsset> {
        self.games.get(&database_id)?.selected(kind)
    }

    pub fn exact(&self, database_id: i64, kind: ArtworkKind) -> Option<&MediaAsset> {
        self.games.get(&database_id)?.exact(kind)
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
        let added = !game_media.assets.contains_key(&kind);
        let inserted = game_media.insert(
            kind,
            MediaAsset {
                path,
                source: source.to_owned(),
                source_rank: provider_rank(source),
                format_rank: 0,
            },
        );
        if inserted && added {
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
    let mut directories = Vec::with_capacity(2);
    if database_id > 0 {
        directories.push(game_media_directory_in(&root, game_id, database_id)?);
    }
    if !game_id.trim().is_empty() && safe_game_identity(game_id) {
        directories.push(root.join(format!("game-{game_id}")));
    }

    let mut selected = SupplementalMedia::default();
    for directory in directories {
        let candidate = scan_supplemental_directory(&root, &directory)?;
        select_supplemental_asset(&mut selected.video, candidate.video);
        select_supplemental_asset(&mut selected.manual, candidate.manual);
    }
    Ok(selected)
}

pub fn game_media_directory(game_id: &str, database_id: i64) -> Result<PathBuf> {
    game_media_directory_in(&requested_media_directory(), game_id, database_id)
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

fn scan_supplemental_directory(root: &Path, directory: &Path) -> Result<SupplementalMedia> {
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
        let source_rank = provider_rank(&source_name);
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
    Ok(result)
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

fn parse_launchbox_directory(value: &std::ffi::OsStr) -> Option<i64> {
    let value = value.to_str()?;
    let database_id = value.strip_prefix("lb-")?.parse::<i64>().ok()?;
    (database_id > 0).then_some(database_id)
}

fn scan_game_directory(path: &Path, skipped_entries: &mut usize) -> GameMedia {
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
        let source_rank = provider_rank(&source_name);
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

fn provider_rank(source: &str) -> usize {
    PROVIDER_PRIORITY
        .iter()
        .position(|candidate| *candidate == source)
        .unwrap_or(PROVIDER_PRIORITY.len())
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
    pub database_id: i64,
    pub title: String,
    pub platform: String,
    pub requested_kind: ArtworkKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaFetchOutcome {
    Found {
        database_id: i64,
        requested_kind: ArtworkKind,
        fetched_kind: ArtworkKind,
        path: PathBuf,
    },
    Missing {
        database_id: i64,
        requested_kind: ArtworkKind,
    },
    Failed {
        database_id: i64,
        requested_kind: ArtworkKind,
        error: String,
    },
}

pub struct MediaFetchQueue {
    shared: Arc<(Mutex<MediaFetchState>, Condvar)>,
}

#[derive(Default)]
struct MediaFetchState {
    requests: VecDeque<MediaFetchRequest>,
    shutdown: bool,
}

impl MediaFetchQueue {
    pub fn start<F>(root: PathBuf, callback: F) -> Result<Self>
    where
        F: Fn(MediaFetchOutcome) + Send + Sync + 'static,
    {
        let shared = Arc::new((Mutex::new(MediaFetchState::default()), Condvar::new()));
        let callback = Arc::new(callback);
        let base_url = requested_libretro_base_url();
        for worker_index in 0..MEDIA_DOWNLOAD_WORKERS {
            let shared = Arc::clone(&shared);
            let worker_shared = Arc::clone(&shared);
            let callback = Arc::clone(&callback);
            let root = root.clone();
            let base_url = base_url.clone();
            if let Err(error) = std::thread::Builder::new()
                .name(format!("lunchbox-media-fetch-{worker_index}"))
                .spawn(move || {
                    media_fetch_worker(worker_shared, callback, root, base_url, worker_index)
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
        let (state, ready) = &*self.shared;
        let mut state = state
            .lock()
            .map_err(|_| anyhow::anyhow!("artwork retrieval queue is unavailable"))?;
        if state.shutdown {
            bail!("artwork retrieval workers are unavailable");
        }
        if state.requests.len() >= MEDIA_DOWNLOAD_QUEUE_CAPACITY {
            bail!("artwork retrieval queue is full");
        }
        state.requests.push_back(request);
        ready.notify_one();
        Ok(())
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
            while state.requests.is_empty() && !state.shutdown {
                state = match ready.wait(state) {
                    Ok(state) => state,
                    Err(_) => return,
                };
            }
            if state.shutdown {
                return;
            }
            state.requests.pop_back()
        };
        let Some(request) = request else { continue };
        callback(fetch_media(&agent, &root, &base_url, request, worker_index));
    }
}

fn fetch_media(
    agent: &ureq::Agent,
    root: &Path,
    base_url: &str,
    request: MediaFetchRequest,
    worker_index: usize,
) -> MediaFetchOutcome {
    if negative_cache_is_fresh(root, request.database_id, request.requested_kind) {
        return MediaFetchOutcome::Missing {
            database_id: request.database_id,
            requested_kind: request.requested_kind,
        };
    }

    let Some(platform) = libretro_platform_name(&request.platform) else {
        let _ = write_negative_cache(root, &request, "platform is not provided by LibRetro");
        return MediaFetchOutcome::Missing {
            database_id: request.database_id,
            requested_kind: request.requested_kind,
        };
    };

    for fetched_kind in request.requested_kind.retrieval_fallbacks() {
        let Some(type_directory) = fetched_kind.libretro_path_segment() else {
            continue;
        };
        let output = media_output_path(root, request.database_id, *fetched_kind);
        if output.is_file() {
            return MediaFetchOutcome::Found {
                database_id: request.database_id,
                requested_kind: request.requested_kind,
                fetched_kind: *fetched_kind,
                path: output,
            };
        }

        for candidate in libretro_title_candidates(&request.title) {
            let url = format!(
                "{}/{}/{}/{}.png",
                base_url,
                percent_encode_path_segment(platform),
                type_directory,
                percent_encode_path_segment(&candidate)
            );
            match download_png(agent, &url) {
                Ok(Some(bytes)) => match write_download_atomically(&output, &bytes, worker_index) {
                    Ok(()) => {
                        let _ = fs::remove_file(negative_cache_path(
                            root,
                            request.database_id,
                            request.requested_kind,
                        ));
                        return MediaFetchOutcome::Found {
                            database_id: request.database_id,
                            requested_kind: request.requested_kind,
                            fetched_kind: *fetched_kind,
                            path: output,
                        };
                    }
                    Err(error) => {
                        return MediaFetchOutcome::Failed {
                            database_id: request.database_id,
                            requested_kind: request.requested_kind,
                            error: error.to_string(),
                        };
                    }
                },
                Ok(None) => {}
                Err(error) => {
                    return MediaFetchOutcome::Failed {
                        database_id: request.database_id,
                        requested_kind: request.requested_kind,
                        error: error.to_string(),
                    };
                }
            }
        }
    }

    let _ = write_negative_cache(root, &request, "no matching LibRetro thumbnail");
    MediaFetchOutcome::Missing {
        database_id: request.database_id,
        requested_kind: request.requested_kind,
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

fn write_download_atomically(path: &Path, bytes: &[u8], worker_index: usize) -> Result<()> {
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
    fs::write(&temporary, bytes)
        .with_context(|| format!("writing temporary artwork {}", temporary.display()))?;
    if path.is_file() {
        fs::remove_file(&temporary).ok();
        return Ok(());
    }
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) if path.is_file() => {
            fs::remove_file(&temporary).ok();
            Ok(())
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

fn negative_cache_is_fresh(root: &Path, database_id: i64, kind: ArtworkKind) -> bool {
    let path = negative_cache_path(root, database_id, kind);
    let Some(modified) = fs::metadata(&path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
    else {
        return false;
    };
    modified.elapsed().unwrap_or(Duration::ZERO) < NEGATIVE_CACHE_TTL
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
        assert_eq!(index.asset_count, 2);
        let cover = index.exact(1019, ArtworkKind::BoxFront).unwrap();
        assert_eq!(cover.source, "launchbox");
        assert!(cover.path.ends_with("launchbox/box-front.jpg"));
        assert_eq!(
            index.exact(1019, ArtworkKind::Screenshot).unwrap().source,
            "libretro"
        );
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

        let media = scan_supplemental_directory(directory.path(), &directory.path().join("lb-140"))
            .unwrap();
        assert_eq!(media.video.unwrap().source, "local");
        let manual = media.manual.unwrap();
        assert_eq!(manual.source, "minerva");
        assert!(manual.path.ends_with("minerva/manual.zip"));
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
    fn inserting_downloaded_assets_updates_counts_without_double_counting_replacements() {
        let mut index = MediaIndex::default();
        assert!(index.insert_asset(42, ArtworkKind::BoxFront, "first.png".into(), "libretro"));
        assert_eq!(index.games.len(), 1);
        assert_eq!(index.asset_count, 1);
        assert!(!index.insert_asset(42, ArtworkKind::BoxFront, "later.png".into(), "websearch"));
        assert_eq!(index.asset_count, 1);
        assert_eq!(
            index.exact(42, ArtworkKind::BoxFront).unwrap().path,
            PathBuf::from("first.png")
        );
    }

    #[test]
    fn negative_cache_is_scoped_by_game_and_requested_artwork_kind() {
        let directory = tempfile::tempdir().unwrap();
        let request = MediaFetchRequest {
            database_id: 42,
            title: "Missing Game".to_owned(),
            platform: "Nintendo Entertainment System".to_owned(),
            requested_kind: ArtworkKind::BoxFront,
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
}
