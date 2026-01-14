//! Database schema types

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Platform {
    pub id: i64,
    pub name: String,
    pub screenscraper_id: Option<i64>,
    pub retroarch_core: Option<String>,
    pub file_extensions: Option<String>, // JSON array
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Game {
    pub id: String, // UUID
    pub title: String,
    pub platform_id: i64,
    pub launchbox_db_id: Option<i64>,
    pub screenscraper_id: Option<i64>,
    pub igdb_id: Option<i64>,
    pub description: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Option<String>, // JSON array
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Rom {
    pub id: String, // UUID
    pub game_id: Option<String>,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub crc32: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub region: Option<String>,
    pub version: Option<String>,
    pub verified: bool,
    pub last_played: Option<DateTime<Utc>>,
    pub play_count: i64,
    pub play_time_seconds: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Media {
    pub id: String, // UUID
    pub game_id: String,
    pub media_type: String,
    pub file_path: String,
    pub source: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Emulator {
    pub id: String,
    pub name: String,
    pub executable_path: Option<String>,
    pub emulator_type: String, // 'retroarch', 'standalone'
    pub version: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlatformEmulator {
    pub platform_id: i64,
    pub emulator_id: String,
    pub core_name: Option<String>,
    pub is_default: bool,
    pub command_line_args: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Collection {
    pub id: String, // UUID
    pub name: String,
    pub description: Option<String>,
    pub is_smart: bool,
    pub filter_rules: Option<String>, // JSON
    pub created_at: DateTime<Utc>,
}
