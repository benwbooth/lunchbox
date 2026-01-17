//! Settings panel component

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::Cell;
use std::rc::Rc;
use crate::tauri::{
    get_settings, save_settings, get_credential_storage_name,
    test_screenscraper_connection, test_steamgriddb_connection,
    test_igdb_connection, AppSettings,
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

    // Load settings when shown
    let user_modified_for_load = user_modified.clone();
    Effect::new(move || {
        if show.get() && !loaded.get() {
            user_modified_for_load.set(false);
            set_loading.set(true);
            let user_modified_inner = user_modified_for_load.clone();
            spawn_local(async move {
                if let Ok(name) = get_credential_storage_name().await {
                    set_storage_name.set(name);
                }

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
