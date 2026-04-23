//! Settings panel component

use super::ImageSourcesWizard;
use crate::backend_api::{
    AppSettings, EmulatorInfo, EmulatorLaunchTemplateOverride, EmulatorPreferences,
    clear_emulator_launch_template_override, clear_game_emulator_preference,
    clear_platform_emulator_preference, get_all_emulator_launch_template_overrides,
    get_all_emulator_preferences, get_all_emulators, get_all_regions, get_credential_storage_name,
    get_emulators_for_platform, get_platforms, get_settings, save_settings,
    set_emulator_launch_template_override, set_platform_emulator_preference, test_igdb_connection,
    test_screenscraper_connection, test_steamgriddb_connection, test_torrent_connection,
};
use futures::future::join_all;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::Cell;
use std::rc::Rc;

#[component]
pub fn Settings(show: ReadSignal<bool>, on_close: WriteSignal<bool>) -> impl IntoView {
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
    let (testing_torrent, set_testing_torrent) = signal(false);
    let (torrent_test_result, set_torrent_test_result) = signal::<Option<(bool, String)>>(None);

    // Image sources wizard state
    let (show_wizard, set_show_wizard) = signal(false);

    // Per-field password visibility toggles
    let show_pw = [
        RwSignal::new(false), // 0: ScreenScraper dev password
        RwSignal::new(false), // 1: ScreenScraper user password
        RwSignal::new(false), // 2: SteamGridDB API key
        RwSignal::new(false), // 3: IGDB client secret
        RwSignal::new(false), // 4: EmuMovies password
        RwSignal::new(false), // 5: qBittorrent password
    ];

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
                        })
                        .forget();
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
            <div
                class="settings-overlay"
                data-nav-scope="settings"
                data-nav-scope-active="true"
                data-nav-scope-priority="200"
                on:click=move |_| on_close.set(false)
            >
                <div class="settings-panel" on:click=|ev| ev.stop_propagation()>
                    <button
                        class="close-btn"
                        data-nav-back="true"
                        on:click=move |_| on_close.set(false)
                    >
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
                                            type=move || if show_pw[0].get() { "text" } else { "password" }
                                            class="settings-input"
                                            placeholder="Your dev password"
                                            prop:value=move || settings.get().screenscraper.dev_password
                                            on:input=move |ev| settings.update(|s| s.screenscraper.dev_password = event_target_value(&ev))
                                        />
                                        <button type="button" class="password-eye-btn" on:click=move |_| show_pw[0].update(|v| *v = !*v) title="Toggle visibility">
                                            <PasswordEyeIcon visible=show_pw[0] />
                                        </button>
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
                                            type=move || if show_pw[1].get() { "text" } else { "password" }
                                            class="settings-input"
                                            placeholder="Your ScreenScraper password"
                                            prop:value=move || settings.get().screenscraper.user_password.clone().unwrap_or_default()
                                            on:input=move |ev| {
                                                let v = event_target_value(&ev);
                                                settings.update(|s| s.screenscraper.user_password = if v.is_empty() { None } else { Some(v) })
                                            }
                                        />
                                        <button type="button" class="password-eye-btn" on:click=move |_| show_pw[1].update(|v| *v = !*v) title="Toggle visibility">
                                            <PasswordEyeIcon visible=show_pw[1] />
                                        </button>
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
                                            type=move || if show_pw[2].get() { "text" } else { "password" }
                                            class="settings-input"
                                            placeholder="Your SteamGridDB API key"
                                            prop:value=move || settings.get().steamgriddb.api_key
                                            on:input=move |ev| settings.update(|s| s.steamgriddb.api_key = event_target_value(&ev))
                                        />
                                        <button type="button" class="password-eye-btn" on:click=move |_| show_pw[2].update(|v| *v = !*v) title="Toggle visibility">
                                            <PasswordEyeIcon visible=show_pw[2] />
                                        </button>
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
                                            type=move || if show_pw[3].get() { "text" } else { "password" }
                                            class="settings-input"
                                            placeholder="Your Twitch Client Secret"
                                            prop:value=move || settings.get().igdb.client_secret
                                            on:input=move |ev| settings.update(|s| s.igdb.client_secret = event_target_value(&ev))
                                        />
                                        <button type="button" class="password-eye-btn" on:click=move |_| show_pw[3].update(|v| *v = !*v) title="Toggle visibility">
                                            <PasswordEyeIcon visible=show_pw[3] />
                                        </button>
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
                                            type=move || if show_pw[4].get() { "text" } else { "password" }
                                            class="settings-input"
                                            placeholder="Your EmuMovies password"
                                            prop:value=move || settings.get().emumovies.password
                                            on:input=move |ev| settings.update(|s| s.emumovies.password = event_target_value(&ev))
                                        />
                                        <button type="button" class="password-eye-btn" on:click=move |_| show_pw[4].update(|v| *v = !*v) title="Toggle visibility">
                                            <PasswordEyeIcon visible=show_pw[4] />
                                        </button>
                                        <Show when=move || !saving.get() && settings.get().emumovies.password == saved_settings.get().emumovies.password>
                                            <span class="settings-saved-check">"✓"</span>
                                        </Show>
                                    </span>
                                </label>
                            </div>


                            // Torrent / Downloads Section
                            <div class="settings-section">
                                <h3>"Downloads / Torrent"</h3>
                                <p class="settings-help">
                                    "Configure qBittorrent Web UI for downloading ROMs from Minerva Archive."
                                </p>
                                <p class="settings-hint">
                                    "Only qBittorrent Web UI is supported for Minerva downloads."
                                </p>

                                <div class="settings-field">
                                    <label class="settings-label">"qBittorrent Host"</label>
                                    <input
                                        class="settings-input"
                                        placeholder="localhost"
                                        prop:value=move || settings.get().torrent.qbittorrent_host
                                        on:input=move |ev| settings.update(|s| s.torrent.qbittorrent_host = event_target_value(&ev))
                                    />
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"qBittorrent Port"</label>
                                    <input
                                        class="settings-input"
                                        type="number"
                                        placeholder="8080"
                                        prop:value=move || settings.get().torrent.qbittorrent_port.to_string()
                                        on:input=move |ev| settings.update(|s| s.torrent.qbittorrent_port = event_target_value(&ev).parse().unwrap_or(8080))
                                    />
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"qBittorrent Username"</label>
                                    <input
                                        class="settings-input"
                                        placeholder="admin"
                                        prop:value=move || settings.get().torrent.qbittorrent_username
                                        on:input=move |ev| settings.update(|s| s.torrent.qbittorrent_username = event_target_value(&ev))
                                    />
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"qBittorrent Password"</label>
                                    <div class="password-field">
                                        <input
                                            class="settings-input"
                                            type=move || if show_pw[5].get() { "text" } else { "password" }
                                            prop:value=move || settings.get().torrent.qbittorrent_password
                                            on:input=move |ev| settings.update(|s| s.torrent.qbittorrent_password = event_target_value(&ev))
                                        />
                                        <button class="toggle-pw-btn" on:click=move |_| show_pw[5].update(|v| *v = !*v)>
                                            {move || if show_pw[5].get() { "Hide" } else { "Show" }}
                                        </button>
                                    </div>
                                </div>

                                // Test connection button
                                <div class="connection-test">
                                    <button
                                        class="test-btn"
                                        disabled=move || testing_torrent.get()
                                        on:click=move |_| {
                                            set_testing_torrent.set(true);
                                            set_torrent_test_result.set(None);
                                            spawn_local(async move {
                                                match test_torrent_connection().await {
                                                    Ok(result) => set_torrent_test_result.set(Some((result.success, result.message))),
                                                    Err(e) => set_torrent_test_result.set(Some((false, e))),
                                                }
                                                set_testing_torrent.set(false);
                                            });
                                        }
                                    >
                                        {move || if testing_torrent.get() { "Testing..." } else { "Test Connection" }}
                                    </button>
                                    <Show when=move || torrent_test_result.get().is_some()>
                                        {move || torrent_test_result.get().map(|(success, msg)| view! {
                                            <span class=move || if success { "test-result test-success" } else { "test-result test-failure" }>
                                                {msg}
                                            </span>
                                        })}
                                    </Show>
                                </div>

                                // Directory settings
                                <div class="settings-field">
                                    <label class="settings-label">"ROM Directory"</label>
                                    <input
                                        class="settings-input"
                                        placeholder="Default: ~/.local/share/lunchbox/roms"
                                        prop:value=move || settings.get().torrent.rom_directory.clone().unwrap_or_default()
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            settings.update(|s| s.torrent.rom_directory = if v.is_empty() { None } else { Some(v) });
                                        }
                                    />
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"qBittorrent Container ROM Directory"</label>
                                    <input
                                        class="settings-input"
                                        placeholder="Optional: e.g. /downloads/roms"
                                        prop:value=move || settings.get().torrent.qbittorrent_container_rom_directory.clone().unwrap_or_default()
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            settings.update(|s| s.torrent.qbittorrent_container_rom_directory = if v.is_empty() { None } else { Some(v) });
                                        }
                                    />
                                    <p class="settings-hint">"Use this when qBittorrent runs in Docker or another container and sees a different ROM path than Lunchbox."</p>
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"Torrent Library Directory"</label>
                                    <input
                                        class="settings-input"
                                        placeholder="Default: ~/.local/share/lunchbox/torrent-library"
                                        prop:value=move || settings.get().torrent.torrent_library_directory.clone().unwrap_or_default()
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            settings.update(|s| s.torrent.torrent_library_directory = if v.is_empty() { None } else { Some(v) });
                                        }
                                    />
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"qBittorrent Container Torrent Library Directory"</label>
                                    <input
                                        class="settings-input"
                                        placeholder="Optional: e.g. /downloads/torrent-library"
                                        prop:value=move || settings.get().torrent.qbittorrent_container_torrent_library_directory.clone().unwrap_or_default()
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            settings.update(|s| s.torrent.qbittorrent_container_torrent_library_directory = if v.is_empty() { None } else { Some(v) });
                                        }
                                    />
                                    <p class="settings-hint">"Use this when full-torrent downloads should be stored at a different path inside the qBittorrent runtime."</p>
                                </div>
                                <div class="settings-field">
                                    <label class="settings-label">"File Link Mode"</label>
                                    <select
                                        class="settings-input"
                                        prop:value=move || settings.get().torrent.file_link_mode
                                        on:change=move |ev| settings.update(|s| s.torrent.file_link_mode = event_target_value(&ev))
                                    >
                                        <option value="symlink">"Symlink (recommended)"</option>
                                        <option value="hardlink">"Hard link"</option>
                                        <option value="reflink">"Reflink (CoW copy)"</option>
                                        <option value="copy">"Copy"</option>
                                        <option value="leave_in_place">"Leave in place"</option>
                                    </select>
                                    <p class="settings-hint">"Used when you choose a whole-torrent download from the Minerva picker and Lunchbox links the selected game back into the ROM directory."</p>
                                </div>
                            </div>

                            // Emulator Preferences Section
                            <div class="settings-section">
                                <h3>"Emulator Preferences"</h3>
                                <p class="settings-hint">
                                    "Manage your default emulator preferences for games and platforms."
                                </p>
                                <EmulatorPreferencesSection />
                            </div>

                            <div class="settings-section">
                                <h3>"Emulator Launch Commands"</h3>
                                <p class="settings-hint">
                                    "Defaults are generated from Lunchbox's built-in launcher. Save an override to replace the generated command template for a platform, emulator, and runtime."
                                </p>
                                <EmulatorLaunchCommandsSection />
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

