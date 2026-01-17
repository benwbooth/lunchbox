//! Settings panel component

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::path::PathBuf;
use crate::tauri::{
    get_settings, save_settings, test_screenscraper_connection, test_steamgriddb_connection,
    test_igdb_connection, AppSettings, ScreenScraperSettings, SteamGridDBSettings, IGDBSettings,
    EmuMoviesSettings,
};
use super::ImageSourcesWizard;

#[component]
pub fn Settings(
    show: ReadSignal<bool>,
    on_close: WriteSignal<bool>,
) -> impl IntoView {
    // Local state for form fields
    let (launchbox_path, set_launchbox_path) = signal(String::new());
    let (retroarch_path, set_retroarch_path) = signal(String::new());
    let (rom_directories, set_rom_directories) = signal(String::new());

    // ScreenScraper fields
    let (ss_dev_id, set_ss_dev_id) = signal(String::new());
    let (ss_dev_password, set_ss_dev_password) = signal(String::new());
    let (ss_user_id, set_ss_user_id) = signal(String::new());
    let (ss_user_password, set_ss_user_password) = signal(String::new());

    // SteamGridDB fields
    let (sgdb_api_key, set_sgdb_api_key) = signal(String::new());

    // IGDB fields
    let (igdb_client_id, set_igdb_client_id) = signal(String::new());
    let (igdb_client_secret, set_igdb_client_secret) = signal(String::new());

    // EmuMovies fields
    let (em_username, set_em_username) = signal(String::new());
    let (em_password, set_em_password) = signal(String::new());

    // Form state
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);
    let (loaded, set_loaded) = signal(false);

    // Connection test state
    let (testing_ss, set_testing_ss) = signal(false);
    let (ss_test_result, set_ss_test_result) = signal::<Option<(bool, String)>>(None);
    let (testing_sgdb, set_testing_sgdb) = signal(false);
    let (sgdb_test_result, set_sgdb_test_result) = signal::<Option<(bool, String)>>(None);
    let (testing_igdb, set_testing_igdb) = signal(false);
    let (igdb_test_result, set_igdb_test_result) = signal::<Option<(bool, String)>>(None);

    // Image sources wizard state
    let (show_wizard, set_show_wizard) = signal(false);

    // Load settings when shown
    Effect::new(move || {
        if show.get() && !loaded.get() {
            set_loading.set(true);
            spawn_local(async move {
                match get_settings().await {
                    Ok(settings) => {
                        set_launchbox_path.set(
                            settings.launchbox_path
                                .map(|p: PathBuf| p.display().to_string())
                                .unwrap_or_default()
                        );
                        set_retroarch_path.set(
                            settings.retroarch_path
                                .map(|p: PathBuf| p.display().to_string())
                                .unwrap_or_default()
                        );
                        set_rom_directories.set(
                            settings.rom_directories
                                .iter()
                                .map(|p: &PathBuf| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join("\n")
                        );
                        // ScreenScraper
                        set_ss_dev_id.set(settings.screenscraper.dev_id);
                        set_ss_dev_password.set(settings.screenscraper.dev_password);
                        set_ss_user_id.set(settings.screenscraper.user_id.unwrap_or_default());
                        set_ss_user_password.set(settings.screenscraper.user_password.unwrap_or_default());
                        // SteamGridDB
                        set_sgdb_api_key.set(settings.steamgriddb.api_key);
                        // IGDB
                        set_igdb_client_id.set(settings.igdb.client_id);
                        set_igdb_client_secret.set(settings.igdb.client_secret);
                        // EmuMovies
                        set_em_username.set(settings.emumovies.username);
                        set_em_password.set(settings.emumovies.password);

                        set_loaded.set(true);
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

    let on_save = move |_| {
        set_saving.set(true);
        set_save_error.set(None);

        let lb_path = launchbox_path.get();
        let ra_path = retroarch_path.get();
        let rom_dirs = rom_directories.get();
        let dev_id = ss_dev_id.get();
        let dev_password = ss_dev_password.get();
        let user_id = ss_user_id.get();
        let user_password = ss_user_password.get();
        let api_key = sgdb_api_key.get();
        let client_id = igdb_client_id.get();
        let client_secret = igdb_client_secret.get();
        let em_user = em_username.get();
        let em_pass = em_password.get();
        let close_fn = on_close;

        spawn_local(async move {
            let settings = AppSettings {
                launchbox_path: if lb_path.is_empty() { None } else { Some(PathBuf::from(lb_path)) },
                retroarch_path: if ra_path.is_empty() { None } else { Some(PathBuf::from(ra_path)) },
                cache_directory: None,  // Uses default location
                rom_directories: rom_dirs
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| PathBuf::from(l.trim()))
                    .collect(),
                emulators: vec![],
                default_platform_emulators: std::collections::HashMap::new(),
                screenscraper: ScreenScraperSettings {
                    dev_id,
                    dev_password,
                    user_id: if user_id.is_empty() { None } else { Some(user_id) },
                    user_password: if user_password.is_empty() { None } else { Some(user_password) },
                },
                steamgriddb: SteamGridDBSettings {
                    api_key,
                },
                igdb: IGDBSettings {
                    client_id,
                    client_secret,
                },
                emumovies: EmuMoviesSettings {
                    username: em_user,
                    password: em_pass,
                },
            };

            match save_settings(settings).await {
                Ok(_) => {
                    set_saving.set(false);
                    close_fn.set(false);
                }
                Err(e) => {
                    set_saving.set(false);
                    set_save_error.set(Some(e));
                }
            }
        });
    };

    view! {
        <Show when=move || show.get()>
            <div class="settings-overlay" on:click=move |_| on_close.set(false)>
                <div class="settings-panel" on:click=|ev| ev.stop_propagation()>
                    <button class="close-btn" on:click=move |_| on_close.set(false)>
                        "x"
                    </button>
                    <h2 class="settings-title">"Settings"</h2>

                    <Show
                        when=move || !loading.get()
                        fallback=|| view! { <div class="loading">"Loading settings..."</div> }
                    >
                        <div class="settings-form">
                            <div class="settings-section">
                                <h3>"LaunchBox Integration"</h3>
                                <label class="settings-label">
                                    "LaunchBox Installation Path"
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="/path/to/LaunchBox"
                                        prop:value=move || launchbox_path.get()
                                        on:input=move |ev| set_launchbox_path.set(event_target_value(&ev))
                                    />
                                </label>
                                <p class="settings-hint">
                                    "Path to your LaunchBox installation folder (contains Metadata/LaunchBox.Metadata.db)"
                                </p>
                            </div>

                            <div class="settings-section">
                                <h3>"ROM Directories"</h3>
                                <label class="settings-label">
                                    "ROM Paths (one per line)"
                                    <textarea
                                        class="settings-textarea"
                                        placeholder="/mnt/roms\n/mnt/ext/roms"
                                        rows="4"
                                        prop:value=move || rom_directories.get()
                                        on:input=move |ev| set_rom_directories.set(event_target_value(&ev))
                                    />
                                </label>
                            </div>

                            <div class="settings-section">
                                <h3>"Emulators"</h3>
                                <label class="settings-label">
                                    "RetroArch Path"
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="/usr/bin/retroarch"
                                        prop:value=move || retroarch_path.get()
                                        on:input=move |ev| set_retroarch_path.set(event_target_value(&ev))
                                    />
                                </label>
                            </div>

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
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="Your dev ID"
                                        prop:value=move || ss_dev_id.get()
                                        on:input=move |ev| set_ss_dev_id.set(event_target_value(&ev))
                                    />
                                </label>
                                <label class="settings-label">
                                    "Developer Password"
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Your dev password"
                                        prop:value=move || ss_dev_password.get()
                                        on:input=move |ev| set_ss_dev_password.set(event_target_value(&ev))
                                    />
                                </label>
                                <label class="settings-label">
                                    "User ID (optional, for higher rate limits)"
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="Your ScreenScraper username"
                                        prop:value=move || ss_user_id.get()
                                        on:input=move |ev| set_ss_user_id.set(event_target_value(&ev))
                                    />
                                </label>
                                <label class="settings-label">
                                    "User Password"
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Your ScreenScraper password"
                                        prop:value=move || ss_user_password.get()
                                        on:input=move |ev| set_ss_user_password.set(event_target_value(&ev))
                                    />
                                </label>
                                <div class="settings-test-row">
                                    <button
                                        class="settings-test-btn"
                                        on:click=move |_| {
                                            set_testing_ss.set(true);
                                            set_ss_test_result.set(None);
                                            let dev_id = ss_dev_id.get();
                                            let dev_password = ss_dev_password.get();
                                            let user_id = ss_user_id.get();
                                            let user_password = ss_user_password.get();
                                            spawn_local(async move {
                                                let result = test_screenscraper_connection(
                                                    dev_id, dev_password,
                                                    if user_id.is_empty() { None } else { Some(user_id) },
                                                    if user_password.is_empty() { None } else { Some(user_password) },
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
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Your SteamGridDB API key"
                                        prop:value=move || sgdb_api_key.get()
                                        on:input=move |ev| set_sgdb_api_key.set(event_target_value(&ev))
                                    />
                                </label>
                                <div class="settings-test-row">
                                    <button
                                        class="settings-test-btn"
                                        on:click=move |_| {
                                            set_testing_sgdb.set(true);
                                            set_sgdb_test_result.set(None);
                                            let api_key = sgdb_api_key.get();
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
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="Your Twitch Client ID"
                                        prop:value=move || igdb_client_id.get()
                                        on:input=move |ev| set_igdb_client_id.set(event_target_value(&ev))
                                    />
                                </label>
                                <label class="settings-label">
                                    "Twitch Client Secret"
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Your Twitch Client Secret"
                                        prop:value=move || igdb_client_secret.get()
                                        on:input=move |ev| set_igdb_client_secret.set(event_target_value(&ev))
                                    />
                                </label>
                                <div class="settings-test-row">
                                    <button
                                        class="settings-test-btn"
                                        on:click=move |_| {
                                            set_testing_igdb.set(true);
                                            set_igdb_test_result.set(None);
                                            let client_id = igdb_client_id.get();
                                            let client_secret = igdb_client_secret.get();
                                            spawn_local(async move {
                                                let result = test_igdb_connection(client_id, client_secret).await;
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
                                    <input
                                        type="text"
                                        class="settings-input"
                                        placeholder="Your EmuMovies username"
                                        prop:value=move || em_username.get()
                                        on:input=move |ev| set_em_username.set(event_target_value(&ev))
                                    />
                                </label>
                                <label class="settings-label">
                                    "Password"
                                    <input
                                        type="password"
                                        class="settings-input"
                                        placeholder="Your EmuMovies password"
                                        prop:value=move || em_password.get()
                                        on:input=move |ev| set_em_password.set(event_target_value(&ev))
                                    />
                                </label>
                            </div>

                            <Show when=move || save_error.get().is_some()>
                                <div class="settings-error">
                                    {move || save_error.get().unwrap_or_default()}
                                </div>
                            </Show>

                            <div class="settings-actions">
                                <button
                                    class="settings-cancel-btn"
                                    on:click=move |_| on_close.set(false)
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="settings-save-btn"
                                    on:click=on_save
                                    disabled=move || saving.get()
                                >
                                    {move || if saving.get() { "Saving..." } else { "Save" }}
                                </button>
                            </div>
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
