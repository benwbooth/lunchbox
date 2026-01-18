//! Lazy-loading image component with on-demand download from multiple sources
//!
//! Supports:
//! - LaunchBox CDN (primary, requires metadata import)
//! - libretro-thumbnails (free, no account needed)
//! - SteamGridDB (requires API key)

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::VecDeque;
use crate::tauri::{self, file_to_asset_url, log_to_backend, ImageInfo};

/// Maximum concurrent image operations (cache checks + downloads)
const MAX_CONCURRENT_REQUESTS: usize = 6;

/// Request queue for throttling image loads
struct RequestQueue {
    active: usize,
    pending: VecDeque<Box<dyn FnOnce()>>,
}

impl RequestQueue {
    fn new() -> Self {
        Self {
            active: 0,
            pending: VecDeque::new(),
        }
    }
}

thread_local! {
    static REQUEST_QUEUE: RefCell<RequestQueue> = RefCell::new(RequestQueue::new());
}

/// Release a slot and process next pending request
fn release_slot() {
    REQUEST_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        queue.active = queue.active.saturating_sub(1);

        // Process next pending request if any
        if let Some(task) = queue.pending.pop_front() {
            queue.active += 1;
            // Drop borrow before calling task
            drop(queue);
            task();
        }
    });
}

/// Queue a request to run when a slot is available
fn queue_request<F: FnOnce() + 'static>(f: F) {
    REQUEST_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        if queue.active < MAX_CONCURRENT_REQUESTS {
            queue.active += 1;
            drop(queue);
            f();
        } else {
            queue.pending.push_back(Box::new(f));
        }
    });
}

/// Log to backend for debugging
fn log(msg: &str) {
    log_to_backend("info", msg);
}

/// Loading state for an image
#[derive(Debug, Clone, PartialEq)]
pub enum ImageState {
    /// Initial state - checking if image exists
    Loading,
    /// Image is being downloaded from a source
    Downloading {
        /// Progress from 0.0 to 1.0 (or -1.0 for indeterminate)
        progress: f32,
        /// Name of the source being tried (e.g., "LaunchBox", "libretro-thumbnails")
        source: String,
    },
    /// Image is ready to display
    Ready {
        url: String,
        /// Source abbreviation (e.g., "LB", "LR", "SG", "IG")
        source: Option<String>,
    },
    /// No image available for this game
    NoImage,
    /// Error occurred
    Error(String),
}

/// Extract source abbreviation from a file path
fn source_from_path(path: &str) -> Option<String> {
    if path.contains("/steamgriddb/") {
        Some("SG".to_string())
    } else if path.contains("/libretro/") || path.contains("/libretro-thumbnails/") {
        Some("LR".to_string())
    } else if path.contains("/launchbox/") {
        Some("LB".to_string())
    } else if path.contains("/igdb/") {
        Some("IG".to_string())
    } else if path.contains("/emumovies/") {
        Some("EM".to_string())
    } else if path.contains("/screenscraper/") {
        Some("SS".to_string())
    } else {
        None
    }
}

