//! Media download service with priority queue and viewport awareness
//!
//! Handles downloading game media with:
//! - Priority queue (visible games download first)
//! - Cancellation support (when games scroll out of viewport)
//! - Round-robin source selection
//! - Progress events to frontend

use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};

use super::events::MediaEventSender;
use super::media_types::{GameMediaId, MediaSource, NormalizedMediaType};
use super::source_selector::RoundRobinSourceSelector;
use super::LAUNCHBOX_CDN_URL;

/// Default concurrent downloads
const DEFAULT_CONCURRENCY: usize = 6;

/// Download request for the queue
#[derive(Debug, Clone)]
pub struct MediaDownloadRequest {
    /// LaunchBox database ID
    pub launchbox_db_id: i64,
    /// Game title (for sources that need it)
    pub game_title: String,
    /// Platform name (for sources that need it)
    pub platform: String,
    /// Media type to download
    pub media_type: NormalizedMediaType,
    /// Priority (lower = higher priority, visible games get 0)
    pub priority: i32,
}

impl MediaDownloadRequest {
    /// Create a new download request
    pub fn new(
        launchbox_db_id: i64,
        game_title: String,
        platform: String,
        media_type: NormalizedMediaType,
    ) -> Self {
        Self {
            launchbox_db_id,
            game_title,
            platform,
            media_type,
            priority: 100, // Default priority
        }
    }

    /// Set priority (0 = visible/highest priority)
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Internal download task (for future batched downloads)
#[allow(dead_code)]
struct DownloadTask {
    request: MediaDownloadRequest,
    cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Media download service
pub struct MediaDownloadService {
    pool: SqlitePool,
    cache_dir: PathBuf,
    client: reqwest::Client,
    concurrency: usize,
    event_sender: MediaEventSender,
    source_selector: RoundRobinSourceSelector,
    /// Currently downloading game IDs (to avoid duplicates)
    downloading: Arc<RwLock<HashSet<(i64, NormalizedMediaType)>>>,
    /// Pending requests channel
    request_tx: mpsc::Sender<MediaDownloadRequest>,
    /// Currently visible game IDs (for priority)
    viewport_games: Arc<RwLock<HashSet<i64>>>,
    /// Cancel channels by (game_id, media_type)
    cancel_channels: Arc<RwLock<HashMap<(i64, NormalizedMediaType), tokio::sync::oneshot::Sender<()>>>>,
}

impl MediaDownloadService {
    /// Create a new download service
    pub fn new(
        pool: SqlitePool,
        cache_dir: PathBuf,
        event_sender: MediaEventSender,
    ) -> (Self, mpsc::Receiver<MediaDownloadRequest>) {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/1.0")
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        let (request_tx, request_rx) = mpsc::channel(1000);

        let service = Self {
            pool,
            cache_dir,
            client,
            concurrency: DEFAULT_CONCURRENCY,
            event_sender,
            source_selector: RoundRobinSourceSelector::new(),
            downloading: Arc::new(RwLock::new(HashSet::new())),
            request_tx,
            viewport_games: Arc::new(RwLock::new(HashSet::new())),
            cancel_channels: Arc::new(RwLock::new(HashMap::new())),
        };

        (service, request_rx)
    }

    /// Set download concurrency
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Set source selector
    pub fn with_source_selector(mut self, selector: RoundRobinSourceSelector) -> Self {
        self.source_selector = selector;
        self
    }

    /// Get the request sender for queueing downloads
    pub fn request_sender(&self) -> mpsc::Sender<MediaDownloadRequest> {
        self.request_tx.clone()
    }

    /// Request a media download
    pub async fn request_download(&self, request: MediaDownloadRequest) -> Result<()> {
        self.request_tx
            .send(request)
            .await
            .context("Failed to queue download request")?;
        Ok(())
    }

    /// Update the visible games in the viewport
    pub async fn update_viewport(&self, visible_game_ids: Vec<i64>) {
        let mut viewport = self.viewport_games.write().await;
        viewport.clear();
        viewport.extend(visible_game_ids);
    }

