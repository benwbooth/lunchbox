//! HTTP API for development mode
//!
//! Provides HTTP endpoints that mirror the Tauri commands, allowing
//! the frontend to work in a regular browser during development.

use crate::db::schema::{
    extract_region_from_title, normalize_title_for_dedup, normalize_title_for_display,
    Game, GameVariant, Platform,
};
use crate::handlers::{
    self as handlers, Collection, CollectionIdInput, CollectionGameInput,
    CreateCollectionInput,
};
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

type SharedState = Arc<RwLock<AppState>>;

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

/// Create the HTTP API router
pub fn create_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/log", post(frontend_log))
        .route("/api/platforms", get(get_platforms))
        .route("/api/regions", get(get_all_regions))
        .route("/api/games", get(get_games))
        .route("/api/games/count", get(get_game_count))
        .route("/api/games/:uuid", get(get_game_by_uuid))
        .route("/api/games/:uuid/variants", get(get_game_variants))
        .route("/api/settings", get(get_settings).post(save_settings_http))
        .route("/api/credential-storage", get(get_credential_storage))
        .route("/api/stats/:db_id", get(get_play_stats))
        .route("/api/favorites", get(get_favorites))
        .route("/api/favorites/check/:db_id", get(check_is_favorite))
        .route("/api/favorites/:game_id", post(add_favorite).delete(remove_favorite))
        // Collection endpoints
        .route("/rspc/get_collections", get(rspc_get_collections))
        .route("/rspc/create_collection", get(rspc_create_collection))
        .route("/rspc/delete_collection", get(rspc_delete_collection))
        .route("/rspc/get_collection_games", get(rspc_get_collection_games))
        .route("/rspc/add_game_to_collection", get(rspc_add_game_to_collection))
        .route("/rspc/remove_game_from_collection", get(rspc_remove_game_from_collection))
        // rspc-style endpoints for image handling
        .route("/rspc/get_game_image", get(rspc_get_game_image))
        .route("/rspc/check_cached_media", get(rspc_check_cached_media))
        .route("/rspc/download_image_with_fallback", get(rspc_download_image_with_fallback))
        // rspc-style endpoints for video handling
        .route("/rspc/check_cached_video", get(rspc_check_cached_video))
        .route("/rspc/download_game_video", get(rspc_download_game_video))
        // rspc-style endpoints for emulator handling
        .route("/rspc/get_emulators_for_platform", get(rspc_get_emulators_for_platform))
        .route("/rspc/get_emulator", get(rspc_get_emulator))
        .route("/rspc/get_all_emulators", get(rspc_get_all_emulators))
        // rspc-style endpoints for play session
        .route("/rspc/record_play_session", get(rspc_record_play_session))
        // rspc-style endpoints for emulator preferences
        .route("/rspc/get_emulator_preference", get(rspc_get_emulator_preference))
        .route("/rspc/set_game_emulator_preference", get(rspc_set_game_emulator_preference))
        .route("/rspc/set_platform_emulator_preference", get(rspc_set_platform_emulator_preference))
        .route("/rspc/clear_game_emulator_preference", get(rspc_clear_game_emulator_preference))
        .route("/rspc/clear_platform_emulator_preference", get(rspc_clear_platform_emulator_preference))
        .route("/rspc/get_all_emulator_preferences", get(rspc_get_all_emulator_preferences))
        .route("/rspc/clear_all_emulator_preferences", get(rspc_clear_all_emulator_preferences))
        // Emulator installation and launch endpoints
        .route("/rspc/get_emulators_with_status", get(rspc_get_emulators_with_status))
        .route("/rspc/install_emulator", get(rspc_install_emulator))
        .route("/rspc/launch_emulator", get(rspc_launch_emulator))
        .route("/rspc/launch_game", get(rspc_launch_game))
        .route("/rspc/get_current_os", get(rspc_get_current_os))
        // Asset serving for browser dev mode
        .route("/assets/*path", get(serve_asset))
        .layer(cors)
        .with_state(state)
}

/// Health check response with build info
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub build_hash: String,
    pub build_timestamp: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        build_hash: env!("BUILD_HASH").to_string(),
        build_timestamp: env!("BUILD_TIMESTAMP").to_string(),
    })
}

// ============================================================================
// Regions
// ============================================================================

async fn get_all_regions(
    State(state): State<SharedState>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    use crate::db::schema::extract_region_from_title;

    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        // Get unique regions from the region column
        let explicit_regions: Vec<(Option<String>,)> = sqlx::query_as(
            "SELECT DISTINCT region FROM games WHERE region IS NOT NULL AND region != ''"
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut regions: HashSet<String> = HashSet::new();

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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for (title,) in titles {
            if let Some(extracted) = extract_region_from_title(&title) {
                regions.insert(extracted);
            }
        }

        // Sort alphabetically
        let mut result: Vec<String> = regions.into_iter().collect();
        result.sort();

        // Add empty string for "plain/no region" at the start
        result.insert(0, String::new());

        return Ok(Json(result));
    }

    Ok(Json(Vec::new()))
}

#[derive(Debug, Deserialize)]
struct LogMessage {
    level: String,
    message: String,
}

async fn frontend_log(Json(log): Json<LogMessage>) -> &'static str {
    match log.level.as_str() {
        "error" => tracing::error!("[FRONTEND] {}", log.message),
        "warn" => tracing::warn!("[FRONTEND] {}", log.message),
        "info" => tracing::info!("[FRONTEND] {}", log.message),
        _ => tracing::debug!("[FRONTEND] {}", log.message),
    }
    "ok"
}

