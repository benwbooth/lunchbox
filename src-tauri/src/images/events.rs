//! Media download event system
//!
//! Provides event types and sender for communicating download progress
//! from the backend to the frontend via Tauri's event system.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::media_types::{MediaSource, NormalizedMediaType};

/// Event name for media download events
pub const MEDIA_EVENT_NAME: &str = "media-event";

/// Event name for video download events
pub const VIDEO_EVENT_NAME: &str = "video-event";

/// Media download event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MediaEvent {
    /// Download has started
    #[serde(rename_all = "camelCase")]
    Started {
        game_id: i64,
        media_type: String,
        source: String,
    },
    /// Download progress update
    #[serde(rename_all = "camelCase")]
    Progress {
        game_id: i64,
        media_type: String,
        progress: f32,
        source: String,
    },
    /// Download completed successfully
    #[serde(rename_all = "camelCase")]
    Completed {
        game_id: i64,
        media_type: String,
        local_path: String,
        source: String,
    },
    /// Download failed
    #[serde(rename_all = "camelCase")]
    Failed {
        game_id: i64,
        media_type: String,
        error: String,
        source: String,
    },
    /// Download was cancelled
    #[serde(rename_all = "camelCase")]
    Cancelled {
        game_id: i64,
        media_type: String,
    },
}

impl MediaEvent {
    /// Create a Started event
    pub fn started(game_id: i64, media_type: NormalizedMediaType, source: MediaSource) -> Self {
        MediaEvent::Started {
            game_id,
            media_type: media_type.filename().to_string(),
            source: source.as_str().to_string(),
        }
    }

    /// Create a Progress event
    pub fn progress(
        game_id: i64,
        media_type: NormalizedMediaType,
        progress: f32,
        source: MediaSource,
    ) -> Self {
        MediaEvent::Progress {
            game_id,
            media_type: media_type.filename().to_string(),
            progress,
            source: source.as_str().to_string(),
        }
    }

    /// Create a Completed event
    pub fn completed(
        game_id: i64,
        media_type: NormalizedMediaType,
        local_path: String,
        source: MediaSource,
    ) -> Self {
        MediaEvent::Completed {
            game_id,
            media_type: media_type.filename().to_string(),
            local_path,
            source: source.as_str().to_string(),
        }
    }

    /// Create a Failed event
    pub fn failed(
        game_id: i64,
        media_type: NormalizedMediaType,
        error: String,
        source: MediaSource,
    ) -> Self {
        MediaEvent::Failed {
            game_id,
            media_type: media_type.filename().to_string(),
            error,
            source: source.as_str().to_string(),
        }
    }

    /// Create a Cancelled event
    pub fn cancelled(game_id: i64, media_type: NormalizedMediaType) -> Self {
        MediaEvent::Cancelled {
            game_id,
            media_type: media_type.filename().to_string(),
        }
    }

    /// Get the game_id from any event variant
    pub fn game_id(&self) -> i64 {
        match self {
            MediaEvent::Started { game_id, .. } => *game_id,
            MediaEvent::Progress { game_id, .. } => *game_id,
            MediaEvent::Completed { game_id, .. } => *game_id,
            MediaEvent::Failed { game_id, .. } => *game_id,
            MediaEvent::Cancelled { game_id, .. } => *game_id,
        }
    }
}

/// Sender for media events
///
/// Wraps Tauri's AppHandle to emit events to the frontend
#[derive(Clone)]
pub struct MediaEventSender {
    app_handle: Option<AppHandle>,
}

