//! Lazy-loading image component with on-demand download from multiple sources
//!
//! Supports:
//! - LaunchBox CDN (primary, requires metadata import)
//! - libretro-thumbnails (free, no account needed)
//! - SteamGridDB (requires API key)

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use crate::tauri::{self, file_to_asset_url, log_to_backend, ImageInfo};

/// Maximum concurrent image operations (cache checks + downloads)
/// Increased to 30 to handle slow multi-source fallback (each request can take 2-5 seconds)
const MAX_CONCURRENT_REQUESTS: usize = 30;

/// Maximum pending requests in queue
/// Can be lower now that we properly cancel requests when components unmount
const MAX_PENDING_REQUESTS: usize = 200;

/// Maximum entries in the frontend image URL cache
const IMAGE_CACHE_MAX_SIZE: usize = 500;

/// Per-source statistics
#[derive(Clone, Default, Debug)]
pub struct SourceStats {
    pub completed: u32,
    pub failed: u32,
    pub total_time_ms: u64,
    pub total_bytes: u64,
}

impl SourceStats {
    pub fn avg_time_ms(&self) -> u64 {
        if self.completed > 0 {
            self.total_time_ms / self.completed as u64
        } else {
            0
        }
    }
}

/// Download queue statistics for UI display
#[derive(Clone, Default, Debug)]
pub struct QueueStats {
    pub active: usize,
    pub pending: usize,
    pub total_completed: u32,
    pub total_failed: u32,
    pub total_bytes: u64,
    pub start_time_ms: Option<f64>,
    /// Per-source stats: LB, LR, SG, IG, EM, SS, WS
    pub by_source: HashMap<String, SourceStats>,
}

impl QueueStats {
    /// Calculate download speed in bytes per second
    pub fn bytes_per_sec(&self) -> f64 {
        if let Some(start) = self.start_time_ms {
            let now = js_sys::Date::now();
            let elapsed_sec = (now - start) / 1000.0;
            if elapsed_sec > 0.0 {
                return self.total_bytes as f64 / elapsed_sec;
            }
        }
        0.0
    }
}

/// Global signal for queue stats - can be read by other components
static QUEUE_STATS_SIGNAL: std::sync::OnceLock<(ReadSignal<QueueStats>, WriteSignal<QueueStats>)> = std::sync::OnceLock::new();

/// Get the queue stats signal (creates it on first call)
pub fn queue_stats_signal() -> (ReadSignal<QueueStats>, WriteSignal<QueueStats>) {
    *QUEUE_STATS_SIGNAL.get_or_init(|| signal(QueueStats::default()))
}

/// Update queue stats from the request queue
fn update_queue_stats() {
    REQUEST_QUEUE.with(|q| {
        let queue = q.borrow();
        let (_, set_stats) = queue_stats_signal();
        set_stats.update(|stats| {
            stats.active = queue.active;
            stats.pending = queue.pending_count();
            // Reset timing when queue is empty so speed shows 0
            if stats.active == 0 && stats.pending == 0 {
                stats.start_time_ms = None;
            }
        });
    });
}

/// Record a completed download with source and timing info
pub fn record_download_complete(source: &str, time_ms: u64, bytes: u64) {
    let (_, set_stats) = queue_stats_signal();
    set_stats.update(|stats| {
        stats.total_completed += 1;
        stats.total_bytes += bytes;
        if stats.start_time_ms.is_none() {
            stats.start_time_ms = Some(js_sys::Date::now());
        }
        let source_stats = stats.by_source.entry(source.to_string()).or_default();
        source_stats.completed += 1;
        source_stats.total_time_ms += time_ms;
        source_stats.total_bytes += bytes;
    });
}

/// Record a failed download
pub fn record_download_failed(source: &str, time_ms: u64) {
    let (_, set_stats) = queue_stats_signal();
    set_stats.update(|stats| {
        stats.total_failed += 1;
        if stats.start_time_ms.is_none() {
            stats.start_time_ms = Some(js_sys::Date::now());
        }
        let source_stats = stats.by_source.entry(source.to_string()).or_default();
        source_stats.failed += 1;
        source_stats.total_time_ms += time_ms;
    });
}

