//! Tauri commands exposed to the frontend

use crate::db::schema::{
    extract_region_from_title, normalize_title_for_display,
    Game, GameVariant, Platform,
};
use crate::import::{find_game_images, LaunchBoxImporter};
use crate::scanner::{RomFile, RomScanner};
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

/// Scan result
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files: usize,
    pub roms: Vec<RomFile>,
}

/// Import result
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub platforms_imported: usize,
    pub games_imported: usize,
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Lunchbox.", name)
}

#[tauri::command]
pub async fn scan_roms(
    paths: Vec<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<ScanResult, String> {
    tracing::info!("Scanning ROM paths: {:?}", paths);

    let paths: Vec<PathBuf> = if paths.is_empty() {
        let state_guard = state.read().await;
        state_guard.settings.rom_directories.clone()
    } else {
        paths.into_iter().map(PathBuf::from).collect()
    };

    let scanner = RomScanner::new();
    let roms = scanner.scan_directories(&paths);

    Ok(ScanResult {
        total_files: roms.len(),
        roms,
    })
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
            result.push(Platform { id, name, game_count: seen.len() as i64, aliases });
        }
        return Ok(result);
    }

    // Fall back to LaunchBox if available
    if let Some(ref importer) = state_guard.launchbox_importer {
        let platforms = importer
            .get_platforms()
            .await
            .map_err(|e: anyhow::Error| e.to_string())?;

        return Ok(platforms
            .into_iter()
            .map(|p| Platform {
                id: p.platform_key,
                name: p.name,
                game_count: 0,
                aliases: None,
            })
            .collect());
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
                    SELECT g.id, g.title, g.platform_id, p.name as platform,
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
                    SELECT g.id, g.title, g.platform_id, p.name as platform,
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
                SELECT g.id, g.title, g.platform_id, p.name as platform,
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
                        database_id: 0,
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

    // Fall back to LaunchBox if available
    if let Some(ref importer) = state_guard.launchbox_importer {
        let lb_limit = limit.unwrap_or(10000) as i64;
        let games = if let Some(ref query) = search {
            importer
                .search_games(query, lb_limit)
                .await
                .map_err(|e: anyhow::Error| e.to_string())?
        } else if let Some(ref platform_name) = platform {
            importer
                .get_games_by_platform(platform_name)
                .await
                .map_err(|e: anyhow::Error| e.to_string())?
        } else {
            Vec::new()
        };

        let lb_path = state_guard.settings.launchbox_path.clone();

        return Ok(games
            .into_iter()
            .map(|g| {
                // Find box front image for grid display
                let box_front = lb_path.as_ref().and_then(|path| {
                    find_game_images(path, &g.platform, &g.name).box_front
                });

                let display_title = normalize_title_for_display(&g.name);
                Game {
                    id: uuid::Uuid::new_v4().to_string(),
                    database_id: g.database_id,
                    title: g.name.clone(),
                    display_title,
                    platform: g.platform.clone(),
                    platform_id: 0,
                    description: g.overview,
                    release_date: g.release_date,
                    release_year: g.release_year,
                    developer: g.developer,
                    publisher: g.publisher,
                    genres: Some(g.genres),
                    players: None,
                    rating: g.community_rating,
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
                    box_front_path: box_front,
                    screenshot_path: None,
                    variant_count: 1,
                }
            })
            .collect());
    }

    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_game_by_id(
    database_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<Game>, String> {
    let state_guard = state.read().await;

    if let Some(ref importer) = state_guard.launchbox_importer {
        if let Some(g) = importer
            .get_game_by_id(database_id)
            .await
            .map_err(|e: anyhow::Error| e.to_string())?
        {
            // Find images on disk using game name and platform
            let (box_front, screenshot) = if let Some(ref lb_path) = state_guard.settings.launchbox_path {
                let images = find_game_images(lb_path, &g.platform, &g.name);
                (images.box_front, images.screenshot)
            } else {
                (None, None)
            };

            let display_title = normalize_title_for_display(&g.name);
            Ok(Some(Game {
                id: uuid::Uuid::new_v4().to_string(),
                database_id: g.database_id,
                title: g.name.clone(),
                display_title,
                platform: g.platform.clone(),
                platform_id: 0,
                description: g.overview,
                release_date: g.release_date,
                release_year: g.release_year,
                developer: g.developer,
                publisher: g.publisher,
                genres: Some(g.genres),
                players: None,
                rating: g.community_rating,
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
                box_front_path: box_front,
                screenshot_path: screenshot,
                variant_count: 1,
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
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
            SELECT g.id, g.title, g.platform_id, p.name as platform,
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
                variant_count: actual_variant_count,
            }));
        }
    }

    Ok(None)
}

/// Calculate region priority for sorting (lower = better, USA/English first)
fn region_priority(title: &str) -> i32 {
    let title_lower = title.to_lowercase();

    // USA is highest priority
    if title_lower.contains("(usa)") || title_lower.contains("(usa,") || title_lower.contains(", usa)") {
        return 0;
    }

    // Check for English language indicator
    let has_english = title_lower.contains("(en)") || title_lower.contains("(en,")
        || title_lower.contains(", en)") || title_lower.contains(", en,");

    // World with English
    if title_lower.contains("(world)") {
        return if has_english { 1 } else { 2 };
    }

    // English language versions (any region)
    if has_english {
        return 3;
    }

    // Europe (often has English)
    if title_lower.contains("(europe)") || title_lower.contains("(europe,") {
        return 4;
    }

    // Other English-speaking regions
    if title_lower.contains("(australia)") || title_lower.contains("(uk)") || title_lower.contains("(canada)") {
        return 5;
    }

    // Japan
    if title_lower.contains("(japan)") || title_lower.contains("(ja)") {
        return 6;
    }

    // No region info - could be anything, put before non-English
    if !title_lower.contains("(") {
        return 7;
    }

    // Everything else (other languages/regions)
    8
}

/// Get all variants (regions/versions) for a given game by display title
#[tauri::command]
pub async fn get_game_variants(
    display_title: String,
    platform_id: i64,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<GameVariant>, String> {
    let state_guard = state.read().await;

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

        // Convert to vec and sort by region priority (USA/English first)
        let mut variants: Vec<GameVariant> = seen_titles.into_values().collect();
        variants.sort_by(|a, b| {
            let priority_a = region_priority(&a.title);
            let priority_b = region_priority(&b.title);
            priority_a.cmp(&priority_b).then_with(|| a.title.cmp(&b.title))
        });

        return Ok(variants);
    }

    Ok(Vec::new())
}

#[tauri::command]
pub async fn import_launchbox(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<ImportResult, String> {
    let state_guard = state.read().await;

    if state_guard.launchbox_importer.is_none() {
        return Err("LaunchBox not configured".to_string());
    }

    let importer = state_guard.launchbox_importer.as_ref().unwrap();

    let platform_count = importer
        .count_platforms()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    let game_count = importer
        .count_games()
        .await
        .map_err(|e: anyhow::Error| e.to_string())?;

    Ok(ImportResult {
        platforms_imported: platform_count as usize,
        games_imported: game_count as usize,
    })
}

#[tauri::command]
pub async fn launch_game(
    rom_path: String,
    platform: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let emulator = state_guard
        .settings
        .default_platform_emulators
        .get(&platform);

    if let Some(emu_id) = emulator {
        let emu_config = state_guard
            .settings
            .emulators
            .iter()
            .find(|e| &e.id == emu_id);

        if let Some(config) = emu_config {
            let command = config
                .command_template
                .replace("{ROM}", &rom_path)
                .replace("{CORE}", "");

            tracing::info!("Launching: {} {}", config.executable_path.display(), command);

            std::process::Command::new(&config.executable_path)
                .args(command.split_whitespace())
                .spawn()
                .map_err(|e| e.to_string())?;

            return Ok(());
        }
    }

    if let Some(ref retroarch_path) = state_guard.settings.retroarch_path {
        tracing::info!("Launching with RetroArch: {}", rom_path);

        std::process::Command::new(retroarch_path)
            .arg(&rom_path)
            .spawn()
            .map_err(|e| e.to_string())?;

        return Ok(());
    }

    Err("No emulator configured for this platform".to_string())
}

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<AppSettings, String> {
    let state_guard = state.read().await;
    Ok(state_guard.settings.clone())
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

    if settings.launchbox_path != state_guard.settings.launchbox_path {
        if let Some(ref lb_path) = settings.launchbox_path {
            let metadata_path = lb_path.join("Metadata").join("LaunchBox.Metadata.db");
            if metadata_path.exists() {
                match LaunchBoxImporter::connect(&metadata_path).await {
                    Ok(importer) => {
                        state_guard.launchbox_importer = Some(importer);
                        tracing::info!("Reconnected to LaunchBox");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to reconnect to LaunchBox: {}", e);
                    }
                }
            }
        }
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

/// Collection for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub filter_rules: Option<String>,
    pub game_count: i64,
}

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

#[tauri::command]
pub async fn get_collections(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Collection>, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let collections: Vec<(String, String, Option<String>, i64, Option<String>, i64)> = sqlx::query_as(
        r#"
        SELECT c.id, c.name, c.description, c.is_smart, c.filter_rules,
               COUNT(cg.game_id) as game_count
        FROM collections c
        LEFT JOIN collection_games cg ON c.id = cg.collection_id
        GROUP BY c.id
        ORDER BY c.name
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(collections.into_iter().map(|(id, name, description, is_smart, filter_rules, game_count)| {
        Collection {
            id,
            name,
            description,
            is_smart: is_smart != 0,
            filter_rules,
            game_count,
        }
    }).collect())
}

#[tauri::command]
pub async fn create_collection(
    name: String,
    description: Option<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Collection, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO collections (id, name, description, is_smart, filter_rules) VALUES (?, ?, ?, 0, NULL)"
    )
    .bind(&id)
    .bind(&name)
    .bind(&description)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Collection {
        id,
        name,
        description,
        is_smart: false,
        filter_rules: None,
        game_count: 0,
    })
}

#[tauri::command]
pub async fn update_collection(
    id: String,
    name: String,
    description: Option<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("UPDATE collections SET name = ?, description = ? WHERE id = ?")
        .bind(&name)
        .bind(&description)
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn delete_collection(
    id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Delete games from collection first (foreign key)
    sqlx::query("DELETE FROM collection_games WHERE collection_id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete the collection
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_collection_games(
    collection_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Vec<Game>, String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get launchbox_db_ids from the collection_games table
    // Note: We store launchbox_db_id as game_id in collection_games for now
    let game_ids: Vec<(String,)> = sqlx::query_as(
        "SELECT game_id FROM collection_games WHERE collection_id = ? ORDER BY sort_order"
    )
    .bind(&collection_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // If we have a LaunchBox importer, look up each game
    if let Some(ref importer) = state_guard.launchbox_importer {
        let lb_path = state_guard.settings.launchbox_path.clone();
        let mut games = Vec::new();

        for (game_id,) in game_ids {
            // Try to parse as i64 (LaunchBox database ID)
            if let Ok(db_id) = game_id.parse::<i64>() {
                if let Ok(Some(g)) = importer.get_game_by_id(db_id).await {
                    let box_front = lb_path.as_ref().and_then(|path| {
                        find_game_images(path, &g.platform, &g.name).box_front
                    });

                    let display_title = normalize_title_for_display(&g.name);
                    games.push(Game {
                        id: game_id,
                        database_id: g.database_id,
                        title: g.name.clone(),
                        display_title,
                        platform: g.platform.clone(),
                        platform_id: 0,
                        description: g.overview,
                        release_date: g.release_date,
                        release_year: g.release_year,
                        developer: g.developer,
                        publisher: g.publisher,
                        genres: Some(g.genres),
                        players: None,
                        rating: g.community_rating,
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
                        box_front_path: box_front,
                        screenshot_path: None,
                        variant_count: 1,
                    });
                }
            }
        }

        Ok(games)
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub async fn add_game_to_collection(
    collection_id: String,
    game_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get the max sort order for this collection
    let max_order: Option<(i64,)> = sqlx::query_as(
        "SELECT MAX(sort_order) FROM collection_games WHERE collection_id = ?"
    )
    .bind(&collection_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    let next_order = max_order.and_then(|o| o.0.checked_add(1)).unwrap_or(0);

    sqlx::query(
        "INSERT OR IGNORE INTO collection_games (collection_id, game_id, sort_order) VALUES (?, ?, ?)"
    )
    .bind(&collection_id)
    .bind(&game_id)
    .bind(next_order)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_game_from_collection(
    collection_id: String,
    game_id: String,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<(), String> {
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM collection_games WHERE collection_id = ? AND game_id = ?")
        .bind(&collection_id)
        .bind(&game_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub user_info: Option<String>,
}

#[tauri::command]
pub async fn test_screenscraper_connection(
    dev_id: String,
    dev_password: String,
    user_id: Option<String>,
    user_password: Option<String>,
) -> Result<ConnectionTestResult, String> {
    use crate::scraper::{ScreenScraperClient, ScreenScraperConfig};

    if dev_id.is_empty() || dev_password.is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "Developer ID and password are required".to_string(),
            user_info: None,
        });
    }

    let config = ScreenScraperConfig {
        dev_id,
        dev_password,
        user_id: user_id.clone(),
        user_password,
    };

    let client = ScreenScraperClient::new(config);

    // Test by looking up a well-known game (Super Mario Bros CRC)
    match client.lookup_by_checksum(
        "3337EC46",  // CRC32 for Super Mario Bros (NES)
        "811B027E",  // partial MD5
        "",
        40976,
        "Super Mario Bros.nes",
        Some(3),  // NES platform ID
    ).await {
        Ok(_) => {
            let user_msg = user_id.map(|u| format!("Logged in as: {}", u));
            Ok(ConnectionTestResult {
                success: true,
                message: "Successfully connected to ScreenScraper API".to_string(),
                user_info: user_msg,
            })
        }
        Err(e) => {
            let err_str = e.to_string();
            let message = if err_str.contains("401") || err_str.contains("403") {
                "Invalid credentials. Please check your developer ID and password.".to_string()
            } else if err_str.contains("429") {
                "Rate limited. ScreenScraper connection works but you've hit the request limit.".to_string()
            } else {
                format!("Connection failed: {}", err_str)
            };
            Ok(ConnectionTestResult {
                success: false,
                message,
                user_info: None,
            })
        }
    }
}

#[tauri::command]
pub async fn test_steamgriddb_connection(
    api_key: String,
) -> Result<ConnectionTestResult, String> {
    use crate::scraper::{SteamGridDBClient, SteamGridDBConfig};

    if api_key.is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "API key is required".to_string(),
            user_info: None,
        });
    }

    let config = SteamGridDBConfig { api_key };
    let client = SteamGridDBClient::new(config);

    match client.test_connection().await {
        Ok(()) => Ok(ConnectionTestResult {
            success: true,
            message: "Successfully connected to SteamGridDB API".to_string(),
            user_info: None,
        }),
        Err(e) => {
            let err_str = e.to_string();
            let message = if err_str.contains("401") || err_str.contains("403") {
                "Invalid API key. Please check your key.".to_string()
            } else {
                format!("Connection failed: {}", err_str)
            };
            Ok(ConnectionTestResult {
                success: false,
                message,
                user_info: None,
            })
        }
    }
}

#[tauri::command]
pub async fn test_igdb_connection(
    client_id: String,
    client_secret: String,
) -> Result<ConnectionTestResult, String> {
    use crate::scraper::{IGDBClient, IGDBConfig};

    if client_id.is_empty() || client_secret.is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "Client ID and Client Secret are required".to_string(),
            user_info: None,
        });
    }

    let config = IGDBConfig { client_id, client_secret };
    let client = IGDBClient::new(config);

    match client.test_connection().await {
        Ok(found_game) => Ok(ConnectionTestResult {
            success: true,
            message: "Successfully connected to IGDB API".to_string(),
            user_info: Some(found_game),
        }),
        Err(e) => {
            let err_str = e.to_string();
            let message = if err_str.contains("401") || err_str.contains("403") || err_str.contains("invalid") {
                "Invalid credentials. Please check your Twitch Client ID and Secret.".to_string()
            } else {
                format!("Connection failed: {}", err_str)
            };
            Ok(ConnectionTestResult {
                success: false,
                message,
                user_info: None,
            })
        }
    }
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
    let state_guard = state.read().await;

    let pool = state_guard.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let favorites: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform FROM favorites ORDER BY added_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Look up full game info from LaunchBox
    if let Some(ref importer) = state_guard.launchbox_importer {
        let lb_path = state_guard.settings.launchbox_path.clone();
        let mut games = Vec::new();

        for (db_id, _title, _platform) in favorites {
            if let Ok(Some(g)) = importer.get_game_by_id(db_id).await {
                let box_front = lb_path.as_ref().and_then(|path| {
                    find_game_images(path, &g.platform, &g.name).box_front
                });

                let display_title = normalize_title_for_display(&g.name);
                games.push(Game {
                    id: db_id.to_string(),
                    database_id: g.database_id,
                    title: g.name.clone(),
                    display_title,
                    platform: g.platform.clone(),
                    platform_id: 0,
                    description: g.overview,
                    release_date: g.release_date,
                    release_year: g.release_year,
                    developer: g.developer,
                    publisher: g.publisher,
                    genres: Some(g.genres),
                    players: None,
                    rating: g.community_rating,
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
                    box_front_path: box_front,
                    screenshot_path: None,
                    variant_count: 1,
                });
            }
        }

        Ok(games)
    } else {
        Ok(Vec::new())
    }
}
