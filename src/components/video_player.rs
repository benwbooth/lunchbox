//! Video player component for game detail view
//!
//! Displays gameplay videos downloaded from EmuMovies.
//! Features:
//! - Auto-loads video when component mounts
//! - Auto-plays when ready (muted, as required by browsers)
//! - Progress bar during download
//! - HTML5 video player with controls
//! - Full width display at top of details panel

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::tauri::{self, file_to_asset_url};

/// Loading state for a video
#[derive(Debug, Clone, PartialEq)]
pub enum VideoState {
    /// Initial state - not yet checked
    Initial,
    /// Checking if video exists
    Checking,
    /// Video is being downloaded
    Downloading,
    /// Video is ready to play
    Ready(String),
    /// No video available for this game
    NoVideo,
    /// Error occurred
    Error(String),
}

/// Video player component
///
/// Auto-loads and auto-plays video at the top of the game details panel.
#[component]
pub fn VideoPlayer(
    /// Game title (for video lookup)
    game_title: String,
    /// Platform name (for video lookup)
    platform: String,
    /// LaunchBox database ID
    launchbox_db_id: i64,
) -> impl IntoView {
    let (state, set_state) = signal(VideoState::Initial);

    // Store props in signals to avoid closure capture issues
    let title = StoredValue::new(game_title);
    let plat = StoredValue::new(platform);
    let db_id = launchbox_db_id;

    // Auto-load video on mount
    Effect::new(move || {
        // Only load once
        if state.get() != VideoState::Initial {
            return;
        }

        set_state.set(VideoState::Checking);

        let title = title.get_value();
        let plat = plat.get_value();
        let db_id_opt = if db_id > 0 { Some(db_id) } else { None };

        spawn_local(async move {
            // Check cache first
            match tauri::check_cached_video(title.clone(), plat.clone(), db_id_opt).await {
                Ok(Some(cached_path)) => {
                    let url = file_to_asset_url(&cached_path);
                    set_state.set(VideoState::Ready(url));
                    return;
                }
                Ok(None) => {
                    // Not cached - try to download
                }
                Err(_) => {
                    set_state.set(VideoState::NoVideo);
                    return;
                }
            }

            // Try downloading
            set_state.set(VideoState::Downloading);

            match tauri::download_game_video(title.clone(), plat.clone(), db_id_opt).await {
                Ok(local_path) => {
                    let url = file_to_asset_url(&local_path);
                    set_state.set(VideoState::Ready(url));
                }
                Err(e) => {
                    if e.contains("not found") || e.contains("No video") || e.contains("Unknown platform") || e.contains("not configured") {
                        set_state.set(VideoState::NoVideo);
                    } else {
                        set_state.set(VideoState::Error(e));
                    }
                }
            }
        });
    });

    view! {
        <div class="video-player-section">
            {move || match state.get() {
                VideoState::Initial | VideoState::Checking => view! {
                    <div class="video-loading">
                        <div class="loading-spinner"></div>
                        <span>"Checking for video..."</span>
                    </div>
                }.into_any(),
                VideoState::Downloading => view! {
                    <div class="video-downloading">
                        <div class="download-status">
                            <span>"Downloading video..."</span>
                            <div class="download-progress">
                                <div class="progress-bar indeterminate" style="width: 100%"></div>
                            </div>
                        </div>
                    </div>
                }.into_any(),
                VideoState::Ready(url) => view! {
                    <div class="video-container">
                        <video
                            src=url
                            controls
                            autoplay
                            muted
                            loop
                            preload="auto"
                            class="game-video"
                        >
                            "Your browser does not support the video tag."
                        </video>
                    </div>
                }.into_any(),
                // Don't show anything if no video available - clean UI
                VideoState::NoVideo => view! {
                    <div class="video-not-available"></div>
                }.into_any(),
                VideoState::Error(_) => view! {
                    <div class="video-not-available"></div>
                }.into_any(),
            }}
        </div>
    }
}
