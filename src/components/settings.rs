//! Settings panel component

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::Cell;
use std::rc::Rc;
use crate::tauri::{
    get_settings, save_settings, get_credential_storage_name,
    test_screenscraper_connection, test_steamgriddb_connection,
    test_igdb_connection, get_all_regions, AppSettings,
    get_all_emulator_preferences, clear_game_emulator_preference,
    clear_platform_emulator_preference, set_platform_emulator_preference,
    get_emulators_for_platform, get_platforms, EmulatorPreferences, EmulatorInfo,
};
use super::ImageSourcesWizard;

#[component]
pub fn Settings(
    show: ReadSignal<bool>,
    on_close: WriteSignal<bool>,
) -> impl IntoView {
    // Settings state: current values and last-saved values
    let settings = RwSignal::new(AppSettings::default());
    let saved_settings = RwSignal::new(AppSettings::default());

    // Form state
    let (save_error, set_save_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);
    let (loaded, set_loaded) = signal(false);
    let (saving, set_saving) = signal(false);
    let (storage_name, set_storage_name) = signal(String::new());
    let user_modified = Rc::new(Cell::new(false));

    // Connection test state
    let (testing_ss, set_testing_ss) = signal(false);
    let (ss_test_result, set_ss_test_result) = signal::<Option<(bool, String)>>(None);
    let (testing_sgdb, set_testing_sgdb) = signal(false);
    let (sgdb_test_result, set_sgdb_test_result) = signal::<Option<(bool, String)>>(None);
    let (testing_igdb, set_testing_igdb) = signal(false);
    let (igdb_test_result, set_igdb_test_result) = signal::<Option<(bool, String)>>(None);

    // Image sources wizard state
    let (show_wizard, set_show_wizard) = signal(false);

    // Region priority state
    let (all_regions, set_all_regions) = signal::<Vec<String>>(Vec::new());
    let (regions_loading, set_regions_loading) = signal(false);

    // Load settings when shown
    let user_modified_for_load = user_modified.clone();
    Effect::new(move || {
        if show.get() && !loaded.get() {
            user_modified_for_load.set(false);
            set_loading.set(true);
            set_regions_loading.set(true);
            let user_modified_inner = user_modified_for_load.clone();
            spawn_local(async move {
                if let Ok(name) = get_credential_storage_name().await {
                    set_storage_name.set(name);
                }

                // Load all available regions
                if let Ok(regions) = get_all_regions().await {
                    set_all_regions.set(regions);
                }
                set_regions_loading.set(false);

                match get_settings().await {
                    Ok(s) => {
                        settings.set(s.clone());
                        saved_settings.set(s);
                        set_loaded.set(true);
                        gloo_timers::callback::Timeout::new(100, move || {
                            user_modified_inner.set(true);
                        }).forget();
                    }
                    Err(e) => {
                        set_save_error.set(Some(format!("Failed to load settings: {}", e)));
                    }
                }
                set_loading.set(false);
            });
        }
    });

    // Reset loaded state when closed
    Effect::new(move || {
        if !show.get() {
            set_loaded.set(false);
        }
    });

    // Auto-save function
    let do_save = move || {
        if !loaded.get() || saving.get() {
            return;
        }

        set_saving.set(true);
        set_save_error.set(None);

        let current = settings.get();
        spawn_local(async move {
            match save_settings(current.clone()).await {
                Ok(()) => {
                    saved_settings.set(current);
                }
                Err(e) => {
                    set_save_error.set(Some(e));
                }
            }
            set_saving.set(false);
        });
    };

    // Auto-save when settings change (after initial load)
    let user_modified_for_save = user_modified.clone();
    Effect::new(move || {
        let _ = settings.get(); // Track changes
        if loaded.get() && user_modified_for_save.get() {
            do_save();
        }
    });

    view! {
        <Show when=move || show.get()>
            <div class="settings-overlay" on:click=move |_| on_close.set(false)>
                <div class="settings-panel" on:click=|ev| ev.stop_propagation()>
                    <button class="close-btn" on:click=move |_| on_close.set(false)>
                        "×"
                    </button>
                    <h2 class="settings-title">
                        "Settings"
                        <Show when=move || saving.get()>
                            <span class="settings-saving">" (saving...)"</span>
                        </Show>
                    </h2>

                    <Show
                        when=move || !loading.get()
                        fallback=|| view! { <div class="loading">"Loading settings..."</div> }
                    >
                        <div class="settings-form">
                            // Image Sources Wizard Button
                            <div class="settings-section">
                                <h3>"Image Sources"</h3>
                                <p class="settings-hint">
                                    "Configure where to download box art, screenshots, and other game media."
                                </p>
                                <button
                                    class="settings-wizard-btn"
                                    on:click=move |_| set_show_wizard.set(true)
                                >
                                    "Setup Image Sources..."
                                </button>
                            </div>

                            // Region Priority Section
                            <div class="settings-section">
                                <h3>"Region Priority"</h3>
                                <p class="settings-hint">
                                    "Set which regions appear first when a game has multiple versions. Use the arrows to reorder. The top region is preferred."
                                </p>
                                <Show
                                    when=move || !regions_loading.get()
                                    fallback=|| view! { <div class="settings-loading">"Loading regions..."</div> }
                                >
                                    <RegionPriorityList
                                        all_regions=all_regions
                                        settings=settings
                                    />
                                </Show>
                            </div>

                            // ScreenScraper Section
                            <div class="settings-section settings-collapsed">
                                <h3>"ScreenScraper API (Advanced)"</h3>
                                <div class="settings-service-info">
                                    <p class="settings-hint">
                                        "Metadata, box art, screenshots, and videos based on ROM checksums."
                                    </p>
                                    <ol class="settings-steps">
                                        <li>"Register at "<a href="https://www.screenscraper.fr" target="_blank">"screenscraper.fr"</a></li>
                                        <li>"Request API access from your profile"</li>
                                        <li>"Enter developer ID and password below"</li>
                                    </ol>
                                </div>
                                <label class="settings-label">
                                    "Developer ID"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="text"
                                            class="settings-input"
                                            placeholder="Your dev ID"
                                            prop:value=move || settings.get().screenscraper.dev_id
                                            on:input=move |ev| settings.update(|s| s.screenscraper.dev_id = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().screenscraper.dev_id == saved_settings.get().screenscraper.dev_id>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <label class="settings-label">
                                    "Developer Password"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="password"
                                            class="settings-input"
                                            placeholder="Your dev password"
                                            prop:value=move || settings.get().screenscraper.dev_password
                                            on:input=move |ev| settings.update(|s| s.screenscraper.dev_password = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().screenscraper.dev_password == saved_settings.get().screenscraper.dev_password>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <label class="settings-label">
                                    "User ID (optional, for higher rate limits)"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="text"
                                            class="settings-input"
                                            placeholder="Your ScreenScraper username"
                                            prop:value=move || settings.get().screenscraper.user_id.clone().unwrap_or_default()
                                            on:input=move |ev| {
                                                let v = event_target_value(&ev);
                                                settings.update(|s| s.screenscraper.user_id = if v.is_empty() { None } else { Some(v) })
                                            }
                                        />
                                        <Show when=move || !saving.get() && settings.get().screenscraper.user_id == saved_settings.get().screenscraper.user_id>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <label class="settings-label">
                                    "User Password"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="password"
                                            class="settings-input"
                                            placeholder="Your ScreenScraper password"
                                            prop:value=move || settings.get().screenscraper.user_password.clone().unwrap_or_default()
                                            on:input=move |ev| {
                                                let v = event_target_value(&ev);
                                                settings.update(|s| s.screenscraper.user_password = if v.is_empty() { None } else { Some(v) })
                                            }
                                        />
                                        <Show when=move || !saving.get() && settings.get().screenscraper.user_password == saved_settings.get().screenscraper.user_password>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <div class="settings-test-row">
                                    <button
                                        class="settings-test-btn"
                                        on:click=move |_| {
                                            set_testing_ss.set(true);
                                            set_ss_test_result.set(None);
                                            let s = settings.get().screenscraper.clone();
                                            spawn_local(async move {
                                                let result = test_screenscraper_connection(
                                                    s.dev_id, s.dev_password, s.user_id, s.user_password
                                                ).await;
                                                match result {
                                                    Ok(res) => {
                                                        let msg = if let Some(info) = res.user_info {
                                                            format!("{} ({})", res.message, info)
                                                        } else { res.message };
                                                        set_ss_test_result.set(Some((res.success, msg)));
                                                    }
                                                    Err(e) => set_ss_test_result.set(Some((false, format!("Error: {}", e)))),
                                                }
                                                set_testing_ss.set(false);
                                            });
                                        }
                                        disabled=move || testing_ss.get()
                                    >
                                        {move || if testing_ss.get() { "Testing..." } else { "Test Connection" }}
                                    </button>
                                    <Show when=move || ss_test_result.get().is_some()>
                                        <span class={move || if ss_test_result.get().unwrap_or((false, String::new())).0 { "test-success" } else { "test-failure" }}>
                                            {move || ss_test_result.get().map(|(_, m)| m).unwrap_or_default()}
                                        </span>
                                    </Show>
                                </div>
                            </div>

                            // SteamGridDB Section
                            <div class="settings-section">
                                <h3>"SteamGridDB"</h3>
                                <div class="settings-service-info">
                                    <p class="settings-hint">
                                        "Custom game artwork: grids, heroes, logos, and icons."
                                    </p>
                                    <ol class="settings-steps">
                                        <li>"Create account at "<a href="https://www.steamgriddb.com" target="_blank">"steamgriddb.com"</a></li>
                                        <li>"Go to Preferences > API"</li>
                                        <li>"Copy your API key"</li>
                                    </ol>
                                </div>
                                <label class="settings-label">
                                    "API Key"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="password"
                                            class="settings-input"
                                            placeholder="Your SteamGridDB API key"
                                            prop:value=move || settings.get().steamgriddb.api_key
                                            on:input=move |ev| settings.update(|s| s.steamgriddb.api_key = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().steamgriddb.api_key == saved_settings.get().steamgriddb.api_key>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <div class="settings-test-row">
                                    <button
                                        class="settings-test-btn"
                                        on:click=move |_| {
                                            set_testing_sgdb.set(true);
                                            set_sgdb_test_result.set(None);
                                            let api_key = settings.get().steamgriddb.api_key.clone();
                                            spawn_local(async move {
                                                let result = test_steamgriddb_connection(api_key).await;
                                                match result {
                                                    Ok(res) => set_sgdb_test_result.set(Some((res.success, res.message))),
                                                    Err(e) => set_sgdb_test_result.set(Some((false, format!("Error: {}", e)))),
                                                }
                                                set_testing_sgdb.set(false);
                                            });
                                        }
                                        disabled=move || testing_sgdb.get()
                                    >
                                        {move || if testing_sgdb.get() { "Testing..." } else { "Test Connection" }}
                                    </button>
                                    <Show when=move || sgdb_test_result.get().is_some()>
                                        <span class={move || if sgdb_test_result.get().unwrap_or((false, String::new())).0 { "test-success" } else { "test-failure" }}>
                                            {move || sgdb_test_result.get().map(|(_, m)| m).unwrap_or_default()}
                                        </span>
                                    </Show>
                                </div>
                            </div>

                            // IGDB Section
                            <div class="settings-section">
                                <h3>"IGDB (via Twitch)"</h3>
                                <div class="settings-service-info">
                                    <p class="settings-hint">
                                        "Comprehensive game metadata, ratings, and covers from IGDB."
                                    </p>
                                    <ol class="settings-steps">
                                        <li>"Go to "<a href="https://dev.twitch.tv/console" target="_blank">"dev.twitch.tv/console"</a></li>
                                        <li>"Create a new application (any name, http://localhost for OAuth)"</li>
                                        <li>"Copy Client ID and generate a Client Secret"</li>
                                    </ol>
                                </div>
                                <label class="settings-label">
                                    "Twitch Client ID"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="text"
                                            class="settings-input"
                                            placeholder="Your Twitch Client ID"
                                            prop:value=move || settings.get().igdb.client_id
                                            on:input=move |ev| settings.update(|s| s.igdb.client_id = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().igdb.client_id == saved_settings.get().igdb.client_id>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <label class="settings-label">
                                    "Twitch Client Secret"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="password"
                                            class="settings-input"
                                            placeholder="Your Twitch Client Secret"
                                            prop:value=move || settings.get().igdb.client_secret
                                            on:input=move |ev| settings.update(|s| s.igdb.client_secret = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().igdb.client_secret == saved_settings.get().igdb.client_secret>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <div class="settings-test-row">
                                    <button
                                        class="settings-test-btn"
                                        on:click=move |_| {
                                            set_testing_igdb.set(true);
                                            set_igdb_test_result.set(None);
                                            let igdb = settings.get().igdb.clone();
                                            spawn_local(async move {
                                                let result = test_igdb_connection(igdb.client_id, igdb.client_secret).await;
                                                match result {
                                                    Ok(res) => {
                                                        let msg = if let Some(info) = res.user_info {
                                                            format!("{} ({})", res.message, info)
                                                        } else { res.message };
                                                        set_igdb_test_result.set(Some((res.success, msg)));
                                                    }
                                                    Err(e) => set_igdb_test_result.set(Some((false, format!("Error: {}", e)))),
                                                }
                                                set_testing_igdb.set(false);
                                            });
                                        }
                                        disabled=move || testing_igdb.get()
                                    >
                                        {move || if testing_igdb.get() { "Testing..." } else { "Test Connection" }}
                                    </button>
                                    <Show when=move || igdb_test_result.get().is_some()>
                                        <span class={move || if igdb_test_result.get().unwrap_or((false, String::new())).0 { "test-success" } else { "test-failure" }}>
                                            {move || igdb_test_result.get().map(|(_, m)| m).unwrap_or_default()}
                                        </span>
                                    </Show>
                                </div>
                            </div>

                            // EmuMovies Section
                            <div class="settings-section">
                                <h3>"EmuMovies"</h3>
                                <p class="settings-help">
                                    "Premium media including box art, screenshots, and videos."
                                    " Requires EmuMovies account (lifetime access available)."
                                </p>
                                <label class="settings-label">
                                    "Username"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="text"
                                            class="settings-input"
                                            placeholder="Your EmuMovies username"
                                            prop:value=move || settings.get().emumovies.username
                                            on:input=move |ev| settings.update(|s| s.emumovies.username = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().emumovies.username == saved_settings.get().emumovies.username>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                                <label class="settings-label">
                                    "Password"
                                    <span class="settings-input-wrapper">
                                        <input
                                            type="password"
                                            class="settings-input"
                                            placeholder="Your EmuMovies password"
                                            prop:value=move || settings.get().emumovies.password
                                            on:input=move |ev| settings.update(|s| s.emumovies.password = event_target_value(&ev))
                                        />
                                        <Show when=move || !saving.get() && settings.get().emumovies.password == saved_settings.get().emumovies.password>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                            </div>

                            // Emulator Preferences Section
                            <div class="settings-section">
                                <h3>"Emulator Preferences"</h3>
                                <p class="settings-hint">
                                    "Manage your default emulator preferences for games and platforms."
                                </p>
                                <EmulatorPreferencesSection />
                            </div>

                            <Show when=move || save_error.get().is_some()>
                                <div class="settings-error">
                                    {move || save_error.get().unwrap_or_default()}
                                </div>
                            </Show>

                            // Storage location note
                            <Show when=move || !storage_name.get().is_empty()>
                                <p class="settings-storage-note">
                                    "Credentials stored securely in " {move || storage_name.get()}
                                </p>
                            </Show>
                        </div>
                    </Show>
                </div>
            </div>

            // Image Sources Wizard (modal)
            <ImageSourcesWizard
                show=show_wizard
                on_close=set_show_wizard
            />
        </Show>
    }
}

/// Display name for a region (empty string = "No Region / Plain")
fn region_display_name(region: &str) -> String {
    if region.is_empty() {
        "No Region (Plain)".to_string()
    } else {
        region.to_string()
    }
}

/// Reorderable list for region priority using drag and drop
#[component]
fn RegionPriorityList(
    all_regions: ReadSignal<Vec<String>>,
    settings: RwSignal<AppSettings>,
) -> impl IntoView {
    // Track dragging state
    let (dragging_idx, set_dragging_idx) = signal::<Option<usize>>(None);
    let (drop_target_idx, set_drop_target_idx) = signal::<Option<usize>>(None);

    // Compute the display order: user's saved priority first, then remaining regions
    let display_order = move || -> Vec<String> {
        let saved_priority = settings.get().region_priority;
        let all = all_regions.get();

        if !saved_priority.is_empty() {
            let mut result = saved_priority.clone();
            for region in all {
                if !result.contains(&region) {
                    result.push(region);
                }
            }
            result
        } else {
            all
        }
    };

    let move_region = move |from: usize, to: usize| {
        if from != to {
            settings.update(|s| {
                let mut order = display_order();
                if from < order.len() && to <= order.len() {
                    let item = order.remove(from);
                    // Adjust target index if we removed before it
                    let adjusted_to = if from < to { to - 1 } else { to };
                    order.insert(adjusted_to.min(order.len()), item);
                    s.region_priority = order;
                }
            });
        }
    };

    view! {
        <div
            class="region-priority-list"
            on:dragover=move |e| e.prevent_default()
            on:drop=move |e| {
                e.prevent_default();
                if let (Some(from), Some(to)) = (dragging_idx.get(), drop_target_idx.get()) {
                    move_region(from, to);
                }
                set_dragging_idx.set(None);
                set_drop_target_idx.set(None);
            }
        >
            {move || {
                let regions = display_order();
                let len = regions.len();
                regions.into_iter().enumerate().map(|(idx, region)| {
                    let display_name = region_display_name(&region);
                    let is_dragging = move || dragging_idx.get() == Some(idx);
                    let show_drop_before = move || {
                        drop_target_idx.get() == Some(idx) && dragging_idx.get() != Some(idx)
                    };
                    let show_drop_after = move || {
                        idx == len - 1 && drop_target_idx.get() == Some(len) && dragging_idx.get() != Some(idx)
                    };

                    view! {
                        // Drop indicator line before this item
                        <div
                            class="drop-indicator"
                            class:visible=show_drop_before
                        />
                        <div
                            class="region-priority-item"
                            class:dragging=is_dragging
                            draggable="true"
                            on:dragstart=move |_| {
                                set_dragging_idx.set(Some(idx));
                            }
                            on:dragend=move |_| {
                                set_dragging_idx.set(None);
                                set_drop_target_idx.set(None);
                            }
                            on:dragover=move |e| {
                                e.prevent_default();
                                e.stop_propagation();
                                // Show drop indicator before this item
                                set_drop_target_idx.set(Some(idx));
                            }
                        >
                            <span class="drag-handle" title="Drag to reorder">
                                <svg width="10" height="16" viewBox="0 0 10 16">
                                    <circle cx="2" cy="2" r="1.5" fill="currentColor"/>
                                    <circle cx="8" cy="2" r="1.5" fill="currentColor"/>
                                    <circle cx="2" cy="8" r="1.5" fill="currentColor"/>
                                    <circle cx="8" cy="8" r="1.5" fill="currentColor"/>
                                    <circle cx="2" cy="14" r="1.5" fill="currentColor"/>
                                    <circle cx="8" cy="14" r="1.5" fill="currentColor"/>
                                </svg>
                            </span>
                            <span class="region-name">{display_name}</span>
                        </div>
                        // Drop indicator line after last item
                        {(idx == len - 1).then(|| view! {
                            <div
                                class="drop-indicator"
                                class:visible=show_drop_after
                            />
                        })}
                    }
                }).collect_view()
            }}
        </div>
    }
}

/// Data for a platform with its emulators and current preference
#[derive(Clone)]
struct PlatformEmulatorData {
    platform_name: String,
    emulators: Vec<EmulatorInfo>,
    current_preference: Option<String>,
}

/// Emulator preferences management section
#[component]
fn EmulatorPreferencesSection() -> impl IntoView {
    let (platform_data, set_platform_data) = signal::<Vec<PlatformEmulatorData>>(Vec::new());
    let (game_prefs, set_game_prefs) = signal::<Vec<crate::tauri::GameEmulatorPref>>(Vec::new());
    let (loading, set_loading) = signal(true);

    // Load all data on mount
    let load_data = move || {
        set_loading.set(true);
        spawn_local(async move {
            // Load platforms, emulators, and preferences in parallel
            let platforms_fut = get_platforms();
            let prefs_fut = get_all_emulator_preferences();

            let (platforms_res, prefs_res) = futures::join!(platforms_fut, prefs_fut);

            let platforms = platforms_res.unwrap_or_default();
            let prefs = prefs_res.unwrap_or(EmulatorPreferences {
                game_preferences: Vec::new(),
                platform_preferences: Vec::new(),
            });

            // Store game preferences
            set_game_prefs.set(prefs.game_preferences);

            // Build platform -> preference lookup
            let pref_map: std::collections::HashMap<String, String> = prefs.platform_preferences
                .into_iter()
                .map(|p| (p.platform_name, p.emulator_name))
                .collect();

            // Load emulators for each platform
            let mut data = Vec::new();
            for platform in platforms {
                if let Ok(emulators) = get_emulators_for_platform(platform.name.clone()).await {
                    if !emulators.is_empty() {
                        data.push(PlatformEmulatorData {
                            platform_name: platform.name.clone(),
                            emulators,
                            current_preference: pref_map.get(&platform.name).cloned(),
                        });
                    }
                }
            }

            // Sort by platform name
            data.sort_by(|a, b| a.platform_name.cmp(&b.platform_name));
            set_platform_data.set(data);
            set_loading.set(false);
        });
    };

    // Initial load
    Effect::new(move || {
        load_data();
    });

    view! {
        <div class="emulator-prefs-section">
            <Show
                when=move || !loading.get()
                fallback=|| view! { <div class="emulator-loading">"Loading..."</div> }
            >
                // Per-Game Preferences (if any)
                {move || {
                    let gp = game_prefs.get();
                    if gp.is_empty() {
                        view! {}.into_any()
                    } else {
                        view! {
                            <div class="emulator-prefs-subsection">
                                <h4>"Per-Game Overrides"</h4>
                                <div class="emulator-pref-list">
                                    {gp.into_iter().map(|pref| {
                                        let db_id = pref.launchbox_db_id;
                                        let game_title = pref.game_title.clone().unwrap_or_else(|| format!("Game #{}", db_id));
                                        let emulator_name = pref.emulator_name.clone();

                                        view! {
                                            <div class="emulator-pref-item">
                                                <span class="emulator-pref-game">{game_title}</span>
                                                <span class="emulator-pref-arrow">"→"</span>
                                                <span class="emulator-pref-emulator">{emulator_name}</span>
                                                <button
                                                    class="emulator-pref-clear-btn"
                                                    on:click=move |_| {
                                                        spawn_local(async move {
                                                            let _ = clear_game_emulator_preference(db_id).await;
                                                            load_data();
                                                        });
                                                    }
                                                >
                                                    "×"
                                                </button>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }.into_any()
                    }
                }}

                // Platform Defaults Table
                <div class="emulator-prefs-subsection">
                    <h4>"Platform Defaults"</h4>
                    <p class="emulator-prefs-hint">"Select default emulator for each platform, or leave as 'Ask every time'."</p>
                    <div class="platform-emulator-table">
                        {move || {
                            platform_data.get().into_iter().map(|pd| {
                                let platform_name = pd.platform_name.clone();
                                let platform_name_for_change = pd.platform_name.clone();
                                let emulators = pd.emulators.clone();
                                let current = pd.current_preference.clone();

                                view! {
                                    <div class="platform-emulator-row">
                                        <span class="platform-name">{platform_name}</span>
                                        <select
                                            class="emulator-select"
                                            on:change=move |ev| {
                                                let value = event_target_value(&ev);
                                                let platform = platform_name_for_change.clone();
                                                spawn_local(async move {
                                                    if value.is_empty() {
                                                        let _ = clear_platform_emulator_preference(platform).await;
                                                    } else {
                                                        let _ = set_platform_emulator_preference(platform, value).await;
                                                    }
                                                    load_data();
                                                });
                                            }
                                        >
                                            <option value="" selected=current.is_none()>"Ask every time"</option>
                                            {emulators.iter().map(|emu| {
                                                let name = emu.name.clone();
                                                let display = if let Some(ref core) = emu.retroarch_core {
                                                    format!("RetroArch: {}", core)
                                                } else {
                                                    emu.name.clone()
                                                };
                                                let is_selected = current.as_ref() == Some(&name);
                                                view! {
                                                    <option value=name.clone() selected=is_selected>{display}</option>
                                                }
                                            }).collect_view()}
                                        </select>
                                    </div>
                                }
                            }).collect_view()
                        }}
                    </div>
                </div>
            </Show>
        </div>
    }
}