/// Request queue for throttling image loads, ordered by render_index (top-left first)
struct RequestQueue {
    active: usize,
    /// Pending requests ordered by render_index (lower = processed first)
    /// Key is render_index, value is (cache_key, task)
    pending: BTreeMap<usize, (String, Box<dyn FnOnce()>)>,
    /// Keys that have been cancelled - skip these when popping
    cancelled: HashSet<String>,
    /// Map from cache_key to render_index for quick lookup
    key_to_index: HashMap<String, usize>,
}

impl RequestQueue {
    fn new() -> Self {
        Self {
            active: 0,
            pending: BTreeMap::new(),
            cancelled: HashSet::new(),
            key_to_index: HashMap::new(),
        }
    }

    /// Cancel a pending request by key
    fn cancel(&mut self, key: &str) {
        self.cancelled.insert(key.to_string());
        // Also remove from pending queue
        if let Some(idx) = self.key_to_index.remove(key) {
            self.pending.remove(&idx);
        }
    }

    /// Get total pending count (for stats)
    fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Pop the next non-cancelled request (lowest render_index first = top-left)
    fn pop_next(&mut self) -> Option<Box<dyn FnOnce()>> {
        while let Some((&idx, _)) = self.pending.first_key_value() {
            if let Some((key, task)) = self.pending.remove(&idx) {
                self.key_to_index.remove(&key);
                if !self.cancelled.remove(&key) {
                    return Some(task);
                }
                // Cancelled, skip and try next
            }
        }

        // Clean up cancelled set periodically
        if self.cancelled.len() > 100 && self.pending.is_empty() {
            self.cancelled.clear();
        }
        None
    }

    /// Add a request to the queue
    fn enqueue(&mut self, key: String, task: Box<dyn FnOnce()>, render_index: usize) {
        // Clear any previous cancellation for this key - important when re-queueing
        // after scrolling away and back
        self.cancelled.remove(&key);
        self.pending.insert(render_index, (key.clone(), task));
        self.key_to_index.insert(key, render_index);
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

/// Release a slot and process next pending request
fn release_slot() {
    REQUEST_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        queue.active = queue.active.saturating_sub(1);
    });
    update_queue_stats();
    process_queue();
}

/// Cancel a pending request by its cache key
fn cancel_pending_request(key: &str) {
    REQUEST_QUEUE.with(|q| {
        q.borrow_mut().cancel(key);
    });
}

// Track if we've already scheduled queue processing
thread_local! {
    static PROCESS_SCHEDULED: RefCell<bool> = RefCell::new(false);
}

/// Queue a request to run when a slot is available.
/// The key is used to allow cancellation of pending requests.
/// in_viewport items get priority over buffer items.
/// render_index determines order within each priority level (lower = first).
fn queue_request<F: FnOnce() + 'static>(key: String, f: F, render_index: usize, in_viewport: bool) {
    // Viewport items get priority 0-999999, buffer items get 1000000+
    // This ensures viewport items always process before buffer items
    let priority = if in_viewport {
        render_index
    } else {
        render_index.saturating_add(1_000_000)
    };

    REQUEST_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();

        // Drop lowest priority requests if queue is too long
        while queue.pending.len() >= MAX_PENDING_REQUESTS {
            // Remove highest priority value (lowest priority = buffer items far from viewport)
            if let Some((&idx, _)) = queue.pending.last_key_value() {
                if let Some((k, _)) = queue.pending.remove(&idx) {
                    queue.key_to_index.remove(&k);
                }
            }
        }

        queue.enqueue(key, Box::new(f), priority);
    });

    update_queue_stats();

    // Schedule deferred processing so all items in this render batch
    // get queued before we start processing (ensures correct ordering)
    schedule_process_queue();
}

/// Schedule queue processing for next microtask
/// This allows all items in a render batch to queue up before processing starts
fn schedule_process_queue() {
    PROCESS_SCHEDULED.with(|scheduled| {
        if *scheduled.borrow() {
            return; // Already scheduled
        }
        *scheduled.borrow_mut() = true;

        // Use setTimeout(0) to defer to after current render batch completes
        // .forget() prevents the Timeout from being cancelled when dropped
        gloo_timers::callback::Timeout::new(0, || {
            PROCESS_SCHEDULED.with(|s| *s.borrow_mut() = false);
            process_queue();
        }).forget();
    });
}

