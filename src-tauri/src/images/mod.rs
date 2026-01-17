//! Image download and caching service
//!
//! Handles downloading game images from multiple sources:
//! - LaunchBox CDN (primary, requires metadata import)
//! - libretro-thumbnails (free, no account needed)
//! - SteamGridDB (requires API key)
//! - ScreenScraper (requires account)
//!
//! Features:
//! - Parallel downloads with configurable concurrency
//! - Multi-source fallback (tries each source until one succeeds)
//! - Local caching with verification
//! - Progress events for UI updates

pub mod libretro;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};

pub use libretro::{LibRetroImageType, LibRetroThumbnailsClient};

/// LaunchBox CDN base URL for images
pub const LAUNCHBOX_CDN_URL: &str = "https://images.launchbox-app.com";

/// Image source priority (lower = tried first)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageSource {
    /// Local file already on disk (from LaunchBox installation)
    Local,
    /// LaunchBox CDN (requires game_images table populated)
    LaunchBox,
    /// libretro-thumbnails (free, no account)
    LibRetro,
    /// SteamGridDB (requires API key)
    SteamGridDB,
    /// ScreenScraper (requires account, rate limited)
    ScreenScraper,
}

/// Default concurrent downloads
const DEFAULT_CONCURRENCY: usize = 10;

/// Image type priority (lower = higher priority)
#[allow(dead_code)]
fn image_type_priority(image_type: &str) -> i32 {
    match image_type {
        "Box - Front" => 0,
        "Screenshot - Gameplay" => 1,
        "Clear Logo" => 2,
        "Banner" => 3,
        "Screenshot - Game Title" => 4,
        "Box - Back" => 5,
        "Fanart - Background" => 6,
        _ => 10,
    }
}

/// Image info returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    pub id: i64,
    pub launchbox_db_id: i64,
    pub image_type: String,
    pub region: Option<String>,
    pub cdn_url: String,
    pub local_path: Option<String>,
    pub downloaded: bool,
}

/// Download progress event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub game_id: i64,
    pub image_type: String,
    pub progress: f32,     // 0.0 to 1.0
    pub status: String,    // "pending", "downloading", "completed", "failed"
    pub local_path: Option<String>,
    pub error: Option<String>,
}

/// Download request for the queue
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub id: i64,
    pub launchbox_db_id: i64,
    pub filename: String,
    pub image_type: String,
    pub priority: i32,
}

/// Image download service
pub struct ImageService {
    pool: SqlitePool,
    cache_dir: PathBuf,
    client: reqwest::Client,
    concurrency: usize,
    #[allow(dead_code)]
    download_tx: Option<mpsc::Sender<DownloadRequest>>,
}