    /// Cancel downloads for games that are no longer visible
    pub async fn cancel_non_visible(&self) {
        let viewport = self.viewport_games.read().await;
        let mut cancels = self.cancel_channels.write().await;

        let to_cancel: Vec<(i64, NormalizedMediaType)> = cancels
            .keys()
            .filter(|(game_id, _)| !viewport.contains(game_id))
            .cloned()
            .collect();

        for key in to_cancel {
            if let Some(tx) = cancels.remove(&key) {
                let _ = tx.send(());
                self.event_sender.cancelled(key.0, key.1);
            }
        }
    }

    /// Check if a game is currently in the viewport
    pub async fn is_in_viewport(&self, game_id: i64) -> bool {
        self.viewport_games.read().await.contains(&game_id)
    }

    /// Start the download worker loop
    pub async fn run(self: Arc<Self>, mut request_rx: mpsc::Receiver<MediaDownloadRequest>) {
        let semaphore = Arc::new(Semaphore::new(self.concurrency));

        while let Some(request) = request_rx.recv().await {
            let key = (request.launchbox_db_id, request.media_type);

            // Skip if already downloading
            {
                let downloading = self.downloading.read().await;
                if downloading.contains(&key) {
                    continue;
                }
            }

            // Mark as downloading
            {
                let mut downloading = self.downloading.write().await;
                downloading.insert(key);
            }

            // Create cancel channel
            let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
            {
                let mut cancels = self.cancel_channels.write().await;
                cancels.insert(key, cancel_tx);
            }

            // Spawn download task
            let service = self.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                let _permit = permit;
                service.download_media(request, cancel_rx).await;
            });
        }
    }

    /// Download media for a single request
    async fn download_media(
        &self,
        request: MediaDownloadRequest,
        cancel_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        let key = (request.launchbox_db_id, request.media_type);

        // Get source for this game
        let source = self
            .source_selector
            .source_for_game_and_type(request.launchbox_db_id, request.media_type);

        // Emit started event
        self.event_sender
            .started(request.launchbox_db_id, request.media_type, source);

        // Try to download with cancellation support
        let result = tokio::select! {
            _ = cancel_rx => {
                Err(anyhow::anyhow!("Cancelled"))
            }
            result = self.download_from_source(&request, source) => {
                result
            }
        };

        // Clean up
        {
            let mut downloading = self.downloading.write().await;
            downloading.remove(&key);
        }
        {
            let mut cancels = self.cancel_channels.write().await;
            cancels.remove(&key);
        }

        // Emit result event
        match result {
            Ok(local_path) => {
                self.event_sender.completed(
                    request.launchbox_db_id,
                    request.media_type,
                    local_path,
                    source,
                );
            }
            Err(e) if e.to_string() == "Cancelled" => {
                // Already emitted cancelled event
            }
            Err(e) => {
                self.event_sender.failed(
                    request.launchbox_db_id,
                    request.media_type,
                    e.to_string(),
                    source,
                );
            }
        }
    }

    /// Download from a specific source
    async fn download_from_source(
        &self,
        request: &MediaDownloadRequest,
        source: MediaSource,
    ) -> Result<String> {
        match source {
            MediaSource::LaunchBox => {
                self.download_from_launchbox(request).await
            }
            MediaSource::LibRetro => {
                self.download_from_libretro(request).await
            }
            _ => {
                // Other sources not yet implemented - fall back to LaunchBox
                self.download_from_launchbox(request).await
            }
        }
    }

