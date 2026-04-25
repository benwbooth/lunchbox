//! Video player component for game detail view
//!
//! Displays gameplay videos downloaded from EmuMovies.
//! Features:
//! - Auto-loads video when component mounts
//! - Auto-plays when ready (muted, as required by browsers)
//! - Progress bar during download
//! - HTML5 video player with controls
//! - Full width display at top of details panel

use crate::backend_api::{self, file_to_asset_url};
use gloo_timers::callback::{Interval, Timeout};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// Loading state for a video
#[derive(Debug, Clone, PartialEq)]
enum VideoState {
    /// Initial state - not yet checked
    Initial,
    /// Checking if video exists
    Checking,
    /// Video is being downloaded
    Downloading(Option<f32>),
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

fn video_asset_url(path: &str, bust_cache: bool) -> String {
    let mut url = file_to_asset_url(path);
    if bust_cache {
        let separator = if url.contains('?') { '&' } else { '?' };
        url.push(separator);
        url.push_str("v=");
        url.push_str(&(js_sys::Date::now() as i64).to_string());
    }
    url
}

fn normalized_video_progress(progress: Option<f32>) -> Option<f32> {
    progress.map(|value| value.clamp(0.0, 1.0))
}

fn video_progress_percent(progress: Option<f32>) -> i32 {
    (normalized_video_progress(progress).unwrap_or(0.0) * 100.0).round() as i32
}

fn format_video_progress_label(progress: &backend_api::VideoDownloadProgress) -> String {
    let status = progress
        .status
        .clone()
        .unwrap_or_else(|| "Preparing video lookup...".to_string());

    normalized_video_progress(progress.progress)
        .map(|value| format!("{} {}%", status, video_progress_percent(Some(value))))
        .unwrap_or(status)
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
    let (download_status, set_download_status) = signal("Preparing video lookup...".to_string());
    let (load_retry_count, set_load_retry_count) = signal(0u8);
    let cache_key = StoredValue::new(cache_key_str);
    let video_ref: NodeRef<leptos::html::Video> = NodeRef::new();
    let progress_poll: Rc<RefCell<Option<Interval>>> = Rc::new(RefCell::new(None));

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

        let current_state = state.get_untracked();
        let should_skip_cache_probe = matches!(current_state, VideoState::Downloading(_));
        if matches!(
            current_state,
            VideoState::Ready(_) | VideoState::NoVideo | VideoState::Error(_)
        ) {
            return;
        }

        let title = title.get_value();
        let plat = plat.get_value();
        let db_id_opt = if db_id > 0 { Some(db_id) } else { None };

        spawn_local(async move {
            if !should_skip_cache_probe {
                set_state.set(VideoState::Checking);

                match backend_api::check_cached_video(title.clone(), plat.clone(), db_id_opt).await
                {
                    Ok(Some(cached_path)) => {
                        let url = video_asset_url(&cached_path, false);
                        let ready = VideoState::Ready(url);
                        put_cached_video_state(&key, &ready);
                        set_load_retry_count.set(0);
                        set_state.set(ready);
                        return;
                    }
                    Ok(None) => {
                        match backend_api::probe_game_video_available(
                            title.clone(),
                            plat.clone(),
                            db_id_opt,
                        )
                        .await
                        {
                            Ok(true) => {}
                            Ok(false) => {
                                let no_video = VideoState::NoVideo;
                                put_cached_video_state(&key, &no_video);
                                set_state.set(no_video);
                                return;
                            }
                            Err(e) => {
                                let msg = e.to_lowercase();
                                if msg.contains("not configured")
                                    || msg.contains("unknown platform")
                                    || msg.contains("no video")
                                {
                                    let no_video = VideoState::NoVideo;
                                    put_cached_video_state(&key, &no_video);
                                    set_state.set(no_video);
                                    return;
                                }
                                if !(msg.contains("timed out") || msg.contains("task failed")) {
                                    set_state.set(VideoState::Error(e));
                                    return;
                                } else {
                                    // Large platforms like arcade/MAME can take too long to probe.
                                    // Fall through to direct download instead of treating probe timeout
                                    // as definitive failure.
                                }
                            }
                        }
                    }
                    Err(_) => {
                        let no_video = VideoState::NoVideo;
                        put_cached_video_state(&key, &no_video);
                        set_state.set(no_video);
                        return;
                    }
                }
            }

            set_download_status.set("Preparing video lookup...".to_string());
            set_state.set(VideoState::Downloading(None));

            match backend_api::download_game_video(title.clone(), plat.clone(), db_id_opt).await {
                Ok(local_path) => {
                    let url = video_asset_url(&local_path, true);
                    let ready = VideoState::Ready(url);
                    put_cached_video_state(&key, &ready);
                    set_load_retry_count.set(0);
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

    // When a video becomes ready, explicitly reload and play it. This avoids the
    // just-downloaded case where the browser does not pick up the new file until remount.
    Effect::new(move || {
        let current_state = state.get();
        if !matches!(current_state, VideoState::Ready(_)) {
            return;
        }

        let video_ref = video_ref.clone();
        Timeout::new(0, move || {
            if let Some(video) = video_ref.get() {
                video.load();
                let _ = video.play();
            }
        })
        .forget();
    });

    Effect::new(move || {
        let current_state = state.get();
        if !matches!(current_state, VideoState::Downloading(_)) {
            progress_poll.borrow_mut().take();
            return;
        }

        if progress_poll.borrow().is_some() {
            return;
        }

        let title = title.get_value();
        let plat = plat.get_value();
        let db_id_opt = if db_id > 0 { Some(db_id) } else { None };
        let progress_poll = progress_poll.clone();

        let interval = Interval::new(180, move || {
            let title = title.clone();
            let plat = plat.clone();
            spawn_local(async move {
                if let Ok(Some(progress)) =
                    backend_api::get_video_download_progress(title, plat, db_id_opt).await
                {
                    set_download_status.set(format_video_progress_label(&progress));
                    set_state.set(VideoState::Downloading(normalized_video_progress(
                        progress.progress,
                    )));
                }
            });
        });

        *progress_poll.borrow_mut() = Some(interval);
    });

    view! {
        <div class="video-player-section">
            {move || match state.get() {
                VideoState::Initial | VideoState::Checking => view! {
                    <div class="video-not-available"></div>
                }.into_any(),
                VideoState::Downloading(progress) => view! {
                    <div class="video-downloading">
                        <div class="loading-spinner"></div>
                        <span>"Downloading video..."</span>
                        <div class="download-progress">
                            <div
                                class="progress-bar"
                                class:indeterminate=move || normalized_video_progress(progress).is_none()
                                style:width=move || {
                                    normalized_video_progress(progress)
                                        .map(|value| format!("{:.1}%", value * 100.0))
                                        .unwrap_or_else(|| "100%".to_string())
                                }
                            ></div>
                        </div>
                        <span class="video-hint">
                            {move || download_status.get()}
                        </span>
                    </div>
                }.into_any(),
                VideoState::Ready(url) => view! {
                    <div class="video-container">
                        <video
                            node_ref=video_ref
                            src=url
                            controls
                            autoplay
                            muted
                            loop
                            preload="auto"
                            class="game-video"
                            on:loadeddata=move |_| {
                                set_load_retry_count.set(0);
                            }
                            on:playing=move |ev| {
                                set_load_retry_count.set(0);
                                if let Some(target) = ev.target() {
                                    if let Ok(video) = target.dyn_into::<web_sys::HtmlVideoElement>() {
                                        video.set_muted(false);
                                        video.set_volume(0.45);
                                    }
                                }
                            }
                            on:error=move |_| {
                                let retries = load_retry_count.get_untracked();
                                if retries >= 2 {
                                    set_state.set(VideoState::Error("Failed to load downloaded video".to_string()));
                                    return;
                                }

                                set_load_retry_count.set(retries + 1);
                                let key = cache_key.get_value();
                                let title = title.get_value();
                                let plat = plat.get_value();
                                let db_id_opt = if db_id > 0 { Some(db_id) } else { None };

                                spawn_local(async move {
                                    match backend_api::check_cached_video(title, plat, db_id_opt).await {
                                        Ok(Some(cached_path)) => {
                                            let ready = VideoState::Ready(video_asset_url(&cached_path, true));
                                            put_cached_video_state(&key, &ready);
                                            set_state.set(ready);
                                        }
                                        _ => {
                                            set_state.set(VideoState::Error("Failed to load downloaded video".to_string()));
                                        }
                                    }
                                });
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
                VideoState::Error(err) => view! {
                    <div class="video-downloading">
                        <span>"Video lookup failed"</span>
                        <span class="video-hint">{err}</span>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
