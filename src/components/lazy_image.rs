//! Lazy-loading image component with on-demand download from multiple sources
//!
//! Supports:
//! - LaunchBox CDN (primary, requires metadata import)
//! - libretro-thumbnails (free, no account needed)
//! - SteamGridDB (requires API key)

use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::console;
use crate::tauri::{self, file_to_asset_url, ImageInfo};

/// Loading state for an image
#[derive(Debug, Clone, PartialEq)]
pub enum ImageState {
    /// Initial state - checking if image exists
    Loading,
    /// Image is being downloaded
    Downloading { progress: f32 },
    /// Image is ready to display
    Ready { url: String },
    /// No image available for this game
    NoImage,
    /// Error occurred
    Error(String),
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

    // Track if this component is still mounted
    let (mounted, set_mounted) = signal(true);

    // Load image on mount
    Effect::new(move || {
        let db_id = launchbox_db_id;
        let img_type = image_type.clone();
        let title = game_title.clone();
        let plat = platform.clone();

        spawn_local(async move {
            // Check if component is still mounted
            if !mounted.get() {
                return;
            }

            console::log_1(&format!("LazyImage: Loading {} for '{}' (db_id={}, platform={})",
                img_type, title, db_id, plat).into());

            // First, try to get the image info from LaunchBox metadata
            match tauri::get_game_image(db_id, img_type.clone()).await {
                Ok(Some(info)) => {
                    console::log_1(&format!("LazyImage: Found LaunchBox metadata for db_id={}, downloaded={}",
                        db_id, info.downloaded).into());
                    if !mounted.get() {
                        return;
                    }

                    if info.downloaded {
                        // Image already downloaded - use local path
                        if let Some(local_path) = info.local_path {
                            console::log_1(&format!("LazyImage: Using cached image: {}", local_path).into());
                            let url = file_to_asset_url(&local_path);
                            set_state.set(ImageState::Ready { url });
                            return;
                        }
                    }

                    // Need to download from LaunchBox
                    console::log_1(&format!("LazyImage: Downloading from LaunchBox CDN...").into());
                    set_state.set(ImageState::Downloading { progress: 0.0 });
                    if let Ok(local_path) = tauri::download_image(info.id).await {
                        if mounted.get() {
                            console::log_1(&format!("LazyImage: LaunchBox download succeeded: {}", local_path).into());
                            let url = file_to_asset_url(&local_path);
                            set_state.set(ImageState::Ready { url });
                            return;
                        }
                    }

                    // LaunchBox download failed - try multi-source fallback
                    console::log_1(&format!("LazyImage: LaunchBox failed, trying fallback sources...").into());
                    try_fallback_sources(&title, &plat, &img_type, Some(db_id), set_state, mounted).await;
                }
                Ok(None) => {
                    // No LaunchBox metadata - try fallback sources directly
                    console::log_1(&format!("LazyImage: No LaunchBox metadata, trying fallback sources...").into());
                    if mounted.get() {
                        set_state.set(ImageState::Downloading { progress: 0.0 });
                        try_fallback_sources(&title, &plat, &img_type, Some(db_id), set_state, mounted).await;
                    }
                }
                Err(e) => {
                    // Error fetching metadata - try fallback sources directly
                    console::log_1(&format!("LazyImage: Error getting metadata: {}, trying fallback...", e).into());
                    if mounted.get() {
                        set_state.set(ImageState::Downloading { progress: 0.0 });
                        try_fallback_sources(&title, &plat, &img_type, Some(db_id), set_state, mounted).await;
                    }
                }
            }
        });
    });

    // Cleanup on unmount
    on_cleanup(move || {
        set_mounted.set(false);
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
                    view! {
                        <div class=format!("{} lazy-image-loading", class_str)>
                            <div class="loading-spinner"></div>
                        </div>
                    }.into_any()
                }
                ImageState::Downloading { progress } => {
                    let progress_pct = (progress * 100.0) as i32;
                    view! {
                        <div class=format!("{} lazy-image-downloading", class_str)>
                            <div class="download-progress">
                                <div class="progress-bar" style:width=format!("{}%", progress_pct)></div>
                            </div>
                        </div>
                    }.into_any()
                }
                ImageState::Ready { url } => {
                    view! {
                        <img
                            src=url
                            alt=alt_text
                            class=format!("{} lazy-image-ready", class_str)
                            loading="lazy"
                        />
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

/// Try fallback sources (libretro-thumbnails, SteamGridDB)
async fn try_fallback_sources(
    game_title: &str,
    platform: &str,
    image_type: &str,
    launchbox_db_id: Option<i64>,
    set_state: WriteSignal<ImageState>,
    mounted: ReadSignal<bool>,
) {
    if !mounted.get() || game_title.is_empty() {
        console::log_1(&format!("LazyImage: Skipping fallback - unmounted or empty title").into());
        set_state.set(ImageState::NoImage);
        return;
    }

    console::log_1(&format!("LazyImage: Calling download_image_with_fallback('{}', '{}', '{}', {:?})",
        game_title, platform, image_type, launchbox_db_id).into());

    // Try multi-source download
    match tauri::download_image_with_fallback(
        game_title.to_string(),
        platform.to_string(),
        image_type.to_string(),
        launchbox_db_id,
    ).await {
        Ok(local_path) => {
            console::log_1(&format!("LazyImage: Fallback succeeded! Path: {}", local_path).into());
            if mounted.get() {
                let url = file_to_asset_url(&local_path);
                set_state.set(ImageState::Ready { url });
            }
        }
        Err(e) => {
            console::log_1(&format!("LazyImage: All sources failed for '{}': {}", game_title, e).into());
            if mounted.get() {
                set_state.set(ImageState::NoImage);
            }
        }
    }
}

/// Helper function to download an image and update state (for LaunchBox-only path)
#[allow(dead_code)]
async fn download_and_update(
    info: ImageInfo,
    set_state: WriteSignal<ImageState>,
    mounted: ReadSignal<bool>,
) {
    // Trigger download
    match tauri::download_image(info.id).await {
        Ok(local_path) => {
            if mounted.get() {
                let url = file_to_asset_url(&local_path);
                set_state.set(ImageState::Ready { url });
            }
        }
        Err(e) => {
            if mounted.get() {
                console::warn_1(&format!("Failed to download image: {}", e).into());
                // Fall back to CDN URL if download fails
                set_state.set(ImageState::Ready { url: info.cdn_url });
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
