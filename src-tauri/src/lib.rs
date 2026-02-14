pub mod api;
mod commands;
pub mod db;
pub mod emulator;
pub mod endpoints;
pub mod handlers;
pub mod images;
pub mod import;
pub mod keyring_store;
pub mod logging;
pub mod router;
pub mod scanner;
pub mod scraper;
pub mod state;
pub mod tags;

use router::Ctx;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging with rolling file appender
    // Keep the guard alive for the duration of the app
    let _log_guard = logging::init_logging();

    // Build rspc router (shared between Tauri IPC and HTTP)
    let rspc_router = router::build_router();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        // rspc plugin for Tauri IPC
        .plugin(rspc_tauri::plugin(rspc_router.clone(), |_| Ctx {
            state: Arc::new(RwLock::new(AppState::default())),
        }))
        .manage(Arc::new(RwLock::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_platforms,
            commands::get_all_regions,
            commands::get_game_count,
            commands::get_games,
            commands::get_game_by_id,
            commands::get_game_by_uuid,
            commands::get_game_variants,
            commands::get_settings,
            commands::get_credential_storage_name,
            commands::save_settings,
            commands::scrape_rom,
            commands::get_collections,
            commands::create_collection,
            commands::update_collection,
            commands::delete_collection,
            commands::get_collection_games,
            commands::add_game_to_collection,
            commands::remove_game_from_collection,
            commands::record_play_session,
            commands::get_play_stats,
            commands::get_recent_games,
            commands::get_most_played,
            commands::add_favorite,
            commands::remove_favorite,
            commands::is_favorite,
            commands::get_favorites,
            commands::test_screenscraper_connection,
            commands::test_steamgriddb_connection,
            commands::test_igdb_connection,
            commands::test_emumovies_connection,
            // Image commands
            commands::get_game_images,
            commands::get_game_image,
            commands::get_available_image_types,
            commands::download_image,
            commands::download_game_images,
            commands::get_image_cache_stats,
            commands::check_cached_media,
            commands::download_image_with_fallback,
            commands::download_libretro_thumbnail,
            // Unified media download commands
            commands::get_media_types,
            commands::download_unified_media,
            commands::get_cached_media_path,
            // Video download commands
            commands::check_cached_video,
            commands::download_game_video,
            // Emulator commands
            commands::get_emulators_for_platform,
            commands::get_emulator,
            commands::get_all_emulators,
            // Emulator preference commands
            commands::get_emulator_preference,
            commands::set_game_emulator_preference,
            commands::set_platform_emulator_preference,
            commands::clear_game_emulator_preference,
            commands::clear_platform_emulator_preference,
            commands::get_all_emulator_preferences,
            commands::clear_all_emulator_preferences,
            // Emulator installation and launch commands
            commands::get_emulators_with_status,
            commands::install_emulator,
            commands::launch_emulator,
            commands::launch_game,
            commands::get_current_os,
            // Graboid import commands
            commands::get_game_file,
            commands::get_active_import,
            commands::start_graboid_import,
            commands::cancel_import,
            commands::test_graboid_connection,
            commands::get_graboid_prompts,
            commands::save_graboid_prompt,
            commands::delete_graboid_prompt,
            commands::get_effective_graboid_prompt,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = state::initialize_app_state(&handle).await {
                    tracing::error!("Failed to initialize app state: {}", e);
                }
            });

            // Start HTTP API server in dev mode for browser-based development
            #[cfg(debug_assertions)]
            {
                let state = app.state::<Arc<RwLock<AppState>>>().inner().clone();
                let rspc_router_for_http = rspc_router.clone();
                tauri::async_runtime::spawn(async move {
                    // Combine legacy API routes with rspc routes
                    let legacy_router = api::create_router(state.clone());

                    // Create rspc Axum endpoint
                    let rspc_ctx = Ctx { state };
                    let rspc_axum_router = rspc_axum::endpoint(rspc_router_for_http, move || rspc_ctx.clone());

                    // Merge routers - rspc at /rspc, legacy at /api
                    let combined_router = axum::Router::new()
                        .nest("/rspc", rspc_axum_router)
                        .merge(legacy_router);

                    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
                        .await
                        .expect("Failed to bind HTTP API server");
                    tracing::info!("HTTP API server running on http://127.0.0.1:3001");
                    tracing::info!("  - rspc routes: http://127.0.0.1:3001/rspc");
                    tracing::info!("  - legacy routes: http://127.0.0.1:3001/api");
                    axum::serve(listener, combined_router)
                        .await
                        .expect("HTTP API server error");
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
