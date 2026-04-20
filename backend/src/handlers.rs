//! Shared endpoint handlers
//!
//! This module contains the actual implementation logic for all endpoints.
//! Both rspc procedures and HTTP handlers call into these functions, ensuring
//! the logic is defined in exactly one place.
//!
//! To add a new endpoint:
//! 1. Add the handler function here
//! 2. Add wrapper(s) in api.rs
//! 3. Register in api.rs create_router if the legacy HTTP route is needed
//! 4. Register in router.rs if the frontend needs an rspc procedure

use crate::db::schema::EmulatorInfo;
use crate::emulator::{self, EmulatorUpdate, EmulatorWithStatus, LaunchResult};
use crate::state::{AppSettings, AppState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Shared types (used by both rspc and HTTP)
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
    use crate::db::schema::{Game, normalize_title_for_display};
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
            let platform: String = row.get("platform");
            let launchbox_db_id: i64 = row.get("launchbox_db_id");
            games.push(Game {
                id: game_id,
                database_id: launchbox_db_id,
                title: title.clone(),
                display_title,
                platform: crate::arcade::display_platform_name(
                    canonicalize_legacy_platform_name(&platform),
                    &title,
                    (launchbox_db_id > 0).then_some(launchbox_db_id),
                )
                .into_owned(),
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
    let canonical_platform_name = canonicalize_legacy_platform_name(platform_name);

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
    .bind(canonical_platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    emulators.retain(emulator::is_emulator_visible_on_current_os);

    maybe_append_exodos_scummvm(pool, canonical_platform_name, os, false, &mut emulators).await?;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorLaunchProfile {
    pub emulator_name: String,
    pub platform_name: Option<String>,
    pub runtime_kind: String,
    pub args_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorLaunchTemplateOverride {
    pub emulator_name: String,
    pub platform_name: Option<String>,
    pub runtime_kind: String,
    pub command_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameLaunchTemplatePreview {
    pub launchbox_db_id: i64,
    pub platform_name: String,
    pub emulator_name: String,
    pub runtime_kind: String,
    pub is_prepared_install: bool,
    pub default_template: String,
    pub platform_command_template_override: Option<String>,
    pub game_command_template_override: Option<String>,
    pub effective_template: String,
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
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(None);
    };

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
    state: &mut AppState,
    launchbox_db_id: i64,
    emulator_name: &str,
) -> Result<(), String> {
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;

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
    state: &mut AppState,
    platform_name: &str,
    emulator_name: &str,
) -> Result<(), String> {
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;

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
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(());
    };

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
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(());
    };

    sqlx::query("DELETE FROM emulator_preferences WHERE platform_name = ?")
        .bind(platform_name)
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all emulator preferences (for settings UI)
pub async fn get_all_emulator_preferences(state: &AppState) -> Result<EmulatorPreferences, String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(EmulatorPreferences {
            game_preferences: Vec::new(),
            platform_preferences: Vec::new(),
        });
    };

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
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(());
    };

    sqlx::query("DELETE FROM emulator_preferences")
        .execute(db_pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Emulator Launch Profile Handlers
// ============================================================================

fn runtime_kind_label(is_retroarch_core: bool) -> &'static str {
    if is_retroarch_core {
        "retroarch"
    } else {
        "standalone"
    }
}

pub async fn get_all_emulator_launch_profiles(
    state: &AppState,
) -> Result<Vec<EmulatorLaunchProfile>, String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(Vec::new());
    };

    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT emulator_name, platform_name, runtime_kind, args_text
        FROM emulator_launch_profiles
        ORDER BY
            CASE WHEN platform_name = '' THEN 0 ELSE 1 END,
            platform_name,
            emulator_name,
            runtime_kind
        "#,
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(
            |(emulator_name, platform_name, runtime_kind, args_text)| EmulatorLaunchProfile {
                emulator_name,
                platform_name: if platform_name.is_empty() {
                    None
                } else {
                    Some(platform_name)
                },
                runtime_kind,
                args_text,
            },
        )
        .collect())
}

pub async fn get_emulator_launch_profile(
    state: &AppState,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> Result<Option<EmulatorLaunchProfile>, String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(None);
    };
    let runtime_kind = runtime_kind_label(is_retroarch_core);
    let normalized_platform = platform_name.unwrap_or("").trim();

    let exact: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT platform_name, args_text
        FROM emulator_launch_profiles
        WHERE emulator_name = ? AND platform_name = ? AND runtime_kind = ?
        LIMIT 1
        "#,
    )
    .bind(emulator_name)
    .bind(normalized_platform)
    .bind(runtime_kind)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let row = if let Some(row) = exact {
        Some(row)
    } else if !normalized_platform.is_empty() {
        sqlx::query_as(
            r#"
            SELECT platform_name, args_text
            FROM emulator_launch_profiles
            WHERE emulator_name = ? AND platform_name = '' AND runtime_kind = ?
            LIMIT 1
            "#,
        )
        .bind(emulator_name)
        .bind(runtime_kind)
        .fetch_optional(db_pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        None
    };

    Ok(row.map(|(platform_name, args_text)| EmulatorLaunchProfile {
        emulator_name: emulator_name.to_string(),
        platform_name: if platform_name.is_empty() {
            None
        } else {
            Some(platform_name)
        },
        runtime_kind: runtime_kind.to_string(),
        args_text,
    }))
}

pub async fn set_emulator_launch_profile(
    state: &mut AppState,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
    args_text: &str,
) -> Result<(), String> {
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;
    let runtime_kind = runtime_kind_label(is_retroarch_core);
    let normalized_platform = platform_name.unwrap_or("").trim();
    let normalized_args = args_text.trim();

    if normalized_args.is_empty() {
        return clear_emulator_launch_profile(
            state,
            emulator_name,
            platform_name,
            is_retroarch_core,
        )
        .await;
    }

    crate::emulator::parse_launch_args_text(normalized_args)
        .map_err(|e| format!("Invalid launch arguments: {}", e))?;

    sqlx::query(
        r#"
        INSERT INTO emulator_launch_profiles (emulator_name, platform_name, runtime_kind, args_text, updated_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(emulator_name, platform_name, runtime_kind) DO UPDATE SET
            args_text = excluded.args_text,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(emulator_name)
    .bind(normalized_platform)
    .bind(runtime_kind)
    .bind(normalized_args)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn clear_emulator_launch_profile(
    state: &AppState,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> Result<(), String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(());
    };
    let runtime_kind = runtime_kind_label(is_retroarch_core);
    let normalized_platform = platform_name.unwrap_or("").trim();

    sqlx::query(
        r#"
        DELETE FROM emulator_launch_profiles
        WHERE emulator_name = ? AND platform_name = ? AND runtime_kind = ?
        "#,
    )
    .bind(emulator_name)
    .bind(normalized_platform)
    .bind(runtime_kind)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn get_all_emulator_launch_template_overrides(
    state: &AppState,
) -> Result<Vec<EmulatorLaunchTemplateOverride>, String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(Vec::new());
    };

    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT emulator_name, platform_name, runtime_kind, command_template
        FROM emulator_launch_template_overrides
        ORDER BY
            CASE WHEN platform_name = '' THEN 0 ELSE 1 END,
            platform_name,
            emulator_name,
            runtime_kind
        "#,
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(
            |(emulator_name, platform_name, runtime_kind, command_template)| {
                EmulatorLaunchTemplateOverride {
                    emulator_name,
                    platform_name: if platform_name.is_empty() {
                        None
                    } else {
                        Some(platform_name)
                    },
                    runtime_kind,
                    command_template,
                }
            },
        )
        .collect())
}

async fn get_emulator_launch_template_override_internal(
    state: &AppState,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> Result<Option<String>, String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(None);
    };

    let runtime_kind = runtime_kind_label(is_retroarch_core);
    let normalized_platform = platform_name.unwrap_or("").trim();

    let exact: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT command_template
        FROM emulator_launch_template_overrides
        WHERE emulator_name = ? AND platform_name = ? AND runtime_kind = ?
        LIMIT 1
        "#,
    )
    .bind(emulator_name)
    .bind(normalized_platform)
    .bind(runtime_kind)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    if exact.is_some() || normalized_platform.is_empty() {
        return Ok(exact.map(|(template,)| template));
    }

    let fallback: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT command_template
        FROM emulator_launch_template_overrides
        WHERE emulator_name = ? AND platform_name = '' AND runtime_kind = ?
        LIMIT 1
        "#,
    )
    .bind(emulator_name)
    .bind(runtime_kind)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(fallback.map(|(template,)| template))
}

pub async fn set_emulator_launch_template_override(
    state: &mut AppState,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
    command_template: &str,
) -> Result<(), String> {
    let normalized_template = command_template.trim();
    if normalized_template.is_empty() {
        return clear_emulator_launch_template_override(
            state,
            emulator_name,
            platform_name,
            is_retroarch_core,
        )
        .await;
    }

    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;
    let runtime_kind = runtime_kind_label(is_retroarch_core);
    let normalized_platform = platform_name.unwrap_or("").trim();

    sqlx::query(
        r#"
        INSERT INTO emulator_launch_template_overrides (emulator_name, platform_name, runtime_kind, command_template, updated_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(emulator_name, platform_name, runtime_kind) DO UPDATE SET
            command_template = excluded.command_template,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(emulator_name)
    .bind(normalized_platform)
    .bind(runtime_kind)
    .bind(normalized_template)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn clear_emulator_launch_template_override(
    state: &AppState,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> Result<(), String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(());
    };

    let runtime_kind = runtime_kind_label(is_retroarch_core);
    let normalized_platform = platform_name.unwrap_or("").trim();

    sqlx::query(
        r#"
        DELETE FROM emulator_launch_template_overrides
        WHERE emulator_name = ? AND platform_name = ? AND runtime_kind = ?
        "#,
    )
    .bind(emulator_name)
    .bind(normalized_platform)
    .bind(runtime_kind)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn get_game_launch_template_override_internal(
    state: &AppState,
    launchbox_db_id: i64,
    emulator_name: &str,
    is_retroarch_core: bool,
) -> Result<Option<String>, String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(None);
    };

    let runtime_kind = runtime_kind_label(is_retroarch_core);

    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT command_template
        FROM game_launch_template_overrides
        WHERE launchbox_db_id = ? AND emulator_name = ? AND runtime_kind = ?
        LIMIT 1
        "#,
    )
    .bind(launchbox_db_id)
    .bind(emulator_name)
    .bind(runtime_kind)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|(template,)| template))
}

