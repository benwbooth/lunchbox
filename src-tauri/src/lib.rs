pub mod api;
mod commands;
pub mod db;
pub mod import;
pub mod scanner;
pub mod scraper;
pub mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Arc::new(RwLock::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::scan_roms,
            commands::get_platforms,
            commands::get_game_count,
            commands::get_games,
            commands::get_game_by_id,
            commands::get_game_by_uuid,
            commands::get_game_variants,
            commands::import_launchbox,
            commands::launch_game,
            commands::get_settings,
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
        ])
        .setup(|app| {
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
                tauri::async_runtime::spawn(async move {
                    let router = api::create_router(state);
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
                        .await
                        .expect("Failed to bind HTTP API server");
                    tracing::info!("HTTP API server running on http://127.0.0.1:3001");
                    axum::serve(listener, router)
                        .await
                        .expect("HTTP API server error");
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