/// Process pending requests if slots are available
fn process_queue() {
    REQUEST_QUEUE.with(|q| {
        loop {
            let mut queue = q.borrow_mut();
            if queue.active >= MAX_CONCURRENT_REQUESTS {
                break;
            }
            if let Some(task) = queue.pop_next() {
                queue.active += 1;
                drop(queue);
                update_queue_stats();
                task();
            } else {
                break;
            }
        }
    });
    update_queue_stats();
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
    /// Render index for ordering (passed from grid) - lower = processed first
    #[prop(default = 0)]
    render_index: usize,
    /// Whether this item is in the actual viewport (not just buffer) - viewport items get priority
    #[prop(default = false)]
    in_viewport: bool,
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
    use futures::future::AbortHandle;
    use std::sync::Mutex;

    let mounted = Arc::new(AtomicBool::new(true));
    let mounted_for_cleanup = mounted.clone();

    // Store abort handle for cancelling in-flight requests (Arc<Mutex> for thread safety)
    let abort_handle: Arc<Mutex<Option<AbortHandle>>> = Arc::new(Mutex::new(None));
    let abort_handle_for_cleanup = abort_handle.clone();
    let abort_handle_for_effect = abort_handle.clone();

    // Store queue key for cancelling pending (not yet started) requests
    let queue_key_store: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let queue_key_for_cleanup = queue_key_store.clone();
    let queue_key_for_effect = queue_key_store.clone();

    // Main effect for loading images
    Effect::new(move || {
        // Track game_key so this effect re-runs when the game changes
        let (db_id, title, plat, img_type) = game_key.get();
        let mounted = mounted.clone();
        let abort_handle = abort_handle_for_effect.clone();
        let queue_key_store = queue_key_for_effect.clone();

        // Cancel any previous pending request for this component
        if let Some(old_key) = queue_key_store.lock().unwrap().take() {
            cancel_pending_request(&old_key);
        }

        // Abort any previous in-flight request for this component
        if let Some(handle) = abort_handle.lock().unwrap().take() {
            handle.abort();
        }

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

        // Show queued state immediately
        let _ = set_state.try_set(ImageState::Downloading {
            progress: -1.0,
            source: "Queued...".to_string(),
        });

        let db_id_opt = if db_id > 0 { Some(db_id) } else { None };
        let queue_key = cache_key.clone();

        // Store queue key for cancellation on unmount or game change
        *queue_key_store.lock().unwrap() = Some(queue_key.clone());

        // Queue the entire operation (cache check + download) to prevent
        // overwhelming the backend with parallel cache checks
        // Lower render_index = processed first (top-left items)
        queue_request(queue_key, move || {
            use futures::future::{Abortable, AbortHandle};
            let (abort_handle_new, abort_registration) = AbortHandle::new_pair();

            // Store the abort handle so it can be cancelled
            *abort_handle.lock().unwrap() = Some(abort_handle_new);

            let cache_and_download = async move {
                if !mounted.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                // Step 1: Check backend cache first
                let _ = set_state.try_set(ImageState::Downloading {
                    progress: -1.0,
                    source: "Checking...".to_string(),
                });

                match tauri::check_cached_media(
                    title.clone(),
                    plat.clone(),
                    img_type.clone(),
                    db_id_opt,
                ).await {
                    Ok(Some(cached)) => {
                        if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                            let url = file_to_asset_url(&cached.path);
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
                    Ok(None) | Err(_) => {
                        // Cache miss or error - continue to download
                    }
                }

                // Step 2: Download
                if !mounted.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                let _ = set_state.try_set(ImageState::Downloading {
                    progress: -1.0,
                    source: "Searching...".to_string(),
                });

                let start_time = js_sys::Date::now();

                match tauri::download_image_with_fallback(
                    title.clone(),
                    plat.clone(),
                    img_type.clone(),
                    db_id_opt,
                ).await {
                    Ok(local_path) => {
                        let elapsed_ms = (js_sys::Date::now() - start_time) as u64;
                        let source = source_from_path(&local_path);
                        let source_str = source.clone().unwrap_or_else(|| "??".to_string());

                        record_download_complete(&source_str, elapsed_ms, 50_000);

                        if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                            let url = file_to_asset_url(&local_path);
                            IMAGE_URL_CACHE.with(|c| {
                                c.borrow_mut().insert(cache_key.clone(), url.clone(), source_str)
                            });
                            let _ = set_state.try_set(ImageState::Ready { url, source });
                        }
                    }
                    Err(_e) => {
                        let elapsed_ms = (js_sys::Date::now() - start_time) as u64;
                        record_download_failed("--", elapsed_ms);

                        if mounted.load(std::sync::atomic::Ordering::Relaxed) {
                            IMAGE_URL_CACHE.with(|c| {
                                c.borrow_mut().insert_not_found(cache_key.clone())
                            });
                            let _ = set_state.try_set(ImageState::NoImage);
                        }
                    }
                }
            };

            // Wrap in Abortable and spawn
            let abortable = Abortable::new(cache_and_download, abort_registration);
            spawn_local(async move {
                let _ = abortable.await;
                release_slot();
            });
        }, render_index, in_viewport);
    });

    // Cleanup on unmount - cancel pending and abort in-flight requests
    on_cleanup(move || {
        mounted_for_cleanup.store(false, std::sync::atomic::Ordering::Relaxed);
        // Cancel any pending (not yet started) request
        if let Some(key) = queue_key_for_cleanup.lock().unwrap().take() {
            cancel_pending_request(&key);
        }
        // Abort any in-flight download
        if let Some(handle) = abort_handle_for_cleanup.lock().unwrap().take() {
            handle.abort();
        }
    });

    let placeholder = placeholder.clone();
    let class = class.clone();
    // Store title/platform for search link in NoImage state
    let search_title = StoredValue::new(game_title.clone());
    let search_platform = StoredValue::new(platform.clone());

    view! {
        <div class="lazy-image-observer-wrapper">
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
                        // Clone values for the click handler
                        let title_for_click = search_title.get_value();
                        let plat_for_click = search_platform.get_value();
                        let img_type_for_click = image_type.clone();
                        let db_id_for_click = launchbox_db_id;
                        let source_for_click = source.clone();
                        let set_state_for_click = set_state;

                        // Handler to redownload from next source when clicking the badge
                        let on_badge_click = move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation(); // Don't trigger card click
                            if let Some(ref current_src) = source_for_click {
                                let title = title_for_click.clone();
                                let plat = plat_for_click.clone();
                                let img_type = img_type_for_click.clone();
                                let db_id = if db_id_for_click > 0 { Some(db_id_for_click) } else { None };
                                let current = current_src.clone();
                                let set_state = set_state_for_click;

                                spawn_local(async move {
                                    // Show loading state
                                    let _ = set_state.try_set(ImageState::Downloading {
                                        progress: -1.0,
                                        source: format!("Trying next after {}...", current),
                                    });

                                    // Clear the frontend LRU cache for this image
                                    let cache_key = ImageUrlCache::make_key(
                                        db_id.unwrap_or(0),
                                        &title,
                                        &plat,
                                        &img_type,
                                    );
                                    IMAGE_URL_CACHE.with(|c| {
                                        c.borrow_mut().entries.remove(&cache_key);
                                    });

                                    // Call the redownload API
                                    match tauri::redownload_image_from_next_source(
                                        title.clone(),
                                        plat.clone(),
                                        img_type.clone(),
                                        db_id,
                                        current.clone(),
                                    ).await {
                                        Ok(local_path) => {
                                            let source = source_from_path(&local_path);
                                            let url = file_to_asset_url(&local_path);
                                            // Update cache with new result
                                            let source_str = source.clone().unwrap_or_else(|| "??".to_string());
                                            IMAGE_URL_CACHE.with(|c| {
                                                c.borrow_mut().insert(cache_key, url.clone(), source_str);
                                            });
                                            let _ = set_state.try_set(ImageState::Ready { url, source });
                                        }
                                        Err(e) => {
                                            log(&format!("Redownload failed: {}", e));
                                            // Stay on current image but show error briefly
                                            let _ = set_state.try_set(ImageState::Downloading {
                                                progress: -1.0,
                                                source: "No more sources".to_string(),
                                            });
                                        }
                                    }
                                });
                            }
                        };

                        view! {
                            <div class=format!("{} lazy-image-container", class_str)>
                                <img
                                    src=url
                                    alt=alt_text
                                    class="lazy-image-ready"
                                    loading="lazy"
                                />
                                {source.map(|s| view! {
                                    <span
                                        class="image-source-badge"
                                        title="Click to try next image source"
                                        on:click=on_badge_click.clone()
                                    >{s}</span>
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
        </div>
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
