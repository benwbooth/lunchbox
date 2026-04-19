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
use crate::state::{AppSettings, AppState};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    let db_pool = state
        .db_pool
        .as_ref()
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

    Ok(collections
        .into_iter()
        .map(
            |(id, name, description, is_smart, filter_rules, game_count)| Collection {
                id,
                name,
                description,
                is_smart: is_smart != 0,
                filter_rules,
                game_count,
            },
        )
        .collect())
}

pub async fn create_collection(
    state: &AppState,
    input: CreateCollectionInput,
) -> Result<Collection, String> {
    let db_pool = state
        .db_pool
        .as_ref()
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

pub async fn update_collection(
    state: &AppState,
    input: UpdateCollectionInput,
) -> Result<(), String> {
    let db_pool = state
        .db_pool
        .as_ref()
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
    let db_pool = state
        .db_pool
        .as_ref()
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

pub async fn add_game_to_collection(
    state: &AppState,
    input: CollectionGameInput,
) -> Result<bool, String> {
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get max sort order
    let max_order: Option<i64> =
        sqlx::query_scalar("SELECT MAX(sort_order) FROM collection_games WHERE collection_id = ?")
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

pub async fn remove_game_from_collection(
    state: &AppState,
    input: CollectionGameInput,
) -> Result<bool, String> {
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM collection_games WHERE collection_id = ? AND game_id = ?")
        .bind(&input.collection_id)
        .bind(&input.game_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(true)
}

pub async fn get_collection_games(
    state: &AppState,
    input: CollectionIdInput,
) -> Result<Vec<crate::db::schema::Game>, String> {
    use crate::db::schema::{normalize_title_for_display, Game};
    use sqlx::Row;

    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let games_pool = state
        .games_db_pool
        .as_ref()
        .ok_or_else(|| "Games database not initialized".to_string())?;

    // Get game_ids from the collection_games table
    let game_ids: Vec<(String,)> = sqlx::query_as(
        "SELECT game_id FROM collection_games WHERE collection_id = ? ORDER BY sort_order",
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
            "#,
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
                has_game_file: false,
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
    {
        "Windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "Unknown"
    }
}

/// Get all emulators for a platform, filtered by current OS
pub async fn get_emulators_for_platform(
    state: &AppState,
    platform_name: &str,
) -> Result<Vec<EmulatorInfo>, String> {
    let pool = state
        .emulators_db_pool
        .as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let os = current_os();

    let mut emulators: Vec<EmulatorInfo> = sqlx::query_as(
        r#"
        SELECT e.id, e.name, e.homepage, e.supported_os, e.winget_id,
               e.homebrew_formula, e.flatpak_id, e.retroarch_core,
               e.save_directory, e.save_extensions, e.notes
        FROM emulators e
        JOIN platform_emulators pe ON e.id = pe.emulator_id
        WHERE pe.platform_name = ?
        ORDER BY pe.is_recommended DESC, e.name
        "#,
    )
    .bind(platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    emulators.retain(emulator::is_emulator_visible_on_current_os);

    maybe_append_exodos_scummvm(pool, platform_name, os, false, &mut emulators).await?;

    Ok(emulators)
}

/// Get a specific emulator by name
pub async fn get_emulator(state: &AppState, name: &str) -> Result<Option<EmulatorInfo>, String> {
    let pool = state
        .emulators_db_pool
        .as_ref()
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
    let pool = state
        .emulators_db_pool
        .as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let emulators: Vec<EmulatorInfo> = if filter_os {
        let emulators: Vec<EmulatorInfo> = sqlx::query_as(
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
        .map_err(|e| e.to_string())?;

        emulators
            .into_iter()
            .filter(emulator::is_emulator_visible_on_current_os)
            .collect()
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
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // First check for game-specific preference
    let game_pref: Option<(String,)> =
        sqlx::query_as("SELECT emulator_name FROM emulator_preferences WHERE launchbox_db_id = ?")
            .bind(launchbox_db_id)
            .fetch_optional(db_pool)
            .await
            .map_err(|e| e.to_string())?;

    if let Some((emulator_name,)) = game_pref {
        return Ok(Some(emulator_name));
    }

    // Fall back to platform preference
    let platform_pref: Option<(String,)> =
        sqlx::query_as("SELECT emulator_name FROM emulator_preferences WHERE platform_name = ?")
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
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query(
        r#"
        INSERT INTO emulator_preferences (launchbox_db_id, emulator_name, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(launchbox_db_id) DO UPDATE SET
            emulator_name = excluded.emulator_name,
            updated_at = CURRENT_TIMESTAMP
        "#,
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
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query(
        r#"
        INSERT INTO emulator_preferences (platform_name, emulator_name, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(platform_name) DO UPDATE SET
            emulator_name = excluded.emulator_name,
            updated_at = CURRENT_TIMESTAMP
        "#,
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
    let db_pool = state
        .db_pool
        .as_ref()
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
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    sqlx::query("DELETE FROM emulator_preferences WHERE platform_name = ?")
        .bind(platform_name)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all emulator preferences (for settings UI)
pub async fn get_all_emulator_preferences(state: &AppState) -> Result<EmulatorPreferences, String> {
    let db_pool = state
        .db_pool
        .as_ref()
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
            let title: Option<(String,)> =
                sqlx::query_as("SELECT title FROM games WHERE launchbox_db_id = ? LIMIT 1")
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
pub async fn clear_all_emulator_preferences(state: &AppState) -> Result<(), String> {
    let db_pool = state
        .db_pool
        .as_ref()
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
    let pool = state
        .emulators_db_pool
        .as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

    let os = current_os();

    // Query emulators for this platform, filtered by OS.
    // Standalone runtimes without an auto-install method are still useful if the
    // user installed them manually, so keep them visible and let installation
    // status resolve separately.
    let mut emulators: Vec<EmulatorInfo> = sqlx::query_as(
        r#"
        SELECT e.id, e.name, e.homepage, e.supported_os, e.winget_id,
               e.homebrew_formula, e.flatpak_id, e.retroarch_core,
               e.save_directory, e.save_extensions, e.notes
        FROM emulators e
        JOIN platform_emulators pe ON e.id = pe.emulator_id
        WHERE pe.platform_name = ?
        ORDER BY
            pe.is_recommended DESC,
            e.name
        "#,
    )
    .bind(platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    emulators.retain(emulator::is_emulator_visible_on_current_os);

    maybe_append_exodos_scummvm(pool, platform_name, os, true, &mut emulators).await?;

    // Create separate entries for RetroArch cores and standalone emulators
    // An emulator with both will appear twice in the list
    let mut results: Vec<EmulatorWithStatus> = Vec::new();
    let mut retroarch_entries: Vec<EmulatorWithStatus> = Vec::new();
    let mut standalone_entries: Vec<EmulatorWithStatus> = Vec::new();

    for emulator in emulators {
        let is_exodos_scummvm = platform_name == "MS-DOS" && emulator.name == "ScummVM";
        let has_retroarch = emulator.retroarch_core.is_some() && !is_exodos_scummvm;
        let has_standalone = !is_exodos_scummvm;

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
    sort_emulator_statuses(&mut results);

    if let Some(db_pool) = state.db_pool.as_ref() {
        for emulator in &mut results {
            emulator.firmware_statuses = crate::firmware::get_firmware_status(
                &state.settings,
                db_pool,
                &emulator.info,
                platform_name,
                emulator.is_retroarch_core,
            )
            .await?;
        }
    }

    Ok(results)
}

fn sort_emulator_statuses(results: &mut [EmulatorWithStatus]) {
    results.sort_by(|a, b| b.is_installed.cmp(&a.is_installed));
}

async fn maybe_append_exodos_scummvm(
    pool: &sqlx::SqlitePool,
    platform_name: &str,
    os: &str,
    require_installable: bool,
    emulators: &mut Vec<EmulatorInfo>,
) -> Result<(), String> {
    if platform_name != "MS-DOS" || emulators.iter().any(|emulator| emulator.name == "ScummVM") {
        return Ok(());
    }

    let base_query = if require_installable {
        r#"
        SELECT id, name, homepage, supported_os, winget_id,
               homebrew_formula, flatpak_id, retroarch_core,
               save_directory, save_extensions, notes
        FROM emulators
        WHERE name = 'ScummVM'
          AND (supported_os IS NULL OR supported_os LIKE '%' || ? || '%')
          AND (
              retroarch_core IS NOT NULL
              OR (? = 'Linux' AND flatpak_id IS NOT NULL)
              OR (? = 'Windows' AND winget_id IS NOT NULL)
              OR (? = 'macOS' AND homebrew_formula IS NOT NULL)
          )
        "#
    } else {
        r#"
        SELECT id, name, homepage, supported_os, winget_id,
               homebrew_formula, flatpak_id, retroarch_core,
               save_directory, save_extensions, notes
        FROM emulators
        WHERE name = 'ScummVM'
          AND (supported_os IS NULL OR supported_os LIKE '%' || ? || '%')
        "#
    };

    let mut query = sqlx::query_as::<_, EmulatorInfo>(base_query).bind(os);
    if require_installable {
        query = query.bind(os).bind(os).bind(os);
    }

    if let Some(scummvm) = query
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
    {
        emulators.push(scummvm);
    }

    Ok(())
}

/// Check if a specific emulator is installed
pub fn check_emulator_installed(emulator: &EmulatorInfo) -> bool {
    emulator::check_installation(emulator).is_some()
}

/// Install an emulator
/// If `is_retroarch_core` is true, install as RetroArch core; otherwise install standalone
pub async fn install_emulator(
    emulator: &EmulatorInfo,
    is_retroarch_core: bool,
) -> Result<String, String> {
    let path = emulator::install_emulator(emulator, is_retroarch_core).await?;
    Ok(path.to_string_lossy().to_string())
}

pub async fn uninstall_emulator(
    emulator: &EmulatorInfo,
    is_retroarch_core: bool,
) -> Result<(), String> {
    emulator::uninstall_emulator(emulator, is_retroarch_core).await
}

pub async fn install_firmware_for_emulator(
    state: &mut AppState,
    emulator: &EmulatorInfo,
    platform_name: &str,
    is_retroarch_core: bool,
) -> Result<Vec<crate::firmware::FirmwareStatus>, String> {
    let settings = state.settings.clone();
    let minerva_pool = state.minerva_db_pool.clone();
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?
        .clone();

    install_firmware_for_emulator_with_context(
        &settings,
        &db_pool,
        minerva_pool.as_ref(),
        emulator,
        platform_name,
        is_retroarch_core,
    )
    .await
}

pub async fn install_firmware_for_emulator_with_context(
    settings: &AppSettings,
    db_pool: &sqlx::SqlitePool,
    minerva_pool: Option<&sqlx::SqlitePool>,
    emulator: &EmulatorInfo,
    platform_name: &str,
    is_retroarch_core: bool,
) -> Result<Vec<crate::firmware::FirmwareStatus>, String> {
    crate::firmware::ensure_runtime_firmware(
        settings,
        db_pool,
        minerva_pool,
        emulator,
        platform_name,
        is_retroarch_core,
    )
    .await?;

    crate::firmware::get_firmware_status(
        settings,
        db_pool,
        emulator,
        platform_name,
        is_retroarch_core,
    )
    .await
}

/// Launch a game with the specified emulator
pub async fn launch_game_with_emulator(
    state: &mut AppState,
    emulator: &EmulatorInfo,
    rom_path: Option<&str>,
    launchbox_db_id: Option<i64>,
    platform: Option<&str>,
    is_retroarch_core: Option<bool>,
) -> Result<LaunchResult, String> {
    let as_retroarch_core = is_retroarch_core.unwrap_or(emulator.retroarch_core.is_some());

    if let (Some(db_id), Some(platform)) = (launchbox_db_id, platform) {
        let settings = state.settings.clone();
        let resolved_rom_path = if let Some(path) = rom_path.filter(|path| !path.trim().is_empty())
        {
            path.to_string()
        } else {
            let db_pool = crate::state::ensure_user_db(state)
                .await
                .map_err(|e| e.to_string())?;
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT file_path FROM game_files WHERE launchbox_db_id = ? LIMIT 1",
            )
            .bind(db_id)
            .fetch_optional(db_pool)
            .await
            .map_err(|e| e.to_string())?;

            match row {
                Some((path,)) if !path.trim().is_empty() => path,
                _ => {
                    return Ok(LaunchResult {
                        success: false,
                        pid: None,
                        error: Some(
                            "No downloaded eXo archive is available for this game".to_string(),
                        ),
                    })
                }
            }
        };

        if crate::exo::should_use_prepared_install(platform, Path::new(&resolved_rom_path)) {
            let db_pool = crate::state::ensure_user_db(state)
                .await
                .map_err(|e| e.to_string())?;

            let prepared = match crate::exo::prepare_install_for_game(
                &settings,
                db_pool,
                db_id,
                platform,
                Path::new(&resolved_rom_path),
            )
            .await
            {
                Ok(prepared) => prepared,
                Err(e) => {
                    return Ok(LaunchResult {
                        success: false,
                        pid: None,
                        error: Some(e),
                    })
                }
            };

            return match emulator::launch_prepared_install(
                emulator,
                prepared.collection,
                &prepared.install_root,
                &prepared.launch_config_path,
                as_retroarch_core,
            ) {
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
            };
        }

        let minerva_pool = state.minerva_db_pool.clone();
        let db_pool = crate::state::ensure_user_db(state)
            .await
            .map_err(|e| e.to_string())?;
        if let Err(e) = crate::firmware::ensure_runtime_firmware_for_launch(
            &settings,
            db_pool,
            minerva_pool.as_ref(),
            emulator,
            platform,
            as_retroarch_core,
            Path::new(&resolved_rom_path),
        )
        .await
        {
            return Ok(LaunchResult {
                success: false,
                pid: None,
                error: Some(e),
            });
        }

        let firmware_launch_args = crate::firmware::get_launch_firmware_args(
            db_pool,
            emulator,
            platform,
            as_retroarch_core,
        )
        .await
        .map_err(|e| e.to_string())?;

        match emulator::launch_emulator(
            emulator,
            Some(&resolved_rom_path),
            Some(platform),
            as_retroarch_core,
            &firmware_launch_args,
        ) {
            Ok(pid) => {
                return Ok(LaunchResult {
                    success: true,
                    pid: Some(pid),
                    error: None,
                })
            }
            Err(e) => {
                return Ok(LaunchResult {
                    success: false,
                    pid: None,
                    error: Some(e),
                })
            }
        }
    }

    let Some(rom_path) = rom_path.filter(|path| !path.trim().is_empty()) else {
        return Ok(LaunchResult {
            success: false,
            pid: None,
            error: Some("No ROM file path is available for this game".to_string()),
        });
    };

    match emulator::launch_emulator(emulator, Some(rom_path), None, as_retroarch_core, &[]) {
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
    match emulator::launch_emulator(emulator, None, None, is_retroarch_core, &[]) {
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

/// Cancel an active import job
pub async fn cancel_import(state: &AppState, job_id: &str) -> Result<(), String> {
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query("UPDATE graboid_jobs SET status = 'cancelled', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(job_id)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// REMOVED: GraboidPrompt types and all graboid-specific handlers
// (fetch_graboid_job, reconcile_terminal_import_job, start_graboid_import,
//  complete_import, fail_import, test_graboid_connection, update_import_progress,
//  get_graboid_prompts, get_effective_graboid_prompt, save_graboid_prompt,
//  delete_graboid_prompt)
// ============================================================================

// ============================================================================
// Minerva Archive Types & Handlers
// ============================================================================

// NOTE: GraboidPrompt, SaveGraboidPromptInput, DeleteGraboidPromptInput,
// fetch_graboid_job, reconcile_terminal_import_job, start_graboid_import,
// complete_import, fail_import, old cancel_import, test_graboid_connection,
// update_import_progress, get_graboid_prompts, get_effective_graboid_prompt,
// save_graboid_prompt, delete_graboid_prompt — all removed.

/// Check if a game has an imported file
pub async fn get_game_file(
    state: &AppState,
    launchbox_db_id: i64,
) -> Result<Option<GameFile>, String> {
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let row: Option<(i64, String, String, String, Option<i64>, String, String, Option<String>)> = sqlx::query_as(
        "SELECT launchbox_db_id, game_title, platform, file_path, file_size, imported_at, import_source, graboid_job_id FROM game_files WHERE launchbox_db_id = ?"
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(
        |(
            db_id,
            game_title,
            platform,
            file_path,
            file_size,
            imported_at,
            import_source,
            graboid_job_id,
        )| GameFile {
            launchbox_db_id: db_id,
            game_title,
            platform,
            file_path,
            file_size,
            imported_at,
            import_source,
            graboid_job_id,
        },
    ))
}

pub async fn uninstall_game(state: &mut AppState, launchbox_db_id: i64) -> Result<(), String> {
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;

    let game_file_row: Option<(String, String)> =
        sqlx::query_as("SELECT file_path, import_source FROM game_files WHERE launchbox_db_id = ?")
            .bind(launchbox_db_id)
            .fetch_optional(db_pool)
            .await
            .map_err(|e| e.to_string())?;

    let pc_install_row: Option<(String,)> =
        sqlx::query_as("SELECT install_root FROM pc_game_installs WHERE launchbox_db_id = ?")
            .bind(launchbox_db_id)
            .fetch_optional(db_pool)
            .await
            .map_err(|e| e.to_string())?;

    let mut removed_anything = false;

    if let Some((file_path, import_source)) = game_file_row {
        if import_source == "minerva" {
            remove_path_if_exists(std::path::Path::new(&file_path)).await?;
            sqlx::query("DELETE FROM game_files WHERE launchbox_db_id = ?")
                .bind(launchbox_db_id)
                .execute(db_pool)
                .await
                .map_err(|e| e.to_string())?;
            removed_anything = true;
        } else {
            return Err(
                "This game file was not installed by Lunchbox and will not be deleted automatically"
                    .to_string(),
            );
        }
    }

    if let Some((install_root,)) = pc_install_row {
        remove_path_if_exists(std::path::Path::new(&install_root)).await?;
        sqlx::query("DELETE FROM pc_game_installs WHERE launchbox_db_id = ?")
            .bind(launchbox_db_id)
            .execute(db_pool)
            .await
            .map_err(|e| e.to_string())?;
        removed_anything = true;
    }

    if removed_anything {
        Ok(())
    } else {
        Err("No Lunchbox-managed installation was found for this game".to_string())
    }
}

async fn remove_path_if_exists(path: &std::path::Path) -> Result<(), String> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_dir() && !file_type.is_symlink() {
                tokio::fs::remove_dir_all(path)
                    .await
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))
            } else {
                tokio::fs::remove_file(path)
                    .await
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("Failed to inspect {}: {}", path.display(), err)),
    }
}

/// Get an active (pending/in_progress) import job for a game
pub async fn get_active_import(
    state: &AppState,
    launchbox_db_id: i64,
) -> Result<Option<ImportJob>, String> {
    let db_pool = state
        .db_pool
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let row: Option<(String, i64, String, String, String, f64, Option<String>, String, String)> = sqlx::query_as(
        "SELECT id, launchbox_db_id, game_title, platform, status, progress_percent, status_message, created_at, updated_at FROM graboid_jobs WHERE launchbox_db_id = ? AND status IN ('pending', 'in_progress') ORDER BY created_at DESC LIMIT 1"
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some((
        id,
        db_id,
        game_title,
        platform,
        status,
        progress_percent,
        status_message,
        created_at,
        updated_at,
    )) = row
    {
        let missing_live_progress = crate::torrent::get_progress(&id).is_none();

        // Auto-fail stale jobs (no updates for 30+ minutes)
        let stale_minutes = chrono::NaiveDateTime::parse_from_str(&updated_at, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| {
                chrono::Utc::now()
                    .naive_utc()
                    .signed_duration_since(dt)
                    .num_minutes()
            })
            .unwrap_or(0);

        let stale_message = if stale_minutes >= 30 {
            Some("Stale job auto-failed")
        } else if missing_live_progress && stale_minutes >= 1 {
            Some("Lost live download state; retry the download")
        } else {
            None
        };

        if let Some(message) = stale_message {
            let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(message)
                .bind(&id)
                .execute(db_pool)
                .await;
            return Ok(None);
        }

        return Ok(Some(ImportJob {
            id,
            launchbox_db_id: db_id,
            game_title,
            platform,
            status,
            progress_percent,
            status_message,
            created_at,
            updated_at,
        }));
    }

    Ok(None)
}

// ============================================================================
// Minerva Archive Types & Handlers
// ============================================================================

/// A minerva torrent available for a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinervaRom {
    pub torrent_id: i64,
    pub torrent_url: String,
    pub collection: String,
    pub minerva_platform: String,
    pub lunchbox_platform_id: i64,
    pub rom_count: i64,
    pub total_size: i64,
}

fn canonicalize_legacy_platform_name(name: &str) -> &str {
    match name.trim() {
        "Arduboy Inc - Arduboy" => "Arduboy",
        "Atari - 8-bit Family" => "Atari 800",
        other => other,
    }
}

#[derive(Clone, Copy)]
struct MinervaPlatformFallback {
    minerva_platform: &'static str,
    collection: Option<&'static str>,
}

const ATARI_800_MINERVA_FALLBACKS: &[MinervaPlatformFallback] = &[MinervaPlatformFallback {
    minerva_platform: "Atari",
    collection: Some("TOSEC"),
}];
const ARCADE_MINERVA_FALLBACKS: &[MinervaPlatformFallback] = &[
    MinervaPlatformFallback {
        minerva_platform: "ROMs (merged)",
        collection: Some("MAME"),
    },
    MinervaPlatformFallback {
        minerva_platform: "ROMs (split)",
        collection: Some("MAME"),
    },
    MinervaPlatformFallback {
        minerva_platform: "ROMs (non-merged)",
        collection: Some("MAME"),
    },
];

fn minerva_platform_fallbacks(platform_name: &str) -> &'static [MinervaPlatformFallback] {
    match canonicalize_legacy_platform_name(platform_name) {
        "Atari 800" => ATARI_800_MINERVA_FALLBACKS,
        "Arcade" => ARCADE_MINERVA_FALLBACKS,
        _ => &[],
    }
}

async fn resolve_canonical_platform_name(
    games_db: &sqlx::SqlitePool,
    platform_id: i64,
) -> Result<Option<String>, String> {
    let row = sqlx::query_scalar::<_, String>("SELECT name FROM platforms WHERE id = ?")
        .bind(platform_id)
        .fetch_optional(games_db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row.map(|name| canonicalize_legacy_platform_name(&name).to_string()))
}

async fn resolve_equivalent_platform_ids(
    games_db: &sqlx::SqlitePool,
    platform_id: i64,
) -> Result<Vec<i64>, String> {
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM platforms")
        .fetch_all(games_db)
        .await
        .map_err(|e| e.to_string())?;

    let selected_name = rows
        .iter()
        .find_map(|(id, name)| (*id == platform_id).then_some(name.clone()));

    let Some(selected_name) = selected_name else {
        return Ok(vec![platform_id]);
    };

    let canonical = canonicalize_legacy_platform_name(&selected_name).to_string();
    let mut equivalent_ids = Vec::new();
    for (id, name) in rows {
        if canonicalize_legacy_platform_name(&name) == canonical {
            equivalent_ids.push(id);
        }
    }

    if equivalent_ids.is_empty() {
        equivalent_ids.push(platform_id);
    }

    Ok(equivalent_ids)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartMinervaDownloadInput {
    pub torrent_url: String,
    pub file_index: Option<usize>,
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    #[serde(default)]
    pub download_mode: MinervaDownloadMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinervaDownloadMode {
    #[default]
    GameOnly,
    FullTorrent,
}

fn sanitize_download_directory_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '-',
        })
        .collect();

    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "download".to_string()
    } else {
        trimmed.to_string()
    }
}

fn locate_downloaded_file(
    download_dir: &std::path::Path,
    target_filename: &str,
) -> Option<std::path::PathBuf> {
    fn normalized_components_from_str(path: &str) -> Vec<String> {
        path.split(['/', '\\'])
            .filter(|component| !component.is_empty())
            .map(|component| component.to_ascii_lowercase())
            .collect()
    }

    fn normalized_components_from_path(path: &std::path::Path) -> Vec<String> {
        path.components()
            .filter_map(|component| match component {
                std::path::Component::Normal(value) => {
                    Some(value.to_string_lossy().to_ascii_lowercase())
                }
                _ => None,
            })
            .collect()
    }

    let target_components = normalized_components_from_str(target_filename);
    let target_name = target_components.last()?.clone();
    let mut basename_match = None;

    for entry in walkdir::WalkDir::new(download_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let candidate_path = entry.into_path();
        let candidate_components = normalized_components_from_path(&candidate_path);
        if candidate_components.is_empty() {
            continue;
        }

        if candidate_components.ends_with(&target_components) {
            return Some(candidate_path);
        }

        if basename_match.is_none() && candidate_components.last() == Some(&target_name) {
            basename_match = Some(candidate_path);
        }
    }

    basename_match
}

/// Check if the minerva database is available
pub fn has_minerva_db(state: &AppState) -> bool {
    state.minerva_db_pool.is_some()
}

/// Find a minerva torrent for a game's platform
pub async fn get_minerva_rom_for_game(
    state: &AppState,
    launchbox_db_id: i64,
    platform_id: Option<i64>,
) -> Result<Option<MinervaRom>, String> {
    let minerva_pool = state
        .minerva_db_pool
        .as_ref()
        .ok_or_else(|| "Minerva database not available".to_string())?;

    // Look up the game's platform_id from games.db
    let games_db = state
        .games_db_pool
        .as_ref()
        .ok_or_else(|| "Games database not available".to_string())?;

    // Resolve platform_id: use passed value, or look up from game
    let resolved_platform_id = if let Some(pid) = platform_id {
        pid
    } else if launchbox_db_id > 0 {
        match sqlx::query_as::<_, (i64,)>(
            "SELECT platform_id FROM games WHERE launchbox_db_id = ? LIMIT 1",
        )
        .bind(launchbox_db_id)
        .fetch_optional(games_db)
        .await
        .map_err(|e| e.to_string())?
        {
            Some((pid,)) => pid,
            None => return Ok(None),
        }
    } else {
        return Ok(None);
    };
    let platform_id = resolved_platform_id;
    let equivalent_platform_ids = resolve_equivalent_platform_ids(games_db, platform_id).await?;
    let canonical_platform_name = resolve_canonical_platform_name(games_db, platform_id)
        .await?
        .unwrap_or_default();
    let fallback_specs = minerva_platform_fallbacks(&canonical_platform_name);
    let placeholders = vec!["?"; equivalent_platform_ids.len()].join(", ");
    let fallback_clause = if fallback_specs.is_empty() {
        String::new()
    } else {
        let mut clauses = Vec::with_capacity(fallback_specs.len());
        for spec in fallback_specs {
            if spec.collection.is_some() {
                clauses.push("(tp.minerva_platform = ? AND COALESCE(t.collection, '') = ?)");
            } else {
                clauses.push("(tp.minerva_platform = ?)");
            }
        }
        format!(" OR {}", clauses.join(" OR "))
    };

    // Find a torrent for this platform
    let sql = format!(
        "SELECT t.id, t.torrent_url, COALESCE(t.collection, ''), tp.minerva_platform, tp.rom_count, COALESCE(t.total_size, 0)
         FROM minerva_torrent_platforms tp
         JOIN minerva_torrents t ON tp.torrent_id = t.id
         WHERE tp.lunchbox_platform_id IN ({}){}
         ORDER BY CASE WHEN tp.lunchbox_platform_id IN ({}) THEN 0 ELSE 1 END, tp.rom_count DESC
         LIMIT 1",
        placeholders, fallback_clause, placeholders
    );
    let mut query = sqlx::query_as::<_, (i64, String, String, String, i64, i64)>(&sql);
    for pid in &equivalent_platform_ids {
        query = query.bind(pid);
    }
    for spec in fallback_specs {
        query = query.bind(spec.minerva_platform);
        if let Some(collection) = spec.collection {
            query = query.bind(collection);
        }
    }
    for pid in &equivalent_platform_ids {
        query = query.bind(pid);
    }
    let row = query
        .fetch_optional(minerva_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row.map(
        |(torrent_id, torrent_url, collection, minerva_platform, rom_count, total_size)| {
            MinervaRom {
                torrent_id,
                torrent_url,
                collection,
                minerva_platform,
                lunchbox_platform_id: platform_id,
                rom_count,
                total_size,
            }
        },
    ))
}

/// Search for all minerva torrents available for a platform
pub async fn search_minerva(
    state: &AppState,
    _launchbox_db_id: Option<i64>,
    _game_title: Option<String>,
    platform_id: Option<i64>,
) -> Result<Vec<MinervaRom>, String> {
    let minerva_pool = state
        .minerva_db_pool
        .as_ref()
        .ok_or_else(|| "Minerva database not available".to_string())?;

    let pid = match platform_id {
        Some(pid) => pid,
        None => return Ok(Vec::new()),
    };
    let games_db = state
        .games_db_pool
        .as_ref()
        .ok_or_else(|| "Games database not available".to_string())?;
    let equivalent_platform_ids = resolve_equivalent_platform_ids(games_db, pid).await?;
    let canonical_platform_name = resolve_canonical_platform_name(games_db, pid)
        .await?
        .unwrap_or_default();
    let fallback_specs = minerva_platform_fallbacks(&canonical_platform_name);
    let placeholders = vec!["?"; equivalent_platform_ids.len()].join(", ");
    let fallback_clause = if fallback_specs.is_empty() {
        String::new()
    } else {
        let mut clauses = Vec::with_capacity(fallback_specs.len());
        for spec in fallback_specs {
            if spec.collection.is_some() {
                clauses.push("(tp.minerva_platform = ? AND COALESCE(t.collection, '') = ?)");
            } else {
                clauses.push("(tp.minerva_platform = ?)");
            }
        }
        format!(" OR {}", clauses.join(" OR "))
    };

    let sql = format!(
        "SELECT t.id, t.torrent_url, COALESCE(t.collection, ''), tp.minerva_platform, tp.rom_count, COALESCE(t.total_size, 0)
         FROM minerva_torrent_platforms tp
         JOIN minerva_torrents t ON tp.torrent_id = t.id
         WHERE tp.lunchbox_platform_id IN ({}){}
         ORDER BY CASE WHEN tp.lunchbox_platform_id IN ({}) THEN 0 ELSE 1 END, tp.rom_count DESC",
        placeholders, fallback_clause, placeholders
    );
    let mut query = sqlx::query_as::<_, (i64, String, String, String, i64, i64)>(&sql);
    for platform_id in &equivalent_platform_ids {
        query = query.bind(platform_id);
    }
    for spec in fallback_specs {
        query = query.bind(spec.minerva_platform);
        if let Some(collection) = spec.collection {
            query = query.bind(collection);
        }
    }
    for platform_id in &equivalent_platform_ids {
        query = query.bind(platform_id);
    }
    let rows = query
        .fetch_all(minerva_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(
            |(torrent_id, torrent_url, collection, minerva_platform, rom_count, total_size)| {
                MinervaRom {
                    torrent_id,
                    torrent_url,
                    collection,
                    minerva_platform,
                    lunchbox_platform_id: pid,
                    rom_count,
                    total_size,
                }
            },
        )
        .collect())
}

/// Start a minerva ROM download via torrent
pub async fn start_minerva_download(
    state: &mut AppState,
    input: StartMinervaDownloadInput,
) -> Result<ImportJob, String> {
    let torrent_url = input.torrent_url.clone();
    let file_index = input.file_index;
    let download_mode = input.download_mode;

    // Create import job
    let job_id = uuid::Uuid::new_v4().to_string();
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO graboid_jobs (id, launchbox_db_id, game_title, platform, status, progress_percent, status_message)
         VALUES (?, ?, ?, ?, 'in_progress', 0, 'Starting torrent download...')"
    )
    .bind(&job_id)
    .bind(input.launchbox_db_id)
    .bind(&input.game_title)
    .bind(&input.platform)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let rom_dir = state.settings.get_rom_directory();
    let file_link_mode = state.settings.torrent.file_link_mode.clone();
    let download_dir = match download_mode {
        MinervaDownloadMode::GameOnly => {
            state.settings.get_import_directory().join(&input.platform)
        }
        MinervaDownloadMode::FullTorrent => {
            let title_component = sanitize_download_directory_component(&input.game_title);
            let job_suffix = job_id.chars().take(8).collect::<String>();
            state
                .settings
                .get_torrent_library_directory()
                .join(&input.platform)
                .join(format!("{title_component}-{job_suffix}"))
        }
    };
    std::fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;

    // Spawn background download task
    let job_id_bg = job_id.clone();
    let game_title = input.game_title.clone();
    let platform = input.platform.clone();
    let launchbox_db_id = input.launchbox_db_id;
    let db_path = state.user_db_path.clone();
    let app_settings = state.settings.clone();
    let file_link_mode_bg = file_link_mode.clone();
    let rom_dir_bg = rom_dir.clone();
    let download_dir_bg = download_dir.clone();

    tokio::spawn(async move {
        // Step 1: Fetch the torrent file
        crate::torrent::update_progress(
            &job_id_bg,
            crate::torrent::DownloadStatus::FetchingTorrent,
            0.0,
            0,
            0,
            0,
            "Fetching torrent file...",
        );

        // Step 2: Parse torrent to get the target filename
        let files = match crate::torrent::get_torrent_file_listing(&torrent_url).await {
            Ok(f) => f,
            Err(e) => {
                crate::torrent::update_progress(
                    &job_id_bg,
                    crate::torrent::DownloadStatus::Failed,
                    0.0,
                    0,
                    0,
                    0,
                    &format!("Failed to parse torrent: {e}"),
                );
                return;
            }
        };

        let selection_plan = if matches!(download_mode, MinervaDownloadMode::GameOnly) {
            file_index.and_then(|idx| crate::exo::plan_related_downloads(&platform, idx, &files))
        } else {
            None
        };

        let representative_index = selection_plan
            .as_ref()
            .map(|plan| plan.representative_index)
            .or(file_index);
        let target_file =
            representative_index.and_then(|idx| files.iter().find(|f| f.index == idx));
        let target_filename = target_file.map(|f| f.filename.clone()).unwrap_or_default();
        let target_size = target_file.map(|f| f.size).unwrap_or(0);

        if matches!(download_mode, MinervaDownloadMode::GameOnly) && target_file.is_none() {
            crate::torrent::update_progress(
                &job_id_bg,
                crate::torrent::DownloadStatus::Failed,
                0.0,
                0,
                0,
                0,
                "No matching file was selected for this Minerva torrent.",
            );
            if let Some(ref db_path) = db_path {
                if let Ok(pool) = crate::db::init_pool(db_path).await {
                    let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = 'No matching file was selected for this Minerva torrent.', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(&job_id_bg)
                        .execute(&pool)
                        .await;
                }
            }
            return;
        }

        let status_message = match download_mode {
            MinervaDownloadMode::GameOnly => format!("Downloading: {target_filename}"),
            MinervaDownloadMode::FullTorrent => {
                format!("Downloading full torrent for {game_title}")
            }
        };
        let progress_total = match download_mode {
            MinervaDownloadMode::GameOnly => selection_plan
                .as_ref()
                .map(|plan| {
                    plan.requested_indices
                        .iter()
                        .filter_map(|idx| files.iter().find(|file| file.index == *idx))
                        .map(|file| file.size)
                        .sum()
                })
                .unwrap_or(target_size),
            MinervaDownloadMode::FullTorrent => files.iter().map(|file| file.size).sum(),
        };

        crate::torrent::update_progress(
            &job_id_bg,
            crate::torrent::DownloadStatus::Downloading,
            5.0,
            0,
            0,
            progress_total,
            &status_message,
        );

        // Step 3: Add torrent and start download
        let client = match crate::torrent::create_client(&app_settings) {
            Ok(c) => c,
            Err(e) => {
                crate::torrent::update_progress(
                    &job_id_bg,
                    crate::torrent::DownloadStatus::Failed,
                    0.0,
                    0,
                    0,
                    0,
                    &format!("qBittorrent configuration error: {e}"),
                );
                return;
            }
        };

        let requested_indices = match download_mode {
            MinervaDownloadMode::GameOnly => selection_plan
                .as_ref()
                .map(|plan| plan.requested_indices.clone())
                .or_else(|| file_index.map(|idx| vec![idx])),
            MinervaDownloadMode::FullTorrent => None,
        };

        let add_result = client
            .add_torrent(&torrent_url, &download_dir_bg, requested_indices)
            .await;

        let client_job_id = match add_result {
            Ok(job_id) => job_id,
            Err(e) => {
                crate::torrent::update_progress(
                    &job_id_bg,
                    crate::torrent::DownloadStatus::Failed,
                    0.0,
                    0,
                    0,
                    0,
                    &format!("Failed to start download: {e}"),
                );
                if let Some(ref db_path) = db_path {
                    if let Ok(pool) = crate::db::init_pool(db_path).await {
                        let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                            .bind(format!("Download failed: {e}"))
                            .bind(&job_id_bg)
                            .execute(&pool)
                            .await;
                    }
                }
                return;
            }
        };
        crate::torrent::set_client_job_id(&job_id_bg, &client_job_id);

        // Step 4: Poll qBittorrent progress until the requested download completes.
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(7200);

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            if start.elapsed() > timeout {
                crate::torrent::update_progress(
                    &job_id_bg,
                    crate::torrent::DownloadStatus::Failed,
                    0.0,
                    0,
                    0,
                    progress_total,
                    "Download timed out after 2 hours",
                );
                if let Some(ref db_path) = db_path {
                    if let Ok(pool) = crate::db::init_pool(db_path).await {
                        let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = 'Download timed out after 2 hours', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                            .bind(&job_id_bg)
                            .execute(&pool)
                            .await;
                    }
                }
                return;
            }

            let progress = match client.get_progress(&client_job_id).await {
                Ok(Some(progress)) => progress,
                Ok(None) => continue,
                Err(e) => {
                    crate::torrent::update_progress(
                        &job_id_bg,
                        crate::torrent::DownloadStatus::Failed,
                        0.0,
                        0,
                        0,
                        progress_total,
                        &format!("Failed to read qBittorrent progress: {e}"),
                    );
                    if let Some(ref db_path) = db_path {
                        if let Ok(pool) = crate::db::init_pool(db_path).await {
                            let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                                .bind(format!("Failed to read qBittorrent progress: {e}"))
                                .bind(&job_id_bg)
                                .execute(&pool)
                                .await;
                        }
                    }
                    return;
                }
            };

            crate::torrent::update_progress(
                &job_id_bg,
                progress.status,
                progress.progress_percent,
                progress.download_speed,
                progress.downloaded_bytes,
                progress.total_bytes,
                &progress.status_message,
            );

            match progress.status {
                crate::torrent::DownloadStatus::Completed => break,
                crate::torrent::DownloadStatus::Failed => {
                    if let Some(ref db_path) = db_path {
                        if let Ok(pool) = crate::db::init_pool(db_path).await {
                            let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                                .bind(progress.status_message)
                                .bind(&job_id_bg)
                                .execute(&pool)
                                .await;
                        }
                    }
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
                crate::torrent::DownloadStatus::Cancelled => {
                    if let Some(ref db_path) = db_path {
                        if let Ok(pool) = crate::db::init_pool(db_path).await {
                            let _ = sqlx::query("UPDATE graboid_jobs SET status = 'cancelled', status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                                .bind(progress.status_message)
                                .bind(&job_id_bg)
                                .execute(&pool)
                                .await;
                        }
                    }
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
                _ => {}
            }
        }

        let representative_path = if let Some(file) = target_file {
            if let Some(path) = client
                .get_downloaded_file_path(&client_job_id, file.index, &download_dir_bg)
                .await
                .ok()
                .flatten()
            {
                Some(path)
            } else {
                locate_downloaded_file(&download_dir_bg, &file.filename)
            }
        } else {
            None
        };

        let (stored_path, stored_size, completion_message) = match download_mode {
            MinervaDownloadMode::GameOnly => {
                let Some(found_path) = representative_path else {
                    crate::torrent::update_progress(
                        &job_id_bg,
                        crate::torrent::DownloadStatus::Failed,
                        100.0,
                        0,
                        target_size,
                        target_size,
                        "Download finished, but the selected ROM file could not be found on disk.",
                    );
                    if let Some(ref db_path) = db_path {
                        if let Ok(pool) = crate::db::init_pool(db_path).await {
                            let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = 'Download finished, but the selected ROM file could not be found on disk.', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                                .bind(&job_id_bg)
                                .execute(&pool)
                                .await;
                        }
                    }
                    return;
                };

                let file_size = std::fs::metadata(&found_path)
                    .map(|meta| meta.len() as i64)
                    .unwrap_or(target_size as i64);
                (
                    found_path.display().to_string(),
                    file_size,
                    "Download complete".to_string(),
                )
            }
            MinervaDownloadMode::FullTorrent => {
                let Some(found_path) = representative_path else {
                    crate::torrent::update_progress(
                        &job_id_bg,
                        crate::torrent::DownloadStatus::Failed,
                        100.0,
                        0,
                        progress_total,
                        progress_total,
                        "Full torrent finished, but Lunchbox could not locate the selected game inside it.",
                    );
                    if let Some(ref db_path) = db_path {
                        if let Ok(pool) = crate::db::init_pool(db_path).await {
                            let _ = sqlx::query("UPDATE graboid_jobs SET status = 'failed', status_message = 'Full torrent finished, but Lunchbox could not locate the selected game inside it.', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                                .bind(&job_id_bg)
                                .execute(&pool)
                                .await;
                        }
                    }
                    return;
                };

                let linked_path = match crate::torrent::link_rom_file(
                    &found_path,
                    &rom_dir_bg,
                    &platform,
                    &file_link_mode_bg,
                ) {
                    Ok(path) => path,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to link {} into the ROM library after a full torrent download: {e}",
                            found_path.display()
                        );
                        found_path.clone()
                    }
                };

                let file_size = std::fs::metadata(&found_path)
                    .map(|meta| meta.len() as i64)
                    .unwrap_or(target_size as i64);
                (
                    linked_path.display().to_string(),
                    file_size,
                    "Full torrent download complete".to_string(),
                )
            }
        };

        crate::torrent::update_progress(
            &job_id_bg,
            crate::torrent::DownloadStatus::Completed,
            100.0,
            0,
            progress_total,
            progress_total,
            &completion_message,
        );

        if let Some(ref db_path) = db_path {
            if let Ok(pool) = crate::db::init_pool(db_path).await {
                let _ = sqlx::query(
                    "INSERT OR REPLACE INTO game_files (launchbox_db_id, game_title, platform, file_path, file_size, import_source) VALUES (?, ?, ?, ?, ?, 'minerva')"
                )
                .bind(launchbox_db_id).bind(&game_title).bind(&platform)
                .bind(&stored_path).bind(stored_size)
                .execute(&pool).await;
                let _ = sqlx::query("UPDATE graboid_jobs SET status = 'completed', progress_percent = 100, status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(&completion_message)
                    .bind(&job_id_bg)
                    .execute(&pool).await;
            }
        }
        crate::torrent::clear_client_job_id(&job_id_bg);
    });

    Ok(ImportJob {
        id: job_id,
        launchbox_db_id: input.launchbox_db_id,
        game_title: input.game_title,
        platform: input.platform,
        status: "in_progress".to_string(),
        progress_percent: 0.0,
        status_message: Some("Starting torrent download...".to_string()),
        created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    })
}

/// Get download progress for a minerva download
pub fn get_minerva_download_progress(job_id: &str) -> Option<crate::torrent::DownloadProgress> {
    crate::torrent::get_progress(job_id)
}

/// Cancel an active minerva download
pub async fn cancel_minerva_download(state: &AppState, job_id: &str) -> Result<(), String> {
    // Cancel via client
    if let Ok(client) = crate::torrent::create_client(&state.settings) {
        if let Some(client_job_id) = crate::torrent::get_client_job_id(job_id) {
            let _ = client.cancel(&client_job_id).await;
        }
    }

    // Also update local progress tracking
    crate::torrent::update_progress(
        job_id,
        crate::torrent::DownloadStatus::Cancelled,
        0.0,
        0,
        0,
        0,
        "Cancelled",
    );
    crate::torrent::clear_client_job_id(job_id);

    // Update job status in database
    if let Some(db_pool) = state.db_pool.as_ref() {
        let _ = sqlx::query(
            "UPDATE graboid_jobs SET status = 'cancelled', status_message = 'Download cancelled', updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(job_id)
        .execute(db_pool)
        .await;
    }

    Ok(())
}

/// Test the configured qBittorrent Web UI connection
pub async fn test_torrent_connection(state: &AppState) -> Result<(bool, String), String> {
    let client = crate::torrent::create_client(&state.settings).map_err(|e| e.to_string())?;
    match client.test_connection().await {
        Ok(msg) => Ok((true, msg)),
        Err(e) => Ok((false, e.to_string())),
    }
}

// ============================================================================
// Torrent File Listing and Matching
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFileMatch {
    pub index: usize,
    pub filename: String,
    pub size: u64,
    pub match_score: f64,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTorrentFilesInput {
    pub torrent_url: String,
    pub game_title: String,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub launchbox_db_id: Option<i64>,
}

const TORRENT_MATCH_STOP_WORDS: &[&str] = &[
    "a", "an", "and", "for", "in", "of", "on", "or", "the", "to", "with",
];

#[derive(Debug)]
struct TorrentMatchCandidate {
    file_match: TorrentFileMatch,
    exact_match: bool,
    full_query_match: bool,
    all_significant_words_match: bool,
}

fn basename_without_extension(path: &str) -> String {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);

    filename
        .rsplit_once('.')
        .map(|(base, _)| base)
        .unwrap_or(filename)
        .to_string()
}

fn significant_match_words<'a>(normalized_title: &'a str) -> Vec<&'a str> {
    let words: Vec<&str> = normalized_title.split_whitespace().collect();
    let significant_words: Vec<&str> = words
        .iter()
        .copied()
        .filter(|word| word.len() > 1 && !TORRENT_MATCH_STOP_WORDS.contains(word))
        .collect();

    if significant_words.is_empty() {
        words
    } else {
        significant_words
    }
}

fn build_torrent_match_candidate(
    file: crate::torrent::TorrentFileInfo,
    normalized_query: &str,
    query_words: &[&str],
    significant_query_words: &[&str],
) -> Option<TorrentMatchCandidate> {
    let name = basename_without_extension(&file.filename);
    let normalized_file = crate::tags::normalize_title_for_matching(&name);
    if normalized_file.is_empty() {
        return None;
    }

    let file_words: Vec<&str> = normalized_file.split_whitespace().collect();
    if file_words.is_empty() {
        return None;
    }

    let exact_match = normalized_file == normalized_query;
    let full_query_match =
        !normalized_query.is_empty() && normalized_file.contains(normalized_query);
    let matching_query_words = query_words
        .iter()
        .filter(|query_word| file_words.iter().any(|file_word| file_word == *query_word))
        .count();
    let matching_significant_words = significant_query_words
        .iter()
        .filter(|query_word| file_words.iter().any(|file_word| file_word == *query_word))
        .count();
    let all_significant_words_match = !significant_query_words.is_empty()
        && matching_significant_words == significant_query_words.len();

    let score = if exact_match {
        1.0
    } else if full_query_match {
        0.97
    } else if all_significant_words_match {
        let query_overlap = if query_words.is_empty() {
            0.0
        } else {
            matching_query_words as f64 / query_words.len() as f64
        };
        0.85 + (query_overlap * 0.1)
    } else if !significant_query_words.is_empty() && matching_significant_words > 0 {
        let significant_overlap =
            matching_significant_words as f64 / significant_query_words.len() as f64;
        let query_overlap = if query_words.is_empty() {
            0.0
        } else {
            matching_query_words as f64 / query_words.len() as f64
        };
        (significant_overlap * 0.75) + (query_overlap * 0.2)
    } else if matching_query_words > 0 && !query_words.is_empty() {
        matching_query_words as f64 / query_words.len() as f64 * 0.5
    } else {
        0.0
    };

    let region = crate::tags::get_region_tags(&file.filename)
        .into_iter()
        .next();

    Some(TorrentMatchCandidate {
        file_match: TorrentFileMatch {
            index: file.index,
            filename: file.filename,
            size: file.size,
            match_score: score,
            region,
        },
        exact_match,
        full_query_match,
        all_significant_words_match,
    })
}

fn select_torrent_file_matches(
    files: Vec<crate::torrent::TorrentFileInfo>,
    game_title: &str,
    region_priority: &[String],
) -> Vec<TorrentFileMatch> {
    let normalized_query = crate::tags::normalize_title_for_matching(game_title);
    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();
    let significant_query_words = significant_match_words(&normalized_query);

    let mut candidates: Vec<TorrentMatchCandidate> = files
        .into_iter()
        .filter_map(|file| {
            build_torrent_match_candidate(
                file,
                &normalized_query,
                &query_words,
                &significant_query_words,
            )
        })
        .collect();

    candidates.sort_by(|a, b| {
        b.exact_match
            .cmp(&a.exact_match)
            .then_with(|| b.full_query_match.cmp(&a.full_query_match))
            .then_with(|| {
                b.all_significant_words_match
                    .cmp(&a.all_significant_words_match)
            })
            .then_with(|| {
                b.file_match
                    .match_score
                    .partial_cmp(&a.file_match.match_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                crate::region_priority::priority_for_region(
                    a.file_match.region.as_deref(),
                    region_priority,
                )
                .cmp(&crate::region_priority::priority_for_region(
                    b.file_match.region.as_deref(),
                    region_priority,
                ))
            })
            .then_with(|| a.file_match.filename.cmp(&b.file_match.filename))
    });

    let has_exact_match = candidates.iter().any(|candidate| candidate.exact_match);
    let has_full_query_match = candidates
        .iter()
        .any(|candidate| candidate.full_query_match);
    let has_significant_match = candidates
        .iter()
        .any(|candidate| candidate.all_significant_words_match);

    if has_exact_match {
        candidates.retain(|candidate| candidate.exact_match);
        candidates.truncate(15);
    } else if has_full_query_match {
        candidates.retain(|candidate| candidate.full_query_match);
        candidates.truncate(15);
    } else if has_significant_match {
        candidates.retain(|candidate| candidate.all_significant_words_match);
        candidates.truncate(15);
    } else {
        candidates.retain(|candidate| candidate.file_match.match_score > 0.0);
        candidates.truncate(10);
    }

    candidates
        .into_iter()
        .map(|candidate| candidate.file_match)
        .collect()
}

fn is_platform_specific_torrent_candidate(platform: &str, filename: &str) -> bool {
    let normalized_platform = canonicalize_legacy_platform_name(platform);
    let lowercase_filename = filename.to_lowercase();

    match normalized_platform {
        "Atari 800" => {
            lowercase_filename.contains("/atari - 8-bit family/")
                || lowercase_filename.contains("/atari/8bit/")
        }
        _ => true,
    }
}

/// Fetch a torrent file and list its contents with fuzzy matching against a game title
pub async fn list_torrent_files(
    state: &AppState,
    input: ListTorrentFilesInput,
) -> Result<Vec<TorrentFileMatch>, String> {
    let mut files = crate::torrent::get_torrent_file_listing(&input.torrent_url)
        .await
        .map_err(|e| format!("Failed to load torrent metadata: {e}"))?;

    if let Some(ref platform) = input.platform {
        let filtered_files: Vec<_> = files
            .iter()
            .filter(|file| is_platform_specific_torrent_candidate(platform, &file.filename))
            .cloned()
            .collect();
        if !filtered_files.is_empty() {
            files = filtered_files;
        }
    }

    let lookup_title = if let Some(ref platform) = input.platform {
        crate::images::emumovies::resolve_arcade_download_lookup_name_for_torrent(
            platform,
            &input.game_title,
            input.launchbox_db_id,
            &input.torrent_url,
        )
        .into_owned()
    } else {
        input.game_title.clone()
    };

    let mut matches =
        select_torrent_file_matches(files, &lookup_title, &state.settings.region_priority);

    if let Some(ref platform) = input.platform {
        let mut filtered: Vec<TorrentFileMatch> = matches
            .iter()
            .filter(|file| crate::exo::is_primary_download_candidate(platform, &file.filename))
            .cloned()
            .collect();
        if !filtered.is_empty() {
            filtered.sort_by_key(|file| {
                crate::exo::primary_download_priority(platform, &file.filename)
            });
            matches = filtered;
        }
    }

    Ok(matches)
}

// ============================================================================
// ROM Import — Scan & Match
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRomsInput {
    pub directories: Vec<String>,
    pub compute_checksums: bool,
    pub platform_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedRom {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub extension: String,
    pub inner_extension: Option<String>,
    pub detected_platform: Option<String>,
    pub detected_platform_id: Option<i64>,
    pub matched_game_id: Option<String>,
    pub matched_game_title: Option<String>,
    pub matched_launchbox_db_id: Option<i64>,
    pub match_method: Option<String>,
    pub match_confidence: f64,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRomsResult {
    pub roms: Vec<ScannedRom>,
    pub total_scanned: usize,
    pub matched_count: usize,
    pub unmatched_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RomImportEntry {
    pub file_path: String,
    pub launchbox_db_id: i64,
    pub game_title: String,
    pub platform: String,
    pub copy_to_library: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmImportInput {
    pub roms: Vec<RomImportEntry>,
}

/// Scan directories for ROMs and match them to games in the database
pub async fn scan_and_match_roms(
    state: &AppState,
    input: ScanRomsInput,
) -> Result<ScanRomsResult, String> {
    let games_db = state
        .games_db_pool
        .as_ref()
        .ok_or_else(|| "Games database not available".to_string())?;

    // Load platform data for extension matching
    let platforms: Vec<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, name, file_extensions FROM platforms")
            .fetch_all(games_db)
            .await
            .map_err(|e| e.to_string())?;

    // Build extension → platform(s) lookup
    let mut ext_to_platforms: std::collections::HashMap<String, Vec<(i64, String)>> =
        std::collections::HashMap::new();
    for (id, name, exts) in &platforms {
        if let Some(exts_str) = exts {
            for ext in exts_str.split(|c: char| c == ',' || c == ' ' || c == ';') {
                let ext = ext
                    .trim()
                    .to_lowercase()
                    .trim_start_matches('.')
                    .to_string();
                if !ext.is_empty() {
                    ext_to_platforms
                        .entry(ext)
                        .or_default()
                        .push((*id, name.clone()));
                }
            }
        }
    }

    // Build platform name lookup (lowercase → (id, name))
    let platform_name_lookup: std::collections::HashMap<String, (i64, String)> = platforms
        .iter()
        .map(|(id, name, _)| (name.to_lowercase(), (*id, name.clone())))
        .collect();

    // Scan directories
    let dirs: Vec<std::path::PathBuf> = input
        .directories
        .iter()
        .map(std::path::PathBuf::from)
        .collect();

    let scanner = crate::scanner::file_scanner::RomScanner::new();
    let rom_files = if input.compute_checksums {
        scanner.scan_with_checksums(&dirs, None)
    } else {
        scanner.scan_directories(&dirs)
    };

    let total_scanned = rom_files.len();
    let mut results = Vec::with_capacity(total_scanned);
    let mut matched_count = 0usize;

    for rom in rom_files {
        let ext = rom.extension.to_lowercase();
        let is_archive = matches!(ext.as_str(), "zip" | "7z" | "rar");

        // Determine inner extension for archives
        let inner_ext = if is_archive {
            crate::scanner::file_scanner::peek_archive_extension(&rom.path)
        } else {
            None
        };

        // Detect platform
        let effective_ext = inner_ext.as_deref().unwrap_or(&ext);
        let parent_dir = rom
            .path
            .parent()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));

        // Platform from hint, parent dir, or extension
        let detected = if let Some(ref hint) = input.platform_hint {
            platform_name_lookup
                .get(&hint.to_lowercase())
                .map(|(id, name)| (*id, name.clone()))
        } else {
            // Try parent directory name
            let from_dir = parent_dir.as_ref().and_then(|dir| {
                platform_name_lookup
                    .get(&dir.to_lowercase())
                    .map(|(id, name)| (*id, name.clone()))
            });
            from_dir.or_else(|| {
                // Try file extension
                ext_to_platforms.get(effective_ext).and_then(|plats| {
                    if plats.len() == 1 {
                        Some(plats[0].clone())
                    } else {
                        None // ambiguous
                    }
                })
            })
        };

        let (platform_id, platform_name) = match detected {
            Some((id, name)) => (Some(id), Some(name)),
            None => (None, None),
        };

        // Match to game
        let mut match_method = None;
        let mut match_confidence = 0.0;
        let mut matched_game_id = None;
        let mut matched_game_title = None;
        let mut matched_db_id = None;

        if let Some(pid) = platform_id {
            // Try checksum match first
            if let Some(ref checksums) = rom.checksums {
                let row: Option<(String, String, i64)> = sqlx::query_as(
                    "SELECT id, title, launchbox_db_id FROM games WHERE platform_id = ? AND libretro_crc32 = ?",
                )
                .bind(pid)
                .bind(&checksums.crc32)
                .fetch_optional(games_db)
                .await
                .ok()
                .flatten();

                if let Some((gid, title, db_id)) = row {
                    matched_game_id = Some(gid);
                    matched_game_title = Some(title);
                    matched_db_id = Some(db_id);
                    match_method = Some("checksum".to_string());
                    match_confidence = 1.0;
                }
            }

            // Fall back to filename match
            if matched_game_id.is_none() {
                let normalized_rom = crate::tags::normalize_title_for_matching(&rom.clean_name);
                if !normalized_rom.is_empty() {
                    let rom_words: Vec<&str> = normalized_rom.split_whitespace().collect();

                    // Query games for this platform
                    let games: Vec<(String, String, i64)> = sqlx::query_as(
                        "SELECT id, title, launchbox_db_id FROM games WHERE platform_id = ?",
                    )
                    .bind(pid)
                    .fetch_all(games_db)
                    .await
                    .unwrap_or_default();

                    let mut best_score = 0.0f64;
                    for (gid, title, db_id) in &games {
                        let normalized_game = crate::tags::normalize_title_for_matching(title);
                        if normalized_game == normalized_rom {
                            matched_game_id = Some(gid.clone());
                            matched_game_title = Some(title.clone());
                            matched_db_id = Some(*db_id);
                            match_method = Some("filename".to_string());
                            match_confidence = 1.0;
                            break;
                        }

                        let game_words: Vec<&str> = normalized_game.split_whitespace().collect();
                        if !game_words.is_empty() && !rom_words.is_empty() {
                            let matching = rom_words
                                .iter()
                                .filter(|w| game_words.iter().any(|gw| gw == *w))
                                .count();
                            let score =
                                matching as f64 / rom_words.len().max(game_words.len()) as f64;
                            if score > best_score && score >= 0.5 {
                                best_score = score;
                                matched_game_id = Some(gid.clone());
                                matched_game_title = Some(title.clone());
                                matched_db_id = Some(*db_id);
                                match_method = Some("filename".to_string());
                                match_confidence = score;
                            }
                        }
                    }
                }
            }
        }

        if matched_game_id.is_some() {
            matched_count += 1;
        }

        let region = crate::tags::get_region_tags(&rom.file_name)
            .into_iter()
            .next();

        results.push(ScannedRom {
            file_path: rom.path.display().to_string(),
            file_name: rom.file_name.clone(),
            file_size: rom.size,
            extension: ext,
            inner_extension: inner_ext,
            detected_platform: platform_name,
            detected_platform_id: platform_id,
            matched_game_id,
            matched_game_title,
            matched_launchbox_db_id: matched_db_id,
            match_method,
            match_confidence,
            region,
        });
    }

    let unmatched_count = total_scanned - matched_count;

    Ok(ScanRomsResult {
        roms: results,
        total_scanned,
        matched_count,
        unmatched_count,
    })
}

/// Confirm and execute ROM import for selected files
pub async fn confirm_rom_import(
    state: &mut AppState,
    input: ConfirmImportInput,
) -> Result<usize, String> {
    let file_link_mode = state.settings.torrent.file_link_mode.clone();
    let rom_dir = state.settings.get_rom_directory();

    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;
    let mut imported = 0usize;

    for entry in &input.roms {
        let file_path = if entry.copy_to_library {
            let source = std::path::Path::new(&entry.file_path);
            match crate::torrent::link_rom_file(source, &rom_dir, &entry.platform, &file_link_mode)
            {
                Ok(target) => target.display().to_string(),
                Err(e) => {
                    tracing::warn!("Failed to link {}: {e}", entry.file_path);
                    entry.file_path.clone()
                }
            }
        } else {
            entry.file_path.clone()
        };

        let file_size = std::fs::metadata(&entry.file_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        let result = sqlx::query(
            "INSERT OR REPLACE INTO game_files (launchbox_db_id, game_title, platform, file_path, file_size, import_source)
             VALUES (?, ?, ?, ?, ?, 'local')",
        )
        .bind(entry.launchbox_db_id)
        .bind(&entry.game_title)
        .bind(&entry.platform)
        .bind(&file_path)
        .bind(file_size)
        .execute(db_pool)
        .await;

        if result.is_ok() {
            imported += 1;
        }
    }

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::{
        is_platform_specific_torrent_candidate, locate_downloaded_file, minerva_platform_fallbacks,
        select_torrent_file_matches, sort_emulator_statuses,
    };
    use crate::db::schema::EmulatorInfo;
    use crate::emulator::EmulatorWithStatus;
    use crate::torrent::TorrentFileInfo;
    use std::fs;

    #[test]
    fn prefers_exact_title_matches_over_partial_word_overlap() {
        let files = vec![
            TorrentFileInfo {
                index: 0,
                filename: "Nintendo - Nintendo Entertainment System/The Legend of Zelda (USA).zip"
                    .to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "Nintendo - Nintendo Entertainment System/Zelda II - The Adventure of Link (USA).zip"
                    .to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Nintendo - Nintendo Entertainment System/Random Platformer (USA).zip"
                    .to_string(),
                size: 1,
            },
        ];

        let matches = select_torrent_file_matches(files, "The Legend of Zelda", &[]);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].filename,
            "Nintendo - Nintendo Entertainment System/The Legend of Zelda (USA).zip"
        );
        assert_eq!(matches[0].match_score, 1.0);
    }

    #[test]
    fn falls_back_to_full_significant_word_matches_when_exact_title_is_missing() {
        let files = vec![
            TorrentFileInfo {
                index: 0,
                filename: "Collection/Super Mario Bros. 3 (USA) (Rev 1).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "Collection/Super Mario All-Stars (USA).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Collection/Mario Bros. (World).zip".to_string(),
                size: 1,
            },
        ];

        let matches = select_torrent_file_matches(files, "Super Mario Bros. 3", &[]);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].filename,
            "Collection/Super Mario Bros. 3 (USA) (Rev 1).zip"
        );
        assert!(matches[0].match_score >= 0.85);
    }

    #[test]
    fn sorts_matching_regions_as_usa_then_japan_then_asia_then_rest() {
        let files = vec![
            TorrentFileInfo {
                index: 0,
                filename: "Collection/The Legend of Zelda (Europe).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "Collection/The Legend of Zelda (Asia).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Collection/The Legend of Zelda (USA).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 3,
                filename: "Collection/The Legend of Zelda (Japan).zip".to_string(),
                size: 1,
            },
        ];

        let matches = select_torrent_file_matches(files, "The Legend of Zelda", &[]);

        assert_eq!(matches.len(), 4);
        assert_eq!(matches[0].region.as_deref(), Some("USA"));
        assert_eq!(matches[1].region.as_deref(), Some("Japan"));
        assert_eq!(matches[2].region.as_deref(), Some("Asia"));
        assert_eq!(matches[3].region.as_deref(), Some("Europe"));
    }

    #[test]
    fn sorts_matching_regions_using_custom_region_priority() {
        let files = vec![
            TorrentFileInfo {
                index: 0,
                filename: "Collection/The Legend of Zelda (USA).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "Collection/The Legend of Zelda (Japan).zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Collection/The Legend of Zelda (Asia).zip".to_string(),
                size: 1,
            },
        ];

        let custom_order = vec!["Japan".to_string(), "USA".to_string(), "Asia".to_string()];
        let matches = select_torrent_file_matches(files, "The Legend of Zelda", &custom_order);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].region.as_deref(), Some("Japan"));
        assert_eq!(matches[1].region.as_deref(), Some("USA"));
        assert_eq!(matches[2].region.as_deref(), Some("Asia"));
    }

    #[test]
    fn locate_downloaded_file_prefers_exact_suffix_match_when_basenames_collide() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path().join("Nintendo Entertainment System");
        let headerless = root
            .join("Minerva_Myrient/No-Intro/Nintendo - Nintendo Entertainment System (Headerless)");
        let headered = root
            .join("Minerva_Myrient/No-Intro/Nintendo - Nintendo Entertainment System (Headered)");
        fs::create_dir_all(&headerless).unwrap();
        fs::create_dir_all(&headered).unwrap();

        let filename = "Super Mario Bros. 3 (USA) (Rev 1).zip";
        let headerless_file = headerless.join(filename);
        let headered_file = headered.join(filename);
        fs::write(&headerless_file, b"headerless").unwrap();
        fs::write(&headered_file, b"headered").unwrap();

        let found = locate_downloaded_file(
            &root,
            "No-Intro/Nintendo - Nintendo Entertainment System (Headered)/Super Mario Bros. 3 (USA) (Rev 1).zip",
        )
        .unwrap();

        assert_eq!(found, headered_file);
    }

    #[test]
    fn sorts_installed_emulators_ahead_of_uninstalled_entries() {
        fn status(name: &str, is_installed: bool, is_retroarch_core: bool) -> EmulatorWithStatus {
            EmulatorWithStatus {
                info: EmulatorInfo {
                    id: 0,
                    name: name.to_string(),
                    homepage: None,
                    supported_os: None,
                    winget_id: None,
                    homebrew_formula: None,
                    flatpak_id: None,
                    retroarch_core: is_retroarch_core.then(|| "test_core".to_string()),
                    save_directory: None,
                    save_extensions: None,
                    notes: None,
                },
                is_installed,
                install_method: None,
                uninstall_method: None,
                is_retroarch_core,
                display_name: name.to_string(),
                executable_path: None,
                firmware_statuses: Vec::new(),
            }
        }

        let mut emulators = vec![
            status("RetroArch: beetle", false, true),
            status("DOSBox-X", true, false),
            status("RetroArch: mesen", true, true),
            status("ares", false, false),
        ];

        sort_emulator_statuses(&mut emulators);

        assert_eq!(emulators[0].display_name, "DOSBox-X");
        assert_eq!(emulators[1].display_name, "RetroArch: mesen");
        assert!(!emulators[2].is_installed);
        assert!(!emulators[3].is_installed);
    }

    #[test]
    fn atari_800_platform_uses_tosec_fallback_torrent() {
        let fallback_specs = minerva_platform_fallbacks("Atari 800");
        assert_eq!(fallback_specs.len(), 1);
        assert_eq!(fallback_specs[0].minerva_platform, "Atari");
        assert_eq!(fallback_specs[0].collection, Some("TOSEC"));
    }

    #[test]
    fn atari_800_platform_filter_keeps_only_8bit_subtree() {
        assert!(is_platform_specific_torrent_candidate(
            "Atari 800",
            "TOSEC/Atari/8bit/Games/[ATR]/Kennedy Approach (1985)(MicroProse)(US).zip"
        ));
        assert!(is_platform_specific_torrent_candidate(
            "Atari 800",
            "No-Intro/Atari - 8-bit Family/Coco Notes (USA).zip"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Atari 800",
            "TOSEC/Atari/2600 & VCS/Games/Frogger.zip"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Atari 800",
            "TOSEC/Atari/ST/Games/Kennedy Approach (1988)(MicroProse).zip"
        ));
    }

    #[test]
    fn atari_800_matching_finds_game_inside_tosec_atari_torrent() {
        let files: Vec<TorrentFileInfo> = vec![
            TorrentFileInfo {
                index: 0,
                filename: "TOSEC/Atari/2600 & VCS/Games/Kennedy Approach (1985)(MicroProse).zip"
                    .to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "TOSEC/Atari/ST/Games/Kennedy Approach (1988)(MicroProse).zip"
                    .to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 2,
                filename:
                    "TOSEC/Atari/8bit/Games/[ATR]/Kennedy Approach (1985)(MicroProse)(US)[cr].zip"
                        .to_string(),
                size: 1,
            },
        ]
        .into_iter()
        .filter(|file| is_platform_specific_torrent_candidate("Atari 800", &file.filename))
        .collect();

        let matches = select_torrent_file_matches(files, "Kennedy Approach...", &[]);
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].filename,
            "TOSEC/Atari/8bit/Games/[ATR]/Kennedy Approach (1985)(MicroProse)(US)[cr].zip"
        );
    }
}