    /// Download from LaunchBox CDN
    async fn download_from_launchbox(&self, request: &MediaDownloadRequest) -> Result<String> {
        // Look up the filename from the game_images table
        let launchbox_type = request.media_type.to_launchbox_type();

        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT filename FROM game_images
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
        .bind(request.launchbox_db_id)
        .bind(launchbox_type)
        .fetch_optional(&self.pool)
        .await?;

        let filename = row
            .map(|(f,)| f)
            .ok_or_else(|| anyhow::anyhow!("No LaunchBox image found"))?;

        // Build CDN URL
        let url = format!("{}/{}", LAUNCHBOX_CDN_URL, urlencoding::encode(&filename));

        // Build local path using new structure
        let game_id = GameMediaId::from_launchbox_id(request.launchbox_db_id);
        let extension = filename
            .rsplit('.')
            .next()
            .unwrap_or("png");
        let local_path = game_id.media_path(&self.cache_dir, request.media_type, extension);

        // Check if already exists
        if local_path.exists() {
            return Ok(local_path.to_string_lossy().to_string());
        }

        // Create parent directories
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Download
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), url);
        }

        let bytes = response.bytes().await?;
        tokio::fs::write(&local_path, &bytes).await?;

        // Update game_media table
        self.record_download(
            request.launchbox_db_id,
            request.media_type,
            MediaSource::LaunchBox,
            &url,
            &local_path.to_string_lossy(),
        )
        .await?;

        Ok(local_path.to_string_lossy().to_string())
    }

    /// Download from libretro-thumbnails
    async fn download_from_libretro(&self, request: &MediaDownloadRequest) -> Result<String> {
        let _libretro_type_name = request
            .media_type
            .to_libretro_type()
            .ok_or_else(|| anyhow::anyhow!("Media type not supported by libretro"))?;

        let client = super::LibRetroThumbnailsClient::new(self.cache_dir.clone());

        let libretro_image_type = match request.media_type {
            NormalizedMediaType::BoxFront | NormalizedMediaType::BoxBack => {
                super::LibRetroImageType::Boxart
            }
            NormalizedMediaType::Screenshot => super::LibRetroImageType::Snap,
            NormalizedMediaType::TitleScreen => super::LibRetroImageType::Title,
            _ => return Err(anyhow::anyhow!("Unsupported libretro type")),
        };

        // Try to find/download the thumbnail
        let local_path = client
            .find_thumbnail(&request.platform, libretro_image_type, &request.game_title)
            .await
            .ok_or_else(|| anyhow::anyhow!("Not found in libretro-thumbnails"))?;

        // Record in database
        self.record_download(
            request.launchbox_db_id,
            request.media_type,
            MediaSource::LibRetro,
            "",
            &local_path,
        )
        .await?;

        Ok(local_path)
    }

    /// Record a download in the game_media table
    async fn record_download(
        &self,
        launchbox_db_id: i64,
        media_type: NormalizedMediaType,
        source: MediaSource,
        source_url: &str,
        local_path: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO game_media (launchbox_db_id, media_type, source, source_url, local_path, status, downloaded_at)
            VALUES (?, ?, ?, ?, ?, 'completed', CURRENT_TIMESTAMP)
            ON CONFLICT(launchbox_db_id, media_type, source) DO UPDATE SET
                local_path = excluded.local_path,
                status = 'completed',
                downloaded_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(launchbox_db_id)
        .bind(media_type.filename())
        .bind(source.as_str())
        .bind(source_url)
        .bind(local_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the local path for a cached media file (if it exists)
    pub async fn get_cached_path(
        &self,
        launchbox_db_id: i64,
        media_type: NormalizedMediaType,
    ) -> Option<String> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT local_path FROM game_media
            WHERE launchbox_db_id = ? AND media_type = ? AND status = 'completed' AND local_path IS NOT NULL
            LIMIT 1
            "#,
        )
        .bind(launchbox_db_id)
        .bind(media_type.filename())
        .fetch_optional(&self.pool)
        .await
        .ok()?;

        row.map(|(path,)| path)
    }

    /// Check if media is cached
    pub async fn is_cached(
        &self,
        launchbox_db_id: i64,
        media_type: NormalizedMediaType,
    ) -> bool {
        self.get_cached_path(launchbox_db_id, media_type)
            .await
            .is_some()
    }
}
