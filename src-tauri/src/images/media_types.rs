//! Normalized media types and source mappings
//!
//! Provides a unified abstraction over different image sources' naming conventions.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::PathBuf;

/// Normalized media types that map to different source-specific names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizedMediaType {
    BoxFront,
    BoxBack,
    Box3D,
    Screenshot,
    TitleScreen,
    ClearLogo,
    Fanart,
    Banner,
}

impl NormalizedMediaType {
    /// All media types in display order
    pub fn all() -> &'static [NormalizedMediaType] {
        &[
            NormalizedMediaType::BoxFront,
            NormalizedMediaType::BoxBack,
            NormalizedMediaType::Box3D,
            NormalizedMediaType::Screenshot,
            NormalizedMediaType::TitleScreen,
            NormalizedMediaType::ClearLogo,
            NormalizedMediaType::Fanart,
            NormalizedMediaType::Banner,
        ]
    }

    /// Get the filename for this media type (without extension)
    pub fn filename(&self) -> &'static str {
        match self {
            NormalizedMediaType::BoxFront => "box-front",
            NormalizedMediaType::BoxBack => "box-back",
            NormalizedMediaType::Box3D => "box-3d",
            NormalizedMediaType::Screenshot => "screenshot",
            NormalizedMediaType::TitleScreen => "title-screen",
            NormalizedMediaType::ClearLogo => "clear-logo",
            NormalizedMediaType::Fanart => "fanart",
            NormalizedMediaType::Banner => "banner",
        }
    }

    /// Parse from string (kebab-case)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "box-front" => Some(NormalizedMediaType::BoxFront),
            "box-back" => Some(NormalizedMediaType::BoxBack),
            "box-3d" => Some(NormalizedMediaType::Box3D),
            "screenshot" => Some(NormalizedMediaType::Screenshot),
            "title-screen" => Some(NormalizedMediaType::TitleScreen),
            "clear-logo" => Some(NormalizedMediaType::ClearLogo),
            "fanart" => Some(NormalizedMediaType::Fanart),
            "banner" => Some(NormalizedMediaType::Banner),
            _ => None,
        }
    }

    /// Convert from LaunchBox image type name
    pub fn from_launchbox_type(launchbox_type: &str) -> Option<Self> {
        match launchbox_type {
            "Box - Front" => Some(NormalizedMediaType::BoxFront),
            "Box - Back" => Some(NormalizedMediaType::BoxBack),
            "Box - 3D" => Some(NormalizedMediaType::Box3D),
            "Screenshot - Gameplay" | "Screenshot" => Some(NormalizedMediaType::Screenshot),
            "Screenshot - Game Title" => Some(NormalizedMediaType::TitleScreen),
            "Clear Logo" => Some(NormalizedMediaType::ClearLogo),
            "Fanart - Background" | "Fanart" => Some(NormalizedMediaType::Fanart),
            "Banner" => Some(NormalizedMediaType::Banner),
            _ => None,
        }
    }

    /// Get LaunchBox image type name
    pub fn to_launchbox_type(&self) -> &'static str {
        match self {
            NormalizedMediaType::BoxFront => "Box - Front",
            NormalizedMediaType::BoxBack => "Box - Back",
            NormalizedMediaType::Box3D => "Box - 3D",
            NormalizedMediaType::Screenshot => "Screenshot - Gameplay",
            NormalizedMediaType::TitleScreen => "Screenshot - Game Title",
            NormalizedMediaType::ClearLogo => "Clear Logo",
            NormalizedMediaType::Fanart => "Fanart - Background",
            NormalizedMediaType::Banner => "Banner",
        }
    }

    /// Get libretro thumbnail type (if supported)
    pub fn to_libretro_type(&self) -> Option<&'static str> {
        match self {
            NormalizedMediaType::BoxFront | NormalizedMediaType::BoxBack => Some("Named_Boxarts"),
            NormalizedMediaType::Screenshot => Some("Named_Snaps"),
            NormalizedMediaType::TitleScreen => Some("Named_Titles"),
            _ => None,
        }
    }

    /// Get EmuMovies media type (if supported)
    pub fn to_emumovies_type(&self) -> Option<&'static str> {
        match self {
            NormalizedMediaType::BoxFront => Some("Box"),
            NormalizedMediaType::BoxBack => Some("BoxBack"),
            NormalizedMediaType::Box3D => Some("Box3D"),
            NormalizedMediaType::Screenshot => Some("Snap"),
            NormalizedMediaType::TitleScreen => Some("Title"),
            NormalizedMediaType::ClearLogo => Some("Logos"),
            NormalizedMediaType::Fanart => Some("Fanart"),
            NormalizedMediaType::Banner => Some("Banner"),
        }
    }

    /// Get SteamGridDB artwork type (if supported)
    pub fn to_steamgriddb_type(&self) -> Option<&'static str> {
        match self {
            NormalizedMediaType::BoxFront => Some("grid"),
            NormalizedMediaType::ClearLogo => Some("logo"),
            NormalizedMediaType::Fanart => Some("hero"),
            NormalizedMediaType::Banner => Some("grid"), // SteamGridDB uses grid for banners too
            _ => None,
        }
    }

    /// Get human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            NormalizedMediaType::BoxFront => "Box Art",
            NormalizedMediaType::BoxBack => "Box Back",
            NormalizedMediaType::Box3D => "3D Box",
            NormalizedMediaType::Screenshot => "Screenshot",
            NormalizedMediaType::TitleScreen => "Title Screen",
            NormalizedMediaType::ClearLogo => "Clear Logo",
            NormalizedMediaType::Fanart => "Fanart",
            NormalizedMediaType::Banner => "Banner",
        }
    }
}