pub async fn set_game_launch_template_override(
    state: &mut AppState,
    launchbox_db_id: i64,
    emulator_name: &str,
    is_retroarch_core: bool,
    command_template: &str,
) -> Result<(), String> {
    let normalized_template = command_template.trim();
    if normalized_template.is_empty() {
        return clear_game_launch_template_override(
            state,
            launchbox_db_id,
            emulator_name,
            is_retroarch_core,
        )
        .await;
    }

    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;
    let runtime_kind = runtime_kind_label(is_retroarch_core);

    sqlx::query(
        r#"
        INSERT INTO game_launch_template_overrides (launchbox_db_id, emulator_name, runtime_kind, command_template, updated_at)
        VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(launchbox_db_id, emulator_name, runtime_kind) DO UPDATE SET
            command_template = excluded.command_template,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(launchbox_db_id)
    .bind(emulator_name)
    .bind(runtime_kind)
    .bind(normalized_template)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn clear_game_launch_template_override(
    state: &AppState,
    launchbox_db_id: i64,
    emulator_name: &str,
    is_retroarch_core: bool,
) -> Result<(), String> {
    let Some(db_pool) = state.db_pool.as_ref() else {
        return Ok(());
    };

    let runtime_kind = runtime_kind_label(is_retroarch_core);

    sqlx::query(
        r#"
        DELETE FROM game_launch_template_overrides
        WHERE launchbox_db_id = ? AND emulator_name = ? AND runtime_kind = ?
        "#,
    )
    .bind(launchbox_db_id)
    .bind(emulator_name)
    .bind(runtime_kind)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn get_game_launch_template_preview(
    state: &AppState,
    launchbox_db_id: i64,
    platform_name: &str,
    emulator_name: &str,
    is_retroarch_core: bool,
) -> Result<GameLaunchTemplatePreview, String> {
    let prepared_collection = if let Some(db_pool) = state.db_pool.as_ref() {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT file_path FROM game_files WHERE launchbox_db_id = ? LIMIT 1")
                .bind(launchbox_db_id)
                .fetch_optional(db_pool)
                .await
                .map_err(|e| e.to_string())?;

        row.and_then(|(file_path,)| {
            if crate::exo::should_use_prepared_install(platform_name, Path::new(&file_path)) {
                crate::exo::collection_for_platform(platform_name)
            } else {
                None
            }
        })
    } else {
        None
    };
    let is_prepared_install = prepared_collection.is_some();
    let default_template = if let Some(collection) = prepared_collection {
        crate::emulator::default_prepared_launch_command_template(
            emulator_name,
            collection,
            is_retroarch_core,
        )
    } else {
        crate::emulator::default_launch_command_template(
            emulator_name,
            Some(platform_name),
            is_retroarch_core,
        )
    };
    let platform_override = get_emulator_launch_template_override_internal(
        state,
        emulator_name,
        Some(platform_name),
        is_retroarch_core,
    )
    .await?;
    let game_override = get_game_launch_template_override_internal(
        state,
        launchbox_db_id,
        emulator_name,
        is_retroarch_core,
    )
    .await?;

    let effective_template = game_override
        .clone()
        .or_else(|| platform_override.clone())
        .unwrap_or_else(|| default_template.clone());

    Ok(GameLaunchTemplatePreview {
        launchbox_db_id,
        platform_name: platform_name.to_string(),
        emulator_name: emulator_name.to_string(),
        runtime_kind: runtime_kind_label(is_retroarch_core).to_string(),
        is_prepared_install,
        default_template,
        platform_command_template_override: platform_override,
        game_command_template_override: game_override,
        effective_template,
    })
}

async fn resolve_launch_template_override_for_standard_launch(
    state: &AppState,
    launchbox_db_id: Option<i64>,
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> Result<Option<String>, String> {
    if let Some(launchbox_db_id) = launchbox_db_id {
        if let Some(template) = get_game_launch_template_override_internal(
            state,
            launchbox_db_id,
            emulator_name,
            is_retroarch_core,
        )
        .await?
        {
            return Ok(Some(template));
        }
    }

    get_emulator_launch_template_override_internal(
        state,
        emulator_name,
        platform_name,
        is_retroarch_core,
    )
    .await
}

async fn resolve_game_launch_template_override(
    state: &AppState,
    launchbox_db_id: Option<i64>,
    emulator_name: &str,
    is_retroarch_core: bool,
) -> Result<Option<String>, String> {
    let Some(launchbox_db_id) = launchbox_db_id else {
        return Ok(None);
    };

    get_game_launch_template_override_internal(
        state,
        launchbox_db_id,
        emulator_name,
        is_retroarch_core,
    )
    .await
}

async fn resolve_emulator_launch_args(
    state: &AppState,
    emulator: &EmulatorInfo,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> Result<Vec<emulator::LaunchArg>, String> {
    let Some(profile) =
        get_emulator_launch_profile(state, &emulator.name, platform_name, is_retroarch_core)
            .await?
    else {
        return Ok(Vec::new());
    };

    crate::emulator::parse_launch_args_text(&profile.args_text)
        .map_err(|e| format!("Invalid launch arguments for {}: {}", emulator.name, e))
}

async fn build_launch_configuration(
    state: &AppState,
    launchbox_db_id: Option<i64>,
    emulator: &EmulatorInfo,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
    allow_platform_template_override: bool,
) -> Result<emulator::LaunchConfiguration, String> {
    let legacy_args =
        resolve_emulator_launch_args(state, emulator, platform_name, is_retroarch_core).await?;

    let command_template_override = if allow_platform_template_override {
        resolve_launch_template_override_for_standard_launch(
            state,
            launchbox_db_id,
            &emulator.name,
            platform_name,
            is_retroarch_core,
        )
        .await?
    } else {
        resolve_game_launch_template_override(
            state,
            launchbox_db_id,
            &emulator.name,
            is_retroarch_core,
        )
        .await?
    };

    Ok(emulator::LaunchConfiguration {
        command_template_override,
        legacy_args,
    })
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
    let canonical_platform_name = canonicalize_legacy_platform_name(platform_name);

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
    .bind(canonical_platform_name)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    emulators.retain(emulator::is_emulator_visible_on_current_os);

    maybe_append_exodos_scummvm(pool, canonical_platform_name, os, true, &mut emulators).await?;

    // Create separate entries for RetroArch cores and standalone emulators
    // An emulator with both will appear twice in the list
    let mut results: Vec<EmulatorWithStatus> = Vec::new();
    let mut retroarch_entries: Vec<EmulatorWithStatus> = Vec::new();
    let mut standalone_entries: Vec<EmulatorWithStatus> = Vec::new();

    for emulator in emulators {
        let is_exodos_scummvm = canonical_platform_name == "MS-DOS" && emulator.name == "ScummVM";
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
                canonical_platform_name,
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

pub async fn get_emulator_updates(state: &AppState) -> Result<Vec<EmulatorUpdate>, String> {
    let pool = state
        .emulators_db_pool
        .as_ref()
        .ok_or_else(|| "Emulators database not initialized".to_string())?;

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

    emulator::get_available_updates(&emulators).await
}

pub async fn update_emulator(update_key: &str) -> Result<(), String> {
    emulator::apply_update(update_key).await
}

pub async fn install_firmware_for_emulator(
    state: &mut AppState,
    emulator: &EmulatorInfo,
    platform_name: &str,
    is_retroarch_core: bool,
) -> Result<Vec<crate::firmware::FirmwareStatus>, String> {
    let canonical_platform_name = canonicalize_legacy_platform_name(platform_name);
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
        canonical_platform_name,
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
    let canonical_platform_name = canonicalize_legacy_platform_name(platform_name);
    crate::firmware::ensure_runtime_firmware(
        settings,
        db_pool,
        minerva_pool,
        emulator,
        canonical_platform_name,
        is_retroarch_core,
    )
    .await?;

    crate::firmware::get_firmware_status(
        settings,
        db_pool,
        emulator,
        canonical_platform_name,
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
        let runtime_platform = canonicalize_legacy_platform_name(platform);
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
                    });
                }
            }
        };

        if crate::exo::should_use_prepared_install(runtime_platform, Path::new(&resolved_rom_path))
        {
            let db_pool = crate::state::ensure_user_db(state)
                .await
                .map_err(|e| e.to_string())?;

            let prepared = match crate::exo::prepare_install_for_game(
                &settings,
                db_pool,
                db_id,
                runtime_platform,
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
                    });
                }
            };

            let launch_configuration = build_launch_configuration(
                state,
                Some(db_id),
                emulator,
                Some(platform),
                as_retroarch_core,
                false,
            )
            .await?;

            return match emulator::launch_prepared_install(
                emulator,
                prepared.collection,
                &prepared.install_root,
                &prepared.launch_config_path,
                as_retroarch_core,
                &launch_configuration,
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
            runtime_platform,
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
            runtime_platform,
            as_retroarch_core,
        )
        .await
        .map_err(|e| e.to_string())?;
        let mut launch_configuration = build_launch_configuration(
            state,
            Some(db_id),
            emulator,
            Some(platform),
            as_retroarch_core,
            true,
        )
        .await?;
        launch_configuration
            .legacy_args
            .extend(firmware_launch_args);

        match emulator::launch_emulator(
            emulator,
            Some(&resolved_rom_path),
            Some(runtime_platform),
            as_retroarch_core,
            &launch_configuration,
        ) {
            Ok(pid) => {
                return Ok(LaunchResult {
                    success: true,
                    pid: Some(pid),
                    error: None,
                });
            }
            Err(e) => {
                return Ok(LaunchResult {
                    success: false,
                    pid: None,
                    error: Some(e),
                });
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

    let launch_configuration = build_launch_configuration(
        state,
        launchbox_db_id,
        emulator,
        None,
        as_retroarch_core,
        true,
    )
    .await?;

    match emulator::launch_emulator(
        emulator,
        Some(rom_path),
        None,
        as_retroarch_core,
        &launch_configuration,
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
    }
}

/// Launch an emulator (without a ROM)
/// If `is_retroarch_core` is true, launch via RetroArch; otherwise launch standalone
pub fn launch_emulator_only(
    emulator: &EmulatorInfo,
    is_retroarch_core: bool,
) -> Result<LaunchResult, String> {
    match emulator::launch_emulator(
        emulator,
        None,
        None,
        is_retroarch_core,
        &emulator::LaunchConfiguration::default(),
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

    if let Some(row) = row {
        return Ok(Some((|(
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
        })(row)));
    }

    recover_missing_laserdisc_game_file(state, db_pool, launchbox_db_id).await
}

async fn recover_missing_laserdisc_game_file(
    state: &AppState,
    db_pool: &sqlx::SqlitePool,
    launchbox_db_id: i64,
) -> Result<Option<GameFile>, String> {
    let games_pool = match state.games_db_pool.as_ref() {
        Some(pool) => pool,
        None => return Ok(None),
    };

    let game_row: Option<(String, String)> = sqlx::query_as(
        "SELECT g.title, p.name FROM games g JOIN platforms p ON g.platform_id = p.id WHERE g.launchbox_db_id = ?",
    )
    .bind(launchbox_db_id)
    .fetch_optional(games_pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some((game_title, platform)) = game_row else {
        return Ok(None);
    };
    if canonicalize_legacy_platform_name(&platform) != "Arcade" {
        return Ok(None);
    }

    let completed_job: Option<(String,)> = sqlx::query_as(
        "SELECT status_message FROM graboid_jobs WHERE launchbox_db_id = ? AND status = 'completed' ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some((status_message,)) = completed_job else {
        return Ok(None);
    };

    let collection_root = state
        .settings
        .get_import_directory()
        .join("Arcade")
        .join("Minerva_Myrient")
        .join("Laserdisc Collection");

    let recovered_path = if status_message.contains("Hypseus bundle") {
        recover_hypseus_laserdisc_framefile(
            &collection_root.join("Hypseus Singe"),
            launchbox_db_id,
            &game_title,
        )
    } else if status_message.contains("ROM + CHD") {
        recover_mame_laserdisc_rom(&collection_root.join("MAME"), launchbox_db_id, &game_title)
    } else {
        None
    };

    let Some(file_path) = recovered_path else {
        return Ok(None);
    };
    let file_size = std::fs::metadata(&file_path)
        .ok()
        .map(|meta| meta.len() as i64);
    let file_path_text = file_path.display().to_string();

    sqlx::query(
        "INSERT OR REPLACE INTO game_files (launchbox_db_id, game_title, platform, file_path, file_size, import_source) VALUES (?, ?, ?, ?, ?, 'minerva')",
    )
    .bind(launchbox_db_id)
    .bind(&game_title)
    .bind(&platform)
    .bind(&file_path_text)
    .bind(file_size)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Some(GameFile {
        launchbox_db_id,
        game_title,
        platform,
        file_path: file_path_text,
        file_size,
        imported_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        import_source: "minerva".to_string(),
        graboid_job_id: None,
    }))
}

fn recover_hypseus_laserdisc_framefile(
    root: &Path,
    launchbox_db_id: i64,
    game_title: &str,
) -> Option<PathBuf> {
    let resolved_lookup_title = crate::images::emumovies::resolve_arcade_download_lookup_name(
        "Arcade",
        game_title,
        Some(launchbox_db_id),
    )
    .into_owned();
    let lookup_titles = torrent_match_lookup_titles(
        Some("Arcade"),
        game_title,
        &resolved_lookup_title,
        Some("Laserdisc Collection"),
        Some("Hypseus Singe"),
    );

    let mut bundles: BTreeMap<
        String,
        (
            Option<PathBuf>,
            Option<PathBuf>,
            Option<PathBuf>,
            Option<PathBuf>,
            Option<PathBuf>,
        ),
    > = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Some(pseudo_path) = laserdisc_relative_path(entry.path()) else {
            continue;
        };
        let Some((bundle_key, kind)) = parse_arcade_hypseus_laserdisc_asset(&pseudo_path) else {
            continue;
        };
        let bundle = bundles
            .entry(bundle_key)
            .or_insert((None, None, None, None, None));
        match kind {
            ArcadeHypseusLaserdiscAssetKind::RomZip => bundle.1 = Some(entry.path().to_path_buf()),
            ArcadeHypseusLaserdiscAssetKind::Data => bundle.2 = Some(entry.path().to_path_buf()),
            ArcadeHypseusLaserdiscAssetKind::FrameText => {
                bundle.0 = Some(entry.path().to_path_buf())
            }
            ArcadeHypseusLaserdiscAssetKind::Video => bundle.3 = Some(entry.path().to_path_buf()),
            ArcadeHypseusLaserdiscAssetKind::Audio => bundle.4 = Some(entry.path().to_path_buf()),
        }
    }

    bundles
        .into_iter()
        .filter_map(|(bundle_key, (framefile, rom_zip, data, video, audio))| {
            let framefile = framefile?;
            let _rom_zip = rom_zip?;
            let _data = data?;
            let _video = video?;
            let _audio = audio?;
            let best_score = lookup_titles
                .iter()
                .filter_map(|lookup_title| {
                    let normalized_query = crate::tags::normalize_title_for_matching(lookup_title);
                    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();
                    let significant_query_words = significant_match_words(&normalized_query);
                    score_match_name(
                        &bundle_key,
                        &normalized_query,
                        &query_words,
                        &significant_query_words,
                    )
                    .map(|score| score.score)
                })
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
            Some((best_score, framefile))
        })
        .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, framefile)| framefile)
}

fn recover_mame_laserdisc_rom(
    root: &Path,
    launchbox_db_id: i64,
    game_title: &str,
) -> Option<PathBuf> {
    let resolved_lookup_title = crate::images::emumovies::resolve_arcade_download_lookup_name(
        "Arcade",
        game_title,
        Some(launchbox_db_id),
    )
    .into_owned();
    let lookup_titles = torrent_match_lookup_titles(
        Some("Arcade"),
        game_title,
        &resolved_lookup_title,
        Some("Laserdisc Collection"),
        Some("MAME"),
    );

    let mut bundles: BTreeMap<String, (Option<PathBuf>, Option<PathBuf>)> = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Some(pseudo_path) = laserdisc_relative_path(entry.path()) else {
            continue;
        };
        let Some((romset_name, kind)) = parse_arcade_mame_laserdisc_asset(&pseudo_path) else {
            continue;
        };
        let bundle = bundles.entry(romset_name).or_insert((None, None));
        match kind {
            ArcadeMameLaserdiscAssetKind::RomZip => bundle.0 = Some(entry.path().to_path_buf()),
            ArcadeMameLaserdiscAssetKind::Chd => bundle.1 = Some(entry.path().to_path_buf()),
        }
    }

    bundles
        .into_iter()
        .filter_map(|(bundle_key, (rom_zip, chd))| {
            let rom_zip = rom_zip?;
            let _chd = chd?;
            let best_score = lookup_titles
                .iter()
                .filter_map(|lookup_title| {
                    let normalized_query = crate::tags::normalize_title_for_matching(lookup_title);
                    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();
                    let significant_query_words = significant_match_words(&normalized_query);
                    score_match_name(
                        &bundle_key,
                        &normalized_query,
                        &query_words,
                        &significant_query_words,
                    )
                    .map(|score| score.score)
                })
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;
            Some((best_score, rom_zip))
        })
        .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, rom_zip)| rom_zip)
}