// ============================================================================
// Platforms
// ============================================================================

/// Sanitize a platform name for use as a filename
fn platform_name_to_filename(name: &str) -> String {
    name.replace("/", "-")
        .replace(":", "-")
        .replace("&", "and")
        .replace(" ", "_")
}

async fn get_platforms(
    State(state): State<SharedState>,
) -> Result<Json<Vec<Platform>>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let platforms: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT id, name, aliases FROM platforms ORDER BY name"
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut result = Vec::new();
        for (id, name, aliases) in platforms {
            let all_titles: Vec<(String,)> = sqlx::query_as(
                "SELECT title FROM games WHERE platform_id = ?"
            )
            .bind(id)
            .fetch_all(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let mut seen: HashSet<String> = HashSet::new();
            for (title,) in all_titles {
                let normalized = normalize_title_for_dedup(&title);
                seen.insert(normalized);
            }
            // Use database aliases or generate them if not present
            let aliases = aliases.or_else(|| get_platform_search_aliases(&name));
            // Build icon URL from platform name (icons are named after canonical platform names)
            let filename = platform_name_to_filename(&name);
            let icon_url = Some(format!("/assets/platforms/{}.png", filename));
            result.push(Platform { id, name, game_count: seen.len() as i64, aliases, icon_url });
        }
        return Ok(Json(result));
    }

    Ok(Json(Vec::new()))
}

// ============================================================================
// Games
// ============================================================================

