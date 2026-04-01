//! Video player component for game detail view
//!
//! Displays gameplay videos downloaded from EmuMovies.
//! Features:
//! - Auto-loads video when component mounts
//! - Auto-plays when ready (muted, as required by browsers)
//! - Progress bar during download
//! - HTML5 video player with controls
//! - Full width display at top of details panel

use crate::tauri::{self, file_to_asset_url};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

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

thread_local! {
    static VIDEO_STATE_CACHE: RefCell<HashMap<String, VideoState>> = RefCell::new(HashMap::new());
}

fn video_cache_key(game_title: &str, platform: &str, launchbox_db_id: i64) -> String {
    if launchbox_db_id > 0 {
        format!("db:{}", launchbox_db_id)
    } else {
        format!("{}::{}", platform, game_title)
    }
}

fn get_cached_video_state(key: &str) -> Option<VideoState> {
    VIDEO_STATE_CACHE.with(|cache| cache.borrow().get(key).cloned())
}

fn put_cached_video_state(key: &str, state: &VideoState) {
    if matches!(state, VideoState::Ready(_) | VideoState::NoVideo) {
        VIDEO_STATE_CACHE.with(|cache| {
            cache.borrow_mut().insert(key.to_string(), state.clone());
        });
    }
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
    let cache_key_str = video_cache_key(&game_title, &platform, launchbox_db_id);
    let initial_state = get_cached_video_state(&cache_key_str).unwrap_or(VideoState::Initial);
    let (state, set_state) = signal(initial_state);
    let cache_key = StoredValue::new(cache_key_str);

    // Store props in signals to avoid closure capture issues
    let title = StoredValue::new(game_title);
    let plat = StoredValue::new(platform);
    let db_id = launchbox_db_id;

    // Auto-load video on mount
    Effect::new(move || {
        let key = cache_key.get_value();
        if let Some(cached) = get_cached_video_state(&key) {
            set_state.set(cached);
            return;
        }

        // Only load once for this mounted component
        if state.get_untracked() != VideoState::Initial {
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
                    let ready = VideoState::Ready(url);
                    put_cached_video_state(&key, &ready);
                    set_state.set(ready);
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
                    let ready = VideoState::Ready(url);
                    put_cached_video_state(&key, &ready);
                    set_state.set(ready);
                }
                Err(e) => {
                    let msg = e.to_lowercase();
                    if msg.contains("not found")
                        || msg.contains("no video")
                        || msg.contains("unknown platform")
                        || msg.contains("not configured")
                    {
                        let no_video = VideoState::NoVideo;
                        put_cached_video_state(&key, &no_video);
                        set_state.set(no_video);
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
                        <div class="loading-spinner"></div>
                        <span>"Downloading video..."</span>
                        <span class="video-hint">"This may take a few seconds."</span>
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
                            on:playing=move |ev| {
                                if let Some(target) = ev.target() {
                                    if let Ok(video) = target.dyn_into::<web_sys::HtmlVideoElement>() {
                                        video.set_muted(false);
                                        video.set_volume(0.45);
                                    }
                                }
                            }
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
