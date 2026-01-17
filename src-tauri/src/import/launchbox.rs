//! LaunchBox SQLite database importer
//!
//! Imports game metadata from an existing LaunchBox installation's SQLite database.
//! LaunchBox stores its metadata in: `<LaunchBox>/Metadata/LaunchBox.Metadata.db`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;
use std::path::{Path, PathBuf};

/// LaunchBox Game record from their database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LbGame {
    #[sqlx(rename = "DatabaseID")]
    pub database_id: i64,
    #[sqlx(rename = "Name")]
    pub name: String,
    #[sqlx(rename = "CompareName")]
    pub compare_name: String,
    #[sqlx(rename = "ReleaseDate")]
    pub release_date: Option<String>,
    #[sqlx(rename = "ReleaseYear")]
    pub release_year: Option<i32>,
    #[sqlx(rename = "Overview")]
    pub overview: Option<String>,
    #[sqlx(rename = "MaxPlayers")]
    pub max_players: Option<i32>,
    #[sqlx(rename = "ReleaseType")]
    pub release_type: Option<String>,
    #[sqlx(rename = "Cooperative")]
    pub cooperative: bool,
    #[sqlx(rename = "VideoURL")]
    pub video_url: Option<String>,
    #[sqlx(rename = "CommunityRating")]
    pub community_rating: Option<f64>,
    #[sqlx(rename = "Platform")]
    pub platform: String,
    #[sqlx(rename = "ESRB")]
    pub esrb: Option<String>,
    #[sqlx(rename = "Genres")]
    pub genres: String,
    #[sqlx(rename = "Developer")]
    pub developer: Option<String>,
    #[sqlx(rename = "Publisher")]
    pub publisher: Option<String>,
}

/// LaunchBox Platform record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LbPlatform {
    #[sqlx(rename = "PlatformKey")]
    pub platform_key: i64,
    #[sqlx(rename = "Name")]
    pub name: String,
    #[sqlx(rename = "Emulated")]
    pub emulated: bool,
    #[sqlx(rename = "ReleaseDate")]
    pub release_date: Option<String>,
    #[sqlx(rename = "Developer")]
    pub developer: Option<String>,
    #[sqlx(rename = "Manufacturer")]
    pub manufacturer: Option<String>,
    #[sqlx(rename = "Category")]
    pub category: Option<String>,
}

/// LaunchBox Game Image record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LbGameImage {
    #[sqlx(rename = "FileName")]
    pub file_name: String,
    #[sqlx(rename = "DatabaseId")]
    pub database_id: i64,
    #[sqlx(rename = "Type")]
    pub image_type: String,
    #[sqlx(rename = "Region")]
    pub region: Option<String>,
    #[sqlx(rename = "CRC32")]
    pub crc32: i64,
}

/// LaunchBox Emulator record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LbEmulator {
    #[sqlx(rename = "Name")]
    pub name: String,
    #[sqlx(rename = "CommandLine")]
    pub command_line: Option<String>,
    #[sqlx(rename = "ApplicableFileExtensions")]
    pub file_extensions: Option<String>,
    #[sqlx(rename = "URL")]
    pub url: Option<String>,
    #[sqlx(rename = "BinaryFileName")]
    pub binary_file_name: String,
}

/// LaunchBox Emulator-Platform mapping
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LbEmulatorPlatform {
    #[sqlx(rename = "Emulator")]
    pub emulator: String,
    #[sqlx(rename = "Platform")]
    pub platform: String,
    #[sqlx(rename = "CommandLine")]
    pub command_line: Option<String>,
    #[sqlx(rename = "ApplicableFileExtensions")]
    pub file_extensions: Option<String>,
    #[sqlx(rename = "Recommended")]
    pub recommended: bool,
}

/// Importer for LaunchBox's SQLite metadata database
pub struct LaunchBoxImporter {
    pool: SqlitePool,
}

impl LaunchBoxImporter {
    /// Connect to a LaunchBox metadata database (read-only)
    pub async fn connect(db_path: &Path) -> Result<Self> {
        let db_url = format!("sqlite:{}?mode=ro", db_path.display());
        let pool = SqlitePool::connect(&db_url)
            .await
            .context("Failed to connect to LaunchBox database")?;

        Ok(Self { pool })
    }

