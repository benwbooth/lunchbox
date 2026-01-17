//! Image Sources Setup Wizard
//!
//! Helps users configure image download sources with step-by-step instructions.

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::tauri::{
    get_settings, save_settings, test_steamgriddb_connection, test_igdb_connection,
    test_emumovies_connection, test_screenscraper_connection, AppSettings,
};

/// Status of an image source
#[derive(Clone, Copy, PartialEq)]
enum SourceStatus {
    /// No configuration needed - always works
    AlwaysAvailable,
    /// Configured and tested working
    Configured,
    /// Has credentials but not tested
    Untested,
    /// Not configured
    NotConfigured,
}

impl SourceStatus {
    fn icon(&self) -> &'static str {
        match self {
            SourceStatus::AlwaysAvailable => "✓",
            SourceStatus::Configured => "✓",
            SourceStatus::Untested => "?",
            SourceStatus::NotConfigured => "○",
        }
    }

    fn class(&self) -> &'static str {
        match self {
            SourceStatus::AlwaysAvailable => "source-status-ok",
            SourceStatus::Configured => "source-status-ok",
            SourceStatus::Untested => "source-status-untested",
            SourceStatus::NotConfigured => "source-status-none",
        }
    }
}

/// Image source information
struct ImageSource {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    needs_account: bool,
    signup_url: Option<&'static str>,
    signup_instructions: &'static str,
}

const SOURCES: &[ImageSource] = &[
    ImageSource {
        id: "launchbox",
        name: "LaunchBox CDN",
        description: "High-quality images from LaunchBox's public CDN. Covers most games with imported metadata.",
        needs_account: false,
        signup_url: None,
        signup_instructions: "No setup required. Works automatically with imported LaunchBox metadata.",
    },
    ImageSource {
        id: "libretro",
        name: "libretro-thumbnails",
        description: "Free thumbnails for retro games. Good coverage for classic consoles.",
        needs_account: false,
        signup_url: None,
        signup_instructions: "No setup required. Works automatically for supported platforms.",
    },
    ImageSource {
        id: "steamgriddb",
        name: "SteamGridDB",
        description: "Community-driven artwork including grids, heroes, logos, and icons. Great for modern and PC games.",
        needs_account: true,
        signup_url: Some("https://www.steamgriddb.com/profile/preferences/api"),
        signup_instructions: "1. Create a free account at steamgriddb.com\n2. Go to Profile → Preferences → API\n3. Generate an API key\n4. Paste the key below",
    },
    ImageSource {
        id: "igdb",
        name: "IGDB (via Twitch)",
        description: "Comprehensive game database with covers, screenshots, and artwork. Excellent for modern games.",
        needs_account: true,
        signup_url: Some("https://dev.twitch.tv/console/apps"),
        signup_instructions: "1. Go to dev.twitch.tv and log in with Twitch\n2. Click 'Register Your Application'\n3. Name: 'Lunchbox', OAuth Redirect: 'http://localhost'\n4. Category: 'Application Integration'\n5. Copy Client ID and generate a Client Secret",
    },
    ImageSource {
        id: "emumovies",
        name: "EmuMovies",
        description: "Premium media library with box art, screenshots, videos, and manuals. Lifetime access available.",
        needs_account: true,
        signup_url: Some("https://emumovies.com/"),
        signup_instructions: "1. Purchase or use existing EmuMovies account\n2. Enter your username and password below\n3. Lifetime accounts have unlimited API access",
    },
    ImageSource {
        id: "screenscraper",
        name: "ScreenScraper",
        description: "Extensive database with checksum-based matching. Best when scraping ROMs directly.",
        needs_account: true,
        signup_url: Some("https://www.screenscraper.fr/"),
        signup_instructions: "1. Create account at screenscraper.fr\n2. For API access, register as a developer\n3. Note: Rate limited without subscription\n4. Works best with ROM file checksums",
    },
];