fn laserdisc_relative_path(path: &Path) -> Option<String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let (_, suffix) = normalized.split_once("/Laserdisc Collection/")?;
    Some(format!("Laserdisc Collection/{suffix}"))
}

pub async fn uninstall_game(state: &mut AppState, launchbox_db_id: i64) -> Result<(), String> {
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;

    let game_file_row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT platform, file_path, import_source FROM game_files WHERE launchbox_db_id = ?",
    )
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

    if let Some((platform, file_path, import_source)) = game_file_row {
        if import_source == "minerva" {
            remove_path_if_exists(std::path::Path::new(&file_path)).await?;
            remove_arcade_laserdisc_companion_assets(&platform, &file_path).await?;
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

async fn remove_arcade_laserdisc_companion_assets(
    platform: &str,
    file_path: &str,
) -> Result<(), String> {
    if canonicalize_legacy_platform_name(platform) != "Arcade" {
        return Ok(());
    }

    let path = Path::new(file_path);
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok(());
    };
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    let companion_dir = parent.join(stem);
    let companion_chd = companion_dir.join(format!("{stem}.chd"));
    if companion_chd.exists() {
        remove_path_if_exists(&companion_dir).await?;
    }

    if matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("txt" | "m2v" | "ogg")
    ) {
        for extension in ["txt", "m2v", "ogg"] {
            let sibling = parent.join(format!("{stem}.{extension}"));
            if sibling != path && sibling.exists() {
                remove_path_if_exists(&sibling).await?;
            }
        }
    }

    Ok(())
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

fn expand_arcade_laserdisc_roms(
    roms: Vec<MinervaRom>,
    canonical_platform_name: &str,
) -> Vec<MinervaRom> {
    if canonical_platform_name != "Arcade" {
        return roms;
    }

    let mut expanded = roms;
    let has_hypseus_row = expanded.iter().any(|rom| {
        rom.collection.eq_ignore_ascii_case("Laserdisc Collection")
            && rom.minerva_platform.eq_ignore_ascii_case("Hypseus Singe")
    });

    if !has_hypseus_row {
        if let Some(template) = expanded.iter().find(|rom| {
            rom.collection.eq_ignore_ascii_case("Laserdisc Collection")
                && rom.minerva_platform.eq_ignore_ascii_case("MAME")
        }) {
            let mut hypseus = template.clone();
            hypseus.minerva_platform = "Hypseus Singe".to_string();
            expanded.push(hypseus);
        }
    }

    expanded
}

fn canonicalize_legacy_platform_name(name: &str) -> &str {
    let legacy = match name.trim() {
        "Arduboy Inc - Arduboy" => "Arduboy",
        "Atari - 8-bit Family" => "Atari 800",
        other => other,
    };
    crate::arcade::canonicalize_platform_name(legacy)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArcadeMameLaserdiscAssetKind {
    RomZip,
    Chd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArcadeHypseusLaserdiscAssetKind {
    RomZip,
    Data,
    FrameText,
    Video,
    Audio,
}

#[derive(Debug, Clone)]
struct ArcadeMameLaserdiscAsset {
    torrent_url: String,
    file_index: usize,
    filename: String,
    size: u64,
}

#[derive(Debug, Clone)]
struct ArcadeMameLaserdiscPlan {
    romset_name: String,
    primary_asset: ArcadeMameLaserdiscAsset,
    chd_asset: ArcadeMameLaserdiscAsset,
}

#[derive(Debug, Clone)]
struct ArcadeHypseusLaserdiscPlan {
    bundle_name: String,
    representative_asset: ArcadeMameLaserdiscAsset,
    assets: Vec<ArcadeMameLaserdiscAsset>,
}

#[derive(Debug, Clone)]
enum ArcadeLaserdiscPlan {
    Mame(ArcadeMameLaserdiscPlan),
    Hypseus(ArcadeHypseusLaserdiscPlan),
}

#[derive(Debug)]
enum MinervaBatchDownloadError {
    Failed(String),
    Cancelled(String),
}

fn normalize_torrent_listing_path(path: &str) -> String {
    path.trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn arcade_mame_laserdisc_rom_suffix(romset_name: &str) -> String {
    format!("laserdisc collection/mame/roms/{romset_name}.zip")
}

fn arcade_mame_laserdisc_chd_suffix(romset_name: &str) -> String {
    format!("laserdisc collection/mame/chd/{romset_name}/{romset_name}.chd")
}

fn parse_arcade_mame_laserdisc_asset(path: &str) -> Option<(String, ArcadeMameLaserdiscAssetKind)> {
    let normalized = normalize_torrent_listing_path(path);

    if let Some(file_name) = normalized
        .strip_prefix("laserdisc collection/mame/roms/")
        .filter(|remainder| !remainder.contains('/'))
    {
        let romset_name = file_name.strip_suffix(".zip")?;
        if !romset_name.is_empty() {
            return Some((
                romset_name.to_string(),
                ArcadeMameLaserdiscAssetKind::RomZip,
            ));
        }
    }

    if let Some(remainder) = normalized.strip_prefix("laserdisc collection/mame/chd/") {
        let mut parts = remainder.split('/');
        let romset_name = parts.next()?;
        let file_name = parts.next()?;
        if parts.next().is_none() && file_name == format!("{romset_name}.chd") {
            return Some((romset_name.to_string(), ArcadeMameLaserdiscAssetKind::Chd));
        }
    }

    None
}

fn parse_arcade_hypseus_laserdisc_asset(
    path: &str,
) -> Option<(String, ArcadeHypseusLaserdiscAssetKind)> {
    let normalized = normalize_torrent_listing_path(path);
    if !normalized.starts_with("laserdisc collection/") || normalized.contains("/mame/") {
        return None;
    }

    let path = Path::new(&normalized);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())?;
    let kind = match extension.as_str() {
        "zip" => ArcadeHypseusLaserdiscAssetKind::RomZip,
        "dat" => ArcadeHypseusLaserdiscAssetKind::Data,
        "txt" => ArcadeHypseusLaserdiscAssetKind::FrameText,
        "m2v" => ArcadeHypseusLaserdiscAssetKind::Video,
        "ogg" => ArcadeHypseusLaserdiscAssetKind::Audio,
        _ => return None,
    };
    let stem = path.file_stem()?.to_str()?;

    let components = path
        .iter()
        .filter_map(|component| component.to_str())
        .collect::<Vec<_>>();
    let anchor_idx = match kind {
        ArcadeHypseusLaserdiscAssetKind::RomZip => components
            .iter()
            .rposition(|component| component.eq_ignore_ascii_case("roms"))?,
        _ => {
            let mut parent = path.parent()?;
            if parent
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| matches!(value, "video" | "audio" | "sound"))
            {
                parent = parent.parent()?;
            }
            parent
                .iter()
                .filter_map(|component| component.to_str())
                .collect::<Vec<_>>()
                .iter()
                .rposition(|component| {
                    component.eq_ignore_ascii_case("vldp")
                        || component.eq_ignore_ascii_case("singe")
                })?
        }
    };
    let prefix = components
        .iter()
        .take(anchor_idx)
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    Some((format!("{prefix}/{stem}"), kind))
}

fn find_torrent_file_by_suffix(
    files: &[crate::torrent::TorrentFileInfo],
    expected_suffix: &str,
) -> Option<crate::torrent::TorrentFileInfo> {
    let expected = normalize_torrent_listing_path(expected_suffix);
    files
        .iter()
        .find(|file| normalize_torrent_listing_path(&file.filename).ends_with(&expected))
        .cloned()
}

async fn resolve_game_platform_id(
    games_db: &sqlx::SqlitePool,
    launchbox_db_id: i64,
) -> Result<Option<i64>, String> {
    sqlx::query_scalar::<_, i64>("SELECT platform_id FROM games WHERE launchbox_db_id = ? LIMIT 1")
        .bind(launchbox_db_id)
        .fetch_optional(games_db)
        .await
        .map_err(|e| e.to_string())
}

async fn find_arcade_mame_laserdisc_asset(
    state: &AppState,
    launchbox_db_id: i64,
    selected_torrent_url: &str,
    current_files: &[crate::torrent::TorrentFileInfo],
    expected_suffix: &str,
    _expected_kind: ArcadeMameLaserdiscAssetKind,
) -> Result<Option<ArcadeMameLaserdiscAsset>, String> {
    if let Some(file) = find_torrent_file_by_suffix(current_files, expected_suffix) {
        return Ok(Some(ArcadeMameLaserdiscAsset {
            torrent_url: selected_torrent_url.to_string(),
            file_index: file.index,
            filename: file.filename,
            size: file.size,
        }));
    }

    let games_db = state
        .games_db_pool
        .as_ref()
        .ok_or_else(|| "Games database not available".to_string())?;
    let Some(platform_id) = resolve_game_platform_id(games_db, launchbox_db_id).await? else {
        return Ok(None);
    };

    let candidates = search_minerva(state, Some(launchbox_db_id), None, Some(platform_id)).await?;
    for rom in candidates {
        if rom.torrent_url == selected_torrent_url {
            continue;
        }

        let files = crate::torrent::get_torrent_file_listing(&rom.torrent_url)
            .await
            .map_err(|e| format!("Failed to inspect Minerva torrent contents: {e}"))?;
        if let Some(file) = find_torrent_file_by_suffix(&files, expected_suffix) {
            return Ok(Some(ArcadeMameLaserdiscAsset {
                torrent_url: rom.torrent_url,
                file_index: file.index,
                filename: file.filename,
                size: file.size,
            }));
        }
    }

    Ok(None)
}

async fn build_arcade_mame_laserdisc_plan(
    state: &AppState,
    launchbox_db_id: i64,
    selected_torrent_url: &str,
    current_files: &[crate::torrent::TorrentFileInfo],
    selected_file: &crate::torrent::TorrentFileInfo,
) -> Result<Option<ArcadeMameLaserdiscPlan>, String> {
    let Some((romset_name, selected_kind)) =
        parse_arcade_mame_laserdisc_asset(&selected_file.filename)
    else {
        return Ok(None);
    };

    let rom_suffix = arcade_mame_laserdisc_rom_suffix(&romset_name);
    let chd_suffix = arcade_mame_laserdisc_chd_suffix(&romset_name);

    let primary_asset = match selected_kind {
        ArcadeMameLaserdiscAssetKind::RomZip => ArcadeMameLaserdiscAsset {
            torrent_url: selected_torrent_url.to_string(),
            file_index: selected_file.index,
            filename: selected_file.filename.clone(),
            size: selected_file.size,
        },
        ArcadeMameLaserdiscAssetKind::Chd => find_arcade_mame_laserdisc_asset(
            state,
            launchbox_db_id,
            selected_torrent_url,
            current_files,
            &rom_suffix,
            ArcadeMameLaserdiscAssetKind::RomZip,
        )
        .await?
        .ok_or_else(|| {
            format!(
                "This MAME laserdisc title needs both {}.zip and {}.chd, but Lunchbox could not find the ROM zip in Minerva.",
                romset_name, romset_name
            )
        })?,
    };

    let chd_asset = match selected_kind {
        ArcadeMameLaserdiscAssetKind::Chd => ArcadeMameLaserdiscAsset {
            torrent_url: selected_torrent_url.to_string(),
            file_index: selected_file.index,
            filename: selected_file.filename.clone(),
            size: selected_file.size,
        },
        ArcadeMameLaserdiscAssetKind::RomZip => find_arcade_mame_laserdisc_asset(
            state,
            launchbox_db_id,
            selected_torrent_url,
            current_files,
            &chd_suffix,
            ArcadeMameLaserdiscAssetKind::Chd,
        )
        .await?
        .ok_or_else(|| {
            format!(
                "This MAME laserdisc title needs both {}.zip and {}.chd, but Lunchbox could not find the companion CHD in Minerva.",
                romset_name, romset_name
            )
        })?,
    };

    Ok(Some(ArcadeMameLaserdiscPlan {
        romset_name,
        primary_asset,
        chd_asset,
    }))
}

fn build_arcade_hypseus_laserdisc_plan(
    selected_torrent_url: &str,
    current_files: &[crate::torrent::TorrentFileInfo],
    selected_file: &crate::torrent::TorrentFileInfo,
) -> Option<ArcadeHypseusLaserdiscPlan> {
    let (bundle_key, _) = parse_arcade_hypseus_laserdisc_asset(&selected_file.filename)?;

    let mut data_asset = None;
    let mut text_asset = None;
    let mut video_asset = None;
    let mut audio_asset = None;
    let mut rom_zip_asset = None;

    for file in current_files {
        let Some((candidate_key, kind)) = parse_arcade_hypseus_laserdisc_asset(&file.filename)
        else {
            continue;
        };
        if candidate_key != bundle_key {
            continue;
        }

        let asset = ArcadeMameLaserdiscAsset {
            torrent_url: selected_torrent_url.to_string(),
            file_index: file.index,
            filename: file.filename.clone(),
            size: file.size,
        };
        match kind {
            ArcadeHypseusLaserdiscAssetKind::RomZip => rom_zip_asset = Some(asset),
            ArcadeHypseusLaserdiscAssetKind::Data => data_asset = Some(asset),
            ArcadeHypseusLaserdiscAssetKind::FrameText => text_asset = Some(asset),
            ArcadeHypseusLaserdiscAssetKind::Video => video_asset = Some(asset),
            ArcadeHypseusLaserdiscAssetKind::Audio => audio_asset = Some(asset),
        }
    }

    let representative_asset = text_asset
        .clone()
        .or_else(|| data_asset.clone())
        .or_else(|| video_asset.clone())
        .or_else(|| audio_asset.clone())
        .or_else(|| rom_zip_asset.clone())?;
    let assets = vec![
        rom_zip_asset?,
        data_asset?,
        text_asset?,
        video_asset?,
        audio_asset?,
    ];
    let bundle_name = Path::new(&bundle_key)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&bundle_key)
        .to_string();

    Some(ArcadeHypseusLaserdiscPlan {
        bundle_name,
        representative_asset,
        assets,
    })
}

async fn build_arcade_laserdisc_plan(
    state: &AppState,
    launchbox_db_id: i64,
    selected_torrent_url: &str,
    current_files: &[crate::torrent::TorrentFileInfo],
    selected_file: &crate::torrent::TorrentFileInfo,
) -> Result<Option<ArcadeLaserdiscPlan>, String> {
    if let Some(plan) = build_arcade_mame_laserdisc_plan(
        state,
        launchbox_db_id,
        selected_torrent_url,
        current_files,
        selected_file,
    )
    .await?
    {
        return Ok(Some(ArcadeLaserdiscPlan::Mame(plan)));
    }

    Ok(
        build_arcade_hypseus_laserdisc_plan(selected_torrent_url, current_files, selected_file)
            .map(ArcadeLaserdiscPlan::Hypseus),
    )
}

async fn download_minerva_batch(
    settings: &AppSettings,
    job_id: &str,
    download_dir: &Path,
    torrent_url: &str,
    file_index: usize,
    target_filename: &str,
    target_size: u64,
    progress_offset: u64,
    progress_total: u64,
    status_message: &str,
) -> Result<PathBuf, MinervaBatchDownloadError> {
    let client = crate::torrent::create_client(settings).map_err(|e| {
        MinervaBatchDownloadError::Failed(format!("qBittorrent configuration error: {e}"))
    })?;
    let client_job_id = client
        .add_torrent(torrent_url, download_dir, Some(vec![file_index]))
        .await
        .map_err(|e| MinervaBatchDownloadError::Failed(format!("Failed to start download: {e}")))?;
    crate::torrent::set_client_job_id(job_id, &client_job_id);

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(7200);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        if start.elapsed() > timeout {
            return Err(MinervaBatchDownloadError::Failed(
                "Download timed out after 2 hours".to_string(),
            ));
        }

        let progress = match client.get_progress(&client_job_id).await {
            Ok(Some(progress)) => progress,
            Ok(None) => continue,
            Err(e) => {
                return Err(MinervaBatchDownloadError::Failed(format!(
                    "Failed to read qBittorrent progress: {e}"
                )));
            }
        };

        let downloaded = progress_offset.saturating_add(progress.downloaded_bytes.min(target_size));
        let progress_percent = if progress_total == 0 {
            progress.progress_percent
        } else {
            (downloaded as f64 / progress_total as f64) * 100.0
        };

        crate::torrent::update_progress(
            job_id,
            match progress.status {
                crate::torrent::DownloadStatus::Completed => {
                    crate::torrent::DownloadStatus::Downloading
                }
                other => other,
            },
            progress_percent.min(99.9),
            progress.download_speed,
            downloaded,
            progress_total,
            status_message,
        );

        match progress.status {
            crate::torrent::DownloadStatus::Completed => break,
            crate::torrent::DownloadStatus::Failed => {
                return Err(MinervaBatchDownloadError::Failed(progress.status_message));
            }
            crate::torrent::DownloadStatus::Cancelled => {
                return Err(MinervaBatchDownloadError::Cancelled(
                    progress.status_message,
                ));
            }
            _ => {}
        }
    }

    if let Some(path) = client
        .get_downloaded_file_path(&client_job_id, file_index, download_dir)
        .await
        .map_err(|e| {
            MinervaBatchDownloadError::Failed(format!(
                "Failed to locate downloaded file on disk: {e}"
            ))
        })?
    {
        return Ok(path);
    }

    locate_downloaded_file(download_dir, target_filename).ok_or_else(|| {
        MinervaBatchDownloadError::Failed(
            "Download finished, but the selected ROM file could not be found on disk.".to_string(),
        )
    })
}

