//! Shared endpoint handlers
//!
//! This module contains the actual implementation logic for all endpoints.
//! Both Tauri commands (commands.rs) and HTTP handlers (api.rs) call into
//! these functions, ensuring the logic is defined in exactly one place.
//!
//! To add a new endpoint:
//! 1. Add the handler function here
//! 2. Add wrapper in commands.rs using the define_command! macro
//! 3. Add wrapper in api.rs using the define_http_handler! macro
//! 4. Register in lib.rs invoke_handler and api.rs create_router

use crate::db::schema::EmulatorInfo;
use crate::state::AppState;
use serde::{Deserialize, Serialize};

// ============================================================================
// Shared types (used by both Tauri and HTTP)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub filter_rules: Option<String>,
    pub game_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectionInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionIdInput {
    pub collection_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectionInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionGameInput {
    pub collection_id: String,
    pub game_id: String,
}

// ============================================================================
// Handler implementations
// ============================================================================

pub async fn get_collections(state: &AppState) -> Result<Vec<Collection>, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let collections: Vec<(String, String, Option<String>, i64, Option<String>, i64)> = sqlx::query_as(
        r#"
        SELECT c.id, c.name, c.description, c.is_smart, c.filter_rules, COUNT(cg.game_id) as game_count
        FROM collections c
        LEFT JOIN collection_games cg ON c.id = cg.collection_id
        GROUP BY c.id
        ORDER BY c.name
        "#
    )
    .fetch_all(db_pool)
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

pub async fn create_collection(state: &AppState, input: CreateCollectionInput) -> Result<Collection, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO collections (id, name, description, is_smart, filter_rules) VALUES (?, ?, ?, 0, NULL)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.description)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Collection {
        id,
        name: input.name,
        description: input.description,
        is_smart: false,
        filter_rules: None,
        game_count: 0,
    })
}

pub async fn update_collection(state: &AppState, input: UpdateCollectionInput) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("UPDATE collections SET name = ?, description = ? WHERE id = ?")
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn delete_collection(state: &AppState, input: CollectionIdInput) -> Result<bool, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Delete games from collection first
    sqlx::query("DELETE FROM collection_games WHERE collection_id = ?")
        .bind(&input.collection_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete the collection
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(&input.collection_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(true)
}

pub async fn add_game_to_collection(state: &AppState, input: CollectionGameInput) -> Result<bool, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get max sort order
    let max_order: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(sort_order) FROM collection_games WHERE collection_id = ?"
    )
    .bind(&input.collection_id)
    .fetch_one(db_pool)
    .await
    .unwrap_or(Some(0));

    let next_order = max_order.unwrap_or(0) + 1;

    sqlx::query(
        "INSERT OR IGNORE INTO collection_games (collection_id, game_id, sort_order) VALUES (?, ?, ?)"
    )
    .bind(&input.collection_id)
    .bind(&input.game_id)
    .bind(next_order)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(true)
}

pub async fn remove_game_from_collection(state: &AppState, input: CollectionGameInput) -> Result<bool, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM collection_games WHERE collection_id = ? AND game_id = ?")
        .bind(&input.collection_id)
        .bind(&input.game_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(true)
}

pub async fn get_collection_games(state: &AppState, input: CollectionIdInput) -> Result<Vec<crate::db::schema::Game>, String> {
    use crate::db::schema::{normalize_title_for_display, Game};
    use sqlx::Row;

    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let games_pool = state.games_db_pool.as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    // Get game_ids from the collection_games table
    let game_ids: Vec<(String,)> = sqlx::query_as(
        "SELECT game_id FROM collection_games WHERE collection_id = ? ORDER BY sort_order"
    )
    .bind(&input.collection_id)
    .fetch_all(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut games = Vec::new();
    for (game_id,) in game_ids {
        let row_opt = sqlx::query(
            r#"
            SELECT g.id, g.title, g.platform_id, p.name as platform,
                   g.description, g.release_date, g.release_year, g.developer, g.publisher, g.genre,
                   g.players, g.rating, g.launchbox_db_id
            FROM games g
            JOIN platforms p ON g.platform_id = p.id
            WHERE g.id = ?
            LIMIT 1
            "#
        )
        .bind(&game_id)
        .fetch_optional(games_pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(row) = row_opt {
            let title: String = row.get("title");
            let display_title = normalize_title_for_display(&title);
            games.push(Game {
                id: game_id,
                database_id: row.get("launchbox_db_id"),
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
    }

    Ok(games)
}

// ============================================================================
// Emulator handlers
// ============================================================================

/// Get current OS identifier for filtering emulators
fn current_os() -> &'static str {
    #[cfg(target_os = "windows")]
    { "Windows" }
    #[cfg(target_os = "macos")]
    { "macOS" }
    #[cfg(target_os = "linux")]
    { "Linux" }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { "Unknown" }
}

/// Get all emulators for a platform, filtered by current OS
pub async fn get_emulators_for_platform(
    state: &AppState,
    platform_name: &str,
) -> Result<Vec<EmulatorInfo>, String> {
    let pool = state.emulators_db_pool.as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let os = current_os();

    let emulators: Vec<EmulatorInfo> = sqlx::query_as(
        r#"
        SELECT e.id, e.name, e.homepage, e.supported_os, e.winget_id,
               e.homebrew_formula, e.flatpak_id, e.retroarch_core,
               e.save_directory, e.save_extensions, e.notes
        FROM emulators e
        JOIN platform_emulators pe ON e.id = pe.emulator_id
        WHERE pe.platform_name = ?
          AND (e.supported_os IS NULL OR e.supported_os LIKE '%' || ? || '%')
        ORDER BY pe.is_recommended DESC, e.name
        "#,
    )
    .bind(platform_name)
    .bind(os)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(emulators)
}

/// Get a specific emulator by name
pub async fn get_emulator(
    state: &AppState,
    name: &str,
) -> Result<Option<EmulatorInfo>, String> {
    let pool = state.emulators_db_pool.as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let emulator: Option<EmulatorInfo> = sqlx::query_as(
        r#"
        SELECT id, name, homepage, supported_os, winget_id,
               homebrew_formula, flatpak_id, retroarch_core,
               save_directory, save_extensions, notes
        FROM emulators
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(emulator)
}

/// Get all emulators (optionally filtered by current OS)
pub async fn get_all_emulators(
    state: &AppState,
    filter_os: bool,
) -> Result<Vec<EmulatorInfo>, String> {
    let pool = state.emulators_db_pool.as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let emulators: Vec<EmulatorInfo> = if filter_os {
        let os = current_os();
        sqlx::query_as(
            r#"
            SELECT id, name, homepage, supported_os, winget_id,
                   homebrew_formula, flatpak_id, retroarch_core,
                   save_directory, save_extensions, notes
            FROM emulators
            WHERE supported_os IS NULL OR supported_os LIKE '%' || ? || '%'
            ORDER BY name
            "#,
        )
        .bind(os)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            r#"
            SELECT id, name, homepage, supported_os, winget_id,
                   homebrew_formula, flatpak_id, retroarch_core,
                   save_directory, save_extensions, notes
            FROM emulators
            ORDER BY name
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(emulators)
}
