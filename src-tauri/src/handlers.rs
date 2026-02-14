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
use crate::emulator::{self, EmulatorWithStatus, LaunchResult};
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

// ============================================================================
// Emulator Preference Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEmulatorPref {
    pub launchbox_db_id: i64,
    pub emulator_name: String,
    pub game_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformEmulatorPref {
    pub platform_name: String,
    pub emulator_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorPreferences {
    pub game_preferences: Vec<GameEmulatorPref>,
    pub platform_preferences: Vec<PlatformEmulatorPref>,
}

// ============================================================================
// Emulator Preference Handlers
// ============================================================================

/// Get emulator preference for a game (checks game-specific, then platform)
pub async fn get_emulator_preference(
    state: &AppState,
    launchbox_db_id: i64,
    platform_name: &str,
) -> Result<Option<String>, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // First check for game-specific preference
    let game_pref: Option<(String,)> = sqlx::query_as(
        "SELECT emulator_name FROM emulator_preferences WHERE launchbox_db_id = ?"
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some((emulator_name,)) = game_pref {
        return Ok(Some(emulator_name));
    }

    // Fall back to platform preference
    let platform_pref: Option<(String,)> = sqlx::query_as(
        "SELECT emulator_name FROM emulator_preferences WHERE platform_name = ?"
    )
    .bind(platform_name)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(platform_pref.map(|(name,)| name))
}

/// Set emulator preference for a specific game
pub async fn set_game_emulator_preference(
    state: &AppState,
    launchbox_db_id: i64,
    emulator_name: &str,
) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query(
        r#"
        INSERT INTO emulator_preferences (launchbox_db_id, emulator_name, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(launchbox_db_id) DO UPDATE SET
            emulator_name = excluded.emulator_name,
            updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(launchbox_db_id)
    .bind(emulator_name)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Set emulator preference for a platform (all games on that platform)
pub async fn set_platform_emulator_preference(
    state: &AppState,
    platform_name: &str,
    emulator_name: &str,
) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query(
        r#"
        INSERT INTO emulator_preferences (platform_name, emulator_name, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(platform_name) DO UPDATE SET
            emulator_name = excluded.emulator_name,
            updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(platform_name)
    .bind(emulator_name)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Clear a game-specific preference
pub async fn clear_game_emulator_preference(
    state: &AppState,
    launchbox_db_id: i64,
) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM emulator_preferences WHERE launchbox_db_id = ?")
        .bind(launchbox_db_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Clear a platform preference
pub async fn clear_platform_emulator_preference(
    state: &AppState,
    platform_name: &str,
) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM emulator_preferences WHERE platform_name = ?")
        .bind(platform_name)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all emulator preferences (for settings UI)
pub async fn get_all_emulator_preferences(
    state: &AppState,
) -> Result<EmulatorPreferences, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get game preferences
    let game_prefs: Vec<(i64, String)> = sqlx::query_as(
        "SELECT launchbox_db_id, emulator_name FROM emulator_preferences WHERE launchbox_db_id IS NOT NULL ORDER BY updated_at DESC"
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    // Look up game titles from games database
    let mut game_preferences = Vec::new();
    if let Some(games_pool) = state.games_db_pool.as_ref() {
        for (db_id, emulator_name) in game_prefs {
            let title: Option<(String,)> = sqlx::query_as(
                "SELECT title FROM games WHERE launchbox_db_id = ? LIMIT 1"
            )
            .bind(db_id)
            .fetch_optional(games_pool)
            .await
            .ok()
            .flatten();

            game_preferences.push(GameEmulatorPref {
                launchbox_db_id: db_id,
                emulator_name,
                game_title: title.map(|(t,)| t),
            });
        }
    }

    // Get platform preferences
    let platform_prefs: Vec<(String, String)> = sqlx::query_as(
        "SELECT platform_name, emulator_name FROM emulator_preferences WHERE platform_name IS NOT NULL ORDER BY platform_name"
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let platform_preferences = platform_prefs
        .into_iter()
        .map(|(platform_name, emulator_name)| PlatformEmulatorPref {
            platform_name,
            emulator_name,
        })
        .collect();

    Ok(EmulatorPreferences {
        game_preferences,
        platform_preferences,
    })
}

/// Clear all emulator preferences
pub async fn clear_all_emulator_preferences(
    state: &AppState,
) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM emulator_preferences")
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Emulator Installation & Launch Handlers
// ============================================================================

/// Get all emulators for a platform with installation status
pub async fn get_emulators_with_status(
    state: &AppState,
    platform_name: &str,
) -> Result<Vec<EmulatorWithStatus>, String> {
    let pool = state.emulators_db_pool.as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let os = current_os();

    // Query emulators for this platform, filtered by OS
    // We get all emulators that have either a RetroArch core OR a standalone installer
    let emulators: Vec<EmulatorInfo> = sqlx::query_as(
        r#"
        SELECT e.id, e.name, e.homepage, e.supported_os, e.winget_id,
               e.homebrew_formula, e.flatpak_id, e.retroarch_core,
               e.save_directory, e.save_extensions, e.notes
        FROM emulators e
        JOIN platform_emulators pe ON e.id = pe.emulator_id
        WHERE pe.platform_name = ?
          AND (e.supported_os IS NULL OR e.supported_os LIKE '%' || ? || '%')
          AND (
              e.retroarch_core IS NOT NULL
              OR (? = 'Linux' AND e.flatpak_id IS NOT NULL)
              OR (? = 'Windows' AND e.winget_id IS NOT NULL)
              OR (? = 'macOS' AND e.homebrew_formula IS NOT NULL)
          )
        ORDER BY
            pe.is_recommended DESC,
            e.name
        "#,
    )
    .bind(platform_name)
    .bind(os)
    .bind(os)
    .bind(os)
    .bind(os)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Create separate entries for RetroArch cores and standalone emulators
    // An emulator with both will appear twice in the list
    let mut results: Vec<EmulatorWithStatus> = Vec::new();
    let mut retroarch_entries: Vec<EmulatorWithStatus> = Vec::new();
    let mut standalone_entries: Vec<EmulatorWithStatus> = Vec::new();

    for emulator in emulators {
        let has_retroarch = emulator.retroarch_core.is_some();
        let has_standalone = match os {
            "Linux" => emulator.flatpak_id.is_some(),
            "Windows" => emulator.winget_id.is_some(),
            "macOS" => emulator.homebrew_formula.is_some(),
            _ => false,
        };

        // Add RetroArch core entry if available
        if has_retroarch {
            retroarch_entries.push(emulator::add_status_as_retroarch(emulator.clone()));
        }

        // Add standalone entry if available
        if has_standalone {
            standalone_entries.push(emulator::add_status_as_standalone(emulator));
        }
    }

    // RetroArch cores first, then standalone emulators
    results.extend(retroarch_entries);
    results.extend(standalone_entries);

    Ok(results)
}

/// Check if a specific emulator is installed
pub fn check_emulator_installed(emulator: &EmulatorInfo) -> bool {
    emulator::check_installation(emulator).is_some()
}

/// Install an emulator
/// If `is_retroarch_core` is true, install as RetroArch core; otherwise install standalone
pub async fn install_emulator(emulator: &EmulatorInfo, is_retroarch_core: bool) -> Result<String, String> {
    let path = emulator::install_emulator(emulator, is_retroarch_core).await?;
    Ok(path.to_string_lossy().to_string())
}

/// Launch a game with the specified emulator
pub fn launch_game_with_emulator(
    emulator: &EmulatorInfo,
    rom_path: &str,
) -> Result<LaunchResult, String> {
    match emulator::launch_game(emulator, rom_path) {
        Ok(pid) => Ok(LaunchResult {
            success: true,
            pid: Some(pid),
            error: None,
        }),
        Err(e) => Ok(LaunchResult {
            success: false,
            pid: None,
            error: Some(e),
        }),
    }
}

/// Launch an emulator (without a ROM)
/// If `is_retroarch_core` is true, launch via RetroArch; otherwise launch standalone
pub fn launch_emulator_only(
    emulator: &EmulatorInfo,
    is_retroarch_core: bool,
) -> Result<LaunchResult, String> {
    match emulator::launch_emulator(emulator, None, is_retroarch_core) {
        Ok(pid) => Ok(LaunchResult {
            success: true,
            pid: Some(pid),
            error: None,
        }),
        Err(e) => Ok(LaunchResult {
            success: false,
            pid: None,
            error: Some(e),
        }),
    }
}

/// Get the current operating system name
pub fn get_current_os() -> String {
    emulator::current_os().to_string()
}

// ============================================================================
// Graboid Import Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFile {
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub imported_at: String,
    pub import_source: String,
    pub graboid_job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportJob {
    pub id: String,
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub status: String,
    pub progress_percent: f64,
    pub status_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartImportInput {
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraboidPrompt {
    pub id: i64,
    pub scope: String,
    pub platform: Option<String>,
    pub launchbox_db_id: Option<i64>,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGraboidPromptInput {
    pub scope: String,
    pub platform: Option<String>,
    pub launchbox_db_id: Option<i64>,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGraboidPromptInput {
    pub scope: String,
    pub platform: Option<String>,
    pub launchbox_db_id: Option<i64>,
}

// ============================================================================
// Graboid Import Handlers
// ============================================================================

/// Check if a game has an imported file
pub async fn get_game_file(state: &AppState, launchbox_db_id: i64) -> Result<Option<GameFile>, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let row: Option<(i64, String, String, String, Option<i64>, String, String, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform, file_path, file_size, imported_at, import_source, graboid_job_id FROM game_files WHERE launchbox_db_id = ?"
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|(db_id, game_title, platform, file_path, file_size, imported_at, import_source, graboid_job_id)| GameFile {
        launchbox_db_id: db_id,
        game_title,
        platform,
        file_path,
        file_size,
        imported_at,
        import_source,
        graboid_job_id,
    }))
}

/// Get an active (pending/in_progress) import job for a game
pub async fn get_active_import(state: &AppState, launchbox_db_id: i64) -> Result<Option<ImportJob>, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let row: Option<(String, i64, String, String, String, f64, Option<String>, String, String)> = sqlx::query_as(
        "SELECT id, launchbox_db_id, game_title, platform, status, progress_percent, status_message, created_at, updated_at FROM graboid_jobs WHERE launchbox_db_id = ? AND status IN ('pending', 'in_progress') ORDER BY created_at DESC LIMIT 1"
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|(id, db_id, game_title, platform, status, progress_percent, status_message, created_at, updated_at)| ImportJob {
        id,
        launchbox_db_id: db_id,
        game_title,
        platform,
        status,
        progress_percent,
        status_message,
        created_at,
        updated_at,
    }))
}

/// Start a Graboid import job
pub async fn start_graboid_import(state: &AppState, input: StartImportInput) -> Result<ImportJob, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let graboid = &state.settings.graboid;
    if graboid.server_url.is_empty() {
        return Err("Graboid server URL not configured".to_string());
    }
    if graboid.api_key.is_empty() {
        return Err("Graboid API key not configured".to_string());
    }

    // Build the destination path
    let import_dir = state.settings.get_import_directory();
    let platform_dir = input.platform.replace("/", "-").replace(":", "-");
    let destination = import_dir.join(&platform_dir);

    // Query game metadata from games database (region, checksums, etc.)
    let mut metadata = serde_json::json!({
        "game_title": input.game_title,
        "platform": input.platform,
        "launchbox_db_id": input.launchbox_db_id,
    });
    if let Some(ref games_pool) = state.games_db_pool {
        let row: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT region, libretro_crc32, libretro_md5, libretro_sha1, libretro_serial FROM games WHERE launchbox_db_id = ?"
            )
            .bind(input.launchbox_db_id)
            .fetch_optional(games_pool)
            .await
            .unwrap_or(None);

        if let Some((region, crc32, md5, sha1, serial)) = row {
            if let Some(r) = region { metadata["region"] = serde_json::Value::String(r); }
            if let Some(v) = crc32 { metadata["crc32"] = serde_json::Value::String(v); }
            if let Some(v) = md5 { metadata["md5"] = serde_json::Value::String(v); }
            if let Some(v) = sha1 { metadata["sha1"] = serde_json::Value::String(v); }
            if let Some(v) = serial { metadata["serial"] = serde_json::Value::String(v); }
        }
    }
    let mut prompt_parts: Vec<String> = vec![metadata.to_string()];

    // Global default prompt (user-configured instructions)
    if !graboid.default_prompt.is_empty() {
        prompt_parts.push(graboid.default_prompt.clone());
    }

    // Platform-specific prompt addition
    let platform_prompt: Option<(String,)> = sqlx::query_as(
        "SELECT prompt FROM graboid_prompts WHERE scope = 'platform' AND platform = ?"
    )
    .bind(&input.platform)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;
    if let Some((prompt,)) = platform_prompt {
        prompt_parts.push(prompt);
    }

    // Game-specific prompt addition
    let game_prompt: Option<(String,)> = sqlx::query_as(
        "SELECT prompt FROM graboid_prompts WHERE scope = 'game' AND launchbox_db_id = ?"
    )
    .bind(input.launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;
    if let Some((prompt,)) = game_prompt {
        prompt_parts.push(prompt);
    }

    let combined_prompt = prompt_parts.join("\n");

    // POST to Graboid API
    let client = reqwest::Client::new();
    let graboid_url = format!("{}/api/v1/jobs", graboid.server_url.trim_end_matches('/'));

    let dest_str = destination.to_string_lossy().to_string();
    let import_dir_str = import_dir.to_string_lossy().to_string();

    let body = serde_json::json!({
        "prompt": combined_prompt,
        "destination_path": dest_str,
        "file_operation": "copy",
        "local_write_whitelist": [&import_dir_str],
        "local_read_whitelist": [&import_dir_str],
        "metadata": {
            "game_title": input.game_title,
            "platform": input.platform,
            "launchbox_db_id": input.launchbox_db_id,
        }
    });

    let response = client
        .post(&graboid_url)
        .header("X-API-Key", &graboid.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Graboid: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Graboid API error ({}): {}", status, text));
    }

    let job_response: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse Graboid response: {}", e))?;

    let job_id = job_response["id"].as_str()
        .ok_or_else(|| "Graboid response missing job ID".to_string())?
        .to_string();

    // Insert into local database
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO graboid_jobs (id, launchbox_db_id, game_title, platform, status, progress_percent, created_at, updated_at) VALUES (?, ?, ?, ?, 'pending', 0, ?, ?)"
    )
    .bind(&job_id)
    .bind(input.launchbox_db_id)
    .bind(&input.game_title)
    .bind(&input.platform)
    .bind(&now)
    .bind(&now)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(ImportJob {
        id: job_id,
        launchbox_db_id: input.launchbox_db_id,
        game_title: input.game_title,
        platform: input.platform,
        status: "pending".to_string(),
        progress_percent: 0.0,
        status_message: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Complete an import job - record the downloaded file
pub async fn complete_import(state: &AppState, job_id: &str, file_path: &str, file_size: Option<i64>) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Get job info
    let job: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform FROM graboid_jobs WHERE id = ?"
    )
    .bind(job_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let (launchbox_db_id, game_title, platform) = job
        .ok_or_else(|| format!("Job {} not found", job_id))?;

    // Update job status
    sqlx::query(
        "UPDATE graboid_jobs SET status = 'completed', progress_percent = 100, updated_at = ? WHERE id = ?"
    )
    .bind(&now)
    .bind(job_id)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    // Insert into game_files
    sqlx::query(
        "INSERT OR REPLACE INTO game_files (launchbox_db_id, game_title, platform, file_path, file_size, imported_at, import_source, graboid_job_id) VALUES (?, ?, ?, ?, ?, ?, 'graboid', ?)"
    )
    .bind(launchbox_db_id)
    .bind(&game_title)
    .bind(&platform)
    .bind(file_path)
    .bind(file_size)
    .bind(&now)
    .bind(job_id)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Mark an import job as failed
pub async fn fail_import(state: &AppState, job_id: &str, error: &str) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "UPDATE graboid_jobs SET status = 'failed', status_message = ?, updated_at = ? WHERE id = ?"
    )
    .bind(error)
    .bind(&now)
    .bind(job_id)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Cancel an import job
pub async fn cancel_import(state: &AppState, job_id: &str) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let graboid = &state.settings.graboid;

    // Send DELETE to Graboid API to cancel the job
    if !graboid.server_url.is_empty() && !graboid.api_key.is_empty() {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/jobs/{}", graboid.server_url.trim_end_matches('/'), job_id);
        let _ = client
            .delete(&url)
            .header("X-API-Key", &graboid.api_key)
            .send()
            .await;
    }

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "UPDATE graboid_jobs SET status = 'cancelled', updated_at = ? WHERE id = ?"
    )
    .bind(&now)
    .bind(job_id)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Test connection to Graboid server
pub async fn test_graboid_connection(server_url: &str, api_key: &str) -> crate::router::ConnectionTestResult {
    if server_url.is_empty() {
        return crate::router::ConnectionTestResult {
            success: false,
            message: "Server URL is empty".to_string(),
            user_info: None,
        };
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let url = format!("{}/health", server_url.trim_end_matches('/'));

    match client
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                crate::router::ConnectionTestResult {
                    success: true,
                    message: "Connected to Graboid successfully".to_string(),
                    user_info: None,
                }
            } else {
                crate::router::ConnectionTestResult {
                    success: false,
                    message: format!("Graboid returned HTTP {}", response.status()),
                    user_info: None,
                }
            }
        }
        Err(e) => crate::router::ConnectionTestResult {
            success: false,
            message: format!("Failed to connect: {}", e),
            user_info: None,
        },
    }
}

/// Update import job progress (called from SSE proxy)
pub async fn update_import_progress(state: &AppState, job_id: &str, progress: f64, message: Option<&str>, status: Option<&str>) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if let Some(new_status) = status {
        sqlx::query(
            "UPDATE graboid_jobs SET status = ?, progress_percent = ?, status_message = ?, updated_at = ? WHERE id = ?"
        )
        .bind(new_status)
        .bind(progress)
        .bind(message)
        .bind(&now)
        .bind(job_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;
    } else {
        sqlx::query(
            "UPDATE graboid_jobs SET progress_percent = ?, status_message = ?, updated_at = ? WHERE id = ?"
        )
        .bind(progress)
        .bind(message)
        .bind(&now)
        .bind(job_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ============================================================================
// Graboid Prompt Handlers
// ============================================================================

/// Get all graboid prompts
pub async fn get_graboid_prompts(state: &AppState) -> Result<Vec<GraboidPrompt>, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let rows: Vec<(i64, String, Option<String>, Option<i64>, String)> = sqlx::query_as(
        "SELECT id, scope, platform, launchbox_db_id, prompt FROM graboid_prompts ORDER BY scope, platform, launchbox_db_id"
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows.into_iter().map(|(id, scope, platform, launchbox_db_id, prompt)| GraboidPrompt {
        id,
        scope,
        platform,
        launchbox_db_id,
        prompt,
    }).collect())
}

/// Get the effective prompt for a specific game (combines global + platform + game)
pub async fn get_effective_graboid_prompt(state: &AppState, platform: &str, launchbox_db_id: i64) -> Result<String, String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut parts: Vec<String> = Vec::new();

    // Global default from settings
    if !state.settings.graboid.default_prompt.is_empty() {
        parts.push(state.settings.graboid.default_prompt.clone());
    }

    // Platform-specific prompt
    let platform_prompt: Option<(String,)> = sqlx::query_as(
        "SELECT prompt FROM graboid_prompts WHERE scope = 'platform' AND platform = ?"
    )
    .bind(platform)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;
    if let Some((prompt,)) = platform_prompt {
        parts.push(prompt);
    }

    // Game-specific prompt
    let game_prompt: Option<(String,)> = sqlx::query_as(
        "SELECT prompt FROM graboid_prompts WHERE scope = 'game' AND launchbox_db_id = ?"
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;
    if let Some((prompt,)) = game_prompt {
        parts.push(prompt);
    }

    Ok(parts.join("\n"))
}

/// Save (upsert) a graboid prompt
pub async fn save_graboid_prompt(state: &AppState, input: SaveGraboidPromptInput) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        r#"
        INSERT INTO graboid_prompts (scope, platform, launchbox_db_id, prompt, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(scope, platform, launchbox_db_id) DO UPDATE SET
            prompt = excluded.prompt,
            updated_at = excluded.updated_at
        "#
    )
    .bind(&input.scope)
    .bind(&input.platform)
    .bind(input.launchbox_db_id)
    .bind(&input.prompt)
    .bind(&now)
    .bind(&now)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a graboid prompt
pub async fn delete_graboid_prompt(state: &AppState, input: DeleteGraboidPromptInput) -> Result<(), String> {
    let db_pool = state.db_pool.as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query(
        "DELETE FROM graboid_prompts WHERE scope = ? AND platform IS ? AND launchbox_db_id IS ?"
    )
    .bind(&input.scope)
    .bind(&input.platform)
    .bind(input.launchbox_db_id)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
