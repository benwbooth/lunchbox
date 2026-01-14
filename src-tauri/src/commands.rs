//! Tauri commands exposed to the frontend

use crate::import::{find_game_images, LaunchBoxImporter};
use crate::scanner::{RomFile, RomScanner};
use crate::scraper::{get_screenscraper_platform_id, ScreenScraperClient, ScreenScraperConfig};
use crate::state::{AppSettings, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

type AppStateHandle = Arc<RwLock<AppState>>;

/// Platform for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub game_count: i64,
}

/// Game for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub database_id: i64,
    pub title: String,
    pub platform: String,
    pub platform_id: i64,
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Option<String>,
    pub rating: Option<f64>,
    pub box_front_path: Option<String>,
    pub screenshot_path: Option<String>,
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
        let platforms: Vec<(i64, String, i64)> = sqlx::query_as(
            r#"
            SELECT p.id, p.name, COUNT(g.id) as game_count
            FROM platforms p
            LEFT JOIN games g ON g.platform_id = p.id
            GROUP BY p.id
            ORDER BY p.name
            "#
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        return Ok(platforms
            .into_iter()
            .map(|(id, name, game_count)| Platform { id, name, game_count })
            .collect());
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
            })
            .collect());
    }

    // No database available - show empty state
    Ok(Vec::new())
}

/// Get total count of games for a platform/search
#[tauri::command]
pub async fn get_game_count(
    platform: Option<String>,
    search: Option<String>,
    state: tauri::State<'_, AppStateHandle>,
) -> Result<i64, String> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let count: (i64,) = if let Some(ref query) = search {
            let pattern = format!("%{}%", query);
            if let Some(ref platform_name) = platform {
                sqlx::query_as(
                    "SELECT COUNT(*) FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ? AND g.title LIKE ?"
                )
                .bind(platform_name)
                .bind(&pattern)
                .fetch_one(games_pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query_as("SELECT COUNT(*) FROM games WHERE title LIKE ?")
                    .bind(&pattern)
                    .fetch_one(games_pool)
                    .await
                    .map_err(|e| e.to_string())?
            }
        } else if let Some(ref platform_name) = platform {
            sqlx::query_as(
                "SELECT COUNT(*) FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ?"
            )
            .bind(platform_name)
            .fetch_one(games_pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM games")
                .fetch_one(games_pool)
                .await
                .map_err(|e| e.to_string())?
        };

        return Ok(count.0);
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
    let state_guard = state.read().await;
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    // Try shipped games database first (browse-first mode)
    if let Some(ref games_pool) = state_guard.games_db_pool {
        let games: Vec<(String, String, i64, String, Option<String>, Option<i32>, Option<String>, Option<String>, Option<String>)> = if let Some(ref query) = search {
            // Search by title
            let pattern = format!("%{}%", query);
            sqlx::query_as(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform,
                       g.description, g.release_year, g.developer, g.publisher, g.genre
                FROM games g
                JOIN platforms p ON g.platform_id = p.id
                WHERE g.title LIKE ?
                ORDER BY g.title
                LIMIT ? OFFSET ?
                "#
            )
            .bind(&pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(games_pool)
            .await
            .map_err(|e| e.to_string())?
        } else if let Some(ref platform_name) = platform {
            // Filter by platform
            sqlx::query_as(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform,
                       g.description, g.release_year, g.developer, g.publisher, g.genre
                FROM games g
                JOIN platforms p ON g.platform_id = p.id
                WHERE p.name = ?
                ORDER BY g.title
                LIMIT ? OFFSET ?
                "#
            )
            .bind(platform_name)
            .bind(limit)
            .bind(offset)
            .fetch_all(games_pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            // No filter - return empty (user should select platform first with 147K+ games)
            Vec::new()
        };

        return Ok(games
            .into_iter()
            .map(|(id, title, platform_id, platform, description, release_year, developer, publisher, genre)| Game {
                id: id.clone(),
                database_id: 0, // Not from LaunchBox
                title,
                platform,
                platform_id,
                description,
                release_date: None,
                release_year,
                developer,
                publisher,
                genres: genre,
                rating: None,
                box_front_path: None, // Fetched on-demand via media API
                screenshot_path: None,
            })
            .collect());
    }

    // Fall back to LaunchBox if available
    if let Some(ref importer) = state_guard.launchbox_importer {
        let games = if let Some(ref query) = search {
            importer
                .search_games(query, limit)
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

                Game {
                    id: uuid::Uuid::new_v4().to_string(),
                    database_id: g.database_id,
                    title: g.name,
                    platform: g.platform.clone(),
                    platform_id: 0,
                    description: g.overview,
                    release_date: g.release_date,
                    release_year: g.release_year,
                    developer: g.developer,
                    publisher: g.publisher,
                    genres: Some(g.genres),
                    rating: g.community_rating,
                    box_front_path: box_front,
                    screenshot_path: None,
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

            Ok(Some(Game {
                id: uuid::Uuid::new_v4().to_string(),
                database_id: g.database_id,
                title: g.name,
                platform: g.platform.clone(),
                platform_id: 0,
                description: g.overview,
                release_date: g.release_date,
                release_year: g.release_year,
                developer: g.developer,
                publisher: g.publisher,
                genres: Some(g.genres),
                rating: g.community_rating,
                box_front_path: box_front,
                screenshot_path: screenshot,
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
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
            let game = Game {
                id: uuid::Uuid::new_v4().to_string(),
                database_id: scraped.screenscraper_id,
                title: scraped.name,
                platform: platform.clone(),
                platform_id: 0,
                description: scraped.description,
                release_date: scraped.release_date,
                release_year: None,
                developer: scraped.developer,
                publisher: scraped.publisher,
                genres: Some(scraped.genres.join(", ")),
                rating: scraped.rating,
                box_front_path: scraped.media.box_front,
                screenshot_path: scraped.media.screenshot,
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

                    games.push(Game {
                        id: game_id,
                        database_id: g.database_id,
                        title: g.name,
                        platform: g.platform.clone(),
                        platform_id: 0,
                        description: g.overview,
                        release_date: g.release_date,
                        release_year: g.release_year,
                        developer: g.developer,
                        publisher: g.publisher,
                        genres: Some(g.genres),
                        rating: g.community_rating,
                        box_front_path: box_front,
                        screenshot_path: None,
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

                games.push(Game {
                    id: db_id.to_string(),
                    database_id: g.database_id,
                    title: g.name,
                    platform: g.platform.clone(),
                    platform_id: 0,
                    description: g.overview,
                    release_date: g.release_date,
                    release_year: g.release_year,
                    developer: g.developer,
                    publisher: g.publisher,
                    genres: Some(g.genres),
                    rating: g.community_rating,
                    box_front_path: box_front,
                    screenshot_path: None,
                });
            }
        }

        Ok(games)
    } else {
        Ok(Vec::new())
    }
}
