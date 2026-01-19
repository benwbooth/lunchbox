//! Lazy-loading image component with on-demand download from multiple sources
//!
//! Supports:
//! - LaunchBox CDN (primary, requires metadata import)
//! - libretro-thumbnails (free, no account needed)
//! - SteamGridDB (requires API key)

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use crate::tauri::{self, file_to_asset_url, log_to_backend, ImageInfo};

/// Maximum concurrent image operations (cache checks + downloads)
/// Increased to 30 to handle slow multi-source fallback (each request can take 2-5 seconds)
const MAX_CONCURRENT_REQUESTS: usize = 30;

/// Maximum pending requests in queue - older ones get dropped when exceeded
/// Reduced to 30 to prioritize visible items and avoid long waits
const MAX_PENDING_REQUESTS: usize = 30;

/// Maximum entries in the frontend image URL cache
const IMAGE_CACHE_MAX_SIZE: usize = 500;

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

/// Cached result - either a URL or a "not found" marker
#[derive(Clone)]
enum CachedResult {
    Found { url: String, source: String },
    NotFound,
}

/// LRU cache for image URLs - avoids HTTP roundtrip for recently viewed images
/// Also caches negative results to avoid repeated searches
struct ImageUrlCache {
    /// Map from cache key to (result, access_order)
    entries: HashMap<String, (CachedResult, u64)>,
    /// Counter for LRU ordering
    access_counter: u64,
}

impl ImageUrlCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            access_counter: 0,
        }
    }

    /// Create cache key from game identity
    fn make_key(db_id: i64, title: &str, platform: &str, image_type: &str) -> String {
        if db_id > 0 {
            format!("lb-{}:{}", db_id, image_type)
        } else {
            format!("{}:{}:{}", title, platform, image_type)
        }
    }

    /// Get cached result if available, updating access time
    fn get(&mut self, key: &str) -> Option<CachedResult> {
        if let Some((result, _)) = self.entries.get_mut(key) {
            self.access_counter += 1;
            let result = result.clone();
            // Update access time
            self.entries.get_mut(key).unwrap().1 = self.access_counter;
            Some(result)
        } else {
            None
        }
    }

    /// Insert URL into cache, evicting oldest if at capacity
    fn insert(&mut self, key: String, url: String, source: String) {
        self.insert_result(key, CachedResult::Found { url, source });
    }

    /// Insert a "not found" result into cache
    fn insert_not_found(&mut self, key: String) {
        self.insert_result(key, CachedResult::NotFound);
    }

    /// Internal: insert any result into cache
    fn insert_result(&mut self, key: String, result: CachedResult) {
        // Evict oldest entries if at capacity
        while self.entries.len() >= IMAGE_CACHE_MAX_SIZE {
            // Find oldest entry
            let oldest_key = self.entries
                .iter()
                .min_by_key(|(_, (_, access))| access)
                .map(|(k, _)| k.clone());

            if let Some(k) = oldest_key {
                self.entries.remove(&k);
            } else {
                break;
            }
        }

        self.access_counter += 1;
        self.entries.insert(key, (result, self.access_counter));
    }
}

thread_local! {
    static REQUEST_QUEUE: RefCell<RequestQueue> = RefCell::new(RequestQueue::new());
    static IMAGE_URL_CACHE: RefCell<ImageUrlCache> = RefCell::new(ImageUrlCache::new());
}

/// Release a slot and process next pending request (LIFO - newest first)
fn release_slot() {
    REQUEST_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        let old_active = queue.active;
        queue.active = queue.active.saturating_sub(1);
        let pending_count = queue.pending.len();

        // Process newest pending request first (LIFO) - prioritizes currently visible items
        if let Some(task) = queue.pending.pop_back() {
            queue.active += 1;
            log(&format!("[QUEUE] release_slot: active {}->{}, pending {}->{}",
                old_active, queue.active, pending_count, queue.pending.len()));
            // Drop borrow before calling task
            drop(queue);
            task();
        } else {
            log(&format!("[QUEUE] release_slot: active {}->{}, no pending tasks",
                old_active, queue.active));
        }
    });
}

/// Queue a request to run when a slot is available
fn queue_request<F: FnOnce() + 'static>(f: F) {
    REQUEST_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        if queue.active < MAX_CONCURRENT_REQUESTS {
            queue.active += 1;
            log(&format!("[QUEUE] queue_request: running immediately, active={}", queue.active));
            drop(queue);
            f();
        } else {
            // Drop oldest requests if queue is too long (they're likely off-screen now)
            let dropped = if queue.pending.len() >= MAX_PENDING_REQUESTS {
                let mut count = 0;
                while queue.pending.len() >= MAX_PENDING_REQUESTS {
                    queue.pending.pop_front(); // Remove oldest (front) since we use LIFO
                    count += 1;
                }
                count
            } else {
                0
            };
            queue.pending.push_back(Box::new(f));
            if dropped > 0 {
                log(&format!("[QUEUE] queue_request: queued (dropped {} old), active={}, pending={}",
                    dropped, queue.active, queue.pending.len()));
            }
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
    } else if path.contains("/websearch/") {
        Some("WS".to_string())
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

        // Step 0: Check frontend LRU cache first (instant, no HTTP)
        let cache_key = ImageUrlCache::make_key(db_id, &title, &plat, &img_type);
        let frontend_cached = IMAGE_URL_CACHE.with(|c| c.borrow_mut().get(&cache_key));

        if let Some(cached_result) = frontend_cached {
            match cached_result {
                CachedResult::Found { url, source } => {
                    let _ = set_state.try_set(ImageState::Ready {
                        url,
                        source: Some(source),
                    });
                    return;
                }
                CachedResult::NotFound => {
                    // Negative result cached - don't search again
                    let _ = set_state.try_set(ImageState::NoImage);
                    return;
                }
            }
        }

        // Step 1: Backend cache check - NOT queued, runs immediately
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
                        // Store in frontend cache for instant access next time
                        IMAGE_URL_CACHE.with(|c| {
                            c.borrow_mut().insert(cache_key.clone(), url.clone(), cached.source.clone())
                        });
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
            let cache_key2 = cache_key.clone();

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
                                // Store in frontend cache
                                let source_str = source.clone().unwrap_or_default();
                                IMAGE_URL_CACHE.with(|c| {
                                    c.borrow_mut().insert(cache_key2, url.clone(), source_str)
                                });
                                let _ = set_state.try_set(ImageState::Ready { url, source });
                            }
                        }
                        Err(_e) => {
                            if mounted2.load(std::sync::atomic::Ordering::Relaxed) {
                                // Cache the negative result so we don't search again
                                IMAGE_URL_CACHE.with(|c| {
                                    c.borrow_mut().insert_not_found(cache_key2.clone())
                                });
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
    // Store title/platform for search link in NoImage state
    let search_title = StoredValue::new(game_title.clone());
    let search_platform = StoredValue::new(platform.clone());

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
                    // Generate Google Images search URL
                    let title = search_title.get_value();
                    let plat = search_platform.get_value();
                    let search_query = format!("{} {} box art", title, plat);
                    let search_url = format!(
                        "https://www.google.com/search?tbm=isch&q={}",
                        urlencoding::encode(&search_query)
                    );
                    view! {
                        <div class=format!("{} lazy-image-placeholder", class_str)>
                            <span class="placeholder-char">{char}</span>
                            <a
                                href=search_url
                                target="_blank"
                                class="search-online-link"
                                title="Search for image online"
                            >
                                "search"
                            </a>
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