/// Lazy-loading image component
///
/// Automatically loads images from local cache or downloads from multiple sources:
/// 1. LaunchBox CDN (if metadata imported)
/// 2. libretro-thumbnails (free fallback)
/// 3. SteamGridDB (if API key configured)
#[component]
pub fn LazyImage(
    /// LaunchBox database ID of the game
    launchbox_db_id: i64,
    /// Game title (for fallback sources)
    #[prop(default = "".to_string())]
    game_title: String,
    /// Platform name (for fallback sources)
    #[prop(default = "".to_string())]
    platform: String,
    /// Image type to display (e.g., "Box - Front")
    #[prop(default = "Box - Front".to_string())]
    image_type: String,
    /// Alt text for the image
    #[prop(default = "".to_string())]
    alt: String,
    /// CSS class for the image element
    #[prop(default = "".to_string())]
    class: String,
    /// Single character placeholder to show if no image
    #[prop(optional)]
    placeholder: Option<String>,
) -> impl IntoView {
    let (state, set_state) = signal(ImageState::Loading);

    // Create a signal to track the game identity - this makes the Effect re-run when props change
    // This is critical for virtual scrolling where components may be reused with different props
    let (game_key, set_game_key) = signal((launchbox_db_id, game_title.clone(), platform.clone(), image_type.clone()));

    // Update game_key when props differ (handles component reuse in virtual scroll)
    let current_props = (launchbox_db_id, game_title.clone(), platform.clone(), image_type.clone());
    if game_key.get_untracked() != current_props {
        set_game_key.set(current_props);
        set_state.set(ImageState::Loading);
    }

    // Use Arc<AtomicBool> for mounted flag - survives after component disposal and is thread-safe
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let mounted = Arc::new(AtomicBool::new(true));
    let mounted_for_cleanup = mounted.clone();

    Effect::new(move || {
        // Track game_key so this effect re-runs when the game changes
        let (db_id, title, plat, img_type) = game_key.get();
        let mounted = mounted.clone();

        // Skip if no title
        if title.is_empty() {
            let _ = set_state.try_set(ImageState::NoImage);
            return;
        }

        // Step 1: Fast cache check - NOT queued, runs immediately
        spawn_local(async move {
            if !mounted.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            let db_id_opt = if db_id > 0 { Some(db_id) } else { None };
            match tauri::check_cached_media(
                title.clone(),
                plat.clone(),
                img_type.clone(),
                db_id_opt,
            ).await {
                Ok(Some(cached)) => {
                    if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                        let url = file_to_asset_url(&cached.path);
                        let _ = set_state.try_set(ImageState::Ready {
                            url,
                            source: Some(cached.source),
                        });
                    }
                    return; // Cache hit - done!
                }
                Ok(None) => {
                    // Cache miss - need to download
                }
                Err(_e) => {
                    // Error checking cache - try downloading anyway
                }
            }

            // Step 2: Queue download (only downloads are rate-limited)
            if !mounted.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let _ = set_state.try_set(ImageState::Downloading {
                progress: -1.0,
                source: "Queued...".to_string(),
            });

            let title2 = title.clone();
            let plat2 = plat.clone();
            let img_type2 = img_type.clone();
            let mounted2 = mounted.clone();

            queue_request(move || {
                spawn_local(async move {
                    if !mounted2.load(std::sync::atomic::Ordering::Relaxed) {
                        release_slot();
                        return;
                    }

                    let _ = set_state.try_set(ImageState::Downloading {
                        progress: -1.0,
                        source: "Searching...".to_string(),
                    });

                    match tauri::download_image_with_fallback(
                        title2.clone(),
                        plat2.clone(),
                        img_type2.clone(),
                        db_id_opt,
                    ).await {
                        Ok(local_path) => {
                            if mounted2.load(std::sync::atomic::Ordering::Relaxed) {
                                let source = source_from_path(&local_path);
                                let url = file_to_asset_url(&local_path);
                                let _ = set_state.try_set(ImageState::Ready { url, source });
                            }
                        }
                        Err(_e) => {
                            if mounted2.load(std::sync::atomic::Ordering::Relaxed) {
                                let _ = set_state.try_set(ImageState::NoImage);
                            }
                        }
                    }
                    release_slot();
                });
            });
        });
    });

    // Cleanup on unmount
    on_cleanup(move || {
        mounted_for_cleanup.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    let placeholder = placeholder.clone();
    let class = class.clone();

    view! {
        {move || {
            let current_state = state.get();
            let class_str = class.clone();
            let placeholder = placeholder.clone();
            let alt_text = alt.clone();

            match current_state {
                ImageState::Loading => {
                    let char = placeholder.clone().unwrap_or_else(|| "?".to_string());
                    view! {
                        <div class=format!("{} lazy-image-loading", class_str)>
                            <span class="placeholder-char">{char}</span>
                            <div class="download-status-bottom">
                                <div class="download-progress">
                                    <div class="progress-bar indeterminate"></div>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }
                ImageState::Downloading { progress, source } => {
                    let char = placeholder.clone().unwrap_or_else(|| "?".to_string());
                    // progress -1.0 means indeterminate (searching), 0.0-1.0 means actual progress
                    let is_indeterminate = progress < 0.0;
                    let progress_pct = if is_indeterminate { 100 } else { (progress * 100.0) as i32 };
                    let bar_class = if is_indeterminate { "progress-bar indeterminate" } else { "progress-bar" };
                    view! {
                        <div class=format!("{} lazy-image-downloading", class_str)>
                            <span class="placeholder-char">{char}</span>
                            <div class="download-status-bottom">
                                <div class="download-progress">
                                    <div class=bar_class style:width=format!("{}%", progress_pct)></div>
                                </div>
                                <div class="download-source">{source}</div>
                            </div>
                        </div>
                    }.into_any()
                }
                ImageState::Ready { url, source } => {
                    view! {
                        <div class=format!("{} lazy-image-container", class_str)>
                            <img
                                src=url
                                alt=alt_text
                                class="lazy-image-ready"
                                loading="lazy"
                            />
                            {source.map(|s| view! {
                                <span class="image-source-badge">{s}</span>
                            })}
                        </div>
                    }.into_any()
                }
                ImageState::NoImage => {
                    let char = placeholder.unwrap_or_else(|| "?".to_string());
                    view! {
                        <div class=format!("{} lazy-image-placeholder", class_str)>
                            {char}
                        </div>
                    }.into_any()
                }
                ImageState::Error(_e) => {
                    let char = placeholder.unwrap_or_else(|| "!".to_string());
                    view! {
                        <div class=format!("{} lazy-image-error", class_str)>
                            {char}
                        </div>
                    }.into_any()
                }
            }
        }}
    }
}

/// Helper function to download an image and update state (for LaunchBox-only path)
#[allow(dead_code)]
async fn download_and_update(
    info: ImageInfo,
    set_state: WriteSignal<ImageState>,
    mounted: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    // Trigger download
    match tauri::download_image(info.id).await {
        Ok(local_path) => {
            if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                let source = source_from_path(&local_path);
                let url = file_to_asset_url(&local_path);
                let _ = set_state.try_set(ImageState::Ready { url, source });
            }
        }
        Err(e) => {
            if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                log(&format!("Failed to download image: {}", e));
                // Fall back to CDN URL if download fails
                let _ = set_state.try_set(ImageState::Ready { url: info.cdn_url, source: Some("LB".to_string()) });
            }
        }
    }
}

/// A simpler version that just takes an optional local path and CDN URL
/// For use when you already have the image info
#[component]
pub fn GameImage(
    /// Local file path (if already downloaded)
    local_path: Option<String>,
    /// CDN URL (fallback if not downloaded)
    cdn_url: Option<String>,
    /// Alt text for the image
    #[prop(default = "".to_string())]
    alt: String,
    /// CSS class for the image element
    #[prop(default = "".to_string())]
    class: String,
    /// Placeholder character if no image
    #[prop(optional)]
    placeholder: Option<String>,
) -> impl IntoView {
    // Determine the URL to use
    let url = local_path
        .as_ref()
        .map(|p| file_to_asset_url(p))
        .or(cdn_url);

    match url {
        Some(u) => {
            view! {
                <img
                    src=u
                    alt=alt
                    class=format!("{} game-image", class)
                    loading="lazy"
                />
            }.into_any()
        }
        None => {
            let char = placeholder.unwrap_or_else(|| "?".to_string());
            view! {
                <div class=format!("{} game-image-placeholder", class)>
                    {char}
                </div>
            }.into_any()
        }
    }
}
