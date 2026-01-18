//! Video player component for game detail view
//!
//! Displays gameplay videos downloaded from EmuMovies.
//! Features:
//! - Lazy loading (only downloads when section is visible)
//! - Progress bar during download
//! - HTML5 video player with controls

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::tauri::{self, file_to_asset_url};

/// Loading state for a video
#[derive(Debug, Clone, PartialEq)]
pub enum VideoState {
    /// Initial state - checking if video exists
    Checking,
    /// Video is being downloaded
    Downloading { progress: f32 },
    /// Video is ready to play
    Ready { url: String },
    /// No video available for this game
    NoVideo,
    /// Error occurred
    Error(String),
}

/// Video player component
///
/// Automatically checks for cached video and displays download progress
/// if video needs to be fetched from EmuMovies.
#[component]
pub fn VideoPlayer(
    /// Game title (for video lookup)
    game_title: String,
    /// Platform name (for video lookup)
    platform: String,
    /// LaunchBox database ID
    launchbox_db_id: i64,
) -> impl IntoView {
    let (state, set_state) = signal(VideoState::Checking);
    let (is_expanded, set_is_expanded) = signal(false);

    // Use Arc<AtomicBool> for mounted flag
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let mounted = Arc::new(AtomicBool::new(true));
    let mounted_for_cleanup = mounted.clone();

    // Check for cached video when expanded
    Effect::new(move || {
        let expanded = is_expanded.get();
        if !expanded {
            return;
        }

        let title = game_title.clone();
        let plat = platform.clone();
        let db_id = launchbox_db_id;
        let mounted = mounted.clone();

        spawn_local(async move {
            if !mounted.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            let db_id_opt = if db_id > 0 { Some(db_id) } else { None };

            // Check cache first
            match tauri::check_cached_video(
                title.clone(),
                plat.clone(),
                db_id_opt,
            ).await {
                Ok(Some(cached_path)) => {
                    if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                        let url = file_to_asset_url(&cached_path);
                        let _ = set_state.try_set(VideoState::Ready { url });
                    }
                    return;
                }
                Ok(None) => {
                    // Not cached - need to download
                }
                Err(_) => {
                    if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                        // If cache check fails, assume no video
                        let _ = set_state.try_set(VideoState::NoVideo);
                    }
                    return;
                }
            }

            // Start download
            if !mounted.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let _ = set_state.try_set(VideoState::Downloading { progress: -1.0 });

            match tauri::download_game_video(
                title.clone(),
                plat.clone(),
                db_id_opt,
            ).await {
                Ok(local_path) => {
                    if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                        let url = file_to_asset_url(&local_path);
                        let _ = set_state.try_set(VideoState::Ready { url });
                    }
                }
                Err(e) => {
                    if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                        // Check if it's a "not found" error vs an actual error
                        if e.contains("not found") || e.contains("No video") || e.contains("Unknown platform") {
                            let _ = set_state.try_set(VideoState::NoVideo);
                        } else {
                            let _ = set_state.try_set(VideoState::Error(e));
                        }
                    }
                }
            }
        });
    });

    // Cleanup on unmount
    on_cleanup(move || {
        mounted_for_cleanup.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    view! {
        <div class="video-player-section">
            <div
                class="video-header"
                on:click=move |_| set_is_expanded.update(|e| *e = !*e)
            >
                <span class="video-toggle">
                    {move || if is_expanded.get() { "▼" } else { "▶" }}
                </span>
                <h3>"Video"</h3>
                {move || match state.get() {
                    VideoState::Ready { .. } => view! {
                        <span class="video-badge ready">"Available"</span>
                    }.into_any(),
                    VideoState::Downloading { .. } => view! {
                        <span class="video-badge downloading">"Downloading..."</span>
                    }.into_any(),
                    VideoState::NoVideo => view! {
                        <span class="video-badge no-video">"Not Available"</span>
                    }.into_any(),
                    VideoState::Error(_) => view! {
                        <span class="video-badge error">"Error"</span>
                    }.into_any(),
                    VideoState::Checking => view! {
                        <span class="video-badge checking">"..."</span>
                    }.into_any(),
                }}
            </div>

            <Show when=move || is_expanded.get()>
                <div class="video-content">
                    {move || match state.get() {
                        VideoState::Checking => view! {
                            <div class="video-loading">
                                <div class="loading-spinner"></div>
                                <span>"Checking for video..."</span>
                            </div>
                        }.into_any(),
                        VideoState::Downloading { progress } => {
                            let is_indeterminate = progress < 0.0;
                            let progress_pct = if is_indeterminate { 100 } else { (progress * 100.0) as i32 };
                            let bar_class = if is_indeterminate { "progress-bar indeterminate" } else { "progress-bar" };
                            view! {
                                <div class="video-downloading">
                                    <div class="download-status">
                                        <span>"Downloading video from EmuMovies..."</span>
                                        <div class="download-progress">
                                            <div class=bar_class style:width=format!("{}%", progress_pct)></div>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        VideoState::Ready { url } => view! {
                            <div class="video-container">
                                <video
                                    src=url
                                    controls
                                    preload="metadata"
                                    class="game-video"
                                >
                                    "Your browser does not support the video tag."
                                </video>
                            </div>
                        }.into_any(),
                        VideoState::NoVideo => view! {
                            <div class="video-not-available">
                                <span>"No video available for this game."</span>
                                <span class="video-hint">"Videos are sourced from EmuMovies."</span>
                            </div>
                        }.into_any(),
                        VideoState::Error(e) => view! {
                            <div class="video-error">
                                <span>"Error loading video"</span>
                                <span class="error-detail">{e}</span>
                            </div>
                        }.into_any(),
                    }}
                </div>
            </Show>
        </div>
    }
}