#[component]
pub fn ImageSourcesWizard(
    show: ReadSignal<bool>,
    on_close: WriteSignal<bool>,
) -> impl IntoView {
    // Current step/expanded source
    let (expanded_source, set_expanded_source) = signal::<Option<&'static str>>(None);

    // Settings state
    let (settings, set_settings) = signal::<Option<AppSettings>>(None);
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (save_error, set_save_error) = signal::<Option<String>>(None);

    // Test results for each source
    let (steamgriddb_status, set_steamgriddb_status) = signal(SourceStatus::NotConfigured);
    let (igdb_status, set_igdb_status) = signal(SourceStatus::NotConfigured);
    let (emumovies_status, set_emumovies_status) = signal(SourceStatus::NotConfigured);
    let (screenscraper_status, set_screenscraper_status) = signal(SourceStatus::NotConfigured);

    // Testing state
    let (testing, set_testing) = signal::<Option<&'static str>>(None);
    let (test_result, set_test_result) = signal::<Option<(bool, String)>>(None);

    // Input fields for each source
    let (sgdb_api_key, set_sgdb_api_key) = signal(String::new());
    let (igdb_client_id, set_igdb_client_id) = signal(String::new());
    let (igdb_client_secret, set_igdb_client_secret) = signal(String::new());
    let (em_username, set_em_username) = signal(String::new());
    let (em_password, set_em_password) = signal(String::new());
    let (ss_dev_id, set_ss_dev_id) = signal(String::new());
    let (ss_dev_password, set_ss_dev_password) = signal(String::new());
    let (ss_user_id, set_ss_user_id) = signal(String::new());
    let (ss_user_password, set_ss_user_password) = signal(String::new());

    // Load settings
    Effect::new(move || {
        if show.get() {
            set_loading.set(true);
            spawn_local(async move {
                match get_settings().await {
                    Ok(s) => {
                        // Update input fields
                        set_sgdb_api_key.set(s.steamgriddb.api_key.clone());
                        set_igdb_client_id.set(s.igdb.client_id.clone());
                        set_igdb_client_secret.set(s.igdb.client_secret.clone());
                        set_em_username.set(s.emumovies.username.clone());
                        set_em_password.set(s.emumovies.password.clone());
                        set_ss_dev_id.set(s.screenscraper.dev_id.clone());
                        set_ss_dev_password.set(s.screenscraper.dev_password.clone());
                        set_ss_user_id.set(s.screenscraper.user_id.clone().unwrap_or_default());
                        set_ss_user_password.set(s.screenscraper.user_password.clone().unwrap_or_default());

                        // Update status based on whether credentials exist
                        if !s.steamgriddb.api_key.is_empty() {
                            set_steamgriddb_status.set(SourceStatus::Untested);
                        }
                        if !s.igdb.client_id.is_empty() && !s.igdb.client_secret.is_empty() {
                            set_igdb_status.set(SourceStatus::Untested);
                        }
                        if !s.emumovies.username.is_empty() && !s.emumovies.password.is_empty() {
                            set_emumovies_status.set(SourceStatus::Untested);
                        }
                        if !s.screenscraper.dev_id.is_empty() && !s.screenscraper.dev_password.is_empty() {
                            set_screenscraper_status.set(SourceStatus::Untested);
                        }

                        set_settings.set(Some(s));
                    }
                    Err(e) => {
                        set_save_error.set(Some(format!("Failed to load settings: {}", e)));
                    }
                }
                set_loading.set(false);
            });
        }
    });

    // Save settings helper
    let save_current_settings = move || {
        set_saving.set(true);
        set_save_error.set(None);

        let current = settings.get();
        if let Some(mut s) = current {
            s.steamgriddb.api_key = sgdb_api_key.get();
            s.igdb.client_id = igdb_client_id.get();
            s.igdb.client_secret = igdb_client_secret.get();
            s.emumovies.username = em_username.get();
            s.emumovies.password = em_password.get();
            s.screenscraper.dev_id = ss_dev_id.get();
            s.screenscraper.dev_password = ss_dev_password.get();
            s.screenscraper.user_id = if ss_user_id.get().is_empty() { None } else { Some(ss_user_id.get()) };
            s.screenscraper.user_password = if ss_user_password.get().is_empty() { None } else { Some(ss_user_password.get()) };

            spawn_local(async move {
                match save_settings(s.clone()).await {
                    Ok(_) => {
                        set_settings.set(Some(s));
                    }
                    Err(e) => {
                        set_save_error.set(Some(e));
                    }
                }
                set_saving.set(false);
            });
        }
    };

    // Test connection handlers
    let test_steamgriddb = move |_| {
        set_testing.set(Some("steamgriddb"));
        set_test_result.set(None);
        let api_key = sgdb_api_key.get();
        spawn_local(async move {
            match test_steamgriddb_connection(api_key).await {
                Ok(result) => {
                    if result.success {
                        set_steamgriddb_status.set(SourceStatus::Configured);
                    }
                    set_test_result.set(Some((result.success, result.message)));
                }
                Err(e) => {
                    set_test_result.set(Some((false, e)));
                }
            }
            set_testing.set(None);
        });
    };

    let test_igdb = move |_| {
        set_testing.set(Some("igdb"));
        set_test_result.set(None);
        let client_id = igdb_client_id.get();
        let client_secret = igdb_client_secret.get();
        spawn_local(async move {
            match test_igdb_connection(client_id, client_secret).await {
                Ok(result) => {
                    if result.success {
                        set_igdb_status.set(SourceStatus::Configured);
                    }
                    let msg = if let Some(info) = result.user_info {
                        format!("{} ({})", result.message, info)
                    } else {
                        result.message
                    };
                    set_test_result.set(Some((result.success, msg)));
                }
                Err(e) => {
                    set_test_result.set(Some((false, e)));
                }
            }
            set_testing.set(None);
        });
    };

    let test_emumovies = move |_| {
        set_testing.set(Some("emumovies"));
        set_test_result.set(None);
        let username = em_username.get();
        let password = em_password.get();
        spawn_local(async move {
            match test_emumovies_connection(username, password).await {
                Ok(result) => {
                    if result.success {
                        set_emumovies_status.set(SourceStatus::Configured);
                    }
                    set_test_result.set(Some((result.success, result.message)));
                }
                Err(e) => {
                    set_test_result.set(Some((false, e)));
                }
            }
            set_testing.set(None);
        });
    };

    let test_screenscraper = move |_| {
        set_testing.set(Some("screenscraper"));
        set_test_result.set(None);
        let dev_id = ss_dev_id.get();
        let dev_password = ss_dev_password.get();
        let user_id = ss_user_id.get();
        let user_password = ss_user_password.get();
        spawn_local(async move {
            match test_screenscraper_connection(
                dev_id,
                dev_password,
                if user_id.is_empty() { None } else { Some(user_id) },
                if user_password.is_empty() { None } else { Some(user_password) },
            ).await {
                Ok(result) => {
                    if result.success {
                        set_screenscraper_status.set(SourceStatus::Configured);
                    }
                    let msg = if let Some(info) = result.user_info {
                        format!("{} ({})", result.message, info)
                    } else {
                        result.message
                    };
                    set_test_result.set(Some((result.success, msg)));
                }
                Err(e) => {
                    set_test_result.set(Some((false, e)));
                }
            }
            set_testing.set(None);
        });
    };

    let get_status = move |id: &'static str| -> SourceStatus {
        match id {
            "launchbox" | "libretro" => SourceStatus::AlwaysAvailable,
            "steamgriddb" => steamgriddb_status.get(),
            "igdb" => igdb_status.get(),
            "emumovies" => emumovies_status.get(),
            "screenscraper" => screenscraper_status.get(),
            _ => SourceStatus::NotConfigured,
        }
    };

    view! {
        <Show when=move || show.get()>
            <div class="wizard-overlay" on:click=move |_| on_close.set(false)>
                <div class="wizard-modal" on:click=|e| e.stop_propagation()>
                    <div class="wizard-header">
                        <h2>"Image Sources Setup"</h2>
                        <button class="wizard-close" on:click=move |_| on_close.set(false)>"×"</button>
                    </div>

                    <Show when=move || loading.get()>
                        <div class="wizard-loading">"Loading settings..."</div>
                    </Show>

                    <Show when=move || !loading.get()>
                        <div class="wizard-content">
                            <p class="wizard-intro">
                                "Configure image sources to download box art, screenshots, and other media. "
                                "Sources are tried in order until an image is found."
                            </p>

                            <div class="source-list">
                                {SOURCES.iter().enumerate().map(|(priority, source)| {
                                    let source_id = source.id;
                                    let is_expanded = move || expanded_source.get() == Some(source_id);

                                    view! {
                                        <div class="source-item">
                                            <div
                                                class="source-header"
                                                on:click=move |_| {
                                                    if is_expanded() {
                                                        set_expanded_source.set(None);
                                                    } else {
                                                        set_expanded_source.set(Some(source_id));
                                                        set_test_result.set(None);
                                                    }
                                                }
                                            >
                                                <span class="source-priority">{priority + 1}</span>
                                                <span class={move || format!("source-status {}", get_status(source_id).class())}>
                                                    {move || get_status(source_id).icon()}
                                                </span>
                                                <div class="source-info">
                                                    <span class="source-name">{source.name}</span>
                                                    <span class="source-desc">{source.description}</span>
                                                </div>
                                                <span class="source-expand">{move || if is_expanded() { "▼" } else { "▶" }}</span>
                                            </div>

                                            <Show when=is_expanded>
                                                <div class="source-details">
                                                    {if !source.needs_account {
                                                        view! {
                                                            <div class="source-no-setup">
                                                                <p class="setup-instructions">{source.signup_instructions}</p>
                                                                <div class="source-status-badge source-ready">
                                                                    "Ready to use - no configuration needed"
                                                                </div>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <div class="source-setup">
                                                                <pre class="setup-instructions">{source.signup_instructions}</pre>

                                                                {source.signup_url.map(|url| view! {
                                                                    <a
                                                                        href=url
                                                                        target="_blank"
                                                                        class="signup-link"
                                                                    >
                                                                        "Open signup page →"
                                                                    </a>
                                                                })}

                                                                <div class="source-inputs">
                                                                    {match source_id {
                                                                        "steamgriddb" => view! {
                                                                            <label class="wizard-label">
                                                                                "API Key"
                                                                                <input
                                                                                    type="password"
                                                                                    class="wizard-input"
                                                                                    placeholder="Your SteamGridDB API key"
                                                                                    prop:value=move || sgdb_api_key.get()
                                                                                    on:input=move |ev| set_sgdb_api_key.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <button
                                                                                class="test-btn"
                                                                                on:click=test_steamgriddb
                                                                                disabled=move || testing.get().is_some()
                                                                            >
                                                                                {move || if testing.get() == Some("steamgriddb") { "Testing..." } else { "Test Connection" }}
                                                                            </button>
                                                                        }.into_any(),
                                                                        "igdb" => view! {
                                                                            <label class="wizard-label">
                                                                                "Twitch Client ID"
                                                                                <input
                                                                                    type="text"
                                                                                    class="wizard-input"
                                                                                    placeholder="Your Twitch Client ID"
                                                                                    prop:value=move || igdb_client_id.get()
                                                                                    on:input=move |ev| set_igdb_client_id.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <label class="wizard-label">
                                                                                "Twitch Client Secret"
                                                                                <input
                                                                                    type="password"
                                                                                    class="wizard-input"
                                                                                    placeholder="Your Twitch Client Secret"
                                                                                    prop:value=move || igdb_client_secret.get()
                                                                                    on:input=move |ev| set_igdb_client_secret.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <button
                                                                                class="test-btn"
                                                                                on:click=test_igdb
                                                                                disabled=move || testing.get().is_some()
                                                                            >
                                                                                {move || if testing.get() == Some("igdb") { "Testing..." } else { "Test Connection" }}
                                                                            </button>
                                                                        }.into_any(),
                                                                        "emumovies" => view! {
                                                                            <label class="wizard-label">
                                                                                "Username"
                                                                                <input
                                                                                    type="text"
                                                                                    class="wizard-input"
                                                                                    placeholder="Your EmuMovies username"
                                                                                    prop:value=move || em_username.get()
                                                                                    on:input=move |ev| set_em_username.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <label class="wizard-label">
                                                                                "Password"
                                                                                <input
                                                                                    type="password"
                                                                                    class="wizard-input"
                                                                                    placeholder="Your EmuMovies password"
                                                                                    prop:value=move || em_password.get()
                                                                                    on:input=move |ev| set_em_password.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <button
                                                                                class="test-btn"
                                                                                on:click=test_emumovies
                                                                                disabled=move || testing.get().is_some()
                                                                            >
                                                                                {move || if testing.get() == Some("emumovies") { "Testing..." } else { "Test Connection" }}
                                                                            </button>
                                                                        }.into_any(),
                                                                        "screenscraper" => view! {
                                                                            <label class="wizard-label">
                                                                                "Developer ID"
                                                                                <input
                                                                                    type="text"
                                                                                    class="wizard-input"
                                                                                    placeholder="ScreenScraper Developer ID"
                                                                                    prop:value=move || ss_dev_id.get()
                                                                                    on:input=move |ev| set_ss_dev_id.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <label class="wizard-label">
                                                                                "Developer Password"
                                                                                <input
                                                                                    type="password"
                                                                                    class="wizard-input"
                                                                                    placeholder="ScreenScraper Developer Password"
                                                                                    prop:value=move || ss_dev_password.get()
                                                                                    on:input=move |ev| set_ss_dev_password.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <label class="wizard-label">
                                                                                "User ID (optional)"
                                                                                <input
                                                                                    type="text"
                                                                                    class="wizard-input"
                                                                                    placeholder="For higher rate limits"
                                                                                    prop:value=move || ss_user_id.get()
                                                                                    on:input=move |ev| set_ss_user_id.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <label class="wizard-label">
                                                                                "User Password (optional)"
                                                                                <input
                                                                                    type="password"
                                                                                    class="wizard-input"
                                                                                    placeholder="For higher rate limits"
                                                                                    prop:value=move || ss_user_password.get()
                                                                                    on:input=move |ev| set_ss_user_password.set(event_target_value(&ev))
                                                                                    on:blur=move |_| save_current_settings()
                                                                                />
                                                                            </label>
                                                                            <button
                                                                                class="test-btn"
                                                                                on:click=test_screenscraper
                                                                                disabled=move || testing.get().is_some()
                                                                            >
                                                                                {move || if testing.get() == Some("screenscraper") { "Testing..." } else { "Test Connection" }}
                                                                            </button>
                                                                        }.into_any(),
                                                                        _ => view! { <div></div> }.into_any(),
                                                                    }}
                                                                </div>

                                                                <Show when=move || test_result.get().is_some() && expanded_source.get() == Some(source_id)>
                                                                    <div class={move || if test_result.get().map(|(ok, _)| ok).unwrap_or(false) { "test-result test-success" } else { "test-result test-failure" }}>
                                                                        {move || test_result.get().map(|(_, msg)| msg).unwrap_or_default()}
                                                                    </div>
                                                                </Show>
                                                            </div>
                                                        }.into_any()
                                                    }}
                                                </div>
                                            </Show>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>

                            <Show when=move || save_error.get().is_some()>
                                <div class="wizard-error">
                                    {move || save_error.get().unwrap_or_default()}
                                </div>
                            </Show>

                            <Show when=move || saving.get()>
                                <div class="wizard-saving">"Saving..."</div>
                            </Show>
                        </div>

                        <div class="wizard-footer">
                            <div class="wizard-summary">
                                <span class="summary-item">
                                    <span class="source-status source-status-ok">"✓"</span>
                                    {move || {
                                        let mut count = 2; // launchbox + libretro always available
                                        if steamgriddb_status.get() == SourceStatus::Configured { count += 1; }
                                        if igdb_status.get() == SourceStatus::Configured { count += 1; }
                                        if emumovies_status.get() == SourceStatus::Configured { count += 1; }
                                        if screenscraper_status.get() == SourceStatus::Configured { count += 1; }
                                        format!("{} sources ready", count)
                                    }}
                                </span>
                            </div>
                            <button class="wizard-done-btn" on:click=move |_| on_close.set(false)>
                                "Done"
                            </button>
                        </div>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
