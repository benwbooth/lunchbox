//! Tauri commands exposed to the frontend

use crate::db::schema::{
    extract_region_from_title, normalize_title_for_display,
    Game, GameVariant, Platform,
};
use crate::handlers::{
    self, Collection, CollectionIdInput, CollectionGameInput,
    CreateCollectionInput, UpdateCollectionInput,
};
use crate::images::{CacheStats, ImageInfo, ImageService};
use crate::scraper::{get_screenscraper_platform_id, ScreenScraperClient, ScreenScraperConfig};
use crate::state::{AppSettings, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

type AppStateHandle = Arc<RwLock<AppState>>;

/// Generate search aliases for a platform name (used if not in database)
fn get_platform_search_aliases(name: &str) -> Option<String> {
    let aliases = match name {
        // Nintendo
        "Nintendo Entertainment System" => "NES, Famicom, FC, nes, famicom",
        "Super Nintendo Entertainment System" => "SNES, Super Famicom, SFC, snes, snesna",
        "Nintendo 64" => "N64, n64",
        "Nintendo GameCube" => "GC, NGC, GameCube, gc, gamecube",
        "Nintendo Game Boy" => "GB, Game Boy, gb",
        "Nintendo Game Boy Color" => "GBC, Game Boy Color, gbc",
        "Nintendo Game Boy Advance" => "GBA, Game Boy Advance, gba",
        "Nintendo DS" => "NDS, DS, nds",
        "Nintendo 3DS" => "3DS, n3ds, 3ds",
        "Nintendo Wii" => "Wii, wii",
        "Nintendo Wii U" => "Wii U, WiiU, wiiu",
        "Nintendo Switch" => "Switch, NS, switch",
        "Nintendo Virtual Boy" => "VB, Virtual Boy, virtualboy",
        // Sega
        "Sega Master System" => "SMS, Master System, mastersystem",
        "Sega Genesis" => "MD, Mega Drive, Genesis, genesis, megadrive",
        "Sega CD" => "SCD, Mega CD, Sega CD, segacd, megacd",
        "Sega 32X" => "32X, sega32x",
        "Sega Saturn" => "SS, Saturn, saturn",
        "Sega Dreamcast" => "DC, Dreamcast, dreamcast",
        "Sega Game Gear" => "GG, Game Gear, gamegear",
        // Sony
        "Sony Playstation" => "PS1, PSX, PS, PlayStation, psx",
        "Sony Playstation 2" => "PS2, PlayStation 2, ps2",
        "Sony Playstation 3" => "PS3, PlayStation 3, ps3",
        "Sony PSP" => "PSP, PlayStation Portable, psp",
        "Sony Playstation Vita" => "PSV, Vita, PS Vita, psvita",
        // NEC
        "NEC TurboGrafx-16" => "PCE, PC Engine, TG16, TurboGrafx-16, tg16, pcengine",
        "NEC TurboGrafx-CD" => "PCECD, PC Engine CD, TG-CD, TurboGrafx-CD, tg-cd, pcenginecd",
        "NEC PC-98" => "PC98, PC-98, pc98",
        // SNK
        "SNK Neo Geo Pocket" => "NGP, Neo Geo Pocket, ngp",
        "SNK Neo Geo Pocket Color" => "NGPC, Neo Geo Pocket Color, ngpc",
        "SNK Neo Geo AES" => "AES, MVS, Neo Geo, neogeo",
        "SNK Neo Geo CD" => "Neo Geo CD, neogeocd, neogeocdjp",
        // Atari
        "Atari 2600" => "2600, VCS, atari2600",
        "Atari 5200" => "5200, atari5200",
        "Atari 7800" => "7800, atari7800",
        "Atari Lynx" => "Lynx, lynx",
        "Atari Jaguar" => "Jaguar, Jag, atarijaguar",
        "Atari Jaguar CD" => "Jaguar CD, atarijaguarcd",
        // Commodore
        "Commodore 64" => "C64, c64",
        "Commodore Amiga" => "Amiga, amiga",
        "Commodore VIC-20" => "VIC-20, VIC20, vic20",
        "Commodore 16" => "C16, c16",
        // Other
        "MS-DOS" => "DOS, dos",
        "Microsoft MSX" => "MSX, msx",
        "Microsoft MSX2" => "MSX2, msx2",
        "Microsoft Xbox" => "Xbox, xbox",
        "Microsoft Xbox 360" => "X360, 360, Xbox 360, xbox360",
        "Sinclair ZX Spectrum" => "ZX, ZX Spectrum, zxspectrum",
        "Amstrad CPC" => "CPC, amstradcpc",
        "Arcade" => "MAME, arcade, fbneo",
        "Panasonic 3DO" => "3DO, 3do",
        "Philips CD-i" => "CD-i, CDi, cdimono1",
        "Bandai WonderSwan" => "WS, WonderSwan, wonderswan",
        "Bandai WonderSwan Color" => "WSC, WonderSwan Color, wonderswancolor",
        "Coleco ColecoVision" => "Coleco, ColecoVision, colecovision",
        "Mattel Intellivision" => "Intellivision, intellivision",
        "GCE Vectrex" => "Vectrex, vectrex",
        "Sharp X68000" => "X68000, x68000",
        "ScummVM" => "ScummVM, scummvm",
        _ => return None,
    };
    Some(aliases.to_string())
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Lunchbox.", name)
}

/// Sanitize a platform name for use as a filename
fn platform_name_to_filename(name: &str) -> String {
    name.replace("/", "-")
        .replace(":", "-")
        .replace("&", "and")
        .replace(" ", "_")
}

#[tauri::command]
pub async fn get_platforms(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Platform>, String> {
    let state_guard = state.read().await;

    // Try shipped games database first (browse-first mode)
    if let Some(ref games_pool) = state_guard.games_db_pool {
        // Get all platforms with aliases
        let platforms: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT id, name, aliases FROM platforms ORDER BY name"
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        // For each platform, count deduplicated games (by normalized title)
        let mut result = Vec::new();
        for (id, name, aliases) in platforms {
            let all_titles: Vec<(String,)> = sqlx::query_as(
                "SELECT title FROM games WHERE platform_id = ?"
            )
            .bind(id)
            .fetch_all(games_pool)
            .await
            .map_err(|e| e.to_string())?;

            // Count unique normalized titles
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for (title,) in all_titles {
                let normalized = normalize_title_for_display(&title).to_lowercase();
                seen.insert(normalized);
            }
            // Use database aliases or generate them if not present
            let aliases = aliases.or_else(|| get_platform_search_aliases(&name));
            // Build icon URL from platform name (icons are named after canonical platform names)
            let filename = platform_name_to_filename(&name);
            let icon_url = Some(format!("/assets/platforms/{}.png", filename));
            result.push(Platform { id, name, game_count: seen.len() as i64, aliases, icon_url });
        }
        return Ok(result);
    }

    // No database available - show empty state
    Ok(Vec::new())
}

/// Get total count of games for a platform/search (deduplicated by normalized title)
#[tauri::command]
pub async fn get_game_count(
    platform: Option<String>,
    search: Option<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<i64, String> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        // Fetch all matching titles
        let titles: Vec<(String,)> = if let Some(ref query) = search {
            let pattern = format!("%{}%", query);
            if let Some(ref platform_name) = platform {
                sqlx::query_as(
                    "SELECT g.title FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ? AND g.title LIKE ?"
                )
                .bind(platform_name)
                .bind(&pattern)
                .fetch_all(games_pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query_as("SELECT title FROM games WHERE title LIKE ?")
                    .bind(&pattern)
                    .fetch_all(games_pool)
                    .await
                    .map_err(|e| e.to_string())?
            }
        } else if let Some(ref platform_name) = platform {
            sqlx::query_as(
                "SELECT g.title FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ?"
            )
            .bind(platform_name)
            .fetch_all(games_pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query_as("SELECT title FROM games")
                .fetch_all(games_pool)
                .await
                .map_err(|e| e.to_string())?
        };

        // Count unique normalized titles
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (title,) in titles {
            let normalized = normalize_title_for_display(&title).to_lowercase();
            seen.insert(normalized);
        }
        return Ok(seen.len() as i64);
    }

    Ok(0)
}

#[tauri::command]
pub async fn get_games(
    platform: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Game>, String> {
    use std::collections::HashMap;

    let state_guard = state.read().await;
    let limit = limit.map(|l| l as usize);
    let offset = offset.unwrap_or(0) as usize;

    // Try shipped games database first (browse-first mode)
    if let Some(ref games_pool) = state_guard.games_db_pool {
        // Fetch all games for the platform/search, then deduplicate and paginate
        // We need all games to properly deduplicate variants
        let raw_rows = if let Some(ref query) = search {
            let pattern = format!("%{}%", query);
            if let Some(ref platform_name) = platform {
                // Search within a specific platform
                sqlx::query(
                    r#"
                    SELECT g.id, g.title, g.platform_id, p.name as platform, g.launchbox_db_id,
                           g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                           g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                           g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
                    FROM games g
                    JOIN platforms p ON g.platform_id = p.id
                    WHERE p.name = ? AND g.title LIKE ?
                    ORDER BY g.title
                    "#
                )
                .bind(platform_name)
                .bind(&pattern)
                .fetch_all(games_pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                // Search across all platforms
                sqlx::query(
                    r#"
                    SELECT g.id, g.title, g.platform_id, p.name as platform, g.launchbox_db_id,
                           g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                           g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                           g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
                    FROM games g
                    JOIN platforms p ON g.platform_id = p.id
                    WHERE g.title LIKE ?
                    ORDER BY g.title
                    "#
                )
                .bind(&pattern)
                .fetch_all(games_pool)
                .await
                .map_err(|e| e.to_string())?
            }
        } else if let Some(ref platform_name) = platform {
            sqlx::query(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform, g.launchbox_db_id,
                       g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                       g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                       g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
                FROM games g
                JOIN platforms p ON g.platform_id = p.id
                WHERE p.name = ?
                ORDER BY g.title
                "#
            )
            .bind(platform_name)
            .fetch_all(games_pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            Vec::new()
        };

        // Deduplicate by normalized title - keep best metadata and track unique variant titles
        // variant_titles tracks unique full titles per normalized title (e.g., "Baseball (USA)", "Baseball (Japan)")
        let mut grouped: HashMap<String, (Game, std::collections::HashSet<String>)> = HashMap::new();

        for row in raw_rows {
            use sqlx::Row;
            let id: String = row.get("id");
            let title: String = row.get("title");
            let platform_id: i64 = row.get("platform_id");
            let platform: String = row.get("platform");
            let launchbox_db_id: i64 = row.get("launchbox_db_id");
            let description: Option<String> = row.get("description");
            let release_date: Option<String> = row.get("release_date");
            let release_year: Option<i32> = row.get("release_year");
            let developer: Option<String> = row.get("developer");
            let publisher: Option<String> = row.get("publisher");
            let genre: Option<String> = row.get("genre");
            let players: Option<String> = row.get("players");
            let rating: Option<f64> = row.get("rating");
            let rating_count: Option<i64> = row.get("rating_count");
            let esrb: Option<String> = row.get("esrb");
            let cooperative: Option<i32> = row.get("cooperative");
            let video_url: Option<String> = row.get("video_url");
            let wikipedia_url: Option<String> = row.get("wikipedia_url");
            let release_type: Option<String> = row.get("release_type");
            let notes: Option<String> = row.get("notes");
            let sort_title: Option<String> = row.get("sort_title");
            let series: Option<String> = row.get("series");
            let region: Option<String> = row.get("region");
            let play_mode: Option<String> = row.get("play_mode");
            let version: Option<String> = row.get("version");
            let status: Option<String> = row.get("status");
            let steam_app_id: Option<i64> = row.get("steam_app_id");
            let display_title = normalize_title_for_display(&title);
            let key = display_title.to_lowercase();

            grouped.entry(key)
                .and_modify(|(existing, variant_titles)| {
                    variant_titles.insert(title.clone());
                    // Prefer entries with more metadata
                    if existing.description.is_none() && description.is_some() {
                        existing.description = description.clone();
                    }
                    if existing.developer.is_none() && developer.is_some() {
                        existing.developer = developer.clone();
                    }
                    if existing.publisher.is_none() && publisher.is_some() {
                        existing.publisher = publisher.clone();
                    }
                    if existing.genres.is_none() && genre.is_some() {
                        existing.genres = genre.clone();
                    }
                    if existing.release_date.is_none() && release_date.is_some() {
                        existing.release_date = release_date.clone();
                    }
                    if existing.release_year.is_none() && release_year.is_some() {
                        existing.release_year = release_year;
                    }
                    if existing.players.is_none() && players.is_some() {
                        existing.players = players.clone();
                    }
                    if existing.rating.is_none() && rating.is_some() {
                        existing.rating = rating;
                    }
                    if existing.rating_count.is_none() && rating_count.is_some() {
                        existing.rating_count = rating_count;
                    }
                    if existing.esrb.is_none() && esrb.is_some() {
                        existing.esrb = esrb.clone();
                    }
                    if existing.cooperative.is_none() && cooperative.is_some() {
                        existing.cooperative = cooperative.map(|c| c != 0);
                    }
                    if existing.video_url.is_none() && video_url.is_some() {
                        existing.video_url = video_url.clone();
                    }
                    if existing.wikipedia_url.is_none() && wikipedia_url.is_some() {
                        existing.wikipedia_url = wikipedia_url.clone();
                    }
                    if existing.release_type.is_none() && release_type.is_some() {
                        existing.release_type = release_type.clone();
                    }
                    if existing.notes.is_none() && notes.is_some() {
                        existing.notes = notes.clone();
                    }
                    if existing.sort_title.is_none() && sort_title.is_some() {
                        existing.sort_title = sort_title.clone();
                    }
                    if existing.series.is_none() && series.is_some() {
                        existing.series = series.clone();
                    }
                    if existing.region.is_none() && region.is_some() {
                        existing.region = region.clone();
                    }
                    if existing.play_mode.is_none() && play_mode.is_some() {
                        existing.play_mode = play_mode.clone();
                    }
                    if existing.version.is_none() && version.is_some() {
                        existing.version = version.clone();
                    }
                    if existing.status.is_none() && status.is_some() {
                        existing.status = status.clone();
                    }
                    if existing.steam_app_id.is_none() && steam_app_id.is_some() {
                        existing.steam_app_id = steam_app_id;
                    }
                })
                .or_insert_with(|| {
                    let mut variant_titles = std::collections::HashSet::new();
                    variant_titles.insert(title.clone());
                    (Game {
                        id,
                        database_id: launchbox_db_id,
                        title: title.clone(),
                        display_title: display_title.clone(),
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
                    }, variant_titles)
                });
        }

        // Convert to vec, update variant counts (unique titles, not raw rows), sort, and paginate
        let mut games: Vec<Game> = grouped.into_iter()
            .map(|(_, (mut game, variant_titles))| {
                game.variant_count = variant_titles.len() as i32;
                game
            })
            .collect();

        games.sort_by(|a, b| a.display_title.to_lowercase().cmp(&b.display_title.to_lowercase()));

        // Apply pagination after deduplication (if limit specified)
        let games: Vec<Game> = if let Some(lim) = limit {
            games.into_iter().skip(offset).take(lim).collect()
        } else {
            games.into_iter().skip(offset).collect()
        };
        return Ok(games);
    }

    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_game_by_id(
    database_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<Game>, String> {
    let state_guard = state.read().await;

    // Look up by launchbox_db_id in the games database
    if let Some(ref games_pool) = state_guard.games_db_pool {
        use sqlx::Row;
        let row_opt = sqlx::query(
            r#"
            SELECT g.id, g.title, g.platform_id, p.name as platform,
                   g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                   g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                   g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
            FROM games g
            JOIN platforms p ON g.platform_id = p.id
            WHERE g.launchbox_db_id = ?
            LIMIT 1
            "#
        )
        .bind(database_id)
        .fetch_optional(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(row) = row_opt {
            let title: String = row.get("title");
            let display_title = normalize_title_for_display(&title);
            return Ok(Some(Game {
                id: row.get::<i64, _>("id").to_string(),
                database_id,
                title,
                display_title,
                platform: row.get("platform"),
                platform_id: row.get("platform_id"),
                description: row.get("description"),
                release_date: row.get("release_date"),
                release_year: row.get("release_year"),
                developer: row.get("developer"),
                publisher: row.get("publisher"),
                genres: row.get("genre"),
                players: row.get("players"),
                rating: row.get("rating"),
                rating_count: row.get("rating_count"),
                esrb: row.get("esrb"),
                cooperative: row.get("cooperative"),
                video_url: row.get("video_url"),
                wikipedia_url: row.get("wikipedia_url"),
                release_type: row.get("release_type"),
                notes: row.get("notes"),
                sort_title: row.get("sort_title"),
                series: row.get("series"),
                region: row.get("region"),
                play_mode: row.get("play_mode"),
                version: row.get("version"),
                status: row.get("status"),
                steam_app_id: row.get("steam_app_id"),
                box_front_path: None,
                screenshot_path: None,
                variant_count: 1,
            }));
        }
    }

    Ok(None)
}

/// Get a game by its UUID (for the shipped games database)
#[tauri::command]
pub async fn get_game_by_uuid(
    game_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<Game>, String> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let row_opt = sqlx::query(
            r#"
            SELECT g.id, g.title, g.platform_id, p.name as platform, g.launchbox_db_id,
                   g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                   g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                   g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
            FROM games g
            JOIN platforms p ON g.platform_id = p.id
            WHERE g.id = ?
            "#
        )
        .bind(&game_id)
        .fetch_optional(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(row) = row_opt {
            use sqlx::Row;
            let id: String = row.get("id");
            let title: String = row.get("title");
            let platform_id: i64 = row.get("platform_id");
            let platform: String = row.get("platform");
            let launchbox_db_id: i64 = row.get("launchbox_db_id");
            let description: Option<String> = row.get("description");
            let release_date: Option<String> = row.get("release_date");
            let release_year: Option<i32> = row.get("release_year");
            let developer: Option<String> = row.get("developer");
            let publisher: Option<String> = row.get("publisher");
            let genre: Option<String> = row.get("genre");
            let players: Option<String> = row.get("players");
            let rating: Option<f64> = row.get("rating");
            let rating_count: Option<i64> = row.get("rating_count");
            let esrb: Option<String> = row.get("esrb");
            let cooperative: Option<i32> = row.get("cooperative");
            let video_url: Option<String> = row.get("video_url");
            let wikipedia_url: Option<String> = row.get("wikipedia_url");
            let release_type: Option<String> = row.get("release_type");
            let notes: Option<String> = row.get("notes");
            let sort_title: Option<String> = row.get("sort_title");
            let series: Option<String> = row.get("series");
            let region: Option<String> = row.get("region");
            let play_mode: Option<String> = row.get("play_mode");
            let version: Option<String> = row.get("version");
            let status: Option<String> = row.get("status");
            let steam_app_id: Option<i64> = row.get("steam_app_id");

            let display_title = normalize_title_for_display(&title);

            // Count matching variants
            let normalized_lower = display_title.to_lowercase();
            let all_titles: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT title FROM games WHERE platform_id = ?"
            )
            .bind(platform_id)
            .fetch_all(games_pool)
            .await
            .unwrap_or_default();

            let actual_variant_count = all_titles
                .iter()
                .filter(|(t,)| normalize_title_for_display(t).to_lowercase() == normalized_lower)
                .count() as i32;

            return Ok(Some(Game {
                id,
                database_id: launchbox_db_id,
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
                variant_count: actual_variant_count,
            }));
        }
    }

    Ok(None)
}

/// Default region priority order
const DEFAULT_REGION_PRIORITY: &[&str] = &[
    "",       // No region (unspecified/plain version)
    "USA",
    "World",
    "Japan",
    "Europe",
    "Australia",
    "Canada",
    "Brazil",
    "Korea",
    "Asia",
    "China",
    "France",
    "Germany",
    "Italy",
    "Spain",
    "United Kingdom",
];

/// Calculate region priority for sorting (lower = better)
/// Uses custom region order if provided, falls back to default
/// Public wrapper for use from api.rs
pub fn region_priority_for_title(title: &str, custom_order: &[String]) -> i32 {
    region_priority(title, custom_order)
}

/// Calculate region priority for sorting (lower = better)
/// Uses custom region order if provided, falls back to default
fn region_priority(title: &str, custom_order: &[String]) -> i32 {
    let title_lower = title.to_lowercase();

    // No region info (plain version) - check first
    if !title_lower.contains("(") {
        // If custom order includes "" (empty/plain), use its position
        if let Some(pos) = custom_order.iter().position(|r| r.is_empty()) {
            return pos as i32;
        }
        return 0; // Default: plain versions first
    }

    // Check each region in custom order (or default)
    let regions: Vec<&str> = if custom_order.is_empty() {
        DEFAULT_REGION_PRIORITY.to_vec()
    } else {
        custom_order.iter().map(|s| s.as_str()).collect()
    };

    for (priority, region) in regions.iter().enumerate() {
        if region.is_empty() {
            continue; // Already handled plain versions above
        }
        let region_lower = region.to_lowercase();
        // Match patterns like (USA), (USA, Europe), (Europe, USA)
        let pattern1 = format!("({})", region_lower);
        let pattern2 = format!("({},", region_lower);
        let pattern3 = format!(", {})", region_lower);

        if title_lower.contains(&pattern1)
            || title_lower.contains(&pattern2)
            || title_lower.contains(&pattern3)
        {
            return priority as i32;
        }
    }

    // Everything else
    regions.len() as i32
}

/// Get all unique regions from the games database
#[tauri::command]
pub async fn get_all_regions(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        // Get unique regions from the region column
        let explicit_regions: Vec<(Option<String>,)> = sqlx::query_as(
            "SELECT DISTINCT region FROM games WHERE region IS NOT NULL AND region != ''"
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut regions: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Add explicit regions from region column
        for (region,) in explicit_regions {
            if let Some(r) = region {
                regions.insert(r);
            }
        }

        // Also extract regions from title parentheses (e.g., "Game (USA)")
        let titles: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT title FROM games WHERE title LIKE '%(%'"
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        for (title,) in titles {
            if let Some(extracted) = extract_region_from_title(&title) {
                regions.insert(extracted);
            }
        }

        // Sort by default priority order first, then alphabetically for unknown regions
        let mut result: Vec<String> = regions.into_iter().collect();
        result.sort_by(|a, b| {
            let pos_a = DEFAULT_REGION_PRIORITY.iter().position(|&r| r == a.as_str());
            let pos_b = DEFAULT_REGION_PRIORITY.iter().position(|&r| r == b.as_str());
            match (pos_a, pos_b) {
                (Some(pa), Some(pb)) => pa.cmp(&pb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(b),
            }
        });

        // Add empty string for "plain/no region" at the start if not present
        if !result.iter().any(|r| r.is_empty()) {
            result.insert(0, String::new());
        }

        return Ok(result);
    }

    Ok(Vec::new())
}

/// Get all variants (regions/versions) for a given game by display title
#[tauri::command]
pub async fn get_game_variants(
    display_title: String,
    platform_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<GameVariant>, String> {
    let state_guard = state.read().await;

    // Get custom region priority from settings
    let custom_region_order = state_guard.settings.region_priority.clone();

    if let Some(ref games_pool) = state_guard.games_db_pool {
        // Search for all games that normalize to this display title
        let all_games: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT id, title
            FROM games
            WHERE platform_id = ?
            ORDER BY title
            "#
        )
        .bind(platform_id)
        .fetch_all(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        let normalized_search = display_title.to_lowercase();

        // Filter to matching titles
        let matching: Vec<(String, String)> = all_games
            .into_iter()
            .filter(|(_, title)| {
                let normalized = normalize_title_for_display(title).to_lowercase();
                normalized == normalized_search
            })
            .collect();

        // Deduplicate by title (different ROM dumps have same title but different IDs)
        // Keep the first entry for each unique title
        let mut seen_titles: HashMap<String, GameVariant> = HashMap::new();
        for (id, title) in matching {
            if !seen_titles.contains_key(&title) {
                let region = extract_region_from_title(&title);
                seen_titles.insert(title.clone(), GameVariant { id, title, region });
            }
        }

        // Convert to vec and sort by region priority (uses user's preference if set)
        let mut variants: Vec<GameVariant> = seen_titles.into_values().collect();
        variants.sort_by(|a, b| {
            let priority_a = region_priority(&a.title, &custom_region_order);
            let priority_b = region_priority(&b.title, &custom_region_order);
            priority_a.cmp(&priority_b).then_with(|| a.title.cmp(&b.title))
        });

        return Ok(variants);
    }

    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<AppSettings, String> {
    let state_guard = state.read().await;
    Ok(state_guard.settings.clone())
}

#[tauri::command]
pub fn get_credential_storage_name() -> String {
    crate::keyring_store::get_credential_storage_name().to_string()
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let mut state_guard = state.write().await;

    if let Some(ref pool) = state_guard.db_pool {
        crate::state::save_settings(pool, &settings)
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;
    }

    state_guard.settings = settings;
    Ok(())
}

/// Result of scraping a ROM
#[derive(Debug, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub success: bool,
    pub game: Option<Game>,
    pub error: Option<String>,
}

// Collection type is imported from handlers module

#[tauri::command]
pub async fn scrape_rom(
    rom_path: String,
    platform: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<ScrapeResult, String> {
    let state_guard = state.read().await;

    // Check if ScreenScraper is configured
    let ss_settings = &state_guard.settings.screenscraper;
    if ss_settings.dev_id.is_empty() || ss_settings.dev_password.is_empty() {
        return Ok(ScrapeResult {
            success: false,
            game: None,
            error: Some("ScreenScraper credentials not configured".to_string()),
        });
    }

    // Get platform ID for ScreenScraper
    let platform_id = get_screenscraper_platform_id(&platform);

    // Calculate checksums for the ROM
    let path = PathBuf::from(&rom_path);
    let checksums = crate::scanner::Checksums::calculate(&path)
        .map_err(|e| format!("Failed to calculate checksums: {}", e))?;

    let file_name = path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Create ScreenScraper client
    let config = ScreenScraperConfig {
        dev_id: ss_settings.dev_id.clone(),
        dev_password: ss_settings.dev_password.clone(),
        user_id: ss_settings.user_id.clone(),
        user_password: ss_settings.user_password.clone(),
    };
    let client = ScreenScraperClient::new(config);

    // Look up the game
    match client.lookup_by_checksum(
        &checksums.crc32,
        &checksums.md5,
        &checksums.sha1,
        checksums.size,
        file_name,
        platform_id,
    ).await {
        Ok(Some(scraped)) => {
            let display_title = normalize_title_for_display(&scraped.name);
            let game = Game {
                id: uuid::Uuid::new_v4().to_string(),
                database_id: scraped.screenscraper_id,
                title: scraped.name.clone(),
                display_title,
                platform: platform.clone(),
                platform_id: 0,
                description: scraped.description,
                release_date: scraped.release_date,
                release_year: None,
                developer: scraped.developer,
                publisher: scraped.publisher,
                genres: Some(scraped.genres.join(", ")),
                players: None,
                rating: scraped.rating,
                rating_count: None,
                esrb: None,
                cooperative: None,
                video_url: None,
                wikipedia_url: None,
                release_type: None,
                notes: None,
                sort_title: None,
                series: None,
                region: None,
                play_mode: None,
                version: None,
                status: None,
                steam_app_id: None,
                box_front_path: scraped.media.box_front,
                screenshot_path: scraped.media.screenshot,
                variant_count: 1,
            };
            Ok(ScrapeResult {
                success: true,
                game: Some(game),
                error: None,
            })
        }
        Ok(None) => Ok(ScrapeResult {
            success: false,
            game: None,
            error: Some("Game not found in ScreenScraper".to_string()),
        }),
        Err(e) => Ok(ScrapeResult {
            success: false,
            game: None,
            error: Some(e.to_string()),
        }),
    }
}

// ============ Collection Commands ============
// These call shared handlers from handlers.rs to ensure logic is defined once.

#[tauri::command]
pub async fn get_collections(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Collection>, String> {
    let state_guard = state.read().await;
    handlers::get_collections(&state_guard).await
}

#[tauri::command]
pub async fn create_collection(
    name: String,
    description: Option<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Collection, String> {
    let state_guard = state.read().await;
    handlers::create_collection(&state_guard, CreateCollectionInput { name, description }).await
}

#[tauri::command]
pub async fn update_collection(
    id: String,
    name: String,
    description: Option<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::update_collection(&state_guard, UpdateCollectionInput { id, name, description }).await
}

#[tauri::command]
pub async fn delete_collection(
    id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::delete_collection(&state_guard, CollectionIdInput { collection_id: id }).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_collection_games(
    collection_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Game>, String> {
    let state_guard = state.read().await;
    handlers::get_collection_games(&state_guard, CollectionIdInput { collection_id }).await
}

#[tauri::command]
pub async fn add_game_to_collection(
    collection_id: String,
    game_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::add_game_to_collection(&state_guard, CollectionGameInput { collection_id, game_id }).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_game_from_collection(
    collection_id: String,
    game_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::remove_game_from_collection(&state_guard, CollectionGameInput { collection_id, game_id }).await?;
    Ok(())
}

// ============ Play Statistics Commands ============

/// Play statistics for a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayStats {
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub play_count: i64,
    pub total_play_time_seconds: i64,
    pub last_played: Option<String>,
    pub first_played: Option<String>,
}

#[tauri::command]
pub async fn record_play_session(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Insert or update play stats
    sqlx::query(
        r#"
        INSERT INTO play_stats (launchbox_db_id, game_title, platform, play_count, last_played, first_played)
        VALUES (?, ?, ?, 1, ?, ?)
        ON CONFLICT(launchbox_db_id) DO UPDATE SET
            play_count = play_count + 1,
            last_played = excluded.last_played
        "#
    )
    .bind(launchbox_db_id)
    .bind(&game_title)
    .bind(&platform)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Recorded play session for: {} ({})", game_title, launchbox_db_id);
    Ok(())
}

#[tauri::command]
pub async fn get_play_stats(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<PlayStats>, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let stats: Option<(i64, String, String, i64, i64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform, play_count, total_play_time_seconds, last_played, first_played FROM play_stats WHERE launchbox_db_id = ?"
    )
    .bind(launchbox_db_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(stats.map(|(db_id, title, platform, count, time, last, first)| PlayStats {
        launchbox_db_id: db_id,
        game_title: title,
        platform,
        play_count: count,
        total_play_time_seconds: time,
        last_played: last,
        first_played: first,
    }))
}

#[tauri::command]
pub async fn get_recent_games(
    limit: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<PlayStats>, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let limit = limit.unwrap_or(10);

    let stats: Vec<(i64, String, String, i64, i64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform, play_count, total_play_time_seconds, last_played, first_played FROM play_stats WHERE last_played IS NOT NULL ORDER BY last_played DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(stats.into_iter().map(|(db_id, title, platform, count, time, last, first)| PlayStats {
        launchbox_db_id: db_id,
        game_title: title,
        platform,
        play_count: count,
        total_play_time_seconds: time,
        last_played: last,
        first_played: first,
    }).collect())
}

#[tauri::command]
pub async fn get_most_played(
    limit: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<PlayStats>, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let limit = limit.unwrap_or(10);

    let stats: Vec<(i64, String, String, i64, i64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform, play_count, total_play_time_seconds, last_played, first_played FROM play_stats ORDER BY play_count DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(stats.into_iter().map(|(db_id, title, platform, count, time, last, first)| PlayStats {
        launchbox_db_id: db_id,
        game_title: title,
        platform,
        play_count: count,
        total_play_time_seconds: time,
        last_played: last,
        first_played: first,
    }).collect())
}

// ============ Service Connection Tests ============

// Re-export ConnectionTestResult from router for Tauri commands
pub use crate::router::ConnectionTestResult;

#[tauri::command]
pub async fn test_screenscraper_connection(
    dev_id: String,
    dev_password: String,
    user_id: Option<String>,
    user_password: Option<String>,
) -> Result<ConnectionTestResult, String> {
    Ok(crate::router::test_screenscraper_impl(dev_id, dev_password, user_id, user_password).await)
}

#[tauri::command]
pub async fn test_steamgriddb_connection(api_key: String) -> Result<ConnectionTestResult, String> {
    Ok(crate::router::test_steamgriddb_impl(api_key).await)
}

#[tauri::command]
pub async fn test_igdb_connection(
    client_id: String,
    client_secret: String,
) -> Result<ConnectionTestResult, String> {
    Ok(crate::router::test_igdb_impl(client_id, client_secret).await)
}

#[tauri::command]
pub async fn test_emumovies_connection(
    username: String,
    password: String,
) -> Result<ConnectionTestResult, String> {
    Ok(crate::router::test_emumovies_impl(username, password).await)
}

// ============ Favorites Commands ============

#[tauri::command]
pub async fn add_favorite(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query(
        "INSERT OR IGNORE INTO favorites (launchbox_db_id, game_title, platform) VALUES (?, ?, ?)"
    )
    .bind(launchbox_db_id)
    .bind(&game_title)
    .bind(&platform)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_favorite(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM favorites WHERE launchbox_db_id = ?")
        .bind(launchbox_db_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn is_favorite(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<bool, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT launchbox_db_id FROM favorites WHERE launchbox_db_id = ?"
    )
    .bind(launchbox_db_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(exists.is_some())
}

#[tauri::command]
pub async fn get_favorites(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Game>, String> {
    use sqlx::Row;

    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let favorites: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform FROM favorites ORDER BY added_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Look up full game info from games database
    let games_pool = state_guard.games_db_pool.as_ref();
    let mut games = Vec::new();

    for (db_id, title, platform) in favorites {
        // Try to get full game info from games_db
        if let Some(gp) = games_pool {
            let row_opt = sqlx::query(r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform,
                       g.launchbox_db_id,
                       g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                       g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                       g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status,
                       g.steam_app_id
                FROM games g
                JOIN platforms p ON g.platform_id = p.id
                WHERE g.launchbox_db_id = ?
                LIMIT 1
            "#)
            .bind(db_id)
            .fetch_optional(gp)
            .await
            .map_err(|e| e.to_string())?;

            if let Some(row) = row_opt {
                let title_str: String = row.get("title");
                let display_title = normalize_title_for_display(&title_str);
                let region = extract_region_from_title(&title_str);

                games.push(Game {
                    id: row.get::<i64, _>("id").to_string(),
                    database_id: db_id,  // We already have this from favorites table
                    title: title_str,
                    display_title,
                    platform: row.get("platform"),
                    platform_id: row.get("platform_id"),
                    description: row.get("description"),
                    release_date: row.get("release_date"),
                    release_year: row.get("release_year"),
                    developer: row.get("developer"),
                    publisher: row.get("publisher"),
                    genres: row.get("genre"),
                    players: row.get("players"),
                    rating: row.get("rating"),
                    rating_count: row.get("rating_count"),
                    esrb: row.get("esrb"),
                    cooperative: row.get("cooperative"),
                    video_url: row.get("video_url"),
                    wikipedia_url: row.get("wikipedia_url"),
                    release_type: row.get("release_type"),
                    notes: row.get("notes"),
                    sort_title: row.get("sort_title"),
                    series: row.get("series"),
                    region: region.or_else(|| row.get("region")),
                    play_mode: row.get("play_mode"),
                    version: row.get("version"),
                    status: row.get("status"),
                    steam_app_id: row.get("steam_app_id"),
                    box_front_path: None,
                    screenshot_path: None,
                    variant_count: 1,
                });
                continue;
            }
        }

        // Fallback: return minimal info from favorites table
        let display_title = normalize_title_for_display(&title);
        games.push(Game {
            id: db_id.to_string(),
            database_id: db_id,
            title: title.clone(),
            display_title,
            platform,
            platform_id: 0,
            description: None,
            release_date: None,
            release_year: None,
            developer: None,
            publisher: None,
            genres: None,
            players: None,
            rating: None,
            rating_count: None,
            esrb: None,
            cooperative: None,
            video_url: None,
            wikipedia_url: None,
            release_type: None,
            notes: None,
            sort_title: None,
            series: None,
            region: None,
            play_mode: None,
            version: None,
            status: None,
            steam_app_id: None,
            box_front_path: None,
            screenshot_path: None,
            variant_count: 1,
        });
    }

    Ok(games)
}

// ============ Image Commands ============

/// Get the cache directory for images (uses media_directory)
pub fn get_cache_dir(settings: &AppSettings) -> PathBuf {
    settings.get_media_directory()
}

/// Get all images for a game
#[tauri::command]
pub async fn get_game_images(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<ImageInfo>, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir);
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    service.get_game_images(launchbox_db_id).await
        .map_err(|e| e.to_string())
}

/// Get a specific image type for a game
#[tauri::command]
pub async fn get_game_image(
    launchbox_db_id: i64,
    image_type: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<ImageInfo>, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir);
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    service.get_image_by_type(launchbox_db_id, &image_type).await
        .map_err(|e| e.to_string())
}

/// Get available image types for a game
#[tauri::command]
pub async fn get_available_image_types(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir);
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    service.get_available_types(launchbox_db_id).await
        .map_err(|e| e.to_string())
}

/// Download a specific image and return its local path
#[tauri::command]
pub async fn download_image(
    image_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<String, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir);
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    service.download_image(image_id).await
        .map_err(|e| e.to_string())
}

/// Download images for a game (box front and screenshot by default)
#[tauri::command]
pub async fn download_game_images(
    launchbox_db_id: i64,
    image_types: Option<Vec<String>>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<String>, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir);
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    service.download_game_images(launchbox_db_id, image_types).await
        .map_err(|e| e.to_string())
}

/// Get image cache statistics
#[tauri::command]
pub async fn get_image_cache_stats(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<CacheStats, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir);
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    service.get_cache_stats().await
        .map_err(|e| e.to_string())
}

/// Result from cache check
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedMediaResult {
    pub path: String,
    pub source: String,
}

/// Check if media is cached locally (fast path - no network requests)
#[tauri::command]
pub async fn check_cached_media(
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<CachedMediaResult>, String> {
    let state_guard = state.read().await;
    let cache_dir = get_cache_dir(&state_guard.settings);

    // Compute game_id
    let game_id = crate::images::get_game_cache_id(
        launchbox_db_id,
        &game_title,
        &platform,
    );

    // Check cache
    if let Some((path, source)) = crate::images::find_cached_media(&cache_dir, &game_id, &image_type) {
        return Ok(Some(CachedMediaResult {
            path: path.to_string_lossy().to_string(),
            source: source.abbreviation().to_string(),
        }));
    }

    Ok(None)
}

/// Download an image with fallback to multiple sources
///
/// Tries sources in order:
/// 1. LaunchBox CDN
/// 2. libretro-thumbnails
/// 3. SteamGridDB
/// 4. IGDB
/// 5. EmuMovies
/// 6. ScreenScraper
#[tauri::command]
pub async fn download_image_with_fallback(
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<String, String> {
    let state_guard = state.read().await;

    let games_pool = state_guard.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    // Look up platform info to get launchbox_name and libretro_name
    let platform_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_name, libretro_name FROM platforms WHERE name = ?"
    )
    .bind(&platform)
    .fetch_optional(games_pool)
    .await
    .ok()
    .flatten();

    let (launchbox_platform, libretro_platform) = platform_info
        .map(|(lb, lr)| (lb, lr))
        .unwrap_or((None, None));

    // Look up libretro_title if we have a launchbox_db_id
    let libretro_title: Option<String> = if let Some(db_id) = launchbox_db_id {
        sqlx::query_scalar("SELECT libretro_title FROM games WHERE launchbox_db_id = ?")
            .bind(db_id)
            .fetch_optional(games_pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let cache_dir = get_cache_dir(&state_guard.settings);
    let mut service = ImageService::new(games_pool.clone(), cache_dir.clone());
    if let Some(ref images_pool) = state_guard.images_db_pool {
        service = service.with_images_pool(images_pool.clone());
    }

    // Create SteamGridDB client if configured
    let steamgriddb_client = if !state_guard.settings.steamgriddb.api_key.is_empty() {
        Some(crate::scraper::SteamGridDBClient::new(
            crate::scraper::SteamGridDBConfig {
                api_key: state_guard.settings.steamgriddb.api_key.clone(),
            }
        ))
    } else {
        None
    };

    // Create IGDB client if configured
    let igdb_client = if !state_guard.settings.igdb.client_id.is_empty()
        && !state_guard.settings.igdb.client_secret.is_empty()
    {
        Some(crate::scraper::IGDBClient::new(
            crate::scraper::IGDBConfig {
                client_id: state_guard.settings.igdb.client_id.clone(),
                client_secret: state_guard.settings.igdb.client_secret.clone(),
            }
        ))
    } else {
        None
    };

    // Create EmuMovies client if configured
    let emumovies_client = if !state_guard.settings.emumovies.username.is_empty()
        && !state_guard.settings.emumovies.password.is_empty()
    {
        Some(crate::images::EmuMoviesClient::new(
            crate::images::EmuMoviesConfig {
                username: state_guard.settings.emumovies.username.clone(),
                password: state_guard.settings.emumovies.password.clone(),
            },
            cache_dir.clone(),
        ))
    } else {
        None
    };

    // Create ScreenScraper client if configured
    let screenscraper_client = if !state_guard.settings.screenscraper.dev_id.is_empty()
        && !state_guard.settings.screenscraper.dev_password.is_empty()
    {
        Some(crate::scraper::ScreenScraperClient::new(
            crate::scraper::ScreenScraperConfig {
                dev_id: state_guard.settings.screenscraper.dev_id.clone(),
                dev_password: state_guard.settings.screenscraper.dev_password.clone(),
                user_id: state_guard.settings.screenscraper.user_id.clone(),
                user_password: state_guard.settings.screenscraper.user_password.clone(),
            }
        ))
    } else {
        None
    };

    service.download_with_fallback(
        &game_title,
        &platform,
        &image_type,
        launchbox_db_id,
        launchbox_platform.as_deref(),
        libretro_platform.as_deref(),
        libretro_title.as_deref(),
        steamgriddb_client.as_ref(),
        igdb_client.as_ref(),
        emumovies_client.as_ref(),
        screenscraper_client.as_ref(),
    ).await
        .map_err(|e| e.to_string())
}

/// Download a thumbnail from libretro-thumbnails
#[tauri::command]
pub async fn download_libretro_thumbnail(
    game_title: String,
    platform: String,
    image_type: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<String>, String> {
    let state_guard = state.read().await;

    let cache_dir = get_cache_dir(&state_guard.settings);

    let libretro_type = match image_type.as_str() {
        "Box - Front" | "Box - Back" => crate::images::LibRetroImageType::Boxart,
        "Screenshot - Gameplay" | "Screenshot" => crate::images::LibRetroImageType::Snap,
        "Screenshot - Game Title" => crate::images::LibRetroImageType::Title,
        _ => return Ok(None),
    };

    let client = crate::images::LibRetroThumbnailsClient::new(cache_dir);
    let result = client.find_thumbnail(&platform, libretro_type, &game_title).await;

    Ok(result)
}

// ============ Media Download Service Commands ============

/// Get all available normalized media types
#[tauri::command]
pub fn get_media_types() -> Vec<serde_json::Value> {
    crate::images::NormalizedMediaType::all()
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.filename(),
                "name": t.display_name(),
            })
        })
        .collect()
}

/// Download media for a game using the new unified system
///
/// Uses round-robin source selection based on game ID
#[tauri::command]
pub async fn download_unified_media(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    media_type: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<String, String> {
    use crate::images::{MediaEventSender, NormalizedMediaType, MediaSource, GameMediaId, RoundRobinSourceSelector};

    let normalized_type = NormalizedMediaType::from_str(&media_type)
        .ok_or_else(|| format!("Unknown media type: {}", media_type))?;

    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let cache_dir = get_cache_dir(&state_guard.settings);

    // Create event sender
    let event_sender = MediaEventSender::new(app_handle);

    // Get source using round-robin selection
    let selector = RoundRobinSourceSelector::new();
    let source = selector.source_for_game_and_type(launchbox_db_id, normalized_type);

    // Emit started event
    event_sender.started(launchbox_db_id, normalized_type, source);

    // Build local path using new structure
    let game_id = GameMediaId::from_launchbox_id(launchbox_db_id);
    let local_path = game_id.media_path(&cache_dir, normalized_type, "png");

    // Check if already exists
    if local_path.exists() {
        let path_str = local_path.to_string_lossy().to_string();
        event_sender.completed(launchbox_db_id, normalized_type, path_str.clone(), source);
        return Ok(path_str);
    }

    // Create parent directories
    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }

    // Try to download based on source
    let result = match source {
        MediaSource::LaunchBox => {
            download_from_launchbox_unified(
                pool,
                &cache_dir,
                launchbox_db_id,
                normalized_type,
                &local_path,
            ).await
        }
        MediaSource::LibRetro => {
            download_from_libretro_unified(
                &cache_dir,
                &game_title,
                &platform,
                normalized_type,
            ).await
        }
        _ => {
            // Fall back to LaunchBox for unimplemented sources
            download_from_launchbox_unified(
                pool,
                &cache_dir,
                launchbox_db_id,
                normalized_type,
                &local_path,
            ).await
        }
    };

    match result {
        Ok(path) => {
            // Record in game_media table
            let _ = sqlx::query(
                r#"
                INSERT INTO game_media (launchbox_db_id, media_type, source, local_path, status, downloaded_at)
                VALUES (?, ?, ?, ?, 'completed', CURRENT_TIMESTAMP)
                ON CONFLICT(launchbox_db_id, media_type, source) DO UPDATE SET
                    local_path = excluded.local_path,
                    status = 'completed',
                    downloaded_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(launchbox_db_id)
            .bind(normalized_type.filename())
            .bind(source.as_str())
            .bind(&path)
            .execute(pool)
            .await;

            event_sender.completed(launchbox_db_id, normalized_type, path.clone(), source);
            Ok(path)
        }
        Err(e) => {
            event_sender.failed(launchbox_db_id, normalized_type, e.to_string(), source);
            Err(e.to_string())
        }
    }
}

async fn download_from_launchbox_unified(
    pool: &sqlx::sqlite::SqlitePool,
    _cache_dir: &std::path::Path,
    launchbox_db_id: i64,
    media_type: crate::images::NormalizedMediaType,
    local_path: &std::path::Path,
) -> Result<String, anyhow::Error> {
    let launchbox_type = media_type.to_launchbox_type();

    // Look up filename from game_images table
    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT filename FROM game_images
        WHERE launchbox_db_id = ? AND image_type = ?
        ORDER BY
            CASE region
                WHEN 'North America' THEN 0
                WHEN 'United States' THEN 1
                WHEN 'World' THEN 2
                WHEN 'Europe' THEN 3
                ELSE 10
            END,
            filename
        LIMIT 1
        "#,
    )
    .bind(launchbox_db_id)
    .bind(launchbox_type)
    .fetch_optional(pool)
    .await?;

    let filename = row
        .map(|(f,)| f)
        .ok_or_else(|| anyhow::anyhow!("No LaunchBox image found for type {}", launchbox_type))?;

    // Build CDN URL
    let url = format!("{}/{}", crate::images::LAUNCHBOX_CDN_URL, urlencoding::encode(&filename));

    // Determine actual local path with correct extension
    let extension = filename.rsplit('.').next().unwrap_or("png");
    let actual_path = local_path.with_extension(extension);

    // Download
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("HTTP {}: {}", response.status(), url);
    }

    let bytes = response.bytes().await?;
    tokio::fs::write(&actual_path, &bytes).await?;

    Ok(actual_path.to_string_lossy().to_string())
}

async fn download_from_libretro_unified(
    cache_dir: &std::path::Path,
    game_title: &str,
    platform: &str,
    media_type: crate::images::NormalizedMediaType,
) -> Result<String, anyhow::Error> {
    let libretro_type = match media_type {
        crate::images::NormalizedMediaType::BoxFront |
        crate::images::NormalizedMediaType::BoxBack => crate::images::LibRetroImageType::Boxart,
        crate::images::NormalizedMediaType::Screenshot => crate::images::LibRetroImageType::Snap,
        crate::images::NormalizedMediaType::TitleScreen => crate::images::LibRetroImageType::Title,
        _ => anyhow::bail!("Media type not supported by libretro"),
    };

    let client = crate::images::LibRetroThumbnailsClient::new(cache_dir.to_path_buf());
    client
        .find_thumbnail(platform, libretro_type, game_title)
        .await
        .ok_or_else(|| anyhow::anyhow!("Not found in libretro-thumbnails"))
}

/// Get the cached path for a media file (if it exists)
#[tauri::command]
pub async fn get_cached_media_path(
    launchbox_db_id: i64,
    media_type: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<String>, String> {
    let normalized_type = crate::images::NormalizedMediaType::from_str(&media_type)
        .ok_or_else(|| format!("Unknown media type: {}", media_type))?;

    let state_guard = state.read().await;

    let pool = match state_guard.db_pool.as_ref() {
        Some(p) => p,
        None => return Ok(None),
    };

    // Check game_media table
    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT local_path FROM game_media
        WHERE launchbox_db_id = ? AND media_type = ? AND status = 'completed' AND local_path IS NOT NULL
        LIMIT 1
        "#,
    )
    .bind(launchbox_db_id)
    .bind(normalized_type.filename())
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|(path,)| path))
}

// ============ Video Download Commands ============

/// Check if a video is cached for a game
#[tauri::command]
pub async fn check_cached_video(
    game_title: String,
    platform: String,
    launchbox_db_id: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<String>, String> {
    let state_guard = state.read().await;
    let cache_dir = get_cache_dir(&state_guard.settings);

    // Build the expected video path
    let game_id = match launchbox_db_id {
        Some(id) => crate::images::GameMediaId::from_launchbox_id(id),
        None => {
            // Fall back to computing hash from platform and title
            let games_pool = state_guard.games_db_pool.as_ref()
                .ok_or_else(|| "Games database not initialized".to_string())?;

            // Get platform_id
            let platform_id: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM platforms WHERE name = ?"
            )
            .bind(&platform)
            .fetch_optional(games_pool)
            .await
            .map_err(|e| e.to_string())?;

            let platform_id = platform_id.map(|(id,)| id).unwrap_or(0);
            crate::images::GameMediaId::compute_hash(platform_id, &game_title)
        }
    };

    let video_path = cache_dir
        .join("media")
        .join(game_id.directory_name())
        .join("emumovies")
        .join("video.mp4");

    if video_path.exists() {
        Ok(Some(video_path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

/// Download a video for a game from EmuMovies
#[tauri::command]
pub async fn download_game_video(
    game_title: String,
    platform: String,
    launchbox_db_id: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<String, String> {
    let state_guard = state.read().await;

    // Check if EmuMovies is configured
    if state_guard.settings.emumovies.username.is_empty()
        || state_guard.settings.emumovies.password.is_empty()
    {
        return Err("EmuMovies credentials not configured. Configure them in Settings.".to_string());
    }

    let cache_dir = get_cache_dir(&state_guard.settings);

    // Build the game cache directory
    let game_id = match launchbox_db_id {
        Some(id) => crate::images::GameMediaId::from_launchbox_id(id),
        None => {
            // Fall back to computing hash from platform and title
            let games_pool = state_guard.games_db_pool.as_ref()
                .ok_or_else(|| "Games database not initialized".to_string())?;

            // Get platform_id
            let platform_id: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM platforms WHERE name = ?"
            )
            .bind(&platform)
            .fetch_optional(games_pool)
            .await
            .map_err(|e| e.to_string())?;

            let platform_id = platform_id.map(|(id,)| id).unwrap_or(0);
            crate::images::GameMediaId::compute_hash(platform_id, &game_title)
        }
    };

    let game_cache_dir = cache_dir
        .join("media")
        .join(game_id.directory_name());

    // Create EmuMovies client
    let client = crate::images::EmuMoviesClient::new(
        crate::images::EmuMoviesConfig {
            username: state_guard.settings.emumovies.username.clone(),
            password: state_guard.settings.emumovies.password.clone(),
        },
        cache_dir.clone(),
    );

    // Download the video
    let video_path = client.get_video(&platform, &game_title, &game_cache_dir, None)
        .map_err(|e| e.to_string())?;

    Ok(video_path.to_string_lossy().to_string())
}

// ============ Emulator Commands ============

use crate::db::schema::EmulatorInfo;

/// Get all emulators for a platform, filtered by current OS
#[tauri::command]
pub async fn get_emulators_for_platform(
    platform_name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<EmulatorInfo>, String> {
    let state_guard = state.read().await;
    handlers::get_emulators_for_platform(&state_guard, &platform_name).await
}

/// Get a specific emulator by name
#[tauri::command]
pub async fn get_emulator(
    name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<EmulatorInfo>, String> {
    let state_guard = state.read().await;
    handlers::get_emulator(&state_guard, &name).await
}

/// Get all emulators (optionally filtered by current OS)
#[tauri::command]
pub async fn get_all_emulators(
    filter_os: Option<bool>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<EmulatorInfo>, String> {
    let state_guard = state.read().await;
    handlers::get_all_emulators(&state_guard, filter_os.unwrap_or(true)).await
}

// ============ Emulator Preference Commands ============

use crate::handlers::EmulatorPreferences;
use crate::emulator::{EmulatorWithStatus, LaunchResult};

/// Get emulator preference for a game (checks game-specific, then platform)
#[tauri::command]
pub async fn get_emulator_preference(
    launchbox_db_id: i64,
    platform_name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<String>, String> {
    let state_guard = state.read().await;
    handlers::get_emulator_preference(&state_guard, launchbox_db_id, &platform_name).await
}

/// Set emulator preference for a specific game
#[tauri::command]
pub async fn set_game_emulator_preference(
    launchbox_db_id: i64,
    emulator_name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::set_game_emulator_preference(&state_guard, launchbox_db_id, &emulator_name).await
}

/// Set emulator preference for a platform (all games on that platform)
#[tauri::command]
pub async fn set_platform_emulator_preference(
    platform_name: String,
    emulator_name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::set_platform_emulator_preference(&state_guard, &platform_name, &emulator_name).await
}

/// Clear a game-specific preference
#[tauri::command]
pub async fn clear_game_emulator_preference(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::clear_game_emulator_preference(&state_guard, launchbox_db_id).await
}

/// Clear a platform preference
#[tauri::command]
pub async fn clear_platform_emulator_preference(
    platform_name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::clear_platform_emulator_preference(&state_guard, &platform_name).await
}

/// Get all emulator preferences (for settings UI)
#[tauri::command]
pub async fn get_all_emulator_preferences(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<EmulatorPreferences, String> {
    let state_guard = state.read().await;
    handlers::get_all_emulator_preferences(&state_guard).await
}

/// Clear all emulator preferences
#[tauri::command]
pub async fn clear_all_emulator_preferences(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::clear_all_emulator_preferences(&state_guard).await
}

// ============ Emulator Installation & Launch Commands ============

/// Get all emulators for a platform with installation status
#[tauri::command]
pub async fn get_emulators_with_status(
    platform_name: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<EmulatorWithStatus>, String> {
    let state_guard = state.read().await;
    handlers::get_emulators_with_status(&state_guard, &platform_name).await
}

/// Install an emulator using the appropriate package manager
#[tauri::command]
pub async fn install_emulator(
    emulator_name: String,
    is_retroarch_core: bool,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<String, String> {
    let state_guard = state.read().await;

    // Look up the emulator by name
    let emulator = handlers::get_emulator(&state_guard, &emulator_name)
        .await?
        .ok_or_else(|| format!("Emulator '{}' not found", emulator_name))?;

    handlers::install_emulator(&emulator, is_retroarch_core).await
}

/// Launch a game with the specified emulator
#[tauri::command]
pub async fn launch_game(
    emulator_name: String,
    rom_path: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<LaunchResult, String> {
    let state_guard = state.read().await;

    // Look up the emulator by name
    let emulator = handlers::get_emulator(&state_guard, &emulator_name)
        .await?
        .ok_or_else(|| format!("Emulator '{}' not found", emulator_name))?;

    handlers::launch_game_with_emulator(&emulator, &rom_path)
}

/// Launch an emulator (without a ROM)
#[tauri::command]
pub async fn launch_emulator(
    emulator_name: String,
    is_retroarch_core: bool,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<LaunchResult, String> {
    let state_guard = state.read().await;

    // Look up the emulator by name
    let emulator = handlers::get_emulator(&state_guard, &emulator_name)
        .await?
        .ok_or_else(|| format!("Emulator '{}' not found", emulator_name))?;

    handlers::launch_emulator_only(&emulator, is_retroarch_core)
}

/// Get the current operating system
#[tauri::command]
pub fn get_current_os() -> String {
    handlers::get_current_os()
}

// ============ Graboid Import Commands ============

use crate::handlers::{
    GameFile, ImportJob, StartImportInput,
    GraboidPrompt, SaveGraboidPromptInput, DeleteGraboidPromptInput,
};

/// Check if a game has an imported file
#[tauri::command]
pub async fn get_game_file(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<GameFile>, String> {
    let state_guard = state.read().await;
    handlers::get_game_file(&state_guard, launchbox_db_id).await
}

/// Get active import job for a game
#[tauri::command]
pub async fn get_active_import(
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<ImportJob>, String> {
    let state_guard = state.read().await;
    handlers::get_active_import(&state_guard, launchbox_db_id).await
}

/// Start a Graboid import job
#[tauri::command]
pub async fn start_graboid_import(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<ImportJob, String> {
    let state_guard = state.read().await;
    handlers::start_graboid_import(&state_guard, StartImportInput {
        launchbox_db_id,
        game_title,
        platform,
    }).await
}

/// Cancel an import job
#[tauri::command]
pub async fn cancel_import(
    job_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::cancel_import(&state_guard, &job_id).await
}

/// Test connection to Graboid server
#[tauri::command]
pub async fn test_graboid_connection(
    server_url: String,
    api_key: String,
) -> Result<ConnectionTestResult, String> {
    Ok(handlers::test_graboid_connection(&server_url, &api_key).await)
}

/// Get all graboid prompts
#[tauri::command]
pub async fn get_graboid_prompts(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<GraboidPrompt>, String> {
    let state_guard = state.read().await;
    handlers::get_graboid_prompts(&state_guard).await
}

/// Save a graboid prompt
#[tauri::command]
pub async fn save_graboid_prompt(
    scope: String,
    platform: Option<String>,
    launchbox_db_id: Option<i64>,
    prompt: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::save_graboid_prompt(&state_guard, SaveGraboidPromptInput {
        scope,
        platform,
        launchbox_db_id,
        prompt,
    }).await
}

/// Delete a graboid prompt
#[tauri::command]
pub async fn delete_graboid_prompt(
    scope: String,
    platform: Option<String>,
    launchbox_db_id: Option<i64>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;
    handlers::delete_graboid_prompt(&state_guard, DeleteGraboidPromptInput {
        scope,
        platform,
        launchbox_db_id,
    }).await
}

/// Get the effective graboid prompt for a game (global + platform + game combined)
#[tauri::command]
pub async fn get_effective_graboid_prompt(
    platform: String,
    launchbox_db_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<String, String> {
    let state_guard = state.read().await;
    handlers::get_effective_graboid_prompt(&state_guard, &platform, launchbox_db_id).await
}