impl ImageService {
    /// Create a new image service
    pub fn new(pool: SqlitePool, cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            pool,
            cache_dir,
            client,
            concurrency: DEFAULT_CONCURRENCY,
            download_tx: None,
        }
    }

    /// Set download concurrency
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Get the local cache path for an image
    fn get_cache_path(&self, filename: &str) -> PathBuf {
        self.cache_dir.join("launchbox").join(filename)
    }

    /// Build CDN URL from filename
    fn get_cdn_url(filename: &str) -> String {
        format!("{}/{}", LAUNCHBOX_CDN_URL, urlencoding::encode(filename))
    }

    /// Get image info for a game
    pub async fn get_game_images(
        &self,
        launchbox_db_id: i64,
    ) -> Result<Vec<ImageInfo>> {
        let rows: Vec<(i64, i64, String, String, Option<String>, Option<String>, i64)> =
            sqlx::query_as(
                r#"
                SELECT id, launchbox_db_id, filename, image_type, region, local_path, downloaded
                FROM game_images
                WHERE launchbox_db_id = ?
                ORDER BY
                    CASE image_type
                        WHEN 'Box - Front' THEN 0
                        WHEN 'Screenshot - Gameplay' THEN 1
                        WHEN 'Clear Logo' THEN 2
                        WHEN 'Banner' THEN 3
                        ELSE 10
                    END,
                    region
                "#,
            )
            .bind(launchbox_db_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, db_id, filename, image_type, region, local_path, downloaded)| {
                    ImageInfo {
                        id,
                        launchbox_db_id: db_id,
                        image_type,
                        region,
                        cdn_url: Self::get_cdn_url(&filename),
                        local_path,
                        downloaded: downloaded != 0,
                    }
                },
            )
            .collect())
    }

    /// Get a specific image type for a game (returns first available)
    pub async fn get_image_by_type(
        &self,
        launchbox_db_id: i64,
        image_type: &str,
    ) -> Result<Option<ImageInfo>> {
        let row: Option<(i64, i64, String, String, Option<String>, Option<String>, i64)> =
            sqlx::query_as(
                r#"
                SELECT id, launchbox_db_id, filename, image_type, region, local_path, downloaded
                FROM game_images
                WHERE launchbox_db_id = ? AND image_type = ?
                ORDER BY
                    CASE region
                        WHEN 'North America' THEN 0
                        WHEN 'United States' THEN 1
                        WHEN 'World' THEN 2
                        WHEN 'Europe' THEN 3
                        ELSE 10
                    END,
                    filename
                LIMIT 1
                "#,
            )
            .bind(launchbox_db_id)
            .bind(image_type)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(
            |(id, db_id, filename, image_type, region, local_path, downloaded)| {
                ImageInfo {
                    id,
                    launchbox_db_id: db_id,
                    image_type,
                    region,
                    cdn_url: Self::get_cdn_url(&filename),
                    local_path,
                    downloaded: downloaded != 0,
                }
            },
        ))
    }

    /// Get available image types for a game
    pub async fn get_available_types(&self, launchbox_db_id: i64) -> Result<Vec<String>> {
        let types: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT image_type
            FROM game_images
            WHERE launchbox_db_id = ?
            ORDER BY
                CASE image_type
                    WHEN 'Box - Front' THEN 0
                    WHEN 'Screenshot - Gameplay' THEN 1
                    WHEN 'Clear Logo' THEN 2
                    WHEN 'Banner' THEN 3
                    ELSE 10
                END
            "#,
        )
        .bind(launchbox_db_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(types.into_iter().map(|(t,)| t).collect())
    }

    /// Download a single image and update the database
    pub async fn download_image(&self, image_id: i64) -> Result<String> {
        // Get image info
        let row: (String, i64) = sqlx::query_as(
            "SELECT filename, launchbox_db_id FROM game_images WHERE id = ?",
        )
        .bind(image_id)
        .fetch_one(&self.pool)
        .await
        .context("Image not found")?;

        let (filename, _db_id) = row;
        let local_path = self.get_cache_path(&filename);

        // Check if already downloaded
        if local_path.exists() {
            // Update database if needed
            sqlx::query("UPDATE game_images SET downloaded = 1, local_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(local_path.to_string_lossy().to_string())
                .bind(image_id)
                .execute(&self.pool)
                .await?;

            return Ok(local_path.to_string_lossy().to_string());
        }

        // Create parent directories
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Download the image
        let url = Self::get_cdn_url(&filename);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch image")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), url);
        }

        let bytes = response.bytes().await?;
        tokio::fs::write(&local_path, &bytes).await?;

        // Update database
        let local_path_str = local_path.to_string_lossy().to_string();
        sqlx::query(
            "UPDATE game_images SET downloaded = 1, local_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(&local_path_str)
        .bind(image_id)
        .execute(&self.pool)
        .await?;

        Ok(local_path_str)
    }

    /// Download images for a game (prioritizes Box - Front)
    pub async fn download_game_images(
        &self,
        launchbox_db_id: i64,
        image_types: Option<Vec<String>>,
    ) -> Result<Vec<String>> {
        let types_filter = image_types.unwrap_or_else(|| {
            vec![
                "Box - Front".to_string(),
                "Screenshot - Gameplay".to_string(),
            ]
        });

        let mut downloaded_paths = Vec::new();

        for image_type in types_filter {
            if let Some(info) = self.get_image_by_type(launchbox_db_id, &image_type).await? {
                if !info.downloaded {
                    match self.download_image(info.id).await {
                        Ok(path) => downloaded_paths.push(path),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to download {} for game {}: {}",
                                image_type,
                                launchbox_db_id,
                                e
                            );
                        }
                    }
                } else if let Some(path) = info.local_path {
                    downloaded_paths.push(path);
                }
            }
        }

        Ok(downloaded_paths)
    }

    /// Batch download images with concurrency control
    pub async fn batch_download(
        &self,
        image_ids: Vec<i64>,
        progress_tx: Option<mpsc::Sender<DownloadProgress>>,
    ) -> Result<HashMap<i64, String>> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let results = Arc::new(RwLock::new(HashMap::new()));

        // Get all image info
        if image_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: String = image_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!(
            "SELECT id, launchbox_db_id, filename, image_type FROM game_images WHERE id IN ({})",
            placeholders
        );

        let mut q = sqlx::query_as::<_, (i64, i64, String, String)>(&query);
        for id in &image_ids {
            q = q.bind(id);
        }

        let rows: Vec<(i64, i64, String, String)> = q.fetch_all(&self.pool).await?;

        let mut handles = Vec::new();

        for (id, db_id, filename, image_type) in rows {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = self.client.clone();
            let cache_dir = self.cache_dir.clone();
            let pool = self.pool.clone();
            let results = results.clone();
            let progress_tx = progress_tx.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                // Emit progress: downloading
                if let Some(ref tx) = progress_tx {
                    let _ = tx
                        .send(DownloadProgress {
                            game_id: db_id,
                            image_type: image_type.clone(),
                            progress: 0.0,
                            status: "downloading".to_string(),
                            local_path: None,
                            error: None,
                        })
                        .await;
                }

                let local_path = cache_dir.join("launchbox").join(&filename);

                // Check if already exists
                if local_path.exists() {
                    let local_path_str = local_path.to_string_lossy().to_string();
                    results.write().await.insert(id, local_path_str.clone());

                    if let Some(ref tx) = progress_tx {
                        let _ = tx
                            .send(DownloadProgress {
                                game_id: db_id,
                                image_type: image_type.clone(),
                                progress: 1.0,
                                status: "completed".to_string(),
                                local_path: Some(local_path_str),
                                error: None,
                            })
                            .await;
                    }
                    return;
                }

                // Create directories
                if let Some(parent) = local_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }

                // Download
                let url = format!(
                    "{}/{}",
                    LAUNCHBOX_CDN_URL,
                    urlencoding::encode(&filename)
                );

                match client.get(&url).send().await {
                    Ok(response) if response.status().is_success() => {
                        match response.bytes().await {
                            Ok(bytes) => {
                                if let Ok(_) = tokio::fs::write(&local_path, &bytes).await {
                                    let local_path_str = local_path.to_string_lossy().to_string();

                                    // Update database
                                    let _ = sqlx::query(
                                        "UPDATE game_images SET downloaded = 1, local_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                                    )
                                    .bind(&local_path_str)
                                    .bind(id)
                                    .execute(&pool)
                                    .await;

                                    results.write().await.insert(id, local_path_str.clone());

                                    if let Some(ref tx) = progress_tx {
                                        let _ = tx
                                            .send(DownloadProgress {
                                                game_id: db_id,
                                                image_type: image_type.clone(),
                                                progress: 1.0,
                                                status: "completed".to_string(),
                                                local_path: Some(local_path_str),
                                                error: None,
                                            })
                                            .await;
                                    }
                                }
                            }
                            Err(e) => {
                                if let Some(ref tx) = progress_tx {
                                    let _ = tx
                                        .send(DownloadProgress {
                                            game_id: db_id,
                                            image_type,
                                            progress: 0.0,
                                            status: "failed".to_string(),
                                            local_path: None,
                                            error: Some(e.to_string()),
                                        })
                                        .await;
                                }
                            }
                        }
                    }
                    Ok(response) => {
                        if let Some(ref tx) = progress_tx {
                            let _ = tx
                                .send(DownloadProgress {
                                    game_id: db_id,
                                    image_type,
                                    progress: 0.0,
                                    status: "failed".to_string(),
                                    local_path: None,
                                    error: Some(format!("HTTP {}", response.status())),
                                })
                                .await;
                        }
                    }
                    Err(e) => {
                        if let Some(ref tx) = progress_tx {
                            let _ = tx
                                .send(DownloadProgress {
                                    game_id: db_id,
                                    image_type,
                                    progress: 0.0,
                                    status: "failed".to_string(),
                                    local_path: None,
                                    error: Some(e.to_string()),
                                })
                                .await;
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all downloads
        for handle in handles {
            let _ = handle.await;
        }

        let final_results = results.read().await.clone();
        Ok(final_results)
    }

    /// Import game images from LaunchBox metadata into local database
    pub async fn import_images_from_launchbox(
        &self,
        importer: &crate::import::LaunchBoxImporter,
        progress_callback: Option<impl Fn(i64, i64)>,
    ) -> Result<i64> {
        let total = importer.count_game_images().await?;
        let batch_size = 10000i64;
        let mut imported = 0i64;
        let mut offset = 0i64;

        tracing::info!("Importing {} game images from LaunchBox...", total);

        while offset < total {
            let images = importer.get_all_game_images(offset, batch_size).await?;
            let batch_count = images.len() as i64;

            if batch_count == 0 {
                break;
            }

            // Batch insert
            for chunk in images.chunks(1000) {
                let mut values = Vec::new();
                for _ in chunk {
                    values.push("(?, ?, ?, ?, ?, 0)");
                }

                let sql = format!(
                    "INSERT OR IGNORE INTO game_images (launchbox_db_id, filename, image_type, region, crc32, priority) VALUES {}",
                    values.join(", ")
                );

                let mut query = sqlx::query(&sql);
                for img in chunk {
                    query = query
                        .bind(img.database_id)
                        .bind(&img.file_name)
                        .bind(&img.image_type)
                        .bind(&img.region)
                        .bind(img.crc32.to_string());
                }

                query.execute(&self.pool).await?;
            }

            imported += batch_count;
            offset += batch_size;

            if let Some(ref callback) = progress_callback {
                callback(imported, total);
            }

            tracing::debug!("Imported {}/{} game images", imported, total);
        }

        tracing::info!("Finished importing {} game images", imported);
        Ok(imported)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM game_images")
            .fetch_one(&self.pool)
            .await?;

        let (downloaded,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM game_images WHERE downloaded = 1")
                .fetch_one(&self.pool)
                .await?;

        // Calculate disk usage
        let cache_path = self.cache_dir.join("launchbox");
        let disk_usage = if cache_path.exists() {
            calculate_dir_size(&cache_path)?
        } else {
            0
        };

        Ok(CacheStats {
            total_images: total,
            downloaded_images: downloaded,
            disk_usage_bytes: disk_usage,
        })
    }

    /// Download an image using multiple sources with fallback
    ///
    /// Tries sources in order: LaunchBox CDN, libretro-thumbnails, SteamGridDB
    /// Returns the local path on success
    pub async fn download_with_fallback(
        &self,
        game_title: &str,
        platform: &str,
        image_type: &str,
        launchbox_db_id: Option<i64>,
        steamgriddb_client: Option<&crate::scraper::SteamGridDBClient>,
    ) -> Result<String> {
        // 1. Try LaunchBox CDN first (if we have database ID and metadata)
        if let Some(db_id) = launchbox_db_id {
            if let Ok(Some(info)) = self.get_image_by_type(db_id, image_type).await {
                if info.downloaded {
                    if let Some(path) = info.local_path {
                        if std::path::Path::new(&path).exists() {
                            tracing::debug!("Using cached LaunchBox image: {}", path);
                            return Ok(path);
                        }
                    }
                }

                // Try to download from LaunchBox CDN
                match self.download_image(info.id).await {
                    Ok(path) => {
                        tracing::debug!("Downloaded from LaunchBox CDN: {}", path);
                        return Ok(path);
                    }
                    Err(e) => {
                        tracing::debug!("LaunchBox CDN failed, trying fallbacks: {}", e);
                    }
                }
            }
        }

        // 2. Try libretro-thumbnails (free, no account needed)
        let libretro_type = libretro::LibRetroImageType::from_launchbox_type(image_type);
        if let Some(lt) = libretro_type {
            let libretro_client = LibRetroThumbnailsClient::new(self.cache_dir.clone());
            if let Some(path) = libretro_client.find_thumbnail(platform, lt, game_title).await {
                tracing::debug!("Downloaded from libretro-thumbnails: {}", path);
                return Ok(path);
            }
        }

        // 3. Try SteamGridDB (requires API key)
        if let Some(client) = steamgriddb_client {
            if client.has_credentials() {
                if let Ok(result) = client.search_and_get_artwork(game_title).await {
                    if let Some((_, artwork)) = result {
                        // Map image type to SteamGridDB artwork type
                        let url = match image_type {
                            "Box - Front" => artwork.grids.first().map(|a| a.url.clone()),
                            "Banner" => artwork.heroes.first().map(|a| a.url.clone()),
                            "Clear Logo" => artwork.logos.first().map(|a| a.url.clone()),
                            _ => artwork.grids.first().map(|a| a.url.clone()),
                        };

                        if let Some(url) = url {
                            // Download and cache
                            match self.download_from_url(&url, "steamgriddb", game_title, image_type).await {
                                Ok(path) => {
                                    tracing::debug!("Downloaded from SteamGridDB: {}", path);
                                    return Ok(path);
                                }
                                Err(e) => {
                                    tracing::debug!("SteamGridDB download failed: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        anyhow::bail!("No image found from any source for: {} - {} - {}", game_title, platform, image_type)
    }

    /// Download an image from a URL and cache it
    async fn download_from_url(
        &self,
        url: &str,
        source: &str,
        game_title: &str,
        image_type: &str,
    ) -> Result<String> {
        // Create a sanitized filename
        let safe_title = game_title
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
            .collect::<String>();

        let safe_type = image_type.replace(" - ", "_").replace(' ', "_");

        // Get extension from URL or default to .png
        let ext = url
            .rsplit('.')
            .next()
            .filter(|e| ["png", "jpg", "jpeg", "webp", "gif"].contains(e))
            .unwrap_or("png");

        let cache_path = self.cache_dir
            .join(source)
            .join(format!("{}_{}.{}", safe_title, safe_type, ext));

        // Check cache first
        if cache_path.exists() {
            return Ok(cache_path.to_string_lossy().to_string());
        }

        // Download
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch image")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), url);
        }

        let bytes = response.bytes().await?;

        // Create parent directories
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&cache_path, &bytes).await?;

        Ok(cache_path.to_string_lossy().to_string())
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_images: i64,
    pub downloaded_images: i64,
    pub disk_usage_bytes: u64,
}

/// Calculate directory size recursively
fn calculate_dir_size(path: &Path) -> Result<u64> {
    let mut size = 0u64;

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                size += calculate_dir_size(&path)?;
            } else {
                size += entry.metadata()?.len();
            }
        }
    }

    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdn_url() {
        let url = ImageService::get_cdn_url("Nintendo - NES/Box - Front/Super Mario Bros.-01.jpg");
        assert!(url.starts_with("https://images.launchbox-app.com/"));
        assert!(url.contains("Nintendo"));
    }

    #[test]
    fn test_image_type_priority() {
        assert!(image_type_priority("Box - Front") < image_type_priority("Screenshot - Gameplay"));
        assert!(
            image_type_priority("Screenshot - Gameplay") < image_type_priority("Fanart - Background")
        );
    }
}
