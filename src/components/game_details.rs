//! Game details panel

use super::{Box3DViewer, LazyImage, VideoPlayer};
use crate::tauri::{
    self, file_to_asset_url, EmulatorWithStatus, Game, GameFile, GameVariant, PlayStats,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;
use wasm_bindgen::{JsCast, JsValue};

async fn launch_game_with_resolved_rom(
    launchbox_db_id: i64,
    fallback_rom_path: Option<String>,
    emulator_name: String,
    is_retroarch_core: bool,
) -> Result<tauri::LaunchResult, String> {
    let rom_path = match tauri::get_game_file(launchbox_db_id).await {
        Ok(Some(file)) if !file.file_path.trim().is_empty() => file.file_path,
        _ => fallback_rom_path
            .and_then(|path| {
                if path.trim().is_empty() {
                    None
                } else {
                    Some(path)
                }
            })
            .ok_or_else(|| "No ROM file path is available for this game".to_string())?,
    };

    tauri::launch_game(emulator_name, rom_path, is_retroarch_core).await
}

async fn resolve_game_file_for_display(game: &Game) -> Option<GameFile> {
    if game.database_id > 0 {
        if let Ok(Some(file)) = tauri::get_game_file(game.database_id).await {
            return Some(file);
        }
    }

    let mut checked_ids: HashSet<i64> = HashSet::new();

    let variants = tauri::get_game_variants(
        game.id.clone(),
        game.display_title.clone(),
        game.platform_id,
    )
    .await
    .ok()?;

    for variant in variants {
        let variant_game = match tauri::get_game_by_uuid(variant.id).await {
            Ok(Some(g)) => g,
            _ => continue,
        };
        if variant_game.database_id <= 0 || !checked_ids.insert(variant_game.database_id) {
            continue;
        }
        if let Ok(Some(file)) = tauri::get_game_file(variant_game.database_id).await {
            return Some(file);
        }
    }

    None
}

fn pause_game_details_video() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(Some(video_el)) = document.query_selector(".game-video") else {
        return;
    };

    let pause_value = match js_sys::Reflect::get(video_el.as_ref(), &JsValue::from_str("pause")) {
        Ok(value) => value,
        Err(_) => return,
    };
    let Some(pause_fn) = pause_value.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let _ = pause_fn.call0(video_el.as_ref());
}

