//! Endpoint registration macros
//!
//! These macros ensure Tauri commands and HTTP handlers stay in sync.
//! The actual logic lives in handlers.rs.

use axum::Json;
use serde::Serialize;

// ============================================================================
// rspc-style JSON-RPC response types (for HTTP API)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct RspcResponse<T: Serialize> {
    pub result: RspcResult<T>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum RspcResult<T: Serialize> {
    #[serde(rename = "response")]
    Response(T),
    #[serde(rename = "error")]
    Error { code: i32, message: String },
}

pub fn rspc_ok<T: Serialize>(data: T) -> Json<RspcResponse<T>> {
    Json(RspcResponse {
        result: RspcResult::Response(data),
    })
}

pub fn rspc_err<T: Serialize>(message: String) -> Json<RspcResponse<T>> {
    Json(RspcResponse {
        result: RspcResult::Error { code: -1, message },
    })
}

/// Macro to define a Tauri command that calls a handler
///
/// Usage:
/// ```ignore
/// tauri_command!(get_collections, handlers::get_collections);
/// tauri_command!(create_collection, handlers::create_collection, CreateCollectionInput);
/// ```
#[macro_export]
macro_rules! tauri_command {
    // No input version
    ($name:ident, $handler:path) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<$crate::state::AppState>>>,
        ) -> Result<impl serde::Serialize, String> {
            let state_guard = state.read().await;
            $handler(&state_guard).await
        }
    };
    // With input version
    ($name:ident, $handler:path, $input:ty) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<$crate::state::AppState>>>,
            input: $input,
        ) -> Result<impl serde::Serialize, String> {
            let state_guard = state.read().await;
            $handler(&state_guard, input).await
        }
    };
}

/// Macro to define an HTTP handler that calls a handler
///
/// Usage:
/// ```ignore
/// http_handler!(rspc_get_collections, handlers::get_collections, Vec<Collection>);
/// http_handler!(rspc_create_collection, handlers::create_collection, Collection, CreateCollectionInput);
/// ```
#[macro_export]
macro_rules! http_handler {
    // No input version
    ($name:ident, $handler:path, $output:ty) => {
        pub async fn $name(
            axum::extract::State(state): axum::extract::State<SharedState>,
        ) -> impl axum::response::IntoResponse {
            use axum::response::IntoResponse;
            let state_guard = state.read().await;
            match $handler(&state_guard).await {
                Ok(data) => $crate::endpoints::rspc_ok(data).into_response(),
                Err(e) => $crate::endpoints::rspc_err::<$output>(e).into_response(),
            }
        }
    };
    // With input version
    ($name:ident, $handler:path, $output:ty, $input:ty) => {
        pub async fn $name(
            axum::extract::State(state): axum::extract::State<SharedState>,
            axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
        ) -> impl axum::response::IntoResponse {
            use axum::response::IntoResponse;

            let input: $input = match params.get("input") {
                Some(s) => match serde_json::from_str(s) {
                    Ok(i) => i,
                    Err(e) => return $crate::endpoints::rspc_err::<$output>(format!("Invalid input: {}", e)).into_response(),
                },
                None => return $crate::endpoints::rspc_err::<$output>("Missing 'input' parameter".to_string()).into_response(),
            };

            let state_guard = state.read().await;
            match $handler(&state_guard, input).await {
                Ok(data) => $crate::endpoints::rspc_ok(data).into_response(),
                Err(e) => $crate::endpoints::rspc_err::<$output>(e).into_response(),
            }
        }
    };
}

pub use tauri_command;
pub use http_handler;