impl MediaEventSender {
    /// Create a new sender with a Tauri AppHandle
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Some(app_handle),
        }
    }

    /// Create a no-op sender (for testing or when events aren't needed)
    pub fn noop() -> Self {
        Self { app_handle: None }
    }

    /// Emit a media event
    pub fn emit(&self, event: MediaEvent) {
        if let Some(ref app) = self.app_handle {
            if let Err(e) = app.emit(MEDIA_EVENT_NAME, &event) {
                tracing::warn!("Failed to emit media event: {}", e);
            }
        }
    }

    /// Emit a started event
    pub fn started(&self, game_id: i64, media_type: NormalizedMediaType, source: MediaSource) {
        self.emit(MediaEvent::started(game_id, media_type, source));
    }

    /// Emit a progress event
    pub fn progress(
        &self,
        game_id: i64,
        media_type: NormalizedMediaType,
        progress: f32,
        source: MediaSource,
    ) {
        self.emit(MediaEvent::progress(game_id, media_type, progress, source));
    }

    /// Emit a completed event
    pub fn completed(
        &self,
        game_id: i64,
        media_type: NormalizedMediaType,
        local_path: String,
        source: MediaSource,
    ) {
        self.emit(MediaEvent::completed(
            game_id, media_type, local_path, source,
        ));
    }

    /// Emit a failed event
    pub fn failed(
        &self,
        game_id: i64,
        media_type: NormalizedMediaType,
        error: String,
        source: MediaSource,
    ) {
        self.emit(MediaEvent::failed(game_id, media_type, error, source));
    }

    /// Emit a cancelled event
    pub fn cancelled(&self, game_id: i64, media_type: NormalizedMediaType) {
        self.emit(MediaEvent::cancelled(game_id, media_type));
    }
}

/// Video download event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VideoEvent {
    /// Video download has started
    #[serde(rename_all = "camelCase")]
    Started {
        game_id: i64,
        total_bytes: Option<u64>,
    },
    /// Video download progress update
    #[serde(rename_all = "camelCase")]
    Progress {
        game_id: i64,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        progress: f32,
    },
    /// Video download completed successfully
    #[serde(rename_all = "camelCase")]
    Completed {
        game_id: i64,
        local_path: String,
    },
    /// Video download failed
    #[serde(rename_all = "camelCase")]
    Failed {
        game_id: i64,
        error: String,
    },
}

impl VideoEvent {
    /// Create a Started event
    pub fn started(game_id: i64, total_bytes: Option<u64>) -> Self {
        VideoEvent::Started { game_id, total_bytes }
    }

    /// Create a Progress event
    pub fn progress(game_id: i64, downloaded_bytes: u64, total_bytes: Option<u64>) -> Self {
        let progress = total_bytes
            .map(|total| downloaded_bytes as f32 / total as f32)
            .unwrap_or(0.0);
        VideoEvent::Progress {
            game_id,
            downloaded_bytes,
            total_bytes,
            progress,
        }
    }

    /// Create a Completed event
    pub fn completed(game_id: i64, local_path: String) -> Self {
        VideoEvent::Completed { game_id, local_path }
    }

    /// Create a Failed event
    pub fn failed(game_id: i64, error: String) -> Self {
        VideoEvent::Failed { game_id, error }
    }
}

/// Sender for video events
#[derive(Clone)]
pub struct VideoEventSender {
    app_handle: Option<AppHandle>,
}

impl VideoEventSender {
    /// Create a new sender with a Tauri AppHandle
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle: Some(app_handle),
        }
    }

    /// Create a no-op sender (for testing or when events aren't needed)
    pub fn noop() -> Self {
        Self { app_handle: None }
    }

    /// Emit a video event
    pub fn emit(&self, event: VideoEvent) {
        if let Some(ref app) = self.app_handle {
            if let Err(e) = app.emit(VIDEO_EVENT_NAME, &event) {
                tracing::warn!("Failed to emit video event: {}", e);
            }
        }
    }

    /// Emit a started event
    pub fn started(&self, game_id: i64, total_bytes: Option<u64>) {
        self.emit(VideoEvent::started(game_id, total_bytes));
    }

    /// Emit a progress event
    pub fn progress(&self, game_id: i64, downloaded_bytes: u64, total_bytes: Option<u64>) {
        self.emit(VideoEvent::progress(game_id, downloaded_bytes, total_bytes));
    }

    /// Emit a completed event
    pub fn completed(&self, game_id: i64, local_path: String) {
        self.emit(VideoEvent::completed(game_id, local_path));
    }

    /// Emit a failed event
    pub fn failed(&self, game_id: i64, error: String) {
        self.emit(VideoEvent::failed(game_id, error));
    }
}
