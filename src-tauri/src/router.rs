//! rspc router - unified API for both Tauri IPC and HTTP
//!
//! All procedures defined here are automatically available via:
//! - Tauri IPC (when running as desktop app)
//! - HTTP API (when running dev server)

use rspc::{Config, Router};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;

use crate::images::{EmuMoviesClient, EmuMoviesConfig};
use crate::scraper::{
    IGDBClient, IGDBConfig, ScreenScraperClient, ScreenScraperConfig, SteamGridDBClient,
    SteamGridDBConfig,
};
use crate::state::AppState;

/// Shared context for all rspc procedures
#[derive(Clone)]
pub struct Ctx {
    pub state: Arc<tokio::sync::RwLock<AppState>>,
}

/// Result of testing a connection to an image source
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    #[specta(optional)]
    pub user_info: Option<String>,
}

/// Build the rspc router with all procedures
pub fn build_router() -> Arc<Router<Ctx>> {
    Router::<Ctx>::new()
        .config(Config::new())
        // Health check
        .query("health", |t| t(|_ctx, _: ()| Ok("ok".to_string())))
        // Test connection procedures
        .query("test_steamgriddb_connection", |t| {
            #[derive(Debug, Deserialize, Type)]
            struct Args {
                api_key: String,
            }
            t(|_ctx, args: Args| async move {
                let result = test_steamgriddb_impl(args.api_key).await;
                Ok(result)
            })
        })
        .query("test_igdb_connection", |t| {
            #[derive(Debug, Deserialize, Type)]
            struct Args {
                client_id: String,
                client_secret: String,
            }
            t(|_ctx, args: Args| async move {
                let result = test_igdb_impl(args.client_id, args.client_secret).await;
                Ok(result)
            })
        })
        .query("test_emumovies_connection", |t| {
            #[derive(Debug, Deserialize, Type)]
            struct Args {
                username: String,
                password: String,
            }
            t(|_ctx, args: Args| async move {
                let result = test_emumovies_impl(args.username, args.password).await;
                Ok(result)
            })
        })
        .query("test_screenscraper_connection", |t| {
            #[derive(Debug, Deserialize, Type)]
            struct Args {
                dev_id: String,
                dev_password: String,
                #[specta(optional)]
                user_id: Option<String>,
                #[specta(optional)]
                user_password: Option<String>,
            }
            t(|_ctx, args: Args| async move {
                let result = test_screenscraper_impl(
                    args.dev_id,
                    args.dev_password,
                    args.user_id,
                    args.user_password,
                )
                .await;
                Ok(result)
            })
        })
        .build()
        .arced()
}

// ============================================================================
// Implementation functions (shared between rspc and legacy Tauri commands)
// ============================================================================

pub async fn test_steamgriddb_impl(api_key: String) -> ConnectionTestResult {
    if api_key.is_empty() {
        return ConnectionTestResult {
            success: false,
            message: "API key is required".to_string(),
            user_info: None,
        };
    }

    let config = SteamGridDBConfig { api_key };
    let client = SteamGridDBClient::new(config);

    match client.test_connection().await {
        Ok(()) => ConnectionTestResult {
            success: true,
            message: "Successfully connected to SteamGridDB API".to_string(),
            user_info: None,
        },
        Err(e) => {
            let err_str = e.to_string();
            let message = if err_str.contains("401") || err_str.contains("403") {
                "Invalid API key. Please check your SteamGridDB API key.".to_string()
            } else {
                format!("Connection failed: {}", err_str)
            };
            ConnectionTestResult {
                success: false,
                message,
                user_info: None,
            }
        }
    }
}

pub async fn test_igdb_impl(client_id: String, client_secret: String) -> ConnectionTestResult {
    if client_id.is_empty() || client_secret.is_empty() {
        return ConnectionTestResult {
            success: false,
            message: "Client ID and Client Secret are required".to_string(),
            user_info: None,
        };
    }

    let config = IGDBConfig {
        client_id,
        client_secret,
    };
    let client = IGDBClient::new(config);

    match client.test_connection().await {
        Ok(found_game) => ConnectionTestResult {
            success: true,
            message: "Successfully connected to IGDB API".to_string(),
            user_info: Some(found_game),
        },
        Err(e) => {
            let err_str = e.to_string();
            let message =
                if err_str.contains("401") || err_str.contains("403") || err_str.contains("invalid")
                {
                    "Invalid credentials. Please check your Twitch Client ID and Secret.".to_string()
                } else {
                    format!("Connection failed: {}", err_str)
                };
            ConnectionTestResult {
                success: false,
                message,
                user_info: None,
            }
        }
    }
}

pub async fn test_emumovies_impl(username: String, password: String) -> ConnectionTestResult {
    if username.is_empty() || password.is_empty() {
        return ConnectionTestResult {
            success: false,
            message: "Username and password are required".to_string(),
            user_info: None,
        };
    }

    let config = EmuMoviesConfig {
        username: username.clone(),
        password,
    };
    // Use a temp dir for the client since we're just testing
    let client = EmuMoviesClient::new(config, std::path::PathBuf::from("/tmp"));

    match client.test_connection().await {
        Ok(()) => ConnectionTestResult {
            success: true,
            message: "Successfully connected to EmuMovies API".to_string(),
            user_info: Some(format!("Logged in as: {}", username)),
        },
        Err(e) => {
            let err_str = e.to_string();
            let message = if err_str.contains("401") || err_str.contains("403") {
                "Invalid credentials. Please check your username and password.".to_string()
            } else {
                format!("Connection failed: {}", err_str)
            };
            ConnectionTestResult {
                success: false,
                message,
                user_info: None,
            }
        }
    }
}

pub async fn test_screenscraper_impl(
    dev_id: String,
    dev_password: String,
    user_id: Option<String>,
    user_password: Option<String>,
) -> ConnectionTestResult {
    if dev_id.is_empty() || dev_password.is_empty() {
        return ConnectionTestResult {
            success: false,
            message: "Developer ID and password are required".to_string(),
            user_info: None,
        };
    }

    let config = ScreenScraperConfig {
        dev_id,
        dev_password,
        user_id: user_id.clone(),
        user_password,
    };
    let client = ScreenScraperClient::new(config);

    // Test by looking up a well-known game (Super Mario Bros CRC)
    match client
        .lookup_by_checksum(
            "3337EC46", // CRC32 for Super Mario Bros (NES)
            "811B027E", // partial MD5
            "",
            40976,
            "Super Mario Bros.nes",
            Some(3), // NES platform ID
        )
        .await
    {
        Ok(_) => {
            let user_msg = user_id.map(|u| format!("Logged in as: {}", u));
            ConnectionTestResult {
                success: true,
                message: "Successfully connected to ScreenScraper API".to_string(),
                user_info: user_msg,
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("401") || err_str.contains("403") {
                ConnectionTestResult {
                    success: false,
                    message: "Invalid credentials. Please check your developer ID and password.".to_string(),
                    user_info: None,
                }
            } else if err_str.contains("429") {
                // Rate limited but connection works
                ConnectionTestResult {
                    success: true,
                    message: "Connected (rate limited). ScreenScraper connection works but you've hit the request limit.".to_string(),
                    user_info: user_id.clone().map(|u| format!("Logged in as: {}", u)),
                }
            } else {
                ConnectionTestResult {
                    success: false,
                    message: format!("Connection failed: {}", err_str),
                    user_info: None,
                }
            }
        }
    }
}
