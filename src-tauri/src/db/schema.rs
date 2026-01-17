//! Database schema types - Single Source of Truth for game metadata
//!
//! All game metadata fields are defined here. Other modules should use these types
//! rather than defining their own Game structs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Platform from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DbPlatform {
    pub id: i64,
    pub name: String,
    pub launchbox_name: Option<String>,
    pub libretro_name: Option<String>,
    pub screenscraper_id: Option<i64>,
    pub openvgdb_system_id: Option<i64>,
    pub manufacturer: Option<String>,
    pub release_date: Option<String>,
    pub category: Option<String>,
    pub retroarch_core: Option<String>,
    pub file_extensions: Option<String>,
    pub aliases: Option<String>,
}

/// Database row for games table - single source of truth for all game metadata
///
/// This struct matches the database schema exactly. Use sqlx's FromRow derive
/// to automatically map database rows to this struct.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbGame {
    pub id: String,
    pub title: String,
    pub platform_id: i64,

    // External IDs for cross-referencing
    pub launchbox_db_id: Option<i64>,
    pub libretro_crc32: Option<String>,
    pub libretro_md5: Option<String>,
    pub libretro_sha1: Option<String>,
    pub libretro_serial: Option<String>,
    pub screenscraper_id: Option<i64>,
    pub igdb_id: Option<i64>,
    pub steamgriddb_id: Option<i64>,
    pub openvgdb_release_id: Option<i64>,
    pub steam_app_id: Option<i64>,

    // Core metadata (from Metadata.xml)
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub esrb: Option<String>,
    pub cooperative: Option<i32>,
    pub video_url: Option<String>,
    pub wikipedia_url: Option<String>,
    pub release_type: Option<String>,
    pub notes: Option<String>,

    // Extended metadata (from Platform XMLs)
    pub sort_title: Option<String>,
    pub series: Option<String>,
    pub region: Option<String>,
    pub play_mode: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,

    // Import tracking
    pub metadata_source: Option<String>,
}

/// SQL SELECT clause for all game columns
/// Use this constant to ensure consistent column selection across all queries
pub const GAME_COLUMNS: &str = r#"
    g.id, g.title, g.platform_id,
    g.launchbox_db_id, g.libretro_crc32, g.libretro_md5, g.libretro_sha1, g.libretro_serial,
    g.screenscraper_id, g.igdb_id, g.steamgriddb_id, g.openvgdb_release_id, g.steam_app_id,
    g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
    g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
    g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status,
    g.metadata_source
"#;

/// SQL SELECT with platform join - most common query pattern
pub const GAME_SELECT_WITH_PLATFORM: &str = r#"
    SELECT g.id, g.title, g.platform_id, p.name as platform_name,
           g.launchbox_db_id, g.libretro_crc32, g.libretro_md5, g.libretro_sha1, g.libretro_serial,
           g.screenscraper_id, g.igdb_id, g.steamgriddb_id, g.openvgdb_release_id, g.steam_app_id,
           g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
           g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
           g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status,
           g.metadata_source
    FROM games g
    JOIN platforms p ON g.platform_id = p.id
"#;

// ============================================================================
// Other database types (unchanged)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rom {
    pub id: String,
    pub game_id: Option<String>,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub crc32: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub region: Option<String>,
    pub version: Option<String>,
    pub verified: bool,
    pub last_played: Option<String>,
    pub play_count: i64,
    pub play_time_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Media {
    pub id: String,
    pub game_id: String,
    pub media_type: String,
    pub file_path: String,
    pub source: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Emulator {
    pub id: String,
    pub name: String,
    pub executable_path: Option<String>,
    pub emulator_type: String,
    pub version: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlatformEmulator {
    pub platform_id: i64,
    pub emulator_id: String,
    pub core_name: Option<String>,
    pub is_default: bool,
    pub command_line_args: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub filter_rules: Option<String>,
}

/// Alternate name for a game (regional titles, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameAlternateName {
    pub id: i64,
    pub launchbox_db_id: i64,
    pub alternate_name: String,
    pub region: Option<String>,
}

/// Image reference for on-demand download from LaunchBox CDN
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameImage {
    pub id: i64,
    pub launchbox_db_id: i64,
    pub filename: String,
    pub image_type: String,
    pub region: Option<String>,
    pub crc32: Option<String>,
    pub downloaded: bool,
    pub local_path: Option<String>,
}

// ============================================================================
// Game struct for API/frontend responses (wraps DbGame with computed fields)
// ============================================================================

/// Normalize a game title for display by removing region/version suffixes
/// "Super Mario Bros. (USA) (Rev A)" -> "Super Mario Bros."
pub fn normalize_title_for_display(title: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;

    for c in title.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth as i32 - 1).max(0) as usize,
            _ if depth == 0 => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}