impl fmt::Display for NormalizedMediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filename())
    }
}

/// Identifier for game media - either LaunchBox DB ID or computed hash
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameMediaId {
    /// LaunchBox database ID (preferred, stable)
    LaunchBoxId(i64),
    /// SHA256 hash of platform_id + normalized_title (fallback)
    Hash(String),
}

impl GameMediaId {
    /// Create from LaunchBox database ID
    pub fn from_launchbox_id(id: i64) -> Self {
        GameMediaId::LaunchBoxId(id)
    }

    /// Compute hash identifier from platform and title
    pub fn compute_hash(platform_id: i64, title: &str) -> Self {
        let normalized = normalize_title(title);
        let input = format!("{}:{}", platform_id, normalized);
        let hash = Sha256::digest(input.as_bytes());
        GameMediaId::Hash(hex::encode(hash))
    }

    /// Get the directory name for this identifier
    pub fn directory_name(&self) -> String {
        match self {
            GameMediaId::LaunchBoxId(id) => format!("lb-{}", id),
            GameMediaId::Hash(hash) => format!("hash-{}", hash),
        }
    }

    /// Get the full media path for a specific media type
    pub fn media_path(&self, base_dir: &std::path::Path, media_type: NormalizedMediaType, extension: &str) -> PathBuf {
        base_dir
            .join("media")
            .join(self.directory_name())
            .join(format!("{}.{}", media_type.filename(), extension))
    }
}

/// Normalize a game title for hash computation
/// - Lowercase
/// - Remove special characters except alphanumeric and spaces
/// - Collapse multiple spaces
/// - Trim
fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Image source for downloads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaSource {
    LaunchBox,
    LibRetro,
    SteamGridDB,
    IGDB,
    EmuMovies,
    ScreenScraper,
}

impl MediaSource {
    /// All sources in default priority order
    pub fn all() -> &'static [MediaSource] {
        &[
            MediaSource::LaunchBox,
            MediaSource::LibRetro,
            MediaSource::SteamGridDB,
            MediaSource::IGDB,
            MediaSource::EmuMovies,
            MediaSource::ScreenScraper,
        ]
    }

    /// Get the database name for this source
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaSource::LaunchBox => "launchbox",
            MediaSource::LibRetro => "libretro",
            MediaSource::SteamGridDB => "steamgriddb",
            MediaSource::IGDB => "igdb",
            MediaSource::EmuMovies => "emumovies",
            MediaSource::ScreenScraper => "screenscraper",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "launchbox" => Some(MediaSource::LaunchBox),
            "libretro" => Some(MediaSource::LibRetro),
            "steamgriddb" => Some(MediaSource::SteamGridDB),
            "igdb" => Some(MediaSource::IGDB),
            "emumovies" => Some(MediaSource::EmuMovies),
            "screenscraper" => Some(MediaSource::ScreenScraper),
            _ => None,
        }
    }

    /// Get human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            MediaSource::LaunchBox => "LaunchBox",
            MediaSource::LibRetro => "libretro",
            MediaSource::SteamGridDB => "SteamGridDB",
            MediaSource::IGDB => "IGDB",
            MediaSource::EmuMovies => "EmuMovies",
            MediaSource::ScreenScraper => "ScreenScraper",
        }
    }

    /// Check if this source supports the given media type
    pub fn supports_media_type(&self, media_type: NormalizedMediaType) -> bool {
        match self {
            MediaSource::LaunchBox => true, // LaunchBox has all types
            MediaSource::LibRetro => media_type.to_libretro_type().is_some(),
            MediaSource::SteamGridDB => media_type.to_steamgriddb_type().is_some(),
            MediaSource::IGDB => matches!(
                media_type,
                NormalizedMediaType::BoxFront
                    | NormalizedMediaType::Screenshot
                    | NormalizedMediaType::Fanart
            ),
            MediaSource::EmuMovies => media_type.to_emumovies_type().is_some(),
            MediaSource::ScreenScraper => true, // ScreenScraper has most types
        }
    }
}

impl fmt::Display for MediaSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_media_type_roundtrip() {
        for media_type in NormalizedMediaType::all() {
            let filename = media_type.filename();
            let parsed = NormalizedMediaType::from_str(filename).unwrap();
            assert_eq!(*media_type, parsed);
        }
    }

    #[test]
    fn test_launchbox_type_conversion() {
        assert_eq!(
            NormalizedMediaType::from_launchbox_type("Box - Front"),
            Some(NormalizedMediaType::BoxFront)
        );
        assert_eq!(
            NormalizedMediaType::from_launchbox_type("Screenshot - Gameplay"),
            Some(NormalizedMediaType::Screenshot)
        );
    }

    #[test]
    fn test_game_media_id_directory() {
        let lb_id = GameMediaId::LaunchBoxId(12345);
        assert_eq!(lb_id.directory_name(), "lb-12345");

        let hash_id = GameMediaId::compute_hash(1, "Super Mario Bros.");
        assert!(hash_id.directory_name().starts_with("hash-"));
    }

    #[test]
    fn test_title_normalization() {
        assert_eq!(normalize_title("Super Mario Bros."), "super mario bros");
        assert_eq!(normalize_title("The Legend of Zelda: Ocarina of Time"), "the legend of zelda ocarina of time");
        assert_eq!(normalize_title("  Multiple   Spaces  "), "multiple spaces");
    }
}
