//! Image download and caching service
//!
//! Handles downloading game images from multiple sources:
//! - LaunchBox CDN (primary, requires metadata import)
//! - libretro-thumbnails (free, no account needed)
//! - SteamGridDB (requires API key)
//! - IGDB (requires Twitch OAuth credentials)
//! - EmuMovies (requires account)
//! - ScreenScraper (requires account, uses ROM checksums)
//!
//! Features:
//! - Parallel downloads with configurable concurrency
//! - Multi-source fallback (tries each source until one succeeds)
//! - Local caching with verification
//! - Progress events for UI updates
//! - Round-robin source selection for testing

pub mod download_service;
pub mod emumovies;
pub mod events;
pub mod libretro;
pub mod media_types;
pub mod source_selector;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};

pub use download_service::{MediaDownloadService, MediaDownloadRequest};
pub use emumovies::{EmuMoviesClient, EmuMoviesConfig, EmuMoviesMediaType};
pub use events::{MediaEvent, MediaEventSender};
pub use libretro::{LibRetroImageType, LibRetroThumbnailsClient};
pub use media_types::{GameMediaId, MediaSource, NormalizedMediaType};
pub use source_selector::RoundRobinSourceSelector;

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
    /// IGDB (requires Twitch OAuth credentials)
    IGDB,
    /// EmuMovies (requires account)
    EmuMovies,
    /// ScreenScraper (requires account, best with ROM checksums)
    ScreenScraper,
}

impl ImageSource {
    /// Get the folder name for this source
    pub fn folder_name(&self) -> &'static str {
        match self {
            ImageSource::Local => "local",
            ImageSource::LaunchBox => "launchbox",
            ImageSource::LibRetro => "libretro",
            ImageSource::SteamGridDB => "steamgriddb",
            ImageSource::IGDB => "igdb",
            ImageSource::EmuMovies => "emumovies",
            ImageSource::ScreenScraper => "screenscraper",
        }
    }

    /// All sources in priority order for cache lookup
    pub fn all_sources() -> &'static [ImageSource] {
        &[
            ImageSource::LaunchBox,
            ImageSource::LibRetro,
            ImageSource::SteamGridDB,
            ImageSource::IGDB,
            ImageSource::EmuMovies,
            ImageSource::ScreenScraper,
        ]
    }

    /// Get source from folder name
    pub fn from_folder_name(name: &str) -> Option<ImageSource> {
        match name {
            "local" => Some(ImageSource::Local),
            "launchbox" => Some(ImageSource::LaunchBox),
            "libretro" => Some(ImageSource::LibRetro),
            "steamgriddb" => Some(ImageSource::SteamGridDB),
            "igdb" => Some(ImageSource::IGDB),
            "emumovies" => Some(ImageSource::EmuMovies),
            "screenscraper" => Some(ImageSource::ScreenScraper),
            _ => None,
        }
    }

    /// Get 2-letter abbreviation for UI display
    pub fn abbreviation(&self) -> &'static str {
        match self {
            ImageSource::Local => "LC",
            ImageSource::LaunchBox => "LB",
            ImageSource::LibRetro => "LR",
            ImageSource::SteamGridDB => "SG",
            ImageSource::IGDB => "IG",
            ImageSource::EmuMovies => "EM",
            ImageSource::ScreenScraper => "SS",
        }
    }
}

/// Normalize image type to folder-safe name (e.g., "Box - Front" -> "box-front")
pub fn normalize_image_type(image_type: &str) -> String {
    image_type
        .to_lowercase()
        .replace(" - ", "-")
        .replace(' ', "-")
}

/// Get the media cache path for a game/source/type combination
/// Structure: {cache_dir}/{game_id}/{source}/{image_type}.png
/// Note: cache_dir is already the media directory (e.g., ~/.local/share/lunchbox/media)
pub fn get_media_path(
    cache_dir: &Path,
    game_id: &str,
    source: ImageSource,
    image_type: &str,
) -> PathBuf {
    cache_dir
        .join(game_id)
        .join(source.folder_name())
        .join(format!("{}.png", normalize_image_type(image_type)))
}