/// Normalize title for deduplication - removes punctuation, normalizes whitespace, lowercases
/// This allows "Canon - The Legend" and "Canon: The Legend" to be considered the same game
pub fn normalize_title_for_dedup(title: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    let mut last_was_space = true; // Start true to trim leading spaces

    for c in title.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth as i32 - 1).max(0) as usize,
            _ if depth == 0 => {
                if c.is_alphanumeric() {
                    result.push(c.to_ascii_lowercase());
                    last_was_space = false;
                } else if !last_was_space {
                    // Replace any non-alphanumeric with a single space
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ => {}
        }
    }

    result.trim().to_string()
}

/// Extract region from title (e.g., "(USA)" -> "USA")
pub fn extract_region_from_title(title: &str) -> Option<String> {
    let regions = [
        "(USA)", "(World)", "(Europe)", "(Japan)", "(En)", "(Ja)", "(De)", "(Fr)",
        "(USA, Europe)", "(Japan, USA)", "(Japan, Europe)", "(Europe, Australia)",
        "(Korea)", "(Asia)", "(Taiwan)", "(Germany)", "(France)", "(Spain)", "(Italy)",
        "(Brazil)", "(Australia)", "(Netherlands)", "(Sweden)", "(China)",
    ];

    for region in regions {
        if title.contains(region) {
            return Some(region.trim_matches(|c| c == '(' || c == ')').to_string());
        }
    }
    None
}

/// Game for API/frontend display - extends DbGame with computed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub database_id: i64,
    pub title: String,
    pub display_title: String, // Clean title without region/version suffixes
    pub platform: String,
    pub platform_id: i64,
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Option<String>,
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub esrb: Option<String>,
    pub cooperative: Option<bool>,
    pub video_url: Option<String>,
    pub wikipedia_url: Option<String>,
    pub release_type: Option<String>,
    pub notes: Option<String>,
    pub sort_title: Option<String>,
    pub series: Option<String>,
    pub region: Option<String>,
    pub play_mode: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
    pub steam_app_id: Option<i64>,
    pub box_front_path: Option<String>,
    pub screenshot_path: Option<String>,
    pub variant_count: i32, // Number of variants (regions/versions)
}

impl Game {
    /// Create from DbGame with platform name
    pub fn from_db_game(db: DbGame, platform: String) -> Self {
        let display_title = normalize_title_for_display(&db.title);
        Self {
            id: db.id,
            database_id: db.launchbox_db_id.unwrap_or(0),
            title: db.title,
            display_title,
            platform,
            platform_id: db.platform_id,
            description: db.description,
            release_date: db.release_date,
            release_year: db.release_year,
            developer: db.developer,
            publisher: db.publisher,
            genres: db.genre,
            players: db.players,
            rating: db.rating,
            rating_count: db.rating_count,
            esrb: db.esrb,
            cooperative: db.cooperative.map(|c| c != 0),
            video_url: db.video_url,
            wikipedia_url: db.wikipedia_url,
            release_type: db.release_type,
            notes: db.notes,
            sort_title: db.sort_title,
            series: db.series,
            region: db.region,
            play_mode: db.play_mode,
            version: db.version,
            status: db.status,
            steam_app_id: db.steam_app_id,
            box_front_path: None,
            screenshot_path: None,
            variant_count: 1,
        }
    }

    /// Create from raw database row fields (for manual queries)
    #[allow(clippy::too_many_arguments)]
    pub fn from_row_fields(
        id: String,
        title: String,
        platform_id: i64,
        platform: String,
        description: Option<String>,
        release_date: Option<String>,
        release_year: Option<i32>,
        developer: Option<String>,
        publisher: Option<String>,
        genre: Option<String>,
        players: Option<String>,
        rating: Option<f64>,
        rating_count: Option<i64>,
        esrb: Option<String>,
        cooperative: Option<i32>,
        video_url: Option<String>,
        wikipedia_url: Option<String>,
        release_type: Option<String>,
        notes: Option<String>,
        sort_title: Option<String>,
        series: Option<String>,
        region: Option<String>,
        play_mode: Option<String>,
        version: Option<String>,
        status: Option<String>,
        steam_app_id: Option<i64>,
    ) -> Self {
        let display_title = normalize_title_for_display(&title);
        Self {
            id,
            database_id: 0,
            title,
            display_title,
            platform,
            platform_id,
            description,
            release_date,
            release_year,
            developer,
            publisher,
            genres: genre,
            players,
            rating,
            rating_count,
            esrb,
            cooperative: cooperative.map(|c| c != 0),
            video_url,
            wikipedia_url,
            release_type,
            notes,
            sort_title,
            series,
            region,
            play_mode,
            version,
            status,
            steam_app_id,
            box_front_path: None,
            screenshot_path: None,
            variant_count: 1,
        }
    }
}

/// Game variant (region/version) for variant selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVariant {
    pub id: String,
    pub title: String,
    pub region: Option<String>,
}

/// Platform for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub game_count: i64,
    pub aliases: Option<String>,
}