#[derive(Debug, Deserialize)]
struct GamesQuery {
    platform: Option<String>,
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Build search patterns from a query string - splits into words and creates LIKE patterns for each
fn build_search_patterns(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(|word| format!("%{}%", word))
        .collect()
}

async fn get_games(
    State(state): State<SharedState>,
    axum::extract::Query(query): axum::extract::Query<GamesQuery>,
) -> Result<Json<Vec<Game>>, (StatusCode, String)> {
    let state_guard = state.read().await;
    let limit = query.limit.map(|l| l as usize);
    let offset = query.offset.unwrap_or(0) as usize;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let raw_rows = if let Some(ref search_query) = query.search {
            // Split search into words for flexible matching
            let patterns = build_search_patterns(search_query);
            if patterns.is_empty() {
                Vec::new()
            } else {
                // Build dynamic WHERE clause for multi-word search (SQLite uses ? for placeholders)
                let like_clauses: Vec<&str> = patterns.iter().map(|_| "g.title LIKE ?").collect();
                let where_clause = like_clauses.join(" AND ");

                if let Some(ref platform_name) = query.platform {
                    let sql = format!(
                        r#"
                        SELECT g.id, g.title, g.platform_id, p.name as platform, COALESCE(g.launchbox_db_id, 0) as launchbox_db_id,
                               g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                               g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                               g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
                        FROM games g
                        JOIN platforms p ON g.platform_id = p.id
                        WHERE p.name = ? AND ({})
                        ORDER BY g.title
                        "#,
                        where_clause
                    );
                    let mut q = sqlx::query(&sql);
                    q = q.bind(platform_name);
                    for pattern in &patterns {
                        q = q.bind(pattern);
                    }
                    q.fetch_all(games_pool)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                } else {
                    let sql = format!(
                        r#"
                        SELECT g.id, g.title, g.platform_id, p.name as platform, COALESCE(g.launchbox_db_id, 0) as launchbox_db_id,
                               g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                               g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                               g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
                        FROM games g
                        JOIN platforms p ON g.platform_id = p.id
                        WHERE {}
                        ORDER BY g.title
                        "#,
                        where_clause
                    );
                    let mut q = sqlx::query(&sql);
                    for pattern in &patterns {
                        q = q.bind(pattern);
                    }
                    q.fetch_all(games_pool)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                }
            }
        } else if let Some(ref platform_name) = query.platform {
            sqlx::query(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform, COALESCE(g.launchbox_db_id, 0) as launchbox_db_id,
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
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform, COALESCE(g.launchbox_db_id, 0) as launchbox_db_id,
                       g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                       g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                       g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
                FROM games g
                JOIN platforms p ON g.platform_id = p.id
                ORDER BY g.title
                "#
            )
            .fetch_all(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        };

        // Deduplicate by normalized title, keeping the "best" variant
        let mut seen: HashMap<String, Game> = HashMap::new();
        let mut variant_counts: HashMap<String, i32> = HashMap::new();

        for row in raw_rows {
            use sqlx::Row;
            let id: String = row.get("id");
            let title: String = row.get("title");
            let platform_id: i64 = row.get("platform_id");
            let platform: String = row.get("platform");
            let launchbox_db_id: i64 = row.get("launchbox_db_id");

            let normalized = normalize_title_for_dedup(&title);
            let key = format!("{}:{}", platform_id, normalized);

            *variant_counts.entry(key.clone()).or_insert(0) += 1;

            if !seen.contains_key(&key) {
                let game = Game {
                    id,
                    database_id: launchbox_db_id,
                    title: title.clone(),
                    display_title: normalize_title_for_display(&title),
                    platform,
                    platform_id,
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
                    cooperative: row.get::<Option<i32>, _>("cooperative").map(|v| v != 0),
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
                };
                seen.insert(key, game);
            }
        }

        // Update variant counts
        for (key, game) in seen.iter_mut() {
            game.variant_count = *variant_counts.get(key).unwrap_or(&1);
        }

        // Sort and paginate
        let mut games: Vec<Game> = seen.into_values().collect();
        games.sort_by(|a, b| a.display_title.to_lowercase().cmp(&b.display_title.to_lowercase()));

        let games: Vec<Game> = if let Some(lim) = limit {
            games.into_iter().skip(offset).take(lim).collect()
        } else {
            games.into_iter().skip(offset).collect()
        };

        return Ok(Json(games));
    }

    Ok(Json(Vec::new()))
}

#[derive(Debug, Deserialize)]
struct GameCountQuery {
    platform: Option<String>,
    search: Option<String>,
}

async fn get_game_count(
    State(state): State<SharedState>,
    axum::extract::Query(query): axum::extract::Query<GameCountQuery>,
) -> Result<Json<i64>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let titles: Vec<(String,)> = if let Some(ref search_query) = query.search {
            // Split search into words for flexible matching
            let patterns = build_search_patterns(search_query);
            if patterns.is_empty() {
                Vec::new()
            } else {
                let like_clauses: Vec<&str> = patterns.iter().map(|_| "g.title LIKE ?").collect();
                let where_clause = like_clauses.join(" AND ");

                if let Some(ref platform_name) = query.platform {
                    let sql = format!(
                        "SELECT g.title FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ? AND ({})",
                        where_clause
                    );
                    let mut q = sqlx::query_as(&sql);
                    q = q.bind(platform_name);
                    for pattern in &patterns {
                        q = q.bind(pattern);
                    }
                    q.fetch_all(games_pool)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                } else {
                    let sql = format!("SELECT title FROM games g WHERE {}", where_clause);
                    let mut q = sqlx::query_as(&sql);
                    for pattern in &patterns {
                        q = q.bind(pattern);
                    }
                    q.fetch_all(games_pool)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                }
            }
        } else if let Some(ref platform_name) = query.platform {
            sqlx::query_as(
                "SELECT g.title FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ?"
            )
            .bind(platform_name)
            .fetch_all(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        } else {
            sqlx::query_as("SELECT title FROM games")
                .fetch_all(games_pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        };

        let mut seen: HashSet<String> = HashSet::new();
        for (title,) in titles {
            let normalized = normalize_title_for_dedup(&title);
            seen.insert(normalized);
        }
        return Ok(Json(seen.len() as i64));
    }

    Ok(Json(0))
}

async fn get_game_by_uuid(
    State(state): State<SharedState>,
    axum::extract::Path(uuid): axum::extract::Path<String>,
) -> Result<Json<Option<Game>>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let row = sqlx::query(
            r#"
            SELECT g.id, g.title, g.platform_id, p.name as platform, COALESCE(g.launchbox_db_id, 0) as launchbox_db_id,
                   g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                   g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url,
                   g.release_type, g.notes, g.sort_title, g.series, g.region, g.play_mode, g.version, g.status, g.steam_app_id
            FROM games g
            JOIN platforms p ON g.platform_id = p.id
            WHERE g.id = ?
            "#
        )
        .bind(&uuid)
        .fetch_optional(games_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(row) = row {
            use sqlx::Row;
            let title: String = row.get("title");
            let platform_id: i64 = row.get("platform_id");
            let launchbox_db_id: i64 = row.get("launchbox_db_id");

            // Count variants using normalize_for_dedup for consistency with list view
            let normalized_for_dedup = normalize_title_for_dedup(&title);
            let all_titles: Vec<(String,)> = sqlx::query_as(
                "SELECT title FROM games WHERE platform_id = ?"
            )
            .bind(platform_id)
            .fetch_all(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let variant_count = all_titles
                .iter()
                .filter(|(t,)| normalize_title_for_dedup(t) == normalized_for_dedup)
                .count() as i64;

            let game = Game {
                id: row.get("id"),
                database_id: launchbox_db_id,
                title: title.clone(),
                display_title: normalize_title_for_display(&title),
                platform: row.get("platform"),
                platform_id,
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
                cooperative: row.get::<Option<i32>, _>("cooperative").map(|v| v != 0),
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
                variant_count: variant_count as i32,
            };

            return Ok(Json(Some(game)));
        }
    }

    Ok(Json(None))
}

#[derive(Debug, Deserialize)]
struct VariantsQuery {
    title: String,
    platform_id: i64,
}

async fn get_game_variants(
    State(state): State<SharedState>,
    axum::extract::Path(uuid): axum::extract::Path<String>,
) -> Result<Json<Vec<GameVariant>>, (StatusCode, String)> {
    let state_guard = state.read().await;

    // Get custom region priority from settings
    let custom_region_order = state_guard.settings.region_priority.clone();

    if let Some(ref games_pool) = state_guard.games_db_pool {
        // First get the game to find its normalized title and platform
        let game_row = sqlx::query("SELECT title, platform_id FROM games WHERE id = ?")
            .bind(&uuid)
            .fetch_optional(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(row) = game_row {
            use sqlx::Row;
            let title: String = row.get("title");
            let platform_id: i64 = row.get("platform_id");
            // Use normalize_for_dedup to match how variants are counted in get_games
            let normalized = normalize_title_for_dedup(&title);

            // Find all variants with the same normalized title
            let variants: Vec<(String, String)> = sqlx::query_as(
                "SELECT id, title FROM games WHERE platform_id = ? ORDER BY title"
            )
            .bind(platform_id)
            .fetch_all(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let mut result: Vec<GameVariant> = variants
                .into_iter()
                .filter(|(_, t)| normalize_title_for_dedup(t) == normalized)
                .map(|(id, title)| GameVariant {
                    id,
                    region: extract_region_from_title(&title),
                    title,
                })
                .collect();

            // Sort by region priority (uses user's preference if set)
            result.sort_by(|a, b| {
                let priority_a = crate::commands::region_priority_for_title(&a.title, &custom_region_order);
                let priority_b = crate::commands::region_priority_for_title(&b.title, &custom_region_order);
                priority_a.cmp(&priority_b).then_with(|| a.title.cmp(&b.title))
            });

            return Ok(Json(result));
        }
    }

    Ok(Json(Vec::new()))
}

// ============================================================================
// Settings
// ============================================================================

async fn get_settings(
    State(state): State<SharedState>,
) -> Result<Json<crate::state::AppSettings>, (StatusCode, String)> {
    let state_guard = state.read().await;
    Ok(Json(state_guard.settings.clone()))
}

async fn save_settings_http(
    State(state): State<SharedState>,
    Json(settings): Json<crate::state::AppSettings>,
) -> Result<(), (StatusCode, String)> {
    let mut state_guard = state.write().await;

    if let Some(ref pool) = state_guard.db_pool {
        crate::state::save_settings(pool, &settings)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    state_guard.settings = settings;
    Ok(())
}

async fn get_credential_storage() -> Json<String> {
    Json(crate::keyring_store::get_credential_storage_name().to_string())
}

// ============================================================================
// Play Stats
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayStats {
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub play_count: i64,
    pub total_play_time_seconds: i64,
    pub last_played: Option<String>,
    pub first_played: Option<String>,
}

async fn get_play_stats(
    State(state): State<SharedState>,
    axum::extract::Path(db_id): axum::extract::Path<i64>,
) -> Result<Json<Option<PlayStats>>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref db_pool) = state_guard.db_pool {
        let row: Option<(i64, String, String, i64, i64, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT launchbox_db_id, game_title, platform, play_count, total_play_time_seconds, last_played, first_played
            FROM play_stats WHERE launchbox_db_id = ?
            "#
        )
        .bind(db_id)
        .fetch_optional(db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some((launchbox_db_id, game_title, platform, play_count, total_play_time_seconds, last_played, first_played)) = row {
            return Ok(Json(Some(PlayStats {
                launchbox_db_id,
                game_title,
                platform,
                play_count,
                total_play_time_seconds,
                last_played,
                first_played,
            })));
        }
    }

    Ok(Json(None))
}

// ============================================================================
// Favorites
// ============================================================================

async fn check_is_favorite(
    State(state): State<SharedState>,
    axum::extract::Path(db_id): axum::extract::Path<i64>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref db_pool) = state_guard.db_pool {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM favorites WHERE launchbox_db_id = ?"
        )
        .bind(db_id)
        .fetch_one(db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        return Ok(Json(count.0 > 0));
    }

    Ok(Json(false))
}

async fn get_favorites(
    State(state): State<SharedState>,
) -> Result<Json<Vec<Game>>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let (Some(ref db_pool), Some(ref games_pool)) = (&state_guard.db_pool, &state_guard.games_db_pool) {
        let favorite_ids: Vec<(String,)> = sqlx::query_as(
            "SELECT game_id FROM favorites ORDER BY added_at DESC"
        )
        .fetch_all(db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut games = Vec::new();
        for (game_id,) in favorite_ids {
            let row = sqlx::query(
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
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some(row) = row {
                use sqlx::Row;
                let title: String = row.get("title");
                games.push(Game {
                    id: row.get("id"),
                    database_id: 0,
                    title: title.clone(),
                    display_title: normalize_title_for_display(&title),
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
                    cooperative: row.get::<Option<i32>, _>("cooperative").map(|v| v != 0),
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
                });
            }
        }
        return Ok(Json(games));
    }

    Ok(Json(Vec::new()))
}

async fn add_favorite(
    State(state): State<SharedState>,
    axum::extract::Path(game_id): axum::extract::Path<String>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref db_pool) = state_guard.db_pool {
        sqlx::query("INSERT OR IGNORE INTO favorites (game_id) VALUES (?)")
            .bind(&game_id)
            .execute(db_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        return Ok(Json(true));
    }

    Ok(Json(false))
}

async fn remove_favorite(
    State(state): State<SharedState>,
    axum::extract::Path(game_id): axum::extract::Path<String>,
) -> Result<Json<bool>, (StatusCode, String)> {
    let state_guard = state.read().await;

    if let Some(ref db_pool) = state_guard.db_pool {
        sqlx::query("DELETE FROM favorites WHERE game_id = ?")
            .bind(&game_id)
            .execute(db_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        return Ok(Json(true));
    }

    Ok(Json(false))
}

// ============================================================================
// Collections - Using shared handlers from handlers.rs
// ============================================================================

async fn rspc_get_collections(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    match handlers::get_collections(&state_guard).await {
        Ok(collections) => rspc_ok(collections).into_response(),
        Err(e) => rspc_err::<Vec<Collection>>(e).into_response(),
    }
}

async fn rspc_create_collection(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Collection>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CreateCollectionInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<Collection>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::create_collection(&state_guard, input).await {
        Ok(collection) => rspc_ok(collection).into_response(),
        Err(e) => rspc_err::<Collection>(e).into_response(),
    }
}

async fn rspc_delete_collection(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<bool>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CollectionIdInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<bool>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::delete_collection(&state_guard, input).await {
        Ok(result) => rspc_ok(result).into_response(),
        Err(e) => rspc_err::<bool>(e).into_response(),
    }
}

async fn rspc_get_collection_games(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Vec<Game>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CollectionIdInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<Vec<Game>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::get_collection_games(&state_guard, input).await {
        Ok(games) => rspc_ok(games).into_response(),
        Err(e) => rspc_err::<Vec<Game>>(e).into_response(),
    }
}

async fn rspc_add_game_to_collection(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<bool>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CollectionGameInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<bool>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::add_game_to_collection(&state_guard, input).await {
        Ok(result) => rspc_ok(result).into_response(),
        Err(e) => rspc_err::<bool>(e).into_response(),
    }
}

async fn rspc_remove_game_from_collection(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<bool>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CollectionGameInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<bool>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::remove_game_from_collection(&state_guard, input).await {
        Ok(result) => rspc_ok(result).into_response(),
        Err(e) => rspc_err::<bool>(e).into_response(),
    }
}

// ============================================================================
// rspc-style Image Endpoints
// ============================================================================

// JSON-RPC response wrapper for rspc compatibility
#[derive(Debug, serde::Serialize)]
struct RspcResponse<T: serde::Serialize> {
    result: RspcResult<T>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "data")]
enum RspcResult<T: serde::Serialize> {
    #[serde(rename = "response")]
    Response(T),
    #[serde(rename = "error")]
    Error { code: i32, message: String },
}

fn rspc_ok<T: serde::Serialize>(data: T) -> Json<RspcResponse<T>> {
    Json(RspcResponse {
        result: RspcResult::Response(data),
    })
}

fn rspc_err<T: serde::Serialize>(message: String) -> (StatusCode, Json<RspcResponse<T>>) {
    (StatusCode::OK, Json(RspcResponse {
        result: RspcResult::Error { code: -1, message },
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetGameImageInput {
    launchbox_db_id: i64,
    image_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    pub id: i64,
    pub launchbox_db_id: i64,
    pub image_type: String,
    pub cdn_url: String,
    pub local_path: Option<String>,
    pub downloaded: bool,
}

async fn rspc_get_game_image(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    // Parse the input parameter (JSON-encoded)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Option<ImageInfo>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: GetGameImageInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<Option<ImageInfo>>(format!("Invalid input: {}", e)).into_response(),
    };

    tracing::info!("rspc_get_game_image: launchbox_db_id={}, image_type={}",
        input.launchbox_db_id, input.image_type);

    let state_guard = state.read().await;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let cache_dir = crate::commands::get_cache_dir(&state_guard.settings);
        let mut service = crate::images::ImageService::new(games_pool.clone(), cache_dir);
        if let Some(ref images_pool) = state_guard.images_db_pool {
            service = service.with_images_pool(images_pool.clone());
        }

        match service.get_image_by_type(input.launchbox_db_id, &input.image_type).await {
            Ok(Some(info)) => {
                return rspc_ok(Some(ImageInfo {
                    id: info.id,
                    launchbox_db_id: info.launchbox_db_id,
                    image_type: info.image_type,
                    cdn_url: info.cdn_url,
                    local_path: info.local_path,
                    downloaded: info.downloaded,
                })).into_response();
            }
            Ok(None) => {
                tracing::info!("  No image metadata found");
                return rspc_ok::<Option<ImageInfo>>(None).into_response();
            }
            Err(e) => {
                tracing::warn!("  Error getting image: {}", e);
                return rspc_err::<Option<ImageInfo>>(e.to_string()).into_response();
            }
        }
    }

    rspc_ok::<Option<ImageInfo>>(None).into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckCachedMediaInput {
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CachedMediaResult {
    path: String,
    source: String,
}

/// Check if media is cached locally (fast path - no network requests)
async fn rspc_check_cached_media(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Option<CachedMediaResult>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CheckCachedMediaInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<Option<CachedMediaResult>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    let cache_dir = crate::commands::get_cache_dir(&state_guard.settings);

    // Compute game_id
    let game_id = crate::images::get_game_cache_id(
        input.launchbox_db_id,
        &input.game_title,
        &input.platform,
    );

    // Check cache
    if let Some((path, source)) = crate::images::find_cached_media(&cache_dir, &game_id, &input.image_type) {
        tracing::info!("check_cached_media: HIT game_id={}, path={:?}", game_id, path);
        return rspc_ok(Some(CachedMediaResult {
            path: path.to_string_lossy().to_string(),
            source: source.abbreviation().to_string(),
        })).into_response();
    }

    tracing::info!("check_cached_media: MISS game_id={}, cache_dir={:?}", game_id, cache_dir);
    rspc_ok::<Option<CachedMediaResult>>(None).into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadImageWithFallbackInput {
    game_title: String,
    platform: String,
    image_type: String,
    launchbox_db_id: Option<i64>,
}

async fn rspc_download_image_with_fallback(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the input parameter (JSON-encoded)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<String>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: DownloadImageWithFallbackInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<String>(format!("Invalid input: {}", e)).into_response(),
    };

    tracing::info!("rspc_download_image_with_fallback: game='{}', platform='{}', type='{}', db_id={:?}",
        input.game_title, input.platform, input.image_type, input.launchbox_db_id);

    let state_guard = state.read().await;

    let games_pool = match state_guard.games_db_pool.as_ref() {
        Some(p) => p,
        None => return rspc_err::<String>("Games database not initialized".to_string()).into_response(),
    };

    // Look up platform info to get launchbox_name and libretro_name
    let platform_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_name, libretro_name FROM platforms WHERE name = ?"
    )
    .bind(&input.platform)
    .fetch_optional(games_pool)
    .await
    .ok()
    .flatten();

    let (launchbox_platform, libretro_platform) = platform_info
        .map(|(lb, lr)| (lb, lr))
        .unwrap_or((None, None));

    // Look up libretro_title if we have a launchbox_db_id
    let libretro_title: Option<String> = if let Some(db_id) = input.launchbox_db_id {
        sqlx::query_scalar("SELECT libretro_title FROM games WHERE launchbox_db_id = ?")
            .bind(db_id)
            .fetch_optional(games_pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let cache_dir = crate::commands::get_cache_dir(&state_guard.settings);
    let mut service = crate::images::ImageService::new(games_pool.clone(), cache_dir.clone());
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

    match service.download_with_fallback(
        &input.game_title,
        &input.platform,
        &input.image_type,
        input.launchbox_db_id,
        launchbox_platform.as_deref(),
        libretro_platform.as_deref(),
        libretro_title.as_deref(),
        steamgriddb_client.as_ref(),
        igdb_client.as_ref(),
        emumovies_client.as_ref(),
        screenscraper_client.as_ref(),
    ).await {
        Ok(path) => {
            tracing::info!("  Download succeeded: {}", path);
            rspc_ok(path).into_response()
        }
        Err(e) => {
            tracing::warn!("  Download failed: {}", e);
            rspc_err::<String>(e.to_string()).into_response()
        }
    }
}

// ============================================================================
// Asset Serving (for browser dev mode)
// ============================================================================

async fn serve_asset(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, String)> {
    use axum::response::Response;

    // The path comes in URL-encoded, decode it
    let decoded_path = urlencoding::decode(&path)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid path encoding: {}", e)))?;

    // Axum's wildcard strips the leading slash, so we need to add it back for absolute paths
    let full_path = if decoded_path.starts_with('/') {
        decoded_path.to_string()
    } else {
        format!("/{}", decoded_path)
    };

    let file_path = std::path::Path::new(&full_path);

    // Check that the path exists and is a file
    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("File not found: {}", decoded_path)));
    }

    if !file_path.is_file() {
        return Err((StatusCode::BAD_REQUEST, "Not a file".to_string()));
    }

    // Determine content type based on extension
    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    };

    // Read the file
    let data = tokio::fs::read(&file_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read file: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=31536000") // Cache for 1 year
        .body(axum::body::Body::from(data))
        .unwrap())
}

// ============================================================================
// Video Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckCachedVideoInput {
    game_title: String,
    platform: String,
    launchbox_db_id: Option<i64>,
}

async fn rspc_check_cached_video(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the input parameter (JSON-encoded)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Option<String>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: CheckCachedVideoInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<Option<String>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    let cache_dir = crate::commands::get_cache_dir(&state_guard.settings);

    // Build the expected video path
    let game_id = match input.launchbox_db_id {
        Some(id) => crate::images::GameMediaId::from_launchbox_id(id),
        None => {
            // Fall back to computing hash from platform and title
            let games_pool = match state_guard.games_db_pool.as_ref() {
                Some(p) => p,
                None => return rspc_err::<Option<String>>("Games database not initialized".to_string()).into_response(),
            };

            // Get platform_id
            let platform_id: Option<(i64,)> = match sqlx::query_as(
                "SELECT id FROM platforms WHERE name = ?"
            )
            .bind(&input.platform)
            .fetch_optional(games_pool)
            .await {
                Ok(r) => r,
                Err(e) => return rspc_err::<Option<String>>(e.to_string()).into_response(),
            };

            let platform_id = platform_id.map(|(id,)| id).unwrap_or(0);
            crate::images::GameMediaId::compute_hash(platform_id, &input.game_title)
        }
    };

    let video_path = cache_dir
        .join("media")
        .join(game_id.directory_name())
        .join("emumovies")
        .join("video.mp4");

    if video_path.exists() {
        rspc_ok(Some(video_path.to_string_lossy().to_string())).into_response()
    } else {
        rspc_ok::<Option<String>>(None).into_response()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadGameVideoInput {
    game_title: String,
    platform: String,
    launchbox_db_id: Option<i64>,
}

async fn rspc_download_game_video(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the input parameter (JSON-encoded)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<String>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: DownloadGameVideoInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<String>(format!("Invalid input: {}", e)).into_response(),
    };

    tracing::info!("rspc_download_game_video: game='{}', platform='{}', db_id={:?}",
        input.game_title, input.platform, input.launchbox_db_id);

    let state_guard = state.read().await;

    // Check if EmuMovies is configured
    if state_guard.settings.emumovies.username.is_empty()
        || state_guard.settings.emumovies.password.is_empty()
    {
        return rspc_err::<String>("EmuMovies credentials not configured. Configure them in Settings.".to_string()).into_response();
    }

    let cache_dir = crate::commands::get_cache_dir(&state_guard.settings);

    // Build the game cache directory
    let game_id = match input.launchbox_db_id {
        Some(id) => crate::images::GameMediaId::from_launchbox_id(id),
        None => {
            // Fall back to computing hash from platform and title
            let games_pool = match state_guard.games_db_pool.as_ref() {
                Some(p) => p,
                None => return rspc_err::<String>("Games database not initialized".to_string()).into_response(),
            };

            // Get platform_id
            let platform_id: Option<(i64,)> = match sqlx::query_as(
                "SELECT id FROM platforms WHERE name = ?"
            )
            .bind(&input.platform)
            .fetch_optional(games_pool)
            .await {
                Ok(r) => r,
                Err(e) => return rspc_err::<String>(e.to_string()).into_response(),
            };

            let platform_id = platform_id.map(|(id,)| id).unwrap_or(0);
            crate::images::GameMediaId::compute_hash(platform_id, &input.game_title)
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
    match client.get_video(&input.platform, &input.game_title, &game_cache_dir, None) {
        Ok(video_path) => {
            tracing::info!("  Video download succeeded: {}", video_path.display());
            rspc_ok(video_path.to_string_lossy().to_string()).into_response()
        }
        Err(e) => {
            tracing::warn!("  Video download failed: {}", e);
            rspc_err::<String>(e.to_string()).into_response()
        }
    }
}

// ============================================================================
// Emulator Handlers
// ============================================================================

use crate::db::schema::EmulatorInfo;
use crate::emulator::{EmulatorWithStatus, LaunchResult};

async fn rspc_get_emulators_for_platform(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the input parameter (JSON-encoded platform name string)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Vec<EmulatorInfo>>("Missing 'input' parameter".to_string()).into_response(),
    };

    // Input is a JSON-encoded string (e.g., "\"Nintendo Entertainment System\"")
    let platform_name: String = match serde_json::from_str(input_str) {
        Ok(s) => s,
        Err(e) => return rspc_err::<Vec<EmulatorInfo>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::get_emulators_for_platform(&state_guard, &platform_name).await {
        Ok(emulators) => rspc_ok(emulators).into_response(),
        Err(e) => rspc_err::<Vec<EmulatorInfo>>(e).into_response(),
    }
}

async fn rspc_get_emulator(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the input parameter (JSON-encoded emulator name string)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Option<EmulatorInfo>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let name: String = match serde_json::from_str(input_str) {
        Ok(s) => s,
        Err(e) => return rspc_err::<Option<EmulatorInfo>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::get_emulator(&state_guard, &name).await {
        Ok(emulator) => rspc_ok(emulator).into_response(),
        Err(e) => rspc_err::<Option<EmulatorInfo>>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetAllEmulatorsInput {
    #[serde(default = "default_true")]
    filter_os: bool,
}

fn default_true() -> bool {
    true
}

async fn rspc_get_all_emulators(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the optional input parameter
    let filter_os = if let Some(input_str) = params.get("input") {
        let input: GetAllEmulatorsInput = match serde_json::from_str(input_str) {
            Ok(i) => i,
            Err(_) => GetAllEmulatorsInput { filter_os: true },
        };
        input.filter_os
    } else {
        true // Default to filtering by OS
    };

    let state_guard = state.read().await;
    match handlers::get_all_emulators(&state_guard, filter_os).await {
        Ok(emulators) => rspc_ok(emulators).into_response(),
        Err(e) => rspc_err::<Vec<EmulatorInfo>>(e).into_response(),
    }
}

// ============================================================================
// Play Session Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
struct RecordPlaySessionInput {
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
}

async fn rspc_record_play_session(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Parse the input parameter (JSON-encoded)
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<()>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: RecordPlaySessionInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<()>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;

    let pool = match state_guard.db_pool.as_ref() {
        Some(p) => p,
        None => return rspc_err::<()>("User database not initialized".to_string()).into_response(),
    };

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Insert or update play stats
    match sqlx::query(
        r#"
        INSERT INTO play_stats (launchbox_db_id, game_title, platform, play_count, last_played, first_played)
        VALUES (?, ?, ?, 1, ?, ?)
        ON CONFLICT(launchbox_db_id) DO UPDATE SET
            play_count = play_count + 1,
            last_played = excluded.last_played
        "#
    )
    .bind(input.launchbox_db_id)
    .bind(&input.game_title)
    .bind(&input.platform)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await {
        Ok(_) => {
            tracing::info!("Recorded play session for: {} ({})", input.game_title, input.launchbox_db_id);
            rspc_ok(()).into_response()
        }
        Err(e) => rspc_err::<()>(e.to_string()).into_response(),
    }
}

// ============================================================================
// Emulator Preference Handlers
// ============================================================================

use crate::handlers::EmulatorPreferences;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetEmulatorPreferenceInput {
    launchbox_db_id: i64,
    platform_name: String,
}

async fn rspc_get_emulator_preference(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Option<String>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: GetEmulatorPreferenceInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<Option<String>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::get_emulator_preference(&state_guard, input.launchbox_db_id, &input.platform_name).await {
        Ok(pref) => rspc_ok(pref).into_response(),
        Err(e) => rspc_err::<Option<String>>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetGameEmulatorPreferenceInput {
    launchbox_db_id: i64,
    emulator_name: String,
}

async fn rspc_set_game_emulator_preference(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<()>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: SetGameEmulatorPreferenceInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<()>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::set_game_emulator_preference(&state_guard, input.launchbox_db_id, &input.emulator_name).await {
        Ok(()) => rspc_ok(()).into_response(),
        Err(e) => rspc_err::<()>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetPlatformEmulatorPreferenceInput {
    platform_name: String,
    emulator_name: String,
}

async fn rspc_set_platform_emulator_preference(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<()>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: SetPlatformEmulatorPreferenceInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<()>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::set_platform_emulator_preference(&state_guard, &input.platform_name, &input.emulator_name).await {
        Ok(()) => rspc_ok(()).into_response(),
        Err(e) => rspc_err::<()>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClearGameEmulatorPreferenceInput {
    launchbox_db_id: i64,
}

async fn rspc_clear_game_emulator_preference(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<()>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: ClearGameEmulatorPreferenceInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<()>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::clear_game_emulator_preference(&state_guard, input.launchbox_db_id).await {
        Ok(()) => rspc_ok(()).into_response(),
        Err(e) => rspc_err::<()>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClearPlatformEmulatorPreferenceInput {
    platform_name: String,
}

async fn rspc_clear_platform_emulator_preference(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<()>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: ClearPlatformEmulatorPreferenceInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<()>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::clear_platform_emulator_preference(&state_guard, &input.platform_name).await {
        Ok(()) => rspc_ok(()).into_response(),
        Err(e) => rspc_err::<()>(e).into_response(),
    }
}

async fn rspc_get_all_emulator_preferences(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    match handlers::get_all_emulator_preferences(&state_guard).await {
        Ok(prefs) => rspc_ok(prefs).into_response(),
        Err(e) => rspc_err::<EmulatorPreferences>(e).into_response(),
    }
}

async fn rspc_clear_all_emulator_preferences(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    match handlers::clear_all_emulator_preferences(&state_guard).await {
        Ok(()) => rspc_ok(()).into_response(),
        Err(e) => rspc_err::<()>(e).into_response(),
    }
}

// ============================================================================
// Emulator Installation & Launch Handlers
// ============================================================================

async fn rspc_get_emulators_with_status(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<Vec<EmulatorWithStatus>>("Missing 'input' parameter".to_string()).into_response(),
    };

    let platform_name: String = match serde_json::from_str(input_str) {
        Ok(s) => s,
        Err(e) => return rspc_err::<Vec<EmulatorWithStatus>>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;
    match handlers::get_emulators_with_status(&state_guard, &platform_name).await {
        Ok(emulators) => rspc_ok(emulators).into_response(),
        Err(e) => rspc_err::<Vec<EmulatorWithStatus>>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallEmulatorInput {
    emulator_name: String,
}

async fn rspc_install_emulator(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<String>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: InstallEmulatorInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<String>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;

    // Look up the emulator by name
    let emulator = match handlers::get_emulator(&state_guard, &input.emulator_name).await {
        Ok(Some(e)) => e,
        Ok(None) => return rspc_err::<String>(format!("Emulator '{}' not found", input.emulator_name)).into_response(),
        Err(e) => return rspc_err::<String>(e).into_response(),
    };

    match handlers::install_emulator(&emulator).await {
        Ok(path) => rspc_ok(path).into_response(),
        Err(e) => rspc_err::<String>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchEmulatorInput {
    emulator_name: String,
}

async fn rspc_launch_emulator(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<LaunchResult>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: LaunchEmulatorInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<LaunchResult>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;

    // Look up the emulator by name
    let emulator = match handlers::get_emulator(&state_guard, &input.emulator_name).await {
        Ok(Some(e)) => e,
        Ok(None) => return rspc_err::<LaunchResult>(format!("Emulator '{}' not found", input.emulator_name)).into_response(),
        Err(e) => return rspc_err::<LaunchResult>(e).into_response(),
    };

    match handlers::launch_emulator_only(&emulator) {
        Ok(result) => rspc_ok(result).into_response(),
        Err(e) => rspc_err::<LaunchResult>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchGameInput {
    emulator_name: String,
    rom_path: String,
}

async fn rspc_launch_game(
    State(state): State<SharedState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let input_str = match params.get("input") {
        Some(s) => s,
        None => return rspc_err::<LaunchResult>("Missing 'input' parameter".to_string()).into_response(),
    };

    let input: LaunchGameInput = match serde_json::from_str(input_str) {
        Ok(i) => i,
        Err(e) => return rspc_err::<LaunchResult>(format!("Invalid input: {}", e)).into_response(),
    };

    let state_guard = state.read().await;

    // Look up the emulator by name
    let emulator = match handlers::get_emulator(&state_guard, &input.emulator_name).await {
        Ok(Some(e)) => e,
        Ok(None) => return rspc_err::<LaunchResult>(format!("Emulator '{}' not found", input.emulator_name)).into_response(),
        Err(e) => return rspc_err::<LaunchResult>(e).into_response(),
    };

    match handlers::launch_game_with_emulator(&emulator, &input.rom_path) {
        Ok(result) => rspc_ok(result).into_response(),
        Err(e) => rspc_err::<LaunchResult>(e).into_response(),
    }
}

async fn rspc_get_current_os() -> impl IntoResponse {
    rspc_ok(handlers::get_current_os())
}