/// Eye icon that toggles between open (visible) and closed (hidden) states
#[component]
fn PasswordEyeIcon(visible: RwSignal<bool>) -> impl IntoView {
    view! {
        <Show
            when=move || visible.get()
            fallback=|| view! {
                // Eye closed (password hidden)
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                    <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                    <path d="M14.12 14.12a3 3 0 1 1-4.24-4.24"/>
                    <line x1="1" y1="1" x2="23" y2="23"/>
                </svg>
            }
        >
            // Eye open (password visible)
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                <circle cx="12" cy="12" r="3"/>
            </svg>
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

    // Track dragging state by region name (stable across reorders)
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

    // Derive the region name at the target position (for visual indicator)
    // This recomputes whenever drop_target_idx or display_order changes
    let target_region = Memo::new(move |_| {
        let target_idx = drop_target_idx.get()?;
        let order = display_order.get();
        // If target_idx == len, we're inserting at end (return None)
        order.get(target_idx).cloned()
    });

    // Is the target at the very end?
    let target_at_end = Memo::new(move |_| {
        let target_idx = drop_target_idx.get();
        let order = display_order.get();
        match target_idx {
            Some(idx) => idx == order.len(),
            None => false,
        }
    });

    // Calculate drop target based on mouse Y position
    let calculate_drop_target = move |client_y: i32| -> Option<usize> {
        let list_el = list_ref.get()?;
        let list_node: &web_sys::HtmlElement = &list_el;
        let children = list_node.children();

        // Collect midpoints of each item
        let mut item_mids: Vec<f64> = Vec::new();

        for i in 0..children.length() {
            if let Some(child) = children.item(i) {
                if let Some(el) = child.dyn_ref::<web_sys::Element>() {
                    if el.class_list().contains("region-priority-item") {
                        let rect = el.get_bounding_client_rect();
                        let mid = rect.top() + rect.height() / 2.0;
                        item_mids.push(mid);
                    }
                }
            }
        }

        let y = client_y as f64;
        let len = item_mids.len();

        // Find insert position based on cursor Y vs item midpoints
        for (i, mid) in item_mids.iter().enumerate() {
            if y < *mid {
                return Some(i);
            }
        }

        // Below all midpoints = insert at end
        Some(len)
    };

    // Check if dropping at target would actually move the item
    let is_valid_drop = move |from_idx: usize, to_idx: usize| -> bool {
        // Can't drop at same position or immediately after (no movement)
        from_idx != to_idx && from_idx + 1 != to_idx
    };

    // Perform the drop operation
    let perform_drop = move || {
        let order = display_order.get_untracked();
        let to_idx = drop_target_idx.get_untracked();
        let region = dragging_region.get_untracked();

        if let (Some(region), Some(to_idx)) = (region, to_idx) {
            if let Some(from_idx) = order.iter().position(|r| r == &region) {
                if is_valid_drop(from_idx, to_idx) {
                    let mut new_order = order;
                    let item = new_order.remove(from_idx);
                    let insert_at = if from_idx < to_idx {
                        to_idx - 1
                    } else {
                        to_idx
                    };
                    new_order.insert(insert_at.min(new_order.len()), item);
                    settings.update(|s| s.region_priority = new_order);
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
                    if let Some(target) = calculate_drop_target(e.client_y()) {
                        set_drop_target_idx.set(Some(target));
                    }
                }
            }
            on:drop=move |e| {
                e.prevent_default();
                perform_drop();
            }
        >
            <For
                each=move || display_order.get()
                key=|region| region.clone()
                children=move |region| {
                    let display_name = region_display_name(&region);
                    let region_for_indicator = region.clone();
                    let region_for_class = region.clone();
                    let region_for_drag = region;

                    view! {
                        // Drop indicator before this item (compare by region name, not stale index)
                        <Show when=move || {
                            dragging_region.get().is_some()
                                && target_region.get().as_ref() == Some(&region_for_indicator)
                        }>
                            <div class="drop-indicator">"Drop here"</div>
                        </Show>
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
                    }
                }
            />
            // Drop indicator at end (outside For, uses target_at_end memo)
            <Show when=move || dragging_region.get().is_some() && target_at_end.get()>
                <div class="drop-indicator">"Drop here"</div>
            </Show>
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
    let (game_prefs, set_game_prefs) =
        signal::<Vec<crate::backend_api::GameEmulatorPref>>(Vec::new());
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
            let pref_map: std::collections::HashMap<String, String> = prefs
                .platform_preferences
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

fn launch_profile_runtime_label(runtime_kind: &str) -> &'static str {
    if runtime_kind.eq_ignore_ascii_case("retroarch") {
        "RetroArch Core"
    } else {
        "Standalone"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchTemplateRow {
    platform_name: Option<String>,
    emulator_name: String,
    runtime_kind: String,
    default_template: String,
}

impl LaunchTemplateRow {
    fn key(&self) -> String {
        format!(
            "{}\u{1f}|{}\u{1f}|{}",
            self.platform_name.as_deref().unwrap_or(""),
            self.emulator_name,
            self.runtime_kind,
        )
    }
}

fn join_command_template_tokens<I>(tokens: I) -> String
where
    I: IntoIterator<Item = String>,
{
    tokens
        .into_iter()
        .map(|token| {
            if token.is_empty() {
                "''".to_string()
            } else if token.chars().all(|ch| {
                ch.is_ascii_alphanumeric()
                    || matches!(ch, '%' | '_' | '-' | '.' | '/' | ':' | '{' | '}')
            }) {
                token
            } else {
                format!("'{}'", token.replace('\'', "'\"'\"'"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn default_launch_command_template(
    emulator_name: &str,
    platform_name: Option<&str>,
    is_retroarch_core: bool,
) -> String {
    fn is_arcade_family_platform(platform_name: Option<&str>) -> bool {
        platform_name
            .is_some_and(|name| matches!(name, "Arcade" | "Arcade Pinball" | "Arcade Laserdisc"))
    }

    if is_retroarch_core {
        return join_command_template_tokens(vec![
            "--verbose".to_string(),
            "-L".to_string(),
            "%{core}".to_string(),
            "%f".to_string(),
        ]);
    }

    if emulator_name == "Hypseus Singe" && is_arcade_family_platform(platform_name) {
        return join_command_template_tokens(vec![
            "%{hypseus_game}".to_string(),
            "vldp".to_string(),
            "-fullscreen".to_string(),
            "-framefile".to_string(),
            "%{hypseus_framefile}".to_string(),
            "-homedir".to_string(),
            "%{hypseus_support_root}".to_string(),
            "-datadir".to_string(),
            "%{hypseus_support_root}".to_string(),
            "-romdir".to_string(),
            "%{hypseus_romdir}".to_string(),
        ]);
    }

    if emulator_name == "MAME" && is_arcade_family_platform(platform_name) {
        return join_command_template_tokens(vec![
            "-rompath".to_string(),
            "%{mame_rompath}".to_string(),
            "%{mame_romset}".to_string(),
        ]);
    }

    if emulator_name == "Altirra" {
        return join_command_template_tokens(vec![
            "%{altirra_media_switch}".to_string(),
            "%f".to_string(),
        ]);
    }

    "%f".to_string()
}

fn append_launch_template_rows(
    rows: &mut Vec<LaunchTemplateRow>,
    platform_name: Option<String>,
    emulator: &EmulatorInfo,
) {
    rows.push(LaunchTemplateRow {
        platform_name: platform_name.clone(),
        emulator_name: emulator.name.clone(),
        runtime_kind: "standalone".to_string(),
        default_template: default_launch_command_template(
            &emulator.name,
            platform_name.as_deref(),
            false,
        ),
    });

    if emulator.retroarch_core.is_some() {
        rows.push(LaunchTemplateRow {
            platform_name,
            emulator_name: emulator.name.clone(),
            runtime_kind: "retroarch".to_string(),
            default_template: default_launch_command_template(&emulator.name, None, true),
        });
    }
}

fn template_override_for_row(
    overrides: &[EmulatorLaunchTemplateOverride],
    row: &LaunchTemplateRow,
) -> Option<String> {
    overrides
        .iter()
        .find(|override_row| {
            override_row.emulator_name == row.emulator_name
                && override_row.runtime_kind == row.runtime_kind
                && override_row.platform_name == row.platform_name
        })
        .map(|override_row| override_row.command_template.clone())
}

fn sort_launch_template_rows(rows: &mut [LaunchTemplateRow]) {
    rows.sort_by(|left, right| {
        (
            left.platform_name.is_some(),
            left.platform_name.clone().unwrap_or_default(),
            left.emulator_name.clone(),
            left.runtime_kind.clone(),
        )
            .cmp(&(
                right.platform_name.is_some(),
                right.platform_name.clone().unwrap_or_default(),
                right.emulator_name.clone(),
                right.runtime_kind.clone(),
            ))
    });
}

#[component]
fn EmulatorLaunchCommandsSection() -> impl IntoView {
    let (rows, set_rows) = signal::<Vec<LaunchTemplateRow>>(Vec::new());
    let (overrides, set_overrides) = signal::<Vec<EmulatorLaunchTemplateOverride>>(Vec::new());
    let (filter_text, set_filter_text) = signal(String::new());
    let (loading, set_loading) = signal(true);
    let (status, set_status) = signal::<Option<String>>(None);
    let (error, set_error) = signal::<Option<String>>(None);

    Effect::new(move || {
        set_loading.set(true);
        spawn_local(async move {
            set_error.set(None);
            let (platforms_res, overrides_res, emulators_res) = futures::join!(
                get_platforms(),
                get_all_emulator_launch_template_overrides(),
                get_all_emulators(Some(true))
            );

            let mut platform_names = match platforms_res {
                Ok(items) => {
                    let mut names = items
                        .into_iter()
                        .map(|platform| platform.name)
                        .collect::<Vec<_>>();
                    names.sort();
                    names
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load platforms: {}", e)));
                    Vec::new()
                }
            };

            match overrides_res {
                Ok(items) => set_overrides.set(items),
                Err(e) => set_error.set(Some(format!(
                    "Failed to load launch command overrides: {}",
                    e
                ))),
            }

            let mut generated_rows = Vec::new();
            match emulators_res {
                Ok(mut emulators) => {
                    emulators.sort_by(|left, right| left.name.cmp(&right.name));
                    for emulator in &emulators {
                        append_launch_template_rows(&mut generated_rows, None, emulator);
                    }
                }
                Err(e) => set_error.set(Some(format!("Failed to load emulators: {}", e))),
            }

            let platform_emulator_results =
                join_all(platform_names.drain(..).map(|platform_name| async move {
                    let result = get_emulators_for_platform(platform_name.clone()).await;
                    (platform_name, result)
                }))
                .await;

            for (platform_name, result) in platform_emulator_results {
                match result {
                    Ok(mut emulators) => {
                        emulators.sort_by(|left, right| left.name.cmp(&right.name));
                        for emulator in &emulators {
                            append_launch_template_rows(
                                &mut generated_rows,
                                Some(platform_name.clone()),
                                emulator,
                            );
                        }
                    }
                    Err(e) => {
                        set_error.set(Some(format!(
                            "Failed to load launch command rows for {}: {}",
                            platform_name, e
                        )));
                    }
                }
            }

            sort_launch_template_rows(&mut generated_rows);
            set_rows.set(generated_rows);
            set_loading.set(false);
        });
    });

    let filtered_rows = Memo::new(move |_| {
        let query = filter_text.get().trim().to_ascii_lowercase();
        rows.get()
            .into_iter()
            .filter(|row| {
                if query.is_empty() {
                    return true;
                }

                let platform = row.platform_name.as_deref().unwrap_or("all platforms");
                format!(
                    "{} {} {} {}",
                    platform, row.emulator_name, row.runtime_kind, row.default_template
                )
                .to_ascii_lowercase()
                .contains(&query)
            })
            .collect::<Vec<_>>()
    });

    view! {
        <div class="emulator-launch-commands">
            <Show
                when=move || !loading.get()
                fallback=|| view! { <div class="emulator-loading">"Loading..."</div> }
            >
                <Show when=move || error.get().is_some()>
                    {move || error.get().map(|message| view! {
                        <div class="settings-error">{message}</div>
                    })}
                </Show>

                <Show when=move || status.get().is_some()>
                    {move || status.get().map(|message| view! {
                        <div class="settings-storage-note">{message}</div>
                    })}
                </Show>

                <div class="launch-template-toolbar">
                    <input
                        class="settings-input launch-template-filter"
                        type="search"
                        placeholder="Filter by platform, emulator, runtime, or template"
                        prop:value=move || filter_text.get()
                        on:input=move |ev| {
                            set_filter_text.set(event_target_value(&ev));
                            set_status.set(None);
                            set_error.set(None);
                        }
                    />
                    <div class="launch-template-count">
                        {move || format!("{} rows", filtered_rows.get().len())}
                    </div>
                </div>

                <p class="settings-hint">
                    "Use `%f` for the selected file and `%%` for a literal percent. Templates are parsed into argv directly, not through a shell. Existing legacy extra-arg overrides still apply only when no command override is saved."
                </p>

                <div class="launch-template-table-wrap">
                    <Show
                        when=move || !filtered_rows.get().is_empty()
                        fallback=|| view! { <p class="settings-hint">"No launch command rows match the current filter."</p> }
                    >
                        <table class="launch-template-table">
                            <thead>
                                <tr>
                                    <th>"Platform"</th>
                                    <th>"Emulator"</th>
                                    <th>"Runtime"</th>
                                    <th>"Default"</th>
                                    <th>"Override"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || filtered_rows.get()
                                    key=|row| row.key()
                                    children=move |row: LaunchTemplateRow| {
                                        let initial_override =
                                            template_override_for_row(&overrides.get_untracked(), &row)
                                                .unwrap_or_default();
                                        let (draft_template, set_draft_template) = signal(initial_override);
                                        let (row_saving, set_row_saving) = signal(false);
                                        let platform_label = row
                                            .platform_name
                                            .clone()
                                            .unwrap_or_else(|| "All platforms".to_string());
                                        let emulator_name = row.emulator_name.clone();
                                        let runtime_kind = row.runtime_kind.clone();
                                        let default_template = row.default_template.clone();
                                        let save_platform_name = row.platform_name.clone();
                                        let clear_platform_name = row.platform_name.clone();
                                        let save_emulator_name = emulator_name.clone();
                                        let clear_emulator_name = emulator_name.clone();
                                        let save_runtime_kind = runtime_kind.clone();
                                        let clear_runtime_kind = runtime_kind.clone();

                                        view! {
                                            <tr>
                                                <td>{platform_label.clone()}</td>
                                                <td>{emulator_name.clone()}</td>
                                                <td>{launch_profile_runtime_label(&runtime_kind)}</td>
                                                <td>
                                                    <code class="launch-template-default">
                                                        {default_template}
                                                    </code>
                                                </td>
                                                <td>
                                                    <textarea
                                                        class="settings-input launch-template-override"
                                                        rows="3"
                                                        placeholder="Leave blank to use the generated default"
                                                        prop:value=move || draft_template.get()
                                                        on:input=move |ev| {
                                                            set_draft_template.set(event_target_value(&ev));
                                                            set_status.set(None);
                                                            set_error.set(None);
                                                        }
                                                    />
                                                </td>
                                                <td>
                                                    <div class="launch-template-actions">
                                                        <button
                                                            class="emulator-pref-btn emulator-play-btn"
                                                            disabled=move || row_saving.get()
                                                            on:click=move |_| {
                                                                let platform_name = save_platform_name.clone();
                                                                let emulator_name = save_emulator_name.clone();
                                                                let runtime_kind = save_runtime_kind.clone();
                                                                let command_template = draft_template.get_untracked();
                                                                set_row_saving.set(true);
                                                                set_status.set(None);
                                                                set_error.set(None);

                                                                spawn_local(async move {
                                                                    let normalized_template =
                                                                        command_template.trim().to_string();
                                                                    let result = set_emulator_launch_template_override(
                                                                        emulator_name.clone(),
                                                                        platform_name.clone(),
                                                                        runtime_kind == "retroarch",
                                                                        normalized_template.clone(),
                                                                    )
                                                                    .await;

                                                                    match result {
                                                                        Ok(()) => {
                                                                            set_overrides.update(|items| {
                                                                                items.retain(|item| {
                                                                                    !(item.emulator_name == emulator_name
                                                                                        && item.runtime_kind == runtime_kind
                                                                                        && item.platform_name == platform_name)
                                                                                });
                                                                                if !normalized_template.is_empty() {
                                                                                    items.push(EmulatorLaunchTemplateOverride {
                                                                                        emulator_name: emulator_name.clone(),
                                                                                        platform_name: platform_name.clone(),
                                                                                        runtime_kind: runtime_kind.clone(),
                                                                                        command_template: normalized_template.clone(),
                                                                                    });
                                                                                }
                                                                            });
                                                                            set_status.set(Some(format!(
                                                                                "Saved command override for {} ({}, {}).",
                                                                                emulator_name,
                                                                                platform_name
                                                                                    .as_deref()
                                                                                    .unwrap_or("all platforms"),
                                                                                launch_profile_runtime_label(&runtime_kind),
                                                                            )));
                                                                        }
                                                                        Err(e) => set_error.set(Some(format!(
                                                                            "Failed to save launch command override: {}",
                                                                            e
                                                                        ))),
                                                                    }

                                                                    set_row_saving.set(false);
                                                                });
                                                            }
                                                        >
                                                            {move || if row_saving.get() { "Saving..." } else { "Save" }}
                                                        </button>
                                                        <button
                                                            class="emulator-pref-btn emulator-uninstall-btn"
                                                            disabled=move || row_saving.get() || draft_template.get().trim().is_empty()
                                                            on:click=move |_| {
                                                                let platform_name = clear_platform_name.clone();
                                                                let emulator_name = clear_emulator_name.clone();
                                                                let runtime_kind = clear_runtime_kind.clone();
                                                                set_row_saving.set(true);
                                                                set_status.set(None);
                                                                set_error.set(None);

                                                                spawn_local(async move {
                                                                    match clear_emulator_launch_template_override(
                                                                        emulator_name.clone(),
                                                                        platform_name.clone(),
                                                                        runtime_kind == "retroarch",
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(()) => {
                                                                            set_draft_template.set(String::new());
                                                                            set_overrides.update(|items| {
                                                                                items.retain(|item| {
                                                                                    !(item.emulator_name == emulator_name
                                                                                        && item.runtime_kind == runtime_kind
                                                                                        && item.platform_name == platform_name)
                                                                                });
                                                                            });
                                                                            set_status.set(Some(format!(
                                                                                "Cleared command override for {} ({}, {}).",
                                                                                emulator_name,
                                                                                platform_name
                                                                                    .as_deref()
                                                                                    .unwrap_or("all platforms"),
                                                                                launch_profile_runtime_label(&runtime_kind),
                                                                            )));
                                                                        }
                                                                        Err(e) => set_error.set(Some(format!(
                                                                            "Failed to clear launch command override: {}",
                                                                            e
                                                                        ))),
                                                                    }

                                                                    set_row_saving.set(false);
                                                                });
                                                            }
                                                        >
                                                            "Clear"
                                                        </button>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