#[component]
pub fn GameDetails(
    game: ReadSignal<Option<Game>>,
    on_close: WriteSignal<Option<Game>>,
    #[prop(optional)] set_show_settings: Option<WriteSignal<bool>>,
) -> impl IntoView {
    // Local display state - allows switching variants without affecting external state
    let (display_game, set_display_game) = signal::<Option<Game>>(None);
    let (play_stats, set_play_stats) = signal::<Option<PlayStats>>(None);
    let (is_fav, set_is_fav) = signal(false);
    let (variants, set_variants) = signal::<Vec<GameVariant>>(Vec::new());
    let (selected_variant, set_selected_variant) = signal::<Option<String>>(None);
    // Track pending variant load separately from selected (to avoid infinite loops)
    let (pending_variant_load, set_pending_variant_load) = signal::<Option<String>>(None);
    // Emulator picker state
    let (show_emulator_picker, set_show_emulator_picker) = signal(false);
    let (emulators, set_emulators) = signal::<Vec<EmulatorWithStatus>>(Vec::new());
    let (emulators_loading, set_emulators_loading) = signal(false);
    // Per-game emulator preference
    let (game_emulator_pref, set_game_emulator_pref) = signal::<Option<String>>(None);
    // Import state
    let (game_file, set_game_file) = signal::<Option<GameFile>>(None);
    let (import_state_loading, set_import_state_loading) = signal(false);
    let (import_job_id, set_import_job_id) = signal::<Option<String>>(None);
    let (import_error, set_import_error) = signal::<Option<String>>(None);
    // Minerva download state
    let (minerva_rom, set_minerva_rom) = signal::<Option<tauri::MinervaRom>>(None);
    let (minerva_downloading, set_minerva_downloading) = signal(false);
    let (minerva_progress, set_minerva_progress) = signal::<Option<tauri::MinervaDownloadProgress>>(None);
    let (minerva_job_id, set_minerva_job_id) = signal::<Option<String>>(None);
    // Torrent file picker state
    let (show_file_picker, set_show_file_picker) = signal(false);
    let (torrent_files, set_torrent_files) = signal::<Vec<tauri::TorrentFileMatch>>(Vec::new());
    let (selected_file_index, set_selected_file_index) = signal::<Option<usize>>(None);
    let (files_loading, set_files_loading) = signal(false);

    // Initialize display_game from prop when game changes
    Effect::new(move || {
        if let Some(g) = game.get() {
            set_display_game.set(Some(g));
            set_game_file.set(None);
            set_import_job_id.set(None);
            set_import_state_loading.set(true);
            set_minerva_rom.set(None);
            set_minerva_downloading.set(false);
            set_minerva_progress.set(None);
            set_minerva_job_id.set(None);
        } else {
            set_display_game.set(None);
            set_play_stats.set(None);
            set_is_fav.set(false);
            set_variants.set(Vec::new());
            set_selected_variant.set(None);
            set_pending_variant_load.set(None);
            set_game_emulator_pref.set(None);
            set_game_file.set(None);
            set_import_job_id.set(None);
            set_import_state_loading.set(false);
        }
    });

    // Load per-game emulator preference
    Effect::new(move || {
        if let Some(g) = display_game.get() {
            let db_id = g.database_id;
            spawn_local(async move {
                // Get game-specific preference (not platform default)
                if let Ok(prefs) = tauri::get_all_emulator_preferences().await {
                    let game_pref = prefs
                        .game_preferences
                        .into_iter()
                        .find(|p| p.launchbox_db_id == db_id)
                        .map(|p| p.emulator_name);
                    set_game_emulator_pref.set(game_pref);
                }
            });
        }
    });

    // Load play stats, favorite status, and variants when display_game changes
    Effect::new(move || {
        if let Some(g) = display_game.get() {
            set_import_state_loading.set(true);
            let game_snapshot = g.clone();
            let game_id = g.id.clone();
            let db_id = g.database_id;
            let display_title = g.display_title.clone();
            let platform_id = g.platform_id;
            let variant_count = g.variant_count;
            let expected_game_id = game_id.clone();

            // Check if we're switching variants of the same game (variants already loaded)
            // by checking if current game is in the existing variants list
            // Important: do not track `variants` reactively inside this effect.
            // This effect should follow `display_game` only; tracking variants here
            // can create a feedback loop (`set_variants` -> rerun effect -> more async loads),
            // which manifests as panel jitter/shaking.
            let current_variants = variants.get_untracked();
            let is_variant_switch = current_variants.iter().any(|v| v.id == game_id);

            spawn_local(async move {
                let is_current_game = || {
                    display_game
                        .get_untracked()
                        .as_ref()
                        .map(|current| current.id.as_str() == expected_game_id.as_str())
                        .unwrap_or(false)
                };

                // Load play stats
                if let Ok(stats) = tauri::get_play_stats(db_id).await {
                    if !is_current_game() {
                        return;
                    }
                    set_play_stats.set(stats);
                }
                // Check favorite status
                if let Ok(fav) = tauri::is_favorite(db_id).await {
                    if !is_current_game() {
                        return;
                    }
                    set_is_fav.set(fav);
                }

                // Only load variants if this is a new game, not a variant switch
                web_sys::console::log_1(
                    &format!(
                        "Loading variants: is_variant_switch={}, variant_count={}",
                        is_variant_switch, variant_count
                    )
                    .into(),
                );
                if !is_variant_switch && variant_count > 1 {
                    web_sys::console::log_1(
                        &format!("Fetching variants for game_id={}", game_id).into(),
                    );
                    match tauri::get_game_variants(
                        game_id.clone(),
                        display_title.clone(),
                        platform_id,
                    )
                    .await
                    {
                        Ok(vars) => {
                            if !is_current_game() {
                                return;
                            }
                            web_sys::console::log_1(&format!("Got {} variants", vars.len()).into());
                            // Prefer first variant only when it has a valid LaunchBox DB id.
                            // Some region-specific rows carry no launchbox_db_id (0) and break
                            // import/file lookup if auto-selected.
                            let preferred_variant_id = vars.first().map(|v| v.id.clone());
                            set_variants.set(vars);

                            if let Some(preferred_id) = preferred_variant_id {
                                if preferred_id != game_id {
                                    match tauri::get_game_by_uuid(preferred_id.clone()).await {
                                        Ok(Some(preferred_game))
                                            if preferred_game.database_id > 0 =>
                                        {
                                            if !is_current_game() {
                                                return;
                                            }
                                            set_selected_variant.set(Some(preferred_id.clone()));
                                            set_pending_variant_load.set(Some(preferred_id));
                                        }
                                        _ => {
                                            if !is_current_game() {
                                                return;
                                            }
                                            set_selected_variant.set(Some(game_id.clone()));
                                        }
                                    }
                                } else {
                                    if !is_current_game() {
                                        return;
                                    }
                                    set_selected_variant.set(Some(game_id.clone()));
                                }
                            } else {
                                if !is_current_game() {
                                    return;
                                }
                                set_selected_variant.set(Some(game_id.clone()));
                            }
                        }
                        Err(_) => {
                            if !is_current_game() {
                                return;
                            }
                            set_variants.set(Vec::new());
                        }
                    }
                } else if !is_variant_switch {
                    if !is_current_game() {
                        return;
                    }
                    set_variants.set(Vec::new());
                }

                // Check game file (with variant-aware fallback) and active import
                let resolved_file = resolve_game_file_for_display(&game_snapshot).await;
                if !is_current_game() {
                    return;
                }
                set_game_file.set(resolved_file);
                match tauri::get_active_import(db_id).await {
                    Ok(Some(job)) => {
                        if !is_current_game() {
                            return;
                        }
                        set_import_job_id.set(Some(job.id));
                    }
                    _ => {
                        if !is_current_game() {
                            return;
                        }
                        set_import_job_id.set(None);
                    }
                }
                // Check minerva ROM availability
                {
                    if let Ok(rom) = tauri::get_minerva_rom_for_game(db_id, Some(platform_id)).await {
                        if is_current_game() {
                            set_minerva_rom.set(rom);
                        }
                    }
                }

                if is_current_game() {
                    set_import_state_loading.set(false);
                }
            });
        }
    });

    // Minerva download progress polling
    Effect::new(move || {
        if let Some(jid) = minerva_job_id.get() {
            let jid_clone = jid.clone();
            spawn_local(async move {
                loop {
                    match tauri::get_minerva_download_progress(jid_clone.clone()).await {
                        Ok(Some(progress)) => {
                            let done = progress.status == "completed" || progress.status == "failed" || progress.status == "cancelled";
                            set_minerva_progress.set(Some(progress.clone()));
                            if done {
                                set_minerva_downloading.set(false);
                                set_minerva_job_id.set(None);
                                if progress.status == "completed" {
                                    // Refresh game file
                                    if let Some(g) = game.get_untracked() {
                                        if let Ok(Some(file)) = tauri::get_game_file(g.database_id).await {
                                            set_game_file.set(Some(file));
                                        }
                                    }
                                }
                                break;
                            }
                        }
                        Ok(None) => {
                            set_minerva_downloading.set(false);
                            set_minerva_job_id.set(None);
                            break;
                        }
                        Err(_) => break,
                    }
                    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
                        web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1000).unwrap();
                    })).await.unwrap();
                }
            });
        }
    });

    // Load variant game when pending_variant_load changes
    // Use untrack to avoid re-triggering when we clear the signal
    Effect::new(move || {
        let variant_id = pending_variant_load.get();
        if let Some(variant_id) = variant_id {
            let still_visible = variants.get_untracked().iter().any(|v| v.id == variant_id);
            if !still_visible {
                set_pending_variant_load.set(None);
                return;
            }
            // Update selected_variant to show visual selection
            set_selected_variant.set(Some(variant_id.clone()));
            spawn_local(async move {
                let requested_variant_id = variant_id.clone();
                if let Ok(Some(new_game)) =
                    tauri::get_game_by_uuid(requested_variant_id.clone()).await
                {
                    if pending_variant_load.get_untracked().as_deref()
                        == Some(requested_variant_id.as_str())
                    {
                        set_display_game.set(Some(new_game));
                    }
                }
                // Clear after loading completes to prevent re-triggering during load.
                if pending_variant_load.get_untracked().as_deref()
                    == Some(requested_variant_id.as_str())
                {
                    set_pending_variant_load.set(None);
                }
            });
        }
    });


    view! {
        <Show when=move || display_game.get().is_some()>
            {move || {
                display_game.get().map(|g| {
                    let display_title = g.display_title.clone();
                    let first_char = display_title.chars().next().unwrap_or('?').to_string();
                    let platform = g.platform.clone();
                    let description = g.description.clone().unwrap_or_else(|| "No description available.".to_string());
                    let developer = g.developer.clone();
                    let publisher = g.publisher.clone();
                    let genres = g.genres.clone();
                    let year = g.release_year;
                    let release_date = g.release_date.clone();
                    let rating = g.rating;
                    let rating_count = g.rating_count;
                    let players = g.players.clone();
                    let esrb = g.esrb.clone();
                    let cooperative = g.cooperative;
                    let video_url = g.video_url.clone();
                    let wikipedia_url = g.wikipedia_url.clone();
                    let db_id = g.database_id;

                    let title_for_fav = g.title.clone();
                    let platform_for_fav = g.platform.clone();
                    let title_for_select = g.title.clone();
                    let platform_for_select = g.platform.clone();

                    // Store game info for emulator selection
                    let stored_title = StoredValue::new(title_for_select);
                    let stored_platform = StoredValue::new(platform_for_select);
                    let stored_db_id = StoredValue::new(db_id);


                    let on_toggle_favorite = move |_| {
                        let title = title_for_fav.clone();
                        let platform = platform_for_fav.clone();
                        let currently_fav = is_fav.get();
                        spawn_local(async move {
                            if currently_fav {
                                if tauri::remove_favorite(db_id).await.is_ok() {
                                    set_is_fav.set(false);
                                }
                            } else {
                                if tauri::add_favorite(db_id, title, platform).await.is_ok() {
                                    set_is_fav.set(true);
                                }
                            }
                        });
                    };

                    view! {
                        <div class="game-details-overlay" on:click=move |_| on_close.set(None)>
                            <div class="game-details-panel" on:click=|e| e.stop_propagation()>
                                // Title bar with game name and close button
                                <div class="game-details-titlebar">
                                    <h1 class="titlebar-title">{display_title.clone()}</h1>
                                    <button class="titlebar-close" on:click=move |_| on_close.set(None)>"×"</button>
                                </div>

                                // Info area on its own row
                                <div class="game-details-info">
                                    <p class="game-details-platform">{platform}</p>

                                    <div class="game-details-meta">
                                            {developer.map(|d| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Developer"</span>
                                                    <span class="meta-value">{d}</span>
                                                </div>
                                            })}
                                            {publisher.map(|p| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Publisher"</span>
                                                    <span class="meta-value">{p}</span>
                                                </div>
                                            })}
                                            {year.map(|y| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Year"</span>
                                                    <span class="meta-value">{y}</span>
                                                </div>
                                            })}
                                            {release_date.map(|d| {
                                                let formatted = format_date(&d);
                                                view! {
                                                    <div class="meta-item">
                                                        <span class="meta-label">"Release Date"</span>
                                                        <span class="meta-value">{formatted}</span>
                                                    </div>
                                                }
                                            })}
                                            {genres.map(|g| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Genre"</span>
                                                    <span class="meta-value">{g}</span>
                                                </div>
                                            })}
                                            {players.map(|p| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Players"</span>
                                                    <span class="meta-value">{p}</span>
                                                </div>
                                            })}
                                            {esrb.map(|e| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"ESRB"</span>
                                                    <span class="meta-value">{e}</span>
                                                </div>
                                            })}
                                            {cooperative.map(|c| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Co-op"</span>
                                                    <span class="meta-value">{if c { "Yes" } else { "No" }}</span>
                                                </div>
                                            })}
                                            {rating.map(|r| {
                                                let rating_str = format!("{:.1}", r);
                                                let count_str = rating_count.map(|c| format!(" ({} votes)", c)).unwrap_or_default();
                                                view! {
                                                    <div class="meta-item">
                                                        <span class="meta-label">"Rating"</span>
                                                        <span class="meta-value">{rating_str}{count_str}</span>
                                                    </div>
                                                }
                                            })}
                                        </div>
                                        // External links
                                        {(video_url.is_some() || wikipedia_url.is_some()).then(|| {
                                            let video = video_url.clone();
                                            let wiki = wikipedia_url.clone();
                                            view! {
                                                <div class="game-links">
                                                    {video.map(|url| view! {
                                                        <a href=url target="_blank" class="game-link">"Video"</a>
                                                    })}
                                                    {wiki.map(|url| view! {
                                                        <a href=url target="_blank" class="game-link">"Wikipedia"</a>
                                                    })}
                                                </div>
                                            }
                                        })}

                                        // Play statistics
                                        <Show when=move || play_stats.get().is_some()>
                                            {move || play_stats.get().map(|stats| {
                                                let play_count = stats.play_count;
                                                let last_played = stats.last_played
                                                    .map(|s| format_date(&s))
                                                    .unwrap_or_else(|| "Never".to_string());
                                                view! {
                                                    <div class="play-stats">
                                                        <span class="play-stat">
                                                            <span class="stat-value">{play_count}</span>
                                                            " plays"
                                                        </span>
                                                        <span class="play-stat">
                                                            "Last: "
                                                            <span class="stat-value">{last_played}</span>
                                                        </span>
                                                    </div>
                                                }
                                            })}
                                        </Show>

                                        // Per-game emulator preference
                                        <Show when=move || game_emulator_pref.get().is_some()>
                                            {move || game_emulator_pref.get().map(|emu_name| {
                                                view! {
                                                    <div class="game-emulator-pref">
                                                        <span class="pref-label">"Preferred emulator: "</span>
                                                        <span class="pref-value">{emu_name}</span>
                                                        <button
                                                            class="pref-reset-btn"
                                                            on:click=move |_| {
                                                                spawn_local(async move {
                                                                    if tauri::clear_game_emulator_preference(db_id).await.is_ok() {
                                                                        set_game_emulator_pref.set(None);
                                                                    }
                                                                });
                                                            }
                                                            title="Reset to ask every time"
                                                        >
                                                            "Reset"
                                                        </button>
                                                    </div>
                                                }
                                            })}
                                        </Show>

                                        <div class="game-actions">

                                            // Minerva torrent download progress
                                            <Show when=move || minerva_downloading.get()>
                                                <div class="import-section">
                                                    <div class="minerva-progress">
                                                        <div class="minerva-progress-bar">
                                                            <div class="minerva-progress-fill" style=move || {
                                                                let pct = minerva_progress.get().map(|p| p.progress_percent).unwrap_or(0.0);
                                                                format!("width: {:.1}%", pct)
                                                            }></div>
                                                        </div>
                                                        <div class="minerva-progress-text">
                                                            {move || {
                                                                if let Some(p) = minerva_progress.get() {
                                                                    let speed = if p.download_speed > 0 {
                                                                        format!(" ({:.1} MB/s)", p.download_speed as f64 / 1_000_000.0)
                                                                    } else {
                                                                        String::new()
                                                                    };
                                                                    format!("{}{speed}", p.status_message)
                                                                } else {
                                                                    "Starting download...".to_string()
                                                                }
                                                            }}
                                                        </div>
                                                    </div>
                                                    <button class="cancel-import-btn" on:click=move |_| {
                                                        if let Some(jid) = minerva_job_id.get() {
                                                            set_minerva_downloading.set(false);
                                                            set_minerva_job_id.set(None);
                                                            spawn_local(async move {
                                                                let _ = tauri::cancel_minerva_download(jid).await;
                                                            });
                                                        } else {
                                                            set_minerva_downloading.set(false);
                                                        }
                                                    }>"Cancel"</button>
                                                </div>
                                            </Show>

                                            // Import prompt and buttons when no file and not importing
                                            <Show when=move || import_state_loading.get() && import_job_id.get().is_none()>
                                                <div class="import-status-hint">"Checking imported file…"</div>
                                            </Show>

                                            <Show when=move || !import_state_loading.get() && game_file.get().is_none() && import_job_id.get().is_none() && !minerva_downloading.get()>
                                                // Download button (Minerva torrent) — opens file picker
                                                <Show when=move || !show_file_picker.get()>
                                                    <button
                                                        class="import-btn-action minerva-download-btn"
                                                        disabled=move || files_loading.get() || minerva_rom.get().is_none()
                                                        title=move || if minerva_rom.get().is_none() { "No minerva.db — run lunchbox-cli minerva-build first".to_string() } else { "Download ROM via torrent".to_string() }
                                                        on:click=move |_| {
                                                            if let Some(rom) = minerva_rom.get() {
                                                                if let Some(g) = game.get_untracked() {
                                                                    let url = rom.torrent_url.clone();
                                                                    let title = g.display_title.clone();
                                                                    set_files_loading.set(true);
                                                                    set_torrent_files.set(Vec::new());
                                                                    set_selected_file_index.set(None);
                                                                    spawn_local(async move {
                                                                        match tauri::list_torrent_files(url, title).await {
                                                                            Ok(files) => {
                                                                                if let Some(best) = files.first() {
                                                                                    set_selected_file_index.set(Some(best.index));
                                                                                }
                                                                                set_torrent_files.set(files);
                                                                                set_show_file_picker.set(true);
                                                                            }
                                                                            Err(e) => set_import_error.set(Some(e)),
                                                                        }
                                                                        set_files_loading.set(false);
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    >
                                                        {move || if files_loading.get() { "Loading..." } else { "Download" }}
                                                    </button>
                                                    <button
                                                        class="import-btn-action"
                                                        title="Import a local ROM file by path"
                                                        on:click=move |_| {
                                                            if let Some(g) = game.get_untracked() {
                                                                let db_id = g.database_id;
                                                                let title = g.display_title.clone();
                                                                let platform = stored_platform.get_value();
                                                                // Prompt for file path (in Tauri mode, would use native dialog)
                                                                let window = web_sys::window().unwrap();
                                                                if let Some(path) = window.prompt_with_message("Enter path to ROM file:").ok().flatten() {
                                                                    if !path.trim().is_empty() {
                                                                        let path = path.trim().to_string();
                                                                        spawn_local(async move {
                                                                            let entries = vec![tauri::RomImportEntry {
                                                                                file_path: path,
                                                                                launchbox_db_id: db_id,
                                                                                game_title: title,
                                                                                platform,
                                                                                copy_to_library: false,
                                                                            }];
                                                                            match tauri::confirm_rom_import(entries).await {
                                                                                Ok(count) => {
                                                                                    if count > 0 {
                                                                                        // Refresh game file
                                                                                        if let Ok(Some(file)) = tauri::get_game_file(db_id).await {
                                                                                            set_game_file.set(Some(file));
                                                                                        }
                                                                                    }
                                                                                }
                                                                                Err(e) => set_import_error.set(Some(e)),
                                                                            }
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    >"Import"</button>
                                                </Show>

                                                // File picker dialog
                                                <Show when=move || show_file_picker.get()>
                                                    <div class="file-picker-dialog">
                                                        <div class="file-picker-header">
                                                            <h4>"Select ROM to download"</h4>
                                                            <button class="file-picker-close" on:click=move |_| set_show_file_picker.set(false)>"X"</button>
                                                        </div>
                                                        <div class="file-picker-list">
                                                            {move || torrent_files.get().into_iter().map(|file| {
                                                                let idx = file.index;
                                                                let name = file.filename.clone();
                                                                let size_mb = file.size as f64 / (1024.0 * 1024.0);
                                                                let score = file.match_score;
                                                                let region = file.region.clone().unwrap_or_default();
                                                                let is_selected = move || selected_file_index.get() == Some(idx);
                                                                view! {
                                                                    <div
                                                                        class="file-picker-row"
                                                                        class:selected=is_selected
                                                                        on:click=move |_| set_selected_file_index.set(Some(idx))
                                                                    >
                                                                        <div class="file-picker-name">{name}</div>
                                                                        <div class="file-picker-meta">
                                                                            <span class="file-picker-size">{format!("{size_mb:.1} MB")}</span>
                                                                            {(!region.is_empty()).then(|| view! {
                                                                                <span class="file-picker-region">{region}</span>
                                                                            })}
                                                                            {(score > 0.5).then(|| view! {
                                                                                <span class="file-picker-match">{format!("{:.0}% match", score * 100.0)}</span>
                                                                            })}
                                                                        </div>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                        <div class="file-picker-actions">
                                                            <button
                                                                class="import-btn-action"
                                                                disabled=move || selected_file_index.get().is_none()
                                                                on:click=move |_| {
                                                                    if let (Some(rom), Some(file_idx)) = (minerva_rom.get(), selected_file_index.get()) {
                                                                        if let Some(g) = game.get_untracked() {
                                                                            let torrent_url = rom.torrent_url.clone();
                                                                            let db_id = g.database_id;
                                                                            let title = g.display_title.clone();
                                                                            let platform = stored_platform.get_value();
                                                                            set_show_file_picker.set(false);
                                                                            set_minerva_downloading.set(true);
                                                                            set_minerva_progress.set(None);
                                                                            spawn_local(async move {
                                                                                // Check if torrent client is configured
                                                                                // Check if torrent client is configured
                                                                                match tauri::get_settings().await {
                                                                                    Ok(settings) => {
                                                                                        let client = &settings.torrent.client;
                                                                                        if client == "auto" || client.is_empty() {
                                                                                            set_minerva_downloading.set(false);
                                                                                            if let Some(setter) = set_show_settings {
                                                                                                setter.set(true);
                                                                                            }
                                                                                            set_import_error.set(Some("Please select a torrent client in Settings > Downloads / Torrent first.".to_string()));
                                                                                            return;
                                                                                        }
                                                                                    }
                                                                                    Err(e) => {
                                                                                        set_minerva_downloading.set(false);
                                                                                        set_import_error.set(Some(format!("Failed to check settings: {e}")));
                                                                                        return;
                                                                                    }
                                                                                }
                                                                                // Test the configured client
                                                                                match tauri::test_torrent_connection().await {
                                                                                    Ok(result) if !result.success => {
                                                                                        set_minerva_downloading.set(false);
                                                                                        if let Some(setter) = set_show_settings {
                                                                                            setter.set(true);
                                                                                        }
                                                                                        set_import_error.set(Some(format!("Torrent client error: {}", result.message)));
                                                                                        return;
                                                                                    }
                                                                                    Err(e) => {
                                                                                        set_minerva_downloading.set(false);
                                                                                        if let Some(setter) = set_show_settings {
                                                                                            setter.set(true);
                                                                                        }
                                                                                        set_import_error.set(Some(format!("Torrent client error: {e}")));
                                                                                        return;
                                                                                    }
                                                                                    _ => {}
                                                                                }
                                                                                match tauri::start_minerva_download(torrent_url, file_idx, db_id, title, platform).await {
                                                                                    Ok(job) => set_minerva_job_id.set(Some(job.id)),
                                                                                    Err(e) => {
                                                                                        set_minerva_downloading.set(false);
                                                                                        set_import_error.set(Some(e));
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                            >"Download Selected"</button>
                                                            <button
                                                                class="cancel-import-btn"
                                                                on:click=move |_| set_show_file_picker.set(false)
                                                            >"Cancel"</button>
                                                        </div>
                                                    </div>
                                                </Show>
                                            </Show>

                                            // Import error message
                                            <Show when=move || import_error.get().is_some()>
                                                {move || import_error.get().map(|err| view! {
                                                    <div class="import-error">
                                                        <span>{err}</span>
                                                        <button class="import-error-dismiss" on:click=move |_| set_import_error.set(None)>"Dismiss"</button>
                                                    </div>
                                                })}
                                            </Show>

                                            // Play button only when game file exists and not importing
                                            <Show when=move || !import_state_loading.get() && game_file.get().is_some() && import_job_id.get().is_none()>
                                                <button class="play-btn" on:click=move |_| {
                                                    let platform = stored_platform.get_value();
                                                    set_emulators_loading.set(true);
                                                    set_show_emulator_picker.set(true);
                                                    spawn_local(async move {
                                                        match tauri::get_emulators_with_status(platform).await {
                                                            Ok(emu_list) => set_emulators.set(emu_list),
                                                            Err(e) => {
                                                                web_sys::console::error_1(&format!("Failed to fetch emulators: {}", e).into());
                                                                set_emulators.set(Vec::new());
                                                            }
                                                        }
                                                        set_emulators_loading.set(false);
                                                    });
                                                }>"Play"</button>
                                                <span class="game-details-ready-badge" title="ROM downloaded - ready to play">
                                                    "Ready to Play"
                                                </span>
                                            </Show>

                                            <button
                                                class="favorite-btn"
                                                class:is-favorite=move || is_fav.get()
                                                on:click=on_toggle_favorite
                                            >
                                                {move || if is_fav.get() { "Unfavorite" } else { "Favorite" }}
                                            </button>
                                        </div>

                                        // Emulator picker modal
                                        <Show when=move || show_emulator_picker.get()>
                                            <EmulatorPickerModal
                                                emulators=emulators
                                                emulators_loading=emulators_loading
                                                game_file=game_file
                                                stored_title=stored_title
                                                stored_platform=stored_platform
                                                stored_db_id=stored_db_id
                                                set_show_emulator_picker=set_show_emulator_picker
                                            />
                                        </Show>

                                    </div>

                                // Video player, full width
                                <VideoPlayer
                                    game_title=g.title.clone()
                                    platform=g.platform.clone()
                                    launchbox_db_id=db_id
                                />

                                // Media carousel with arrows, full width
                                <MediaCarousel
                                    launchbox_db_id=db_id
                                    game_title=g.title.clone()
                                    platform=g.platform.clone()
                                    placeholder=first_char.clone()
                                />

                                <div class="game-details-description">
                                    <h2>"Description"</h2>
                                    <p>{description}</p>
                                </div>

                                // Variants section
                                <VariantsSection
                                    variants=variants
                                    selected_variant=selected_variant
                                    set_selected_variant=set_pending_variant_load
                                />
                            </div>
                        </div>
                    }
                })
            }}
        </Show>
    }
}

#[component]
fn VariantsSection(
    variants: ReadSignal<Vec<GameVariant>>,
    selected_variant: ReadSignal<Option<String>>,
    set_selected_variant: WriteSignal<Option<String>>,
) -> impl IntoView {
    // Use the actual variants list length, not the game's variant_count
    // This prevents flashing when switching between variants
    view! {
        <Show when=move || { variants.get().len() > 1 }>
            <div class="game-variants-section">
                <h2>"Versions"</h2>
                <p class="variants-hint">"Select a version to play:"</p>
                <div class="variants-list">
                    <For
                        each=move || variants.get()
                        key=|v| v.id.clone()
                        let:variant
                    >
                        <VariantItem
                            variant=variant
                            selected_variant=selected_variant
                            set_selected_variant=set_selected_variant
                        />
                    </For>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn VariantItem(
    variant: GameVariant,
    selected_variant: ReadSignal<Option<String>>,
    set_selected_variant: WriteSignal<Option<String>>,
) -> impl IntoView {
    let variant_id = variant.id.clone();
    let variant_title = variant.title.clone();
    let variant_region = variant.region.clone();
    let variant_id_for_click = variant_id.clone();

    view! {
        <button
            class="variant-item"
            class:selected=move || selected_variant.get().as_ref() == Some(&variant_id)
            on:click=move |_| set_selected_variant.set(Some(variant_id_for_click.clone()))
        >
            <span class="variant-title">{variant_title}</span>
            {variant_region.map(|r| view! {
                <span class="variant-region">{r}</span>
            })}
        </button>
    }
}

/// Media types available in the carousel
const MEDIA_TYPES: &[&str] = &[
    "Box - Front",
    "Box - 3D",
    "Box - Back",
    "Screenshot - Gameplay",
    "Screenshot - Game Title",
    "Clear Logo",
    "Fanart - Background",
];

/// Media carousel with left/right navigation including 3D box view
#[component]
fn MediaCarousel(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    placeholder: String,
) -> impl IntoView {
    let (current_index, set_current_index) = signal(0usize);
    let (available_types, _set_available_types) =
        signal::<Vec<String>>(MEDIA_TYPES.iter().map(|&s| s.to_string()).collect());
    let (box_front_url, set_box_front_url) = signal::<Option<String>>(None);
    let (box_back_url, set_box_back_url) = signal::<Option<String>>(None);

    // Store props for async use
    let title = StoredValue::new(game_title.clone());
    let plat = StoredValue::new(platform.clone());
    let db_id = launchbox_db_id;

    // Pre-load box URLs for 3D viewer in background
    Effect::new(move || {
        let title = title.get_value();
        let plat = plat.get_value();

        spawn_local(async move {
            // Pre-load box front URL for 3D viewer
            if let Ok(path) = tauri::download_image_with_fallback(
                title.clone(),
                plat.clone(),
                "Box - Front".to_string(),
                Some(db_id),
            )
            .await
            {
                set_box_front_url.set(Some(file_to_asset_url(&path)));
            }

            // Pre-load box back URL for 3D viewer
            if let Ok(path) = tauri::download_image_with_fallback(
                title.clone(),
                plat.clone(),
                "Box - Back".to_string(),
                Some(db_id),
            )
            .await
            {
                set_box_back_url.set(Some(file_to_asset_url(&path)));
            }
        });
    });

    let prev = move |_| {
        let types = available_types.get();
        let current = current_index.get();
        if current > 0 {
            set_current_index.set(current - 1);
        } else {
            set_current_index.set(types.len().saturating_sub(1));
        }
    };

    let next = move |_| {
        let types = available_types.get();
        let current = current_index.get();
        if current < types.len() - 1 {
            set_current_index.set(current + 1);
        } else {
            set_current_index.set(0);
        }
    };

    let game_title_for_render = game_title.clone();
    let platform_for_render = platform.clone();
    let placeholder_for_render = placeholder.clone();

    view! {
        <div class="media-carousel">
            <div class="carousel-content">
                {move || {
                    let types = available_types.get();
                    let idx = current_index.get().min(types.len().saturating_sub(1));
                    let current_type = types.get(idx).cloned().unwrap_or_else(|| "Box - Front".to_string());

                    if current_type == "Box - 3D" {
                        // Show 3D box viewer
                        let front = box_front_url.get();
                        let back = box_back_url.get();

                        if let Some(front_url) = front {
                            view! {
                                <div class="carousel-3d-container">
                                    <Box3DViewer
                                        front_url=front_url.clone()
                                        back_url=back.clone()
                                        canvas_id=format!("box3d-{}", db_id)
                                    />
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="carousel-loading">
                                    <div class="loading-spinner"></div>
                                    <span>"Loading 3D view..."</span>
                                </div>
                            }.into_any()
                        }
                    } else {
                        // Show 2D image with LazyImage
                        view! {
                            <LazyImage
                                launchbox_db_id=db_id
                                game_title=game_title_for_render.clone()
                                platform=platform_for_render.clone()
                                image_type=current_type.clone()
                                alt=current_type.clone()
                                class="carousel-image".to_string()
                                placeholder=placeholder_for_render.clone()
                                render_index=0
                                in_viewport=true
                            />
                        }.into_any()
                    }
                }}

                // Overlay arrows
                <button class="carousel-arrow carousel-prev" on:click=prev title="Previous">
                    <svg viewBox="0 0 24 24" fill="currentColor">
                        <path d="M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z"/>
                    </svg>
                </button>
                <button class="carousel-arrow carousel-next" on:click=next title="Next">
                    <svg viewBox="0 0 24 24" fill="currentColor">
                        <path d="M8.59 16.59L10 18l6-6-6-6-1.41 1.41L13.17 12z"/>
                    </svg>
                </button>

                // Media type label
                <div class="carousel-label">
                    {move || {
                        let types = available_types.get();
                        let idx = current_index.get().min(types.len().saturating_sub(1));
                        let current_type = types.get(idx).cloned().unwrap_or_default();
                        let total = types.len();
                        format!("{} ({}/{})", current_type, idx + 1, total)
                    }}
                </div>
            </div>
        </div>
    }
}

fn format_date(date_str: &str) -> String {
    use chrono::{DateTime, NaiveDate, NaiveDateTime};

    // Try parsing as ISO 8601 with timezone (e.g., "2026-01-11T00:00:00+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return dt.format("%b %-d, %Y").to_string();
    }

    // Try parsing as datetime without timezone (e.g., "2026-01-11 23:21:43")
    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return dt.format("%b %-d, %Y").to_string();
    }

    // Try parsing as date only (e.g., "2026-01-11")
    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return d.format("%b %-d, %Y").to_string();
    }

    // Fallback to original string if parsing fails
    date_str.to_string()
}

#[component]
fn EmulatorPickerModal(
    emulators: ReadSignal<Vec<EmulatorWithStatus>>,
    emulators_loading: ReadSignal<bool>,
    game_file: ReadSignal<Option<GameFile>>,
    stored_title: StoredValue<String>,
    stored_platform: StoredValue<String>,
    stored_db_id: StoredValue<i64>,
    set_show_emulator_picker: WriteSignal<bool>,
) -> impl IntoView {
    // Track the current emulator preference
    let (current_pref, set_current_pref) = signal::<Option<String>>(None);
    // Track launching/installing state with progress message
    let (progress_state, set_progress_state) = signal::<Option<String>>(None);
    // Track error state
    let (error_state, set_error_state) = signal::<Option<String>>(None);

    // Load current preference when modal opens
    Effect::new(move || {
        let db_id = stored_db_id.get_value();
        let platform = stored_platform.get_value();
        spawn_local(async move {
            if let Ok(pref) = tauri::get_emulator_preference(db_id, platform).await {
                set_current_pref.set(pref);
            }
        });
    });

    // Can close modal only when not in progress
    let can_close = move || progress_state.get().is_none();

    view! {
        <div class="emulator-picker-overlay" on:click=move |_| {
            if can_close() {
                set_show_emulator_picker.set(false);
            }
        }>
            <div class="emulator-picker-modal" on:click=|e| e.stop_propagation()>
                <div class="emulator-picker-header">
                    <h3>"Select Emulator"</h3>
                    <button
                        class="emulator-picker-close"
                        on:click=move |_| {
                            if can_close() {
                                set_show_emulator_picker.set(false);
                            }
                        }
                        disabled=move || !can_close()
                    >"×"</button>
                </div>
                <div class="emulator-picker-content">
                    // Show progress state (installing/launching)
                    <Show when=move || progress_state.get().is_some()>
                        {move || progress_state.get().map(|msg| view! {
                            <div class="emulator-progress">
                                <div class="loading-spinner"></div>
                                <span>{msg}</span>
                            </div>
                        })}
                    </Show>

                    // Show error state
                    <Show when=move || error_state.get().is_some()>
                        {move || error_state.get().map(|err| view! {
                            <div class="emulator-error">
                                <span class="error-icon">"!"</span>
                                <span>{err}</span>
                                <button class="error-dismiss" on:click=move |_| set_error_state.set(None)>"Dismiss"</button>
                            </div>
                        })}
                    </Show>

                    // Show current preference indicator (when not in progress)
                    <Show when=move || current_pref.get().is_some() && progress_state.get().is_none()>
                        {move || current_pref.get().map(|pref| view! {
                            <div class="emulator-pref-indicator">
                                "Default: " {pref}
                            </div>
                        })}
                    </Show>

                    <Show
                        when=move || !emulators_loading.get() && progress_state.get().is_none()
                        fallback=move || {
                            if emulators_loading.get() {
                                view! { <div class="emulator-loading"><div class="loading-spinner"></div>"Loading emulators..."</div> }.into_any()
                            } else {
                                view! {}.into_any()
                            }
                        }
                    >
                        <Show
                            when=move || !emulators.get().is_empty()
                            fallback=|| view! { <div class="emulator-empty">"No emulators found for this platform."</div> }
                        >
                            <ul class="emulator-list">
                                <For
                                    each=move || emulators.get()
                                    key=|emu| emu.id
                                    children=move |emu: EmulatorWithStatus| {
                                        let name = emu.name.clone();
                                        let display_name = emu.display_name.clone();
                                        let is_installed = emu.is_installed;
                                        let is_retroarch_core = emu.is_retroarch_core;
                                        let install_method = emu.install_method.clone();
                                        let name_for_click = emu.name.clone();
                                        let name_for_game_pref = emu.name.clone();
                                        let name_for_platform_pref = emu.name.clone();
                                        let homepage = emu.homepage.clone();
                                        let notes = emu.notes.clone();

                                        // Handler for launch/install+launch
                                        let on_launch = move |_| {
                                            pause_game_details_video();
                                            let emulator_name = name_for_click.clone();
                                            let title = stored_title.get_value();
                                            let platform = stored_platform.get_value();
                                            let db_id = stored_db_id.get_value();
                                            let fallback_rom_path = game_file.get_untracked().map(|file| file.file_path);
                                            let is_ra = is_retroarch_core;

                                            if is_installed {
                                                // Just launch
                                                set_progress_state.set(Some(format!("Launching {}...", emulator_name)));
                                                spawn_local(async move {
                                                    // Record play session
                                                    let _ = tauri::record_play_session(db_id, title, platform).await;
                                                    // Launch selected emulator with ROM path
                                                    match launch_game_with_resolved_rom(
                                                        db_id,
                                                        fallback_rom_path.clone(),
                                                        emulator_name.clone(),
                                                        is_ra,
                                                    ).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                set_progress_state.set(None);
                                                                set_show_emulator_picker.set(false);
                                                            } else {
                                                                set_progress_state.set(None);
                                                                set_error_state.set(result.error);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            } else {
                                                // Install then launch
                                                set_progress_state.set(Some(format!("Installing {}...", emulator_name)));
                                                let emulator_for_install = emulator_name.clone();
                                                spawn_local(async move {
                                                    match tauri::install_emulator(emulator_for_install.clone(), is_ra).await {
                                                        Ok(_path) => {
                                                            set_progress_state.set(Some(format!("Launching {}...", emulator_for_install)));
                                                            // Record play session
                                                            let _ = tauri::record_play_session(db_id, title, platform).await;
                                                            // Launch selected emulator with ROM path
                                                            match launch_game_with_resolved_rom(
                                                                db_id,
                                                                fallback_rom_path.clone(),
                                                                emulator_for_install.clone(),
                                                                is_ra,
                                                            ).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        set_progress_state.set(None);
                                                                        set_show_emulator_picker.set(false);
                                                                    } else {
                                                                        set_progress_state.set(None);
                                                                        set_error_state.set(result.error);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_progress_state.set(None);
                                                                    set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Install failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            }
                                        };

                                        let on_set_game_pref = move |e: web_sys::MouseEvent| {
                                            e.stop_propagation();
                                            pause_game_details_video();
                                            let emulator_name = name_for_game_pref.clone();
                                            let title = stored_title.get_value();
                                            let platform = stored_platform.get_value();
                                            let db_id = stored_db_id.get_value();
                                            let fallback_rom_path = game_file.get_untracked().map(|file| file.file_path);
                                            let is_ra = is_retroarch_core;

                                            if is_installed {
                                                set_progress_state.set(Some(format!("Launching {}...", emulator_name)));
                                                spawn_local(async move {
                                                    let _ = tauri::set_game_emulator_preference(db_id, emulator_name.clone()).await;
                                                    let _ = tauri::record_play_session(db_id, title, platform).await;
                                                    match launch_game_with_resolved_rom(
                                                        db_id,
                                                        fallback_rom_path.clone(),
                                                        emulator_name.clone(),
                                                        is_ra,
                                                    ).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                set_progress_state.set(None);
                                                                set_show_emulator_picker.set(false);
                                                            } else {
                                                                set_progress_state.set(None);
                                                                set_error_state.set(result.error);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            } else {
                                                set_progress_state.set(Some(format!("Installing {}...", emulator_name)));
                                                let emu_for_install = emulator_name.clone();
                                                spawn_local(async move {
                                                    match tauri::install_emulator(emu_for_install.clone(), is_ra).await {
                                                        Ok(_) => {
                                                            let _ = tauri::set_game_emulator_preference(db_id, emu_for_install.clone()).await;
                                                            set_progress_state.set(Some(format!("Launching {}...", emu_for_install)));
                                                            let _ = tauri::record_play_session(db_id, title, platform).await;
                                                            match launch_game_with_resolved_rom(
                                                                db_id,
                                                                fallback_rom_path.clone(),
                                                                emu_for_install.clone(),
                                                                is_ra,
                                                            ).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        set_progress_state.set(None);
                                                                        set_show_emulator_picker.set(false);
                                                                    } else {
                                                                        set_progress_state.set(None);
                                                                        set_error_state.set(result.error);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_progress_state.set(None);
                                                                    set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Install failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            }
                                        };

                                        let on_set_platform_pref = move |e: web_sys::MouseEvent| {
                                            e.stop_propagation();
                                            pause_game_details_video();
                                            let emulator_name = name_for_platform_pref.clone();
                                            let title = stored_title.get_value();
                                            let platform = stored_platform.get_value();
                                            let db_id = stored_db_id.get_value();
                                            let fallback_rom_path = game_file.get_untracked().map(|file| file.file_path);
                                            let is_ra = is_retroarch_core;

                                            if is_installed {
                                                set_progress_state.set(Some(format!("Launching {}...", emulator_name)));
                                                spawn_local(async move {
                                                    let _ = tauri::set_platform_emulator_preference(platform.clone(), emulator_name.clone()).await;
                                                    let _ = tauri::record_play_session(db_id, title, platform).await;
                                                    match launch_game_with_resolved_rom(
                                                        db_id,
                                                        fallback_rom_path.clone(),
                                                        emulator_name.clone(),
                                                        is_ra,
                                                    ).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                set_progress_state.set(None);
                                                                set_show_emulator_picker.set(false);
                                                            } else {
                                                                set_progress_state.set(None);
                                                                set_error_state.set(result.error);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            } else {
                                                set_progress_state.set(Some(format!("Installing {}...", emulator_name)));
                                                let emu_for_install = emulator_name.clone();
                                                spawn_local(async move {
                                                    match tauri::install_emulator(emu_for_install.clone(), is_ra).await {
                                                        Ok(_) => {
                                                            let _ = tauri::set_platform_emulator_preference(platform.clone(), emu_for_install.clone()).await;
                                                            set_progress_state.set(Some(format!("Launching {}...", emu_for_install)));
                                                            let _ = tauri::record_play_session(db_id, title, platform).await;
                                                            match launch_game_with_resolved_rom(
                                                                db_id,
                                                                fallback_rom_path.clone(),
                                                                emu_for_install.clone(),
                                                                is_ra,
                                                            ).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        set_progress_state.set(None);
                                                                        set_show_emulator_picker.set(false);
                                                                    } else {
                                                                        set_progress_state.set(None);
                                                                        set_error_state.set(result.error);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_progress_state.set(None);
                                                                    set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Install failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            }
                                        };

                                        let is_preferred = {
                                            let name_check = name.clone();
                                            move || current_pref.get().as_ref() == Some(&name_check)
                                        };

                                        // Determine the action button text
                                        let action_text = if is_installed { "Play" } else { "Install & Play" };

                                        view! {
                                            <li class="emulator-item" class:is-installed=is_installed>
                                                <div class="emulator-item-header">
                                                    <span class="emulator-name">{display_name}</span>
                                                    <span class="emulator-status" class:installed=is_installed>
                                                        {if is_installed { "Installed" } else { "Not Installed" }}
                                                    </span>
                                                </div>
                                                <div class="emulator-item-meta">
                                                    {is_retroarch_core.then(|| view! {
                                                        <span class="emulator-badge retroarch">"RetroArch Core"</span>
                                                    })}
                                                    {install_method.clone().map(|method| view! {
                                                        <span class="emulator-badge install-method">{method}</span>
                                                    })}
                                                    {homepage.clone().map(|url| view! {
                                                        <a class="emulator-homepage" href={url} target="_blank">"Website"</a>
                                                    })}
                                                </div>
                                                {notes.clone().map(|n| view! {
                                                    <div class="emulator-notes">{n}</div>
                                                })}
                                                <div class="emulator-pref-buttons">
                                                    <button
                                                        class="emulator-pref-btn emulator-play-btn"
                                                        class:install=!is_installed
                                                        on:click=on_launch
                                                    >
                                                        {action_text}
                                                    </button>
                                                    <button
                                                        class="emulator-pref-btn"
                                                        class:active=is_preferred
                                                        on:click=on_set_game_pref
                                                    >
                                                        "Always for game"
                                                    </button>
                                                    <button
                                                        class="emulator-pref-btn"
                                                        on:click=on_set_platform_pref
                                                    >
                                                        "Always for platform"
                                                    </button>
                                                </div>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </Show>
                    </Show>
                </div>
            </div>
        </div>
    }
}