async fn persist_graboid_job_status(
    db_path: Option<&PathBuf>,
    job_id: &str,
    status: &str,
    message: &str,
) {
    if let Some(db_path) = db_path {
        if let Ok(pool) = crate::db::init_pool(db_path).await {
            let _ = sqlx::query(
                "UPDATE graboid_jobs SET status = ?, status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(status)
            .bind(message)
            .bind(job_id)
            .execute(&pool)
            .await;
        }
    }
}

fn staging_link_mode(mode: &str) -> &str {
    match mode {
        "leave_in_place" => "symlink",
        other => other,
    }
}

fn stage_arcade_mame_laserdisc_layout(
    download_dir: &Path,
    plan: &ArcadeMameLaserdiscPlan,
    primary_source: &Path,
    chd_source: &Path,
    file_link_mode: &str,
) -> Result<PathBuf, String> {
    let primary_target = download_dir.join(format!("{}.zip", plan.romset_name));
    let chd_target = download_dir
        .join(&plan.romset_name)
        .join(format!("{}.chd", plan.romset_name));
    let link_mode = staging_link_mode(file_link_mode);

    crate::torrent::link_file_to_target(primary_source, &primary_target, link_mode)
        .map_err(|e| format!("Failed to stage {}: {}", primary_target.display(), e))?;
    crate::torrent::link_file_to_target(chd_source, &chd_target, link_mode)
        .map_err(|e| format!("Failed to stage {}: {}", chd_target.display(), e))?;

    Ok(primary_target)
}

fn locate_arcade_hypseus_bundle_framefile(
    download_dir: &Path,
    plan: &ArcadeHypseusLaserdiscPlan,
) -> Result<PathBuf, String> {
    let mut framefile_path = None;
    let mut missing_assets = Vec::new();

    for asset in &plan.assets {
        let Some(path) = locate_downloaded_file(download_dir, &asset.filename) else {
            missing_assets.push(asset.filename.clone());
            continue;
        };

        if matches!(
            parse_arcade_hypseus_laserdisc_asset(&asset.filename).map(|(_, kind)| kind),
            Some(ArcadeHypseusLaserdiscAssetKind::FrameText)
        ) {
            framefile_path = Some(path);
        }
    }

    if !missing_assets.is_empty() {
        let missing_list = missing_assets
            .iter()
            .map(|asset| {
                Path::new(asset)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(asset)
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Download finished, but the Hypseus bundle is incomplete. Missing: {missing_list}"
        ));
    }

    framefile_path.ok_or_else(|| {
        "Download finished, but Lunchbox could not locate the Hypseus framefile on disk."
            .to_string()
    })
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

    let roms = rows
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
        .collect();

    Ok(expand_arcade_laserdisc_roms(roms, &canonical_platform_name))
}

/// Start a minerva ROM download via torrent
pub async fn start_minerva_download(
    state: &mut AppState,
    input: StartMinervaDownloadInput,
) -> Result<ImportJob, String> {
    let torrent_url = input.torrent_url.clone();
    let canonical_platform = canonicalize_legacy_platform_name(&input.platform).to_string();
    let file_index = input.file_index;
    let download_mode = input.download_mode;
    let files = crate::torrent::get_torrent_file_listing(&torrent_url)
        .await
        .map_err(|e| format!("Failed to parse torrent: {e}"))?;
    let selection_plan = if matches!(download_mode, MinervaDownloadMode::GameOnly) {
        file_index
            .and_then(|idx| crate::exo::plan_related_downloads(&canonical_platform, idx, &files))
    } else {
        None
    };
    let representative_index = selection_plan
        .as_ref()
        .map(|plan| plan.representative_index)
        .or(file_index);
    let target_file = representative_index
        .and_then(|idx| files.iter().find(|f| f.index == idx))
        .cloned();
    let target_filename = target_file
        .as_ref()
        .map(|f| f.filename.clone())
        .unwrap_or_default();
    let target_size = target_file.as_ref().map(|f| f.size).unwrap_or(0);

    if matches!(download_mode, MinervaDownloadMode::GameOnly) && target_file.is_none() {
        return Err("No matching file was selected for this Minerva torrent.".to_string());
    }

    let arcade_laserdisc_plan = if matches!(download_mode, MinervaDownloadMode::GameOnly) {
        if let Some(target_file) = target_file.as_ref() {
            build_arcade_laserdisc_plan(
                state,
                input.launchbox_db_id,
                &torrent_url,
                &files,
                target_file,
            )
            .await?
        } else {
            None
        }
    } else {
        None
    };

    // Create import job
    let job_id = uuid::Uuid::new_v4().to_string();
    let db_pool = crate::state::ensure_user_db(state)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO graboid_jobs (id, launchbox_db_id, game_title, platform, status, progress_percent, status_message)
         VALUES (?, ?, ?, ?, 'in_progress', 0, 'Preparing download...')"
    )
    .bind(&job_id)
    .bind(input.launchbox_db_id)
    .bind(&input.game_title)
    .bind(&canonical_platform)
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let rom_dir = state.settings.get_rom_directory();
    let file_link_mode = state.settings.torrent.file_link_mode.clone();
    let download_dir = match download_mode {
        MinervaDownloadMode::GameOnly => state
            .settings
            .get_import_directory()
            .join(&canonical_platform),
        MinervaDownloadMode::FullTorrent => {
            let title_component = sanitize_download_directory_component(&input.game_title);
            let job_suffix = job_id.chars().take(8).collect::<String>();
            state
                .settings
                .get_torrent_library_directory()
                .join(&canonical_platform)
                .join(format!("{title_component}-{job_suffix}"))
        }
    };
    std::fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;

    // Spawn background download task
    let job_id_bg = job_id.clone();
    let game_title = input.game_title.clone();
    let platform = canonical_platform.clone();
    let launchbox_db_id = input.launchbox_db_id;
    let db_path = state.user_db_path.clone();
    let app_settings = state.settings.clone();
    let file_link_mode_bg = file_link_mode.clone();
    let rom_dir_bg = rom_dir.clone();
    let download_dir_bg = download_dir.clone();
    let files_bg = files.clone();
    let selection_plan_bg = selection_plan.clone();
    let target_file_bg = target_file.clone();
    let target_filename_bg = target_filename.clone();
    let target_size_bg = target_size;
    let arcade_laserdisc_plan_bg = arcade_laserdisc_plan.clone();

    tokio::spawn(async move {
        // Step 1: Initialize the prepared download plan
        crate::torrent::update_progress(
            &job_id_bg,
            crate::torrent::DownloadStatus::FetchingTorrent,
            0.0,
            0,
            0,
            0,
            "Preparing download...",
        );

        let files = files_bg;
        let selection_plan = selection_plan_bg;
        let target_file = target_file_bg;
        let target_filename = target_filename_bg;
        let target_size = target_size_bg;
        let arcade_laserdisc_plan = arcade_laserdisc_plan_bg;

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

        let status_message = match (&download_mode, arcade_laserdisc_plan.as_ref()) {
            (MinervaDownloadMode::GameOnly, Some(ArcadeLaserdiscPlan::Mame(plan))) => {
                format!(
                    "Downloading required MAME laserdisc files for {}",
                    plan.romset_name
                )
            }
            (MinervaDownloadMode::GameOnly, Some(ArcadeLaserdiscPlan::Hypseus(plan))) => {
                format!(
                    "Downloading Hypseus laserdisc bundle for {}",
                    plan.bundle_name
                )
            }
            (MinervaDownloadMode::GameOnly, None) => format!("Downloading: {target_filename}"),
            (MinervaDownloadMode::FullTorrent, _) => {
                format!("Downloading full torrent for {game_title}")
            }
        };
        let progress_total = match (&download_mode, arcade_laserdisc_plan.as_ref()) {
            (MinervaDownloadMode::GameOnly, Some(ArcadeLaserdiscPlan::Mame(plan))) => {
                plan.primary_asset.size.saturating_add(plan.chd_asset.size)
            }
            (MinervaDownloadMode::GameOnly, Some(ArcadeLaserdiscPlan::Hypseus(plan))) => {
                plan.assets.iter().map(|asset| asset.size).sum()
            }
            (MinervaDownloadMode::GameOnly, None) => selection_plan
                .as_ref()
                .map(|plan| {
                    plan.requested_indices
                        .iter()
                        .filter_map(|idx| files.iter().find(|file| file.index == *idx))
                        .map(|file| file.size)
                        .sum()
                })
                .unwrap_or(target_size),
            (MinervaDownloadMode::FullTorrent, _) => files.iter().map(|file| file.size).sum(),
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

        if let Some(ArcadeLaserdiscPlan::Mame(plan)) = arcade_laserdisc_plan.as_ref() {
            let primary_status = format!("Downloading {}.zip...", plan.romset_name);
            let primary_source = match download_minerva_batch(
                &app_settings,
                &job_id_bg,
                &download_dir_bg,
                &plan.primary_asset.torrent_url,
                plan.primary_asset.file_index,
                &plan.primary_asset.filename,
                plan.primary_asset.size,
                0,
                progress_total,
                &primary_status,
            )
            .await
            {
                Ok(path) => path,
                Err(MinervaBatchDownloadError::Failed(message)) => {
                    crate::torrent::update_progress(
                        &job_id_bg,
                        crate::torrent::DownloadStatus::Failed,
                        0.0,
                        0,
                        0,
                        progress_total,
                        &message,
                    );
                    persist_graboid_job_status(db_path.as_ref(), &job_id_bg, "failed", &message)
                        .await;
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
                Err(MinervaBatchDownloadError::Cancelled(message)) => {
                    persist_graboid_job_status(db_path.as_ref(), &job_id_bg, "cancelled", &message)
                        .await;
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
            };

            let chd_status = format!("Downloading {}.chd...", plan.romset_name);
            let chd_source = match download_minerva_batch(
                &app_settings,
                &job_id_bg,
                &download_dir_bg,
                &plan.chd_asset.torrent_url,
                plan.chd_asset.file_index,
                &plan.chd_asset.filename,
                plan.chd_asset.size,
                plan.primary_asset.size,
                progress_total,
                &chd_status,
            )
            .await
            {
                Ok(path) => path,
                Err(MinervaBatchDownloadError::Failed(message)) => {
                    crate::torrent::update_progress(
                        &job_id_bg,
                        crate::torrent::DownloadStatus::Failed,
                        0.0,
                        0,
                        0,
                        progress_total,
                        &message,
                    );
                    persist_graboid_job_status(db_path.as_ref(), &job_id_bg, "failed", &message)
                        .await;
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
                Err(MinervaBatchDownloadError::Cancelled(message)) => {
                    persist_graboid_job_status(db_path.as_ref(), &job_id_bg, "cancelled", &message)
                        .await;
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
            };

            let staged_primary = match stage_arcade_mame_laserdisc_layout(
                &download_dir_bg,
                plan,
                &primary_source,
                &chd_source,
                &file_link_mode_bg,
            ) {
                Ok(path) => path,
                Err(message) => {
                    crate::torrent::update_progress(
                        &job_id_bg,
                        crate::torrent::DownloadStatus::Failed,
                        100.0,
                        0,
                        progress_total,
                        progress_total,
                        &message,
                    );
                    persist_graboid_job_status(db_path.as_ref(), &job_id_bg, "failed", &message)
                        .await;
                    crate::torrent::clear_client_job_id(&job_id_bg);
                    return;
                }
            };

            let completion_message = format!("Download complete ({} ROM + CHD)", plan.romset_name);
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
                    .bind(launchbox_db_id)
                    .bind(&game_title)
                    .bind(&platform)
                    .bind(staged_primary.display().to_string())
                    .bind(progress_total as i64)
                    .execute(&pool)
                    .await;
                    let _ = sqlx::query("UPDATE graboid_jobs SET status = 'completed', progress_percent = 100, status_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(&completion_message)
                        .bind(&job_id_bg)
                        .execute(&pool)
                        .await;
                }
            }

            crate::torrent::clear_client_job_id(&job_id_bg);
            return;
        }

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
            MinervaDownloadMode::GameOnly => {
                if let Some(ArcadeLaserdiscPlan::Hypseus(plan)) = arcade_laserdisc_plan.as_ref() {
                    Some(plan.assets.iter().map(|asset| asset.file_index).collect())
                } else {
                    selection_plan
                        .as_ref()
                        .map(|plan| plan.requested_indices.clone())
                        .or_else(|| file_index.map(|idx| vec![idx]))
                }
            }
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
            let representative_index = arcade_laserdisc_plan.as_ref().and_then(|plan| match plan {
                ArcadeLaserdiscPlan::Mame(plan) => Some(plan.primary_asset.file_index),
                ArcadeLaserdiscPlan::Hypseus(plan) => Some(plan.representative_asset.file_index),
            });
            let representative_file = representative_index
                .and_then(|idx| files.iter().find(|candidate| candidate.index == idx))
                .unwrap_or(&file);
            if let Some(path) = client
                .get_downloaded_file_path(
                    &client_job_id,
                    representative_file.index,
                    &download_dir_bg,
                )
                .await
                .ok()
                .flatten()
            {
                Some(path)
            } else {
                locate_downloaded_file(&download_dir_bg, &representative_file.filename)
            }
        } else {
            None
        };

        let (stored_path, stored_size, completion_message) = match download_mode {
            MinervaDownloadMode::GameOnly => {
                let found_path = if let Some(ArcadeLaserdiscPlan::Hypseus(plan)) =
                    arcade_laserdisc_plan.as_ref()
                {
                    match locate_arcade_hypseus_bundle_framefile(&download_dir_bg, plan) {
                        Ok(path) => path,
                        Err(message) => {
                            crate::torrent::update_progress(
                                &job_id_bg,
                                crate::torrent::DownloadStatus::Failed,
                                100.0,
                                0,
                                target_size,
                                target_size,
                                &message,
                            );
                            persist_graboid_job_status(
                                db_path.as_ref(),
                                &job_id_bg,
                                "failed",
                                &message,
                            )
                            .await;
                            crate::torrent::clear_client_job_id(&job_id_bg);
                            return;
                        }
                    }
                } else {
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
                    found_path
                };

                let file_size = std::fs::metadata(&found_path)
                    .map(|meta| meta.len() as i64)
                    .unwrap_or(target_size as i64);
                let stored_size = match arcade_laserdisc_plan.as_ref() {
                    Some(ArcadeLaserdiscPlan::Hypseus(_)) => progress_total as i64,
                    _ => file_size,
                };
                let completion_message = match arcade_laserdisc_plan.as_ref() {
                    Some(ArcadeLaserdiscPlan::Hypseus(plan)) => {
                        format!(
                            "Download complete (Hypseus bundle for {})",
                            plan.bundle_name
                        )
                    }
                    _ => "Download complete".to_string(),
                };
                (
                    found_path.display().to_string(),
                    stored_size,
                    completion_message,
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
        platform: canonical_platform,
        status: "in_progress".to_string(),
        progress_percent: 0.0,
        status_message: Some("Preparing download...".to_string()),
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
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub minerva_platform: Option<String>,
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

#[derive(Debug)]
struct NamedMatchScore {
    exact_match: bool,
    full_query_match: bool,
    all_significant_words_match: bool,
    score: f64,
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

fn score_match_name(
    candidate_name: &str,
    normalized_query: &str,
    query_words: &[&str],
    significant_query_words: &[&str],
) -> Option<NamedMatchScore> {
    let normalized_file = crate::tags::normalize_title_for_matching(candidate_name);
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

    Some(NamedMatchScore {
        exact_match,
        full_query_match,
        all_significant_words_match,
        score,
    })
}

fn build_torrent_match_candidate(
    file: crate::torrent::TorrentFileInfo,
    normalized_query: &str,
    query_words: &[&str],
    significant_query_words: &[&str],
) -> Option<TorrentMatchCandidate> {
    let name = basename_without_extension(&file.filename);
    let score = score_match_name(
        &name,
        normalized_query,
        query_words,
        significant_query_words,
    )?;

    let region = crate::tags::get_region_tags(&file.filename)
        .into_iter()
        .next();

    Some(TorrentMatchCandidate {
        file_match: TorrentFileMatch {
            index: file.index,
            filename: file.filename,
            size: file.size,
            match_score: score.score,
            region,
        },
        exact_match: score.exact_match,
        full_query_match: score.full_query_match,
        all_significant_words_match: score.all_significant_words_match,
    })
}

fn is_hypseus_laserdisc_request(
    platform: Option<&str>,
    collection: Option<&str>,
    minerva_platform: Option<&str>,
) -> bool {
    platform
        .map(canonicalize_legacy_platform_name)
        .is_some_and(|platform| platform == "Arcade")
        && collection
            .map(str::trim)
            .is_some_and(|value| value.eq_ignore_ascii_case("Laserdisc Collection"))
        && minerva_platform
            .map(str::trim)
            .is_some_and(|value| value.eq_ignore_ascii_case("Hypseus Singe"))
}

fn is_mame_laserdisc_request(
    platform: Option<&str>,
    collection: Option<&str>,
    minerva_platform: Option<&str>,
) -> bool {
    platform
        .map(canonicalize_legacy_platform_name)
        .is_some_and(|platform| platform == "Arcade")
        && collection
            .map(str::trim)
            .is_some_and(|value| value.eq_ignore_ascii_case("Laserdisc Collection"))
        && minerva_platform
            .map(str::trim)
            .is_some_and(|value| value.eq_ignore_ascii_case("MAME"))
}

fn select_arcade_mame_laserdisc_matches(
    files: Vec<crate::torrent::TorrentFileInfo>,
    game_title: &str,
    region_priority: &[String],
) -> Vec<TorrentFileMatch> {
    #[derive(Debug)]
    struct BundleCandidate {
        rom_zip: crate::torrent::TorrentFileInfo,
        chd: crate::torrent::TorrentFileInfo,
        exact_match: bool,
        full_query_match: bool,
        all_significant_words_match: bool,
        match_score: f64,
    }

    let normalized_query = crate::tags::normalize_title_for_matching(game_title);
    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();
    let significant_query_words = significant_match_words(&normalized_query);

    let mut bundles: std::collections::BTreeMap<
        String,
        (
            Option<crate::torrent::TorrentFileInfo>,
            Option<crate::torrent::TorrentFileInfo>,
        ),
    > = std::collections::BTreeMap::new();
    for file in files {
        let Some((romset_name, kind)) = parse_arcade_mame_laserdisc_asset(&file.filename) else {
            continue;
        };
        let entry = bundles.entry(romset_name).or_insert((None, None));
        match kind {
            ArcadeMameLaserdiscAssetKind::RomZip => entry.0 = Some(file),
            ArcadeMameLaserdiscAssetKind::Chd => entry.1 = Some(file),
        }
    }

    let mut candidates: Vec<BundleCandidate> = bundles
        .into_iter()
        .filter_map(|(romset_name, (rom_zip, chd))| {
            let score = score_match_name(
                &romset_name,
                &normalized_query,
                &query_words,
                &significant_query_words,
            )?;
            Some(BundleCandidate {
                rom_zip: rom_zip?,
                chd: chd?,
                exact_match: score.exact_match,
                full_query_match: score.full_query_match,
                all_significant_words_match: score.all_significant_words_match,
                match_score: score.score,
            })
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
                b.match_score
                    .partial_cmp(&a.match_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                let a_region = crate::tags::get_region_tags(&a.rom_zip.filename)
                    .into_iter()
                    .next()
                    .or_else(|| {
                        crate::tags::get_region_tags(&a.chd.filename)
                            .into_iter()
                            .next()
                    });
                let b_region = crate::tags::get_region_tags(&b.rom_zip.filename)
                    .into_iter()
                    .next()
                    .or_else(|| {
                        crate::tags::get_region_tags(&b.chd.filename)
                            .into_iter()
                            .next()
                    });
                crate::region_priority::priority_for_region(a_region.as_deref(), region_priority)
                    .cmp(&crate::region_priority::priority_for_region(
                        b_region.as_deref(),
                        region_priority,
                    ))
            })
            .then_with(|| {
                let a_size = a.rom_zip.size.saturating_add(a.chd.size);
                let b_size = b.rom_zip.size.saturating_add(b.chd.size);
                b_size.cmp(&a_size)
            })
            .then_with(|| a.rom_zip.filename.cmp(&b.rom_zip.filename))
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
    } else if has_full_query_match {
        candidates.retain(|candidate| candidate.full_query_match);
    } else if has_significant_match {
        candidates.retain(|candidate| candidate.all_significant_words_match);
    } else {
        candidates.retain(|candidate| candidate.match_score > 0.0);
    }

    candidates
        .into_iter()
        .take(5)
        .flat_map(|candidate| {
            [candidate.rom_zip, candidate.chd]
                .into_iter()
                .map(move |file| TorrentFileMatch {
                    index: file.index,
                    filename: file.filename.clone(),
                    size: file.size,
                    match_score: candidate.match_score,
                    region: crate::tags::get_region_tags(&file.filename)
                        .into_iter()
                        .next(),
                })
        })
        .collect()
}

fn select_arcade_hypseus_laserdisc_matches(
    files: Vec<crate::torrent::TorrentFileInfo>,
    game_title: &str,
    region_priority: &[String],
) -> Vec<TorrentFileMatch> {
    #[derive(Debug)]
    struct BundleCandidate {
        members: Vec<crate::torrent::TorrentFileInfo>,
        exact_match: bool,
        full_query_match: bool,
        all_significant_words_match: bool,
        match_score: f64,
    }

    let normalized_query = crate::tags::normalize_title_for_matching(game_title);
    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();
    let significant_query_words = significant_match_words(&normalized_query);

    let mut bundles: std::collections::BTreeMap<
        String,
        (
            Option<crate::torrent::TorrentFileInfo>,
            Option<crate::torrent::TorrentFileInfo>,
            Option<crate::torrent::TorrentFileInfo>,
            Option<crate::torrent::TorrentFileInfo>,
            Option<crate::torrent::TorrentFileInfo>,
        ),
    > = std::collections::BTreeMap::new();

    for file in files {
        let Some((bundle_key, kind)) = parse_arcade_hypseus_laserdisc_asset(&file.filename) else {
            continue;
        };
        let entry = bundles
            .entry(bundle_key)
            .or_insert((None, None, None, None, None));
        match kind {
            ArcadeHypseusLaserdiscAssetKind::RomZip => entry.0 = Some(file),
            ArcadeHypseusLaserdiscAssetKind::Data => entry.1 = Some(file),
            ArcadeHypseusLaserdiscAssetKind::FrameText => entry.2 = Some(file),
            ArcadeHypseusLaserdiscAssetKind::Video => entry.3 = Some(file),
            ArcadeHypseusLaserdiscAssetKind::Audio => entry.4 = Some(file),
        }
    }

    let mut candidates: Vec<BundleCandidate> = bundles
        .into_iter()
        .filter_map(|(bundle_key, (rom_zip, data, text, video, audio))| {
            let members = vec![rom_zip?, data?, text?, video?, audio?];
            let score = score_match_name(
                &bundle_key,
                &normalized_query,
                &query_words,
                &significant_query_words,
            )?;
            Some(BundleCandidate {
                members,
                exact_match: score.exact_match,
                full_query_match: score.full_query_match,
                all_significant_words_match: score.all_significant_words_match,
                match_score: score.score,
            })
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
                b.match_score
                    .partial_cmp(&a.match_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                let a_region = a.members.iter().find_map(|file| {
                    crate::tags::get_region_tags(&file.filename)
                        .into_iter()
                        .next()
                });
                let b_region = b.members.iter().find_map(|file| {
                    crate::tags::get_region_tags(&file.filename)
                        .into_iter()
                        .next()
                });
                crate::region_priority::priority_for_region(a_region.as_deref(), region_priority)
                    .cmp(&crate::region_priority::priority_for_region(
                        b_region.as_deref(),
                        region_priority,
                    ))
            })
            .then_with(|| {
                let a_size: u64 = a.members.iter().map(|file| file.size).sum();
                let b_size: u64 = b.members.iter().map(|file| file.size).sum();
                b_size.cmp(&a_size)
            })
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
    } else if has_full_query_match {
        candidates.retain(|candidate| candidate.full_query_match);
    } else if has_significant_match {
        candidates.retain(|candidate| candidate.all_significant_words_match);
    } else {
        candidates.retain(|candidate| candidate.match_score > 0.0);
    }

    candidates
        .into_iter()
        .take(3)
        .flat_map(|candidate| {
            candidate
                .members
                .into_iter()
                .map(move |file| TorrentFileMatch {
                    index: file.index,
                    filename: file.filename.clone(),
                    size: file.size,
                    match_score: candidate.match_score,
                    region: crate::tags::get_region_tags(&file.filename)
                        .into_iter()
                        .next(),
                })
        })
        .collect()
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

fn torrent_match_lookup_titles(
    platform: Option<&str>,
    game_title: &str,
    resolved_lookup_title: &str,
    collection: Option<&str>,
    minerva_platform: Option<&str>,
) -> Vec<String> {
    let mut titles = Vec::new();

    let is_hypseus_laserdisc = platform
        .map(canonicalize_legacy_platform_name)
        .is_some_and(|platform| platform == "Arcade")
        && collection
            .map(str::trim)
            .is_some_and(|value| value.eq_ignore_ascii_case("Laserdisc Collection"))
        && minerva_platform
            .map(str::trim)
            .is_some_and(|value| !value.eq_ignore_ascii_case("MAME"));

    if is_hypseus_laserdisc {
        titles.push(game_title.to_string());
        if resolved_lookup_title != game_title {
            titles.push(resolved_lookup_title.to_string());
        }
        if let Some(stripped) = resolved_lookup_title.strip_prefix('d') {
            if stripped != resolved_lookup_title && !stripped.is_empty() {
                titles.push(stripped.to_string());
            }
        }
        return titles;
    }

    titles.push(resolved_lookup_title.to_string());
    if resolved_lookup_title != game_title {
        titles.push(game_title.to_string());
    }
    titles
}

fn is_platform_specific_torrent_candidate(
    platform: &str,
    collection: Option<&str>,
    minerva_platform: Option<&str>,
    filename: &str,
) -> bool {
    let normalized_platform = canonicalize_legacy_platform_name(platform);
    let lowercase_filename = filename.to_lowercase();

    match normalized_platform {
        "Atari 800" => {
            lowercase_filename.contains("/atari - 8-bit family/")
                || lowercase_filename.contains("/atari/8bit/")
        }
        "Arcade" => {
            if collection
                .map(str::trim)
                .is_some_and(|value| value.eq_ignore_ascii_case("Laserdisc Collection"))
            {
                let normalized_filename = normalize_torrent_listing_path(filename);
                if let Some(platform_name) = minerva_platform
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    if platform_name.eq_ignore_ascii_case("Hypseus Singe") {
                        return normalized_filename
                            .starts_with("laserdisc collection/hypseus singe/")
                            && parse_arcade_hypseus_laserdisc_asset(filename).is_some();
                    }
                    let expected_prefix = format!(
                        "laserdisc collection/{}/",
                        platform_name.to_ascii_lowercase()
                    );
                    return normalized_filename.starts_with(&expected_prefix);
                }
                return normalized_filename.starts_with("laserdisc collection/");
            }

            if lowercase_filename.contains("/laserdisc collection/") {
                lowercase_filename.contains("/laserdisc collection/mame/roms/")
                    || lowercase_filename.contains("/laserdisc collection/mame/chd/")
            } else {
                true
            }
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
            .filter(|file| {
                is_platform_specific_torrent_candidate(
                    platform,
                    input.collection.as_deref(),
                    input.minerva_platform.as_deref(),
                    &file.filename,
                )
            })
            .cloned()
            .collect();
        if !filtered_files.is_empty() {
            files = filtered_files;
        }
    }

    let resolved_lookup_title = if let Some(ref platform) = input.platform {
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

    let lookup_titles = torrent_match_lookup_titles(
        input.platform.as_deref(),
        &input.game_title,
        &resolved_lookup_title,
        input.collection.as_deref(),
        input.minerva_platform.as_deref(),
    );

    let mut matches = Vec::new();
    for lookup_title in lookup_titles {
        let candidates = if is_mame_laserdisc_request(
            input.platform.as_deref(),
            input.collection.as_deref(),
            input.minerva_platform.as_deref(),
        ) {
            select_arcade_mame_laserdisc_matches(
                files.clone(),
                &lookup_title,
                &state.settings.region_priority,
            )
        } else if is_hypseus_laserdisc_request(
            input.platform.as_deref(),
            input.collection.as_deref(),
            input.minerva_platform.as_deref(),
        ) {
            select_arcade_hypseus_laserdisc_matches(
                files.clone(),
                &lookup_title,
                &state.settings.region_priority,
            )
        } else {
            select_torrent_file_matches(
                files.clone(),
                &lookup_title,
                &state.settings.region_priority,
            )
        };
        if !candidates.is_empty() {
            matches = candidates;
            break;
        }
    }

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
        ArcadeHypseusLaserdiscAssetKind, ArcadeMameLaserdiscAssetKind, MinervaRom,
        expand_arcade_laserdisc_roms, is_platform_specific_torrent_candidate,
        locate_downloaded_file, minerva_platform_fallbacks, parse_arcade_hypseus_laserdisc_asset,
        parse_arcade_mame_laserdisc_asset, select_arcade_hypseus_laserdisc_matches,
        select_arcade_mame_laserdisc_matches, select_torrent_file_matches, sort_emulator_statuses,
        torrent_match_lookup_titles,
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
            None,
            None,
            "TOSEC/Atari/8bit/Games/[ATR]/Kennedy Approach (1985)(MicroProse)(US).zip"
        ));
        assert!(is_platform_specific_torrent_candidate(
            "Atari 800",
            None,
            None,
            "No-Intro/Atari - 8-bit Family/Coco Notes (USA).zip"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Atari 800",
            None,
            None,
            "TOSEC/Atari/2600 & VCS/Games/Frogger.zip"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Atari 800",
            None,
            None,
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
        .filter(|file| {
            is_platform_specific_torrent_candidate("Atari 800", None, None, &file.filename)
        })
        .collect();

        let matches = select_torrent_file_matches(files, "Kennedy Approach...", &[]);
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].filename,
            "TOSEC/Atari/8bit/Games/[ATR]/Kennedy Approach (1985)(MicroProse)(US)[cr].zip"
        );
    }

    #[test]
    fn arcade_platform_hides_non_mame_laserdisc_reference_assets() {
        assert!(is_platform_specific_torrent_candidate(
            "Arcade",
            None,
            None,
            "Laserdisc Collection/MAME/ROMs/dlair.zip"
        ));
        assert!(is_platform_specific_torrent_candidate(
            "Arcade",
            None,
            None,
            "Laserdisc Collection/MAME/CHD/dlair/dlair.chd"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Arcade",
            None,
            None,
            "Laserdisc Collection/Various - Video - Archived/Reference Videos/Dragon's Lair (MAME 7-disc stacked CHD, reference)/dlair.m2v"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Arcade",
            None,
            None,
            "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/Video/lair.ogg"
        ));
    }

    #[test]
    fn arcade_platform_filters_laserdisc_collection_by_minerva_subplatform() {
        assert!(is_platform_specific_torrent_candidate(
            "Arcade",
            Some("Laserdisc Collection"),
            Some("MAME"),
            "Laserdisc Collection/MAME/ROMs/dlair.zip"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Arcade",
            Some("Laserdisc Collection"),
            Some("MAME"),
            "Laserdisc Collection/Hypseus Singe/dlair/dlair.txt"
        ));
        assert!(is_platform_specific_torrent_candidate(
            "Arcade",
            Some("Laserdisc Collection"),
            Some("Hypseus Singe"),
            "Laserdisc Collection/Hypseus Singe/dlair/dlair.txt"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Arcade",
            Some("Laserdisc Collection"),
            Some("Hypseus Singe"),
            "Laserdisc Collection/Hypseus Singe/!Game Artwork/Background/Dragon's Lair.jpg"
        ));
        assert!(!is_platform_specific_torrent_candidate(
            "Arcade",
            Some("Laserdisc Collection"),
            Some("Hypseus Singe"),
            "Laserdisc Collection/Various - Video - Archived/Reference Videos/Dragon's Lair (MAME 7-disc stacked CHD, reference)/dlair.m2v"
        ));
    }

    #[test]
    fn arcade_search_synthesizes_hypseus_laserdisc_row_from_mame_row() {
        let roms = vec![MinervaRom {
            torrent_id: 1,
            torrent_url: "https://example.invalid/laserdisc.torrent".to_string(),
            collection: "Laserdisc Collection".to_string(),
            minerva_platform: "MAME".to_string(),
            lunchbox_platform_id: 15,
            rom_count: 1,
            total_size: 1,
        }];

        let expanded = expand_arcade_laserdisc_roms(roms, "Arcade");
        assert_eq!(expanded.len(), 2);
        assert!(
            expanded
                .iter()
                .any(|rom| rom.collection == "Laserdisc Collection"
                    && rom.minerva_platform == "MAME")
        );
        assert!(
            expanded
                .iter()
                .any(|rom| rom.collection == "Laserdisc Collection"
                    && rom.minerva_platform == "Hypseus Singe")
        );
    }

    #[test]
    fn hypseus_laserdisc_prefers_real_game_title_for_matching() {
        assert_eq!(
            torrent_match_lookup_titles(
                Some("Arcade"),
                "Dragon's Lair",
                "dlair",
                Some("Laserdisc Collection"),
                Some("Hypseus Singe"),
            ),
            vec![
                "Dragon's Lair".to_string(),
                "dlair".to_string(),
                "lair".to_string()
            ]
        );
    }

    #[test]
    fn mame_laserdisc_keeps_shortname_first_for_matching() {
        assert_eq!(
            torrent_match_lookup_titles(
                Some("Arcade"),
                "Dragon's Lair",
                "dlair",
                Some("Laserdisc Collection"),
                Some("MAME"),
            ),
            vec!["dlair".to_string(), "Dragon's Lair".to_string()]
        );
    }

    #[test]
    fn parses_arcade_mame_laserdisc_rom_and_chd_paths() {
        assert_eq!(
            parse_arcade_mame_laserdisc_asset("Laserdisc Collection/MAME/ROMs/dlair.zip"),
            Some(("dlair".to_string(), ArcadeMameLaserdiscAssetKind::RomZip))
        );
        assert_eq!(
            parse_arcade_mame_laserdisc_asset("Laserdisc Collection/MAME/CHD/dlair/dlair.chd"),
            Some(("dlair".to_string(), ArcadeMameLaserdiscAssetKind::Chd))
        );
        assert_eq!(
            parse_arcade_mame_laserdisc_asset(
                "Laserdisc Collection/Various - Video - Archived/Reference Videos/Dragon's Lair/dlair.m2v"
            ),
            None
        );
    }

    #[test]
    fn arcade_mame_laserdisc_matching_prefers_exact_bundle_over_partial_overlap() {
        let files = vec![
            TorrentFileInfo {
                index: 0,
                filename: "Laserdisc Collection/MAME/ROMs/dlair.zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "Laserdisc Collection/MAME/CHD/dlair/dlair.chd".to_string(),
                size: 10,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Laserdisc Collection/MAME/ROMs/dlair2.zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 3,
                filename: "Laserdisc Collection/MAME/CHD/dlair2/dlair2.chd".to_string(),
                size: 10,
            },
        ];

        let matches = select_arcade_mame_laserdisc_matches(files, "dlair", &[]);
        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches
                .iter()
                .map(|file| file.filename.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Laserdisc Collection/MAME/ROMs/dlair.zip",
                "Laserdisc Collection/MAME/CHD/dlair/dlair.chd",
            ]
        );
    }

    #[test]
    fn arcade_mame_laserdisc_matching_keeps_dragons_lair_two_bundle_only() {
        let files = vec![
            TorrentFileInfo {
                index: 0,
                filename: "Laserdisc Collection/MAME/ROMs/dlair.zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 1,
                filename: "Laserdisc Collection/MAME/CHD/dlair/dlair.chd".to_string(),
                size: 10,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Laserdisc Collection/MAME/ROMs/dlair2.zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 3,
                filename: "Laserdisc Collection/MAME/CHD/dlair2/dlair2.chd".to_string(),
                size: 10,
            },
            TorrentFileInfo {
                index: 4,
                filename: "Laserdisc Collection/MAME/ROMs/ep_twarp.zip".to_string(),
                size: 1,
            },
            TorrentFileInfo {
                index: 5,
                filename: "Laserdisc Collection/MAME/CHD/ep_twarp/ep_twarp.chd".to_string(),
                size: 10,
            },
        ];

        let matches = select_arcade_mame_laserdisc_matches(files, "dlair2", &[]);
        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches
                .iter()
                .map(|file| file.filename.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Laserdisc Collection/MAME/ROMs/dlair2.zip",
                "Laserdisc Collection/MAME/CHD/dlair2/dlair2.chd",
            ]
        );
    }

    #[test]
    fn parses_arcade_hypseus_laserdisc_paths() {
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Dragon's Lair/dlair.dat"
            ),
            Some((
                "laserdisc collection/hypseus singe/dragon's lair/dlair".to_string(),
                ArcadeHypseusLaserdiscAssetKind::Data
            ))
        );
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Dragon's Lair/dlair.txt"
            ),
            Some((
                "laserdisc collection/hypseus singe/dragon's lair/dlair".to_string(),
                ArcadeHypseusLaserdiscAssetKind::FrameText
            ))
        );
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Dragon's Lair/dlair.m2v"
            ),
            Some((
                "laserdisc collection/hypseus singe/dragon's lair/dlair".to_string(),
                ArcadeHypseusLaserdiscAssetKind::Video
            ))
        );
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Dragon's Lair/dlair.ogg"
            ),
            Some((
                "laserdisc collection/hypseus singe/dragon's lair/dlair".to_string(),
                ArcadeHypseusLaserdiscAssetKind::Audio
            ))
        );
    }

    #[test]
    fn parses_arcade_hypseus_laserdisc_paths_from_nested_video_dir() {
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/lair.dat"
            ),
            Some((
                "laserdisc collection/hypseus singe/singe2/singe/dragons_lair_1080/lair"
                    .to_string(),
                ArcadeHypseusLaserdiscAssetKind::Data
            ))
        );
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/lair.txt"
            ),
            Some((
                "laserdisc collection/hypseus singe/singe2/singe/dragons_lair_1080/lair"
                    .to_string(),
                ArcadeHypseusLaserdiscAssetKind::FrameText
            ))
        );
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/Video/lair.m2v"
            ),
            Some((
                "laserdisc collection/hypseus singe/singe2/singe/dragons_lair_1080/lair"
                    .to_string(),
                ArcadeHypseusLaserdiscAssetKind::Video
            ))
        );
        assert_eq!(
            parse_arcade_hypseus_laserdisc_asset(
                "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/Video/lair.ogg"
            ),
            Some((
                "laserdisc collection/hypseus singe/singe2/singe/dragons_lair_1080/lair"
                    .to_string(),
                ArcadeHypseusLaserdiscAssetKind::Audio
            ))
        );
    }

    #[test]
    fn hypseus_laserdisc_match_prefers_real_bundle_over_artwork_and_docs() {
        let files = vec![
            crate::torrent::TorrentFileInfo {
                index: 1,
                filename: "Laserdisc Collection/Hypseus Singe/!Game Artwork/Background/Dragon's Lair.jpg".to_string(),
                size: 100,
            },
            crate::torrent::TorrentFileInfo {
                index: 2,
                filename: "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/lair.dat".to_string(),
                size: 100,
            },
            crate::torrent::TorrentFileInfo {
                index: 3,
                filename: "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/lair.txt".to_string(),
                size: 100,
            },
            crate::torrent::TorrentFileInfo {
                index: 4,
                filename: "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/Video/lair.m2v".to_string(),
                size: 1000,
            },
            crate::torrent::TorrentFileInfo {
                index: 5,
                filename: "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/Video/lair.ogg".to_string(),
                size: 1000,
            },
            crate::torrent::TorrentFileInfo {
                index: 6,
                filename: "Laserdisc Collection/Hypseus Singe/Singe2/singe/dragons_lair_1080/Assets/Dragon's Lair nitpick.txt".to_string(),
                size: 10,
            },
        ];

        let matches = select_arcade_hypseus_laserdisc_matches(files, "Dragon's Lair", &[]);
        assert_eq!(matches.len(), 3);
        assert!(matches.iter().any(|m| m.filename.ends_with("/lair.txt")));
        assert!(matches.iter().any(|m| m.filename.ends_with("/lair.m2v")));
        assert!(matches.iter().any(|m| m.filename.ends_with("/lair.ogg")));
    }

    fn new_lazy_user_state() -> (tempfile::TempDir, crate::state::AppState) {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join(crate::db::USER_DB_NAME);
        let state = crate::state::AppState {
            user_db_path: Some(db_path),
            ..crate::state::AppState::default()
        };
        (temp_dir, state)
    }

    #[tokio::test]
    async fn emulator_preferences_default_to_empty_without_user_db() {
        let state = crate::state::AppState::default();

        let pref = super::get_emulator_preference(&state, 123, "Nintendo Entertainment System")
            .await
            .unwrap();
        assert!(pref.is_none());

        let all = super::get_all_emulator_preferences(&state).await.unwrap();
        assert!(all.game_preferences.is_empty());
        assert!(all.platform_preferences.is_empty());
    }

    #[tokio::test]
    async fn emulator_preferences_create_user_db_on_first_write() {
        let (_temp_dir, mut state) = new_lazy_user_state();

        super::set_platform_emulator_preference(
            &mut state,
            "Nintendo Entertainment System",
            "Mesen",
        )
        .await
        .unwrap();

        assert!(state.db_pool.is_some());

        let pref = super::get_emulator_preference(&state, 123, "Nintendo Entertainment System")
            .await
            .unwrap();
        assert_eq!(pref.as_deref(), Some("Mesen"));

        let all = super::get_all_emulator_preferences(&state).await.unwrap();
        assert_eq!(all.platform_preferences.len(), 1);
        assert_eq!(
            all.platform_preferences[0].platform_name,
            "Nintendo Entertainment System"
        );
        assert_eq!(all.platform_preferences[0].emulator_name, "Mesen");
    }

    #[tokio::test]
    async fn emulator_launch_profiles_default_to_empty_without_user_db() {
        let state = crate::state::AppState::default();

        let profile = super::get_emulator_launch_profile(&state, "MAME", Some("Arcade"), false)
            .await
            .unwrap();
        assert!(profile.is_none());

        let all = super::get_all_emulator_launch_profiles(&state)
            .await
            .unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn emulator_launch_profiles_create_user_db_and_fallback_from_platform_override() {
        let (_temp_dir, mut state) = new_lazy_user_state();

        super::set_emulator_launch_profile(&mut state, "MAME", None, false, "-noui")
            .await
            .unwrap();

        assert!(state.db_pool.is_some());

        let fallback = super::get_emulator_launch_profile(&state, "MAME", Some("Arcade"), false)
            .await
            .unwrap()
            .unwrap();
        assert!(fallback.platform_name.is_none());
        assert_eq!(fallback.args_text, "-noui");

        super::set_emulator_launch_profile(
            &mut state,
            "MAME",
            Some("Arcade"),
            false,
            "-skip_gameinfo",
        )
        .await
        .unwrap();

        let exact = super::get_emulator_launch_profile(&state, "MAME", Some("Arcade"), false)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(exact.platform_name.as_deref(), Some("Arcade"));
        assert_eq!(exact.args_text, "-skip_gameinfo");

        let all = super::get_all_emulator_launch_profiles(&state)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);

        super::clear_emulator_launch_profile(&state, "MAME", Some("Arcade"), false)
            .await
            .unwrap();

        let fallback_again =
            super::get_emulator_launch_profile(&state, "MAME", Some("Arcade"), false)
                .await
                .unwrap()
                .unwrap();
        assert!(fallback_again.platform_name.is_none());
        assert_eq!(fallback_again.args_text, "-noui");
    }
}