/// Find cached media for a game by checking all source folders
/// Returns the path and source of the first found image
pub fn find_cached_media(
    cache_dir: &Path,
    game_id: &str,
    image_type: &str,
) -> Option<(PathBuf, ImageSource)> {
    let normalized_type = normalize_image_type(image_type);
    let game_dir = cache_dir.join(game_id);

    for source in ImageSource::all_sources() {
        let path = game_dir
            .join(source.folder_name())
            .join(format!("{}.png", normalized_type));
        if path.exists() {
            tracing::debug!("Cache hit: {} from {:?}", path.display(), source);
            return Some((path, *source));
        }
    }
    None
}

/// Get game ID string for cache path (uses launchbox_db_id or hash)
pub fn get_game_cache_id(launchbox_db_id: Option<i64>, game_title: &str, platform: &str) -> String {
    if let Some(db_id) = launchbox_db_id {
        if db_id > 0 {
            return format!("lb-{}", db_id);
        }
    }
    // Fallback: hash of platform + title
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    platform.to_lowercase().hash(&mut hasher);
    game_title.to_lowercase().hash(&mut hasher);
    format!("hash-{:x}", hasher.finish())
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
    /// Tries sources in order:
    /// 1. LaunchBox CDN (if metadata imported)
    /// 2. libretro-thumbnails (free, no account)
    /// 3. SteamGridDB (if API key configured)
    /// 4. IGDB (if Twitch credentials configured)
    /// 5. EmuMovies (if account configured)
    /// 6. ScreenScraper (if account configured, title-based search)
    ///
    /// Returns the local path on success
    pub async fn download_with_fallback(
        &self,
        game_title: &str,
        platform: &str,
        image_type: &str,
        launchbox_db_id: Option<i64>,
        launchbox_platform: Option<&str>,  // LaunchBox platform name for CDN URLs
        libretro_platform: Option<&str>,   // libretro platform name
        libretro_title: Option<&str>,      // No-Intro title for libretro lookups
        steamgriddb_client: Option<&crate::scraper::SteamGridDBClient>,
        igdb_client: Option<&crate::scraper::IGDBClient>,
        emumovies_client: Option<&EmuMoviesClient>,
        screenscraper_client: Option<&crate::scraper::ScreenScraperClient>,
    ) -> Result<String> {
        // Compute game_id for cache path
        let game_id = get_game_cache_id(launchbox_db_id, game_title, platform);

        tracing::info!("download_with_fallback: game='{}', platform='{}', type='{}', game_id={}",
            game_title, platform, image_type, game_id);

        // Check cache first - this is the fast path for already-downloaded images
        if let Some((path, source)) = find_cached_media(&self.cache_dir, &game_id, image_type) {
            tracing::info!("  Cache hit from {:?}: {}", source, path.display());
            return Ok(path.to_string_lossy().to_string());
        }
        tracing::info!("  No cached image found, trying sources...");

        // 1. Try LaunchBox CDN first (construct URL from platform/type/title)
        if let Some(lb_platform) = launchbox_platform {
            tracing::info!("  [1/6] Trying LaunchBox CDN...");
            // LaunchBox CDN URL format: {platform}/{image_type}/{game_title}-01.jpg
            // Example: Nintendo Entertainment System/Box - Front/Super Mario Bros.-01.jpg
            // Each path segment must be URL encoded separately (not the slashes)
            let cdn_url = format!(
                "{}/{}/{}/{}-01.jpg",
                LAUNCHBOX_CDN_URL,
                urlencoding::encode(lb_platform),
                urlencoding::encode(image_type),
                urlencoding::encode(game_title)
            );
            tracing::info!("  [1/6] Trying URL: {}", cdn_url);
            match self.download_to_cache(&cdn_url, &game_id, ImageSource::LaunchBox, image_type).await {
                Ok(path) => {
                    tracing::info!("  [1/6] SUCCESS from LaunchBox CDN: {}", path);
                    return Ok(path);
                }
                Err(e) => {
                    tracing::info!("  [1/6] LaunchBox CDN failed: {}", e);
                }
            }
        } else {
            tracing::info!("  [1/6] Skipping LaunchBox (no launchbox_platform)");
        }

        // 2. Try libretro-thumbnails (free, no account needed)
        tracing::info!("  [2/6] Trying libretro-thumbnails...");
        let libretro_type = libretro::LibRetroImageType::from_launchbox_type(image_type);
        if let Some(lt) = libretro_type {
            // Use libretro_platform if provided, otherwise fall back to regular platform name
            let lr_platform = libretro_platform.unwrap_or(platform);
            let libretro_client = LibRetroThumbnailsClient::new(self.cache_dir.clone());

            // Build list of titles to try
            let mut titles_to_try = Vec::new();

            // If we have libretro_title (No-Intro format), try it first
            if let Some(lr_title) = libretro_title {
                titles_to_try.push(lr_title.to_string());
            }

            // Also try the game title with common region suffixes
            let base_title = game_title;
            if !base_title.contains('(') {
                // No region code - try common ones
                titles_to_try.push(format!("{} (World)", base_title));
                titles_to_try.push(format!("{} (USA)", base_title));
                titles_to_try.push(format!("{} (USA, Europe)", base_title));
                titles_to_try.push(format!("{} (Europe)", base_title));
                titles_to_try.push(format!("{} (Japan)", base_title));
            }
            // Also try the raw title
            titles_to_try.push(base_title.to_string());

            tracing::info!("  [2/6] libretro type={:?}, platform='{}', trying {} titles...", lt, lr_platform, titles_to_try.len());

            for lr_title in &titles_to_try {
                if let Some(url) = libretro_client.get_thumbnail_url(lr_platform, lt, lr_title) {
                    tracing::info!("  [2/6] Trying URL: {}", url);
                    match self.download_to_cache(&url, &game_id, ImageSource::LibRetro, image_type).await {
                        Ok(path) => {
                            tracing::info!("  [2/6] SUCCESS from libretro-thumbnails: {}", path);
                            return Ok(path);
                        }
                        Err(_) => {
                            // Try next title
                            continue;
                        }
                    }
                }
            }
            tracing::info!("  [2/6] libretro-thumbnails: no image found after trying {} titles", titles_to_try.len());
        } else {
            tracing::info!("  [2/6] Skipping libretro (unsupported image type)");
        }

        // 3. Try SteamGridDB (requires API key)
        tracing::info!("  [3/6] Trying SteamGridDB...");
        if let Some(client) = steamgriddb_client {
            if client.has_credentials() {
                tracing::info!("  [3/6] SteamGridDB has credentials, searching...");
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
                            match self.download_to_cache(&url, &game_id, ImageSource::SteamGridDB, image_type).await {
                                Ok(path) => {
                                    tracing::info!("  [3/6] SUCCESS from SteamGridDB: {}", path);
                                    return Ok(path);
                                }
                                Err(e) => {
                                    tracing::info!("  [3/6] SteamGridDB download failed: {}", e);
                                }
                            }
                        } else {
                            tracing::info!("  [3/6] SteamGridDB: no matching artwork type");
                        }
                    } else {
                        tracing::info!("  [3/6] SteamGridDB: no results found");
                    }
                } else {
                    tracing::info!("  [3/6] SteamGridDB: search failed");
                }
            } else {
                tracing::info!("  [3/6] SteamGridDB: no credentials configured");
            }
        } else {
            tracing::info!("  [3/6] SteamGridDB: client not available");
        }

        // 4. Try IGDB (requires Twitch OAuth credentials)
        tracing::info!("  [4/6] Trying IGDB...");
        if let Some(client) = igdb_client {
            if client.has_credentials() {
                tracing::info!("  [4/6] IGDB has credentials, searching...");
                if let Ok(games) = client.search_games(game_title, 1).await {
                    if let Some(game) = games.first() {
                        // Map image type to IGDB image
                        let image = match image_type {
                            "Box - Front" => game.cover.as_ref(),
                            "Screenshot - Gameplay" | "Screenshot" => {
                                game.screenshots.as_ref().and_then(|s| s.first())
                            }
                            "Fanart - Background" => {
                                game.artworks.as_ref().and_then(|a| a.first())
                            }
                            _ => game.cover.as_ref(),
                        };

                        if let Some(img) = image {
                            // Use 720p size for good quality
                            let url = img.url("720p");
                            match self.download_to_cache(&url, &game_id, ImageSource::IGDB, image_type).await {
                                Ok(path) => {
                                    tracing::info!("  [4/6] SUCCESS from IGDB: {}", path);
                                    return Ok(path);
                                }
                                Err(e) => {
                                    tracing::info!("  [4/6] IGDB download failed: {}", e);
                                }
                            }
                        } else {
                            tracing::info!("  [4/6] IGDB: no matching image type");
                        }
                    } else {
                        tracing::info!("  [4/6] IGDB: no results found");
                    }
                } else {
                    tracing::info!("  [4/6] IGDB: search failed");
                }
            } else {
                tracing::info!("  [4/6] IGDB: no credentials configured");
            }
        } else {
            tracing::info!("  [4/6] IGDB: client not available");
        }

        // 5. Try EmuMovies (requires account, uses FTP - blocking)
        tracing::info!("  [5/6] Trying EmuMovies...");
        if let Some(client) = emumovies_client {
            if client.has_credentials() {
                if let Some(media_type) = emumovies::EmuMoviesMediaType::from_launchbox_type(image_type) {
                    tracing::info!("  [5/6] EmuMovies has credentials, searching via FTP...");
                    let cache_dir = self.cache_dir.clone();
                    let game_id_clone = game_id.clone();
                    let image_type_str = image_type.to_string();
                    let client = client.clone();
                    let platform = platform.to_string();
                    let game_title = game_title.to_string();
                    let result = tokio::task::spawn_blocking(move || {
                        client.download_to_path(&platform, media_type, &game_title, &cache_dir, &game_id_clone, &image_type_str)
                    }).await;
                    if let Ok(Ok(path)) = result {
                        tracing::info!("  [5/6] SUCCESS from EmuMovies: {}", path);
                        return Ok(path);
                    }
                    tracing::info!("  [5/6] EmuMovies: not found");
                } else {
                    tracing::info!("  [5/6] EmuMovies: unsupported media type");
                }
            } else {
                tracing::info!("  [5/6] EmuMovies: no credentials configured");
            }
        } else {
            tracing::info!("  [5/6] EmuMovies: client not available");
        }

        // 6. Try ScreenScraper (requires account)
        tracing::info!("  [6/6] Trying ScreenScraper...");
        if let Some(client) = screenscraper_client {
            if client.has_credentials() {
                let platform_id = crate::scraper::screenscraper::get_screenscraper_platform_id(platform);
                tracing::info!("  [6/6] ScreenScraper has credentials, searching (platform_id={:?})...", platform_id);

                if let Ok(Some(game)) = client.lookup_by_checksum(
                    "",  // CRC32
                    "",  // MD5
                    "",  // SHA1
                    0,   // file size
                    game_title,
                    platform_id,
                ).await {
                    let url = match image_type {
                        "Box - Front" => game.media.box_front,
                        "Box - Back" => game.media.box_back,
                        "Screenshot - Gameplay" | "Screenshot" => game.media.screenshot,
                        "Fanart - Background" => game.media.fanart,
                        "Clear Logo" => game.media.wheel,
                        _ => game.media.box_front,
                    };

                    if let Some(url) = url {
                        match self.download_to_cache(&url, &game_id, ImageSource::ScreenScraper, image_type).await {
                            Ok(path) => {
                                tracing::info!("  [6/6] SUCCESS from ScreenScraper: {}", path);
                                return Ok(path);
                            }
                            Err(e) => {
                                tracing::info!("  [6/6] ScreenScraper download failed: {}", e);
                            }
                        }
                    } else {
                        tracing::info!("  [6/6] ScreenScraper: no matching image type");
                    }
                } else {
                    tracing::info!("  [6/6] ScreenScraper: no results found");
                }
            } else {
                tracing::info!("  [6/6] ScreenScraper: no credentials configured");
            }
        } else {
            tracing::info!("  [6/6] ScreenScraper: client not available");
        }

        tracing::info!("  FAILED: No image found from any source for: {} - {} - {}", game_title, platform, image_type);
        anyhow::bail!("No image found from any source for: {} - {} - {}", game_title, platform, image_type)
    }

    /// Download an image from a URL and cache it using new structure
    async fn download_to_cache(
        &self,
        url: &str,
        game_id: &str,
        source: ImageSource,
        image_type: &str,
    ) -> Result<String> {
        let cache_path = get_media_path(&self.cache_dir, game_id, source, image_type);

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