    /// Get the total count of games in the database
    pub async fn count_games(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM Games")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Get the total count of platforms
    pub async fn count_platforms(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM Platforms")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Fetch all platforms
    pub async fn get_platforms(&self) -> Result<Vec<LbPlatform>> {
        let platforms = sqlx::query_as::<_, LbPlatform>(
            r#"
            SELECT PlatformKey, Name, Emulated, ReleaseDate, Developer, Manufacturer, Category
            FROM Platforms
            ORDER BY Name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(platforms)
    }

    /// Fetch games for a specific platform
    pub async fn get_games_by_platform(&self, platform: &str) -> Result<Vec<LbGame>> {
        let games = sqlx::query_as::<_, LbGame>(
            r#"
            SELECT DatabaseID, Name, CompareName, ReleaseDate, ReleaseYear, Overview,
                   MaxPlayers, ReleaseType, Cooperative, VideoURL, CommunityRating,
                   Platform, ESRB, Genres, Developer, Publisher
            FROM Games
            WHERE Platform = ?
            ORDER BY Name
            "#,
        )
        .bind(platform)
        .fetch_all(&self.pool)
        .await?;

        Ok(games)
    }

    /// Fetch a game by its DatabaseID
    pub async fn get_game_by_id(&self, database_id: i64) -> Result<Option<LbGame>> {
        let game = sqlx::query_as::<_, LbGame>(
            r#"
            SELECT DatabaseID, Name, CompareName, ReleaseDate, ReleaseYear, Overview,
                   MaxPlayers, ReleaseType, Cooperative, VideoURL, CommunityRating,
                   Platform, ESRB, Genres, Developer, Publisher
            FROM Games
            WHERE DatabaseID = ?
            "#,
        )
        .bind(database_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(game)
    }

    /// Search games by name (fuzzy match using CompareName)
    pub async fn search_games(&self, query: &str, limit: i64) -> Result<Vec<LbGame>> {
        // Normalize query similar to how LaunchBox does it
        let normalized = normalize_for_comparison(query);

        let games = sqlx::query_as::<_, LbGame>(
            r#"
            SELECT DatabaseID, Name, CompareName, ReleaseDate, ReleaseYear, Overview,
                   MaxPlayers, ReleaseType, Cooperative, VideoURL, CommunityRating,
                   Platform, ESRB, Genres, Developer, Publisher
            FROM Games
            WHERE CompareName LIKE ?
            ORDER BY Name
            LIMIT ?
            "#,
        )
        .bind(format!("%{}%", normalized))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(games)
    }

    /// Get images for a game by its DatabaseID
    pub async fn get_game_images(&self, database_id: i64) -> Result<Vec<LbGameImage>> {
        let images = sqlx::query_as::<_, LbGameImage>(
            r#"
            SELECT FileName, DatabaseId, Type, Region, CRC32
            FROM GameImages
            WHERE DatabaseId = ?
            "#,
        )
        .bind(database_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(images)
    }

    /// Get images for a game by its DatabaseID, with platform for path resolution
    pub async fn get_game_images_with_platform(&self, database_id: i64) -> Result<Vec<(LbGameImage, String)>> {
        // First get the game to find its platform
        let game = self.get_game_by_id(database_id).await?;
        let platform = game.map(|g| g.platform).unwrap_or_default();

        let images = self.get_game_images(database_id).await?;
        Ok(images.into_iter().map(|img| (img, platform.clone())).collect())
    }

    /// Get all emulators
    pub async fn get_emulators(&self) -> Result<Vec<LbEmulator>> {
        let emulators = sqlx::query_as::<_, LbEmulator>(
            r#"
            SELECT Name, CommandLine, ApplicableFileExtensions, URL, BinaryFileName
            FROM Emulators
            ORDER BY Name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(emulators)
    }

    /// Get emulator-platform mappings for a platform
    pub async fn get_emulators_for_platform(&self, platform: &str) -> Result<Vec<LbEmulatorPlatform>> {
        let mappings = sqlx::query_as::<_, LbEmulatorPlatform>(
            r#"
            SELECT Emulator, Platform, CommandLine, ApplicableFileExtensions, Recommended
            FROM EmulatorPlatforms
            WHERE Platform = ?
            ORDER BY Recommended DESC, Emulator
            "#,
        )
        .bind(platform)
        .fetch_all(&self.pool)
        .await?;

        Ok(mappings)
    }

    /// Get total count of game images
    pub async fn count_game_images(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM GameImages")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Get all game images in batches for import
    /// Returns a stream of image records for memory-efficient processing
    pub async fn get_all_game_images(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LbGameImage>> {
        let images = sqlx::query_as::<_, LbGameImage>(
            r#"
            SELECT FileName, DatabaseId, Type, Region, CRC32
            FROM GameImages
            ORDER BY DatabaseId
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(images)
    }

    /// Get images for multiple games at once (more efficient for batch lookups)
    pub async fn get_images_for_games(&self, database_ids: &[i64]) -> Result<Vec<LbGameImage>> {
        if database_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build placeholders for IN clause
        let placeholders: String = database_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            r#"
            SELECT FileName, DatabaseId, Type, Region, CRC32
            FROM GameImages
            WHERE DatabaseId IN ({})
            ORDER BY DatabaseId, Type
            "#,
            placeholders
        );

        let mut q = sqlx::query_as::<_, LbGameImage>(&query);
        for id in database_ids {
            q = q.bind(id);
        }

        let images = q.fetch_all(&self.pool).await?;
        Ok(images)
    }

    /// Get the first image of a specific type for a game (most common use case)
    pub async fn get_primary_image(
        &self,
        database_id: i64,
        image_type: &str,
    ) -> Result<Option<LbGameImage>> {
        let image = sqlx::query_as::<_, LbGameImage>(
            r#"
            SELECT FileName, DatabaseId, Type, Region, CRC32
            FROM GameImages
            WHERE DatabaseId = ? AND Type = ?
            ORDER BY Region, FileName
            LIMIT 1
            "#,
        )
        .bind(database_id)
        .bind(image_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(image)
    }

    /// Get available image types for a game
    pub async fn get_available_image_types(&self, database_id: i64) -> Result<Vec<String>> {
        let types: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT Type
            FROM GameImages
            WHERE DatabaseId = ?
            ORDER BY Type
            "#,
        )
        .bind(database_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(types.into_iter().map(|(t,)| t).collect())
    }
}

/// Normalize a string for comparison (similar to LaunchBox's CompareName logic)
fn normalize_for_comparison(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Resolved image paths for a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameImagePaths {
    pub box_front: Option<String>,
    pub box_back: Option<String>,
    pub screenshot: Option<String>,
    pub fanart: Option<String>,
    pub clear_logo: Option<String>,
}

/// Find game images on disk by scanning the LaunchBox Images directory
/// Images are stored as: {LaunchBoxPath}/Images/{Platform}/{ImageType}/{Region?}/{GameName}-{NN}.{ext}
pub fn find_game_images(launchbox_path: &Path, platform: &str, game_name: &str) -> GameImagePaths {
    let images_base = launchbox_path.join("Images").join(platform);

    // Escape special characters in game name for file matching
    let safe_name = sanitize_filename(game_name);

    GameImagePaths {
        box_front: find_image_of_type(&images_base, &safe_name, "Box - Front"),
        box_back: find_image_of_type(&images_base, &safe_name, "Box - Back"),
        screenshot: find_image_of_type(&images_base, &safe_name, "Screenshot - Gameplay")
            .or_else(|| find_image_of_type(&images_base, &safe_name, "Screenshot - Game Title")),
        fanart: find_image_of_type(&images_base, &safe_name, "Fanart - Background"),
        clear_logo: find_image_of_type(&images_base, &safe_name, "Clear Logo"),
    }
}

/// Find an image of a specific type for a game
fn find_image_of_type(images_base: &Path, game_name: &str, image_type: &str) -> Option<String> {
    let type_dir = images_base.join(image_type);

    if !type_dir.exists() {
        return None;
    }

    // Check directly in the type folder
    if let Some(path) = find_matching_image(&type_dir, game_name) {
        return Some(path);
    }

    // Check region subdirectories (North America, Europe, Japan, etc.)
    if let Ok(entries) = std::fs::read_dir(&type_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(path) = find_matching_image(&entry.path(), game_name) {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// Find an image file matching the game name pattern
fn find_matching_image(dir: &Path, game_name: &str) -> Option<String> {
    let pattern = format!("{}-", game_name);

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str.starts_with(&pattern) {
                let path = entry.path();
                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();

                if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

/// Sanitize a filename by escaping special characters that would be problematic
fn sanitize_filename(name: &str) -> String {
    // LaunchBox uses the game name directly, but some characters are replaced
    name.replace(':', "_")
        .replace('/', "_")
        .replace('\\', "_")
        .replace('?', "_")
        .replace('*', "_")
        .replace('"', "_")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(normalize_for_comparison("Super Mario Bros."), "supermariobros");
        assert_eq!(normalize_for_comparison("The Legend of Zelda: A Link to the Past"), "thelegendofzeldaalinktothepast");
        assert_eq!(normalize_for_comparison("Sonic the Hedgehog 2"), "sonicthehedgehog2");
    }
}
