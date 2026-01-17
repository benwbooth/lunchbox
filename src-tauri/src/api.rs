//! HTTP API for development mode
//!
//! Provides HTTP endpoints that mirror the Tauri commands, allowing
//! the frontend to work in a regular browser during development.

use crate::db::schema::{
    extract_region_from_title, normalize_title_for_dedup, normalize_title_for_display,
    Game, GameVariant, Platform,
};
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
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
        .route("/api/platforms", get(get_platforms))
        .route("/api/games", get(get_games))
        .route("/api/games/count", get(get_game_count))
        .route("/api/games/:uuid", get(get_game_by_uuid))
        .route("/api/games/:uuid/variants", get(get_game_variants))
        .route("/api/settings", get(get_settings))
        .route("/api/stats/:db_id", get(get_play_stats))
        .route("/api/favorites", get(get_favorites))
        .route("/api/favorites/check/:db_id", get(check_is_favorite))
        .route("/api/favorites/:game_id", post(add_favorite).delete(remove_favorite))
        .layer(cors)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

// ============================================================================
// Platforms
// ============================================================================

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
            result.push(Platform { id, name, game_count: seen.len() as i64, aliases });
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
                        SELECT g.id, g.title, g.platform_id, p.name as platform,
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
                        SELECT g.id, g.title, g.platform_id, p.name as platform,
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
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        } else {
            sqlx::query(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform,
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

            let normalized = normalize_title_for_dedup(&title);
            let key = format!("{}:{}", platform_id, normalized);

            *variant_counts.entry(key.clone()).or_insert(0) += 1;

            if !seen.contains_key(&key) {
                let game = Game {
                    id,
                    database_id: 0,
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
            SELECT g.id, g.title, g.platform_id, p.name as platform,
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
                database_id: 0,
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

            let result: Vec<GameVariant> = variants
                .into_iter()
                .filter(|(_, t)| normalize_title_for_dedup(t) == normalized)
                .map(|(id, title)| GameVariant {
                    id,
                    region: extract_region_from_title(&title),
                    title,
                })
                .collect();

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

