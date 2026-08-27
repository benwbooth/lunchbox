use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaAsset {
    pub path: PathBuf,
    pub source: String,
    source_rank: usize,
    format_rank: usize,
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

    fn insert(&mut self, kind: ArtworkKind, asset: MediaAsset) {
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
}
