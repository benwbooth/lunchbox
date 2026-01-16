//! HTTP API for development mode
//!
//! Provides HTTP endpoints that mirror the Tauri commands, allowing
//! the frontend to work in a regular browser during development.

use crate::commands::{Game, GameVariant, Platform};
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
        .route("/api/favorites", get(get_favorites))
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
        let platforms: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, name FROM platforms ORDER BY name"
        )
        .fetch_all(games_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut result = Vec::new();
        for (id, name) in platforms {
            let all_titles: Vec<(String,)> = sqlx::query_as(
                "SELECT title FROM games WHERE platform_id = ?"
            )
            .bind(id)
            .fetch_all(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let mut seen: HashSet<String> = HashSet::new();
            for (title,) in all_titles {
                let normalized = normalize_title(&title).to_lowercase();
                seen.insert(normalized);
            }
            result.push(Platform { id, name, game_count: seen.len() as i64 });
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

async fn get_games(
    State(state): State<SharedState>,
    axum::extract::Query(query): axum::extract::Query<GamesQuery>,
) -> Result<Json<Vec<Game>>, (StatusCode, String)> {
    let state_guard = state.read().await;
    let limit = query.limit.map(|l| l as usize);
    let offset = query.offset.unwrap_or(0) as usize;

    if let Some(ref games_pool) = state_guard.games_db_pool {
        let raw_rows = if let Some(ref search_query) = query.search {
            let pattern = format!("%{}%", search_query);
            if let Some(ref platform_name) = query.platform {
                sqlx::query(
                    r#"
                    SELECT g.id, g.title, g.platform_id, p.name as platform,
                           g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                           g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url
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
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            } else {
                sqlx::query(
                    r#"
                    SELECT g.id, g.title, g.platform_id, p.name as platform,
                           g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                           g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url
                    FROM games g
                    JOIN platforms p ON g.platform_id = p.id
                    WHERE g.title LIKE ?
                    ORDER BY g.title
                    "#
                )
                .bind(&pattern)
                .fetch_all(games_pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            }
        } else if let Some(ref platform_name) = query.platform {
            sqlx::query(
                r#"
                SELECT g.id, g.title, g.platform_id, p.name as platform,
                       g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                       g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url
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
                       g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url
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

            let normalized = normalize_title(&title).to_lowercase();
            let key = format!("{}:{}", platform_id, normalized);

            *variant_counts.entry(key.clone()).or_insert(0) += 1;

            if !seen.contains_key(&key) {
                let game = Game {
                    id,
                    database_id: 0,
                    title: title.clone(),
                    display_title: normalize_title(&title),
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
            let pattern = format!("%{}%", search_query);
            if let Some(ref platform_name) = query.platform {
                sqlx::query_as(
                    "SELECT g.title FROM games g JOIN platforms p ON g.platform_id = p.id WHERE p.name = ? AND g.title LIKE ?"
                )
                .bind(platform_name)
                .bind(&pattern)
                .fetch_all(games_pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            } else {
                sqlx::query_as("SELECT title FROM games WHERE title LIKE ?")
                    .bind(&pattern)
                    .fetch_all(games_pool)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
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
            let normalized = normalize_title(&title).to_lowercase();
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
                   g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url
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

            // Count variants
            let normalized = normalize_title(&title);
            let variant_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM games WHERE platform_id = ? AND title LIKE ?"
            )
            .bind(platform_id)
            .bind(format!("{}%", normalized))
            .fetch_one(games_pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let game = Game {
                id: row.get("id"),
                database_id: 0,
                title: title.clone(),
                display_title: normalized,
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
                box_front_path: None,
                screenshot_path: None,
                variant_count: variant_count.0 as i32,
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
            let normalized = normalize_title(&title);

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
                .filter(|(_, t)| normalize_title(t) == normalized)
                .map(|(id, title)| GameVariant {
                    id,
                    region: extract_region(&title),
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
// Favorites
// ============================================================================

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
                       g.players, g.rating, g.rating_count, g.esrb, g.cooperative, g.video_url, g.wikipedia_url
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
                    display_title: normalize_title(&title),
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
// Helpers
// ============================================================================

fn normalize_title(title: &str) -> String {
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

fn extract_region(title: &str) -> Option<String> {
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
