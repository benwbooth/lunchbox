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

        let current = settings.get();
        // Only save if settings actually changed
        if current == saved_settings.get() {
            return;
        }

        set_saving.set(true);
        set_save_error.set(None);

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
    use leptos::prelude::NodeRef;
    use wasm_bindgen::JsCast;

    // Track dragging state using signals
    let (dragging_region, set_dragging_region) = signal::<Option<String>>(None);
    let (drop_target_idx, set_drop_target_idx) = signal::<Option<usize>>(None);

    // Ref to the list container for position calculations
    let list_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Compute the display order as a memo
    let display_order = Memo::new(move |_| {
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
    });

    // Calculate drop target based on mouse Y position
    let calculate_drop_target = move |client_y: i32| {
        if let Some(list_el) = list_ref.get() {
            let list_node: &web_sys::HtmlElement = &list_el;
            let children = list_node.children();
            let len = display_order.get().len();

            // Collect midpoints of each item (skip drop indicators, only look at items)
            let mut item_midpoints: Vec<(usize, f64)> = Vec::new();
            let mut item_idx = 0;

            for i in 0..children.length() {
                if let Some(child) = children.item(i) {
                    if let Some(el) = child.dyn_ref::<web_sys::Element>() {
                        // Only process actual items, not drop indicators
                        if el.class_list().contains("region-priority-item") {
                            // Use getBoundingClientRect for accurate viewport-relative position
                            let rect = el.get_bounding_client_rect();
                            let midpoint = rect.top() + rect.height() / 2.0;
                            item_midpoints.push((item_idx, midpoint));
                            item_idx += 1;
                        }
                    }
                }
            }

            // Find which position the cursor is closest to
            let y = client_y as f64;

            // If above all items, drop at position 0
            if let Some((_, first_mid)) = item_midpoints.first() {
                if y < *first_mid {
                    return Some(0);
                }
            }

            // Find the gap between items where cursor is
            for window in item_midpoints.windows(2) {
                if let [(idx, mid1), (_, mid2)] = window {
                    if y >= *mid1 && y < *mid2 {
                        return Some(idx + 1);
                    }
                }
            }

            // If below all items, drop at end
            if let Some((_, last_mid)) = item_midpoints.last() {
                if y >= *last_mid {
                    return Some(len);
                }
            }
        }
        None
    };

    // Perform the drop operation
    let perform_drop = move || {
        if let (Some(from_region), Some(to_idx)) =
            (dragging_region.get_untracked(), drop_target_idx.get_untracked())
        {
            let mut order = display_order.get();
            if let Some(from_idx) = order.iter().position(|r| r == &from_region) {
                if from_idx != to_idx && from_idx + 1 != to_idx {
                    let item = order.remove(from_idx);
                    // Adjust target index after removal
                    let insert_at = if from_idx < to_idx {
                        to_idx - 1
                    } else {
                        to_idx
                    };
                    order.insert(insert_at.min(order.len()), item);
                    settings.update(|s| s.region_priority = order);
                }
            }
        }
        set_dragging_region.set(None);
        set_drop_target_idx.set(None);
    };

    view! {
        <div
            class="region-priority-list"
            node_ref=list_ref
            on:dragover=move |e| {
                e.prevent_default();
                if dragging_region.get().is_some() {
                    let y = e.client_y();
                    if let Some(target) = calculate_drop_target(y) {
                        web_sys::console::log_1(&format!("dragover y={} target={}", y, target).into());
                        set_drop_target_idx.set(Some(target));
                    } else {
                        web_sys::console::log_1(&format!("dragover y={} NO TARGET", y).into());
                    }
                }
            }
            on:drop=move |e| {
                e.prevent_default();
                perform_drop();
            }
            on:dragleave=move |e| {
                // Only clear if leaving the list entirely
                if let Some(related) = e.related_target() {
                    if let Some(list_el) = list_ref.get() {
                        let list_el: &web_sys::Node = &list_el;
                        if !list_el.contains(related.dyn_ref::<web_sys::Node>()) {
                            set_drop_target_idx.set(None);
                        }
                    }
                } else {
                    set_drop_target_idx.set(None);
                }
            }
        >
            <For
                each=move || display_order.get().into_iter().enumerate()
                key=|(_, region)| region.clone()
                children=move |(idx, region)| {
                    let display_name = region_display_name(&region);
                    let region_for_drag = region.clone();
                    let region_for_class = region.clone();
                    let len = display_order.get().len();

                    view! {
                        // Drop indicator before this item
                        <div
                            class=move || {
                                let dominated_region = dragging_region.get();
                                let from_idx = dominated_region.as_ref().and_then(|r| {
                                    display_order.get().iter().position(|x| x == r)
                                });
                                let target = drop_target_idx.get();

                                // Show if this is the drop target and we're dragging
                                // Don't show at positions that wouldn't move the item
                                let dominated_is_dragging = dragging_region.get().is_some();
                                let is_target = target == Some(idx);
                                let would_be_noop = from_idx == Some(idx) || from_idx.map(|i| i + 1) == Some(idx);

                                if dominated_is_dragging && is_target && !would_be_noop {
                                    "drop-indicator visible"
                                } else {
                                    "drop-indicator"
                                }
                            }
                        />
                        <div
                            class=move || {
                                if dragging_region.get().as_ref() == Some(&region_for_class) {
                                    "region-priority-item dragging"
                                } else {
                                    "region-priority-item"
                                }
                            }
                            draggable="true"
                            on:dragstart={
                                let region = region_for_drag.clone();
                                move |_| {
                                    web_sys::console::log_1(&format!("dragstart: {}", region).into());
                                    set_dragging_region.set(Some(region.clone()));
                                }
                            }
                            on:dragend=move |_| {
                                set_dragging_region.set(None);
                                set_drop_target_idx.set(None);
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
                        // Drop indicator after last item
                        {(idx == len - 1).then(|| {
                            view! {
                                <div
                                    class=move || {
                                        let len = display_order.get().len();
                                        let dragging_from_idx = dragging_region.get().as_ref().and_then(|r| {
                                            display_order.get().iter().position(|x| x == r)
                                        });
                                        // Show at end if drop target is len and not dragging from last position
                                        if drop_target_idx.get() == Some(len)
                                            && dragging_region.get().is_some()
                                            && dragging_from_idx != Some(len - 1)
                                        {
                                            "drop-indicator drop-indicator-end visible"
                                        } else {
                                            "drop-indicator drop-indicator-end"
                                        }
                                    }
                                />
                            }
                        })}
                    }
                }
            />
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
