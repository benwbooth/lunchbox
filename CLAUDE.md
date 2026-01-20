# Project Guidelines

## Code Style

- Keep code straightforward - don't go crazy with abstractions
- Refactor to maintain clean code standards when it makes sense
- Never put stubs in to be done later - just do the thing
- No TODOs, no "coming in a future update", no "not yet implemented" - implement it now or break it into smaller steps
- Never add a TODO and leave it - if something needs to be done, do it immediately
- Never bail out with "not implemented" errors - if you add a code path, implement it fully
- Keep main.rs thin - modularize command implementations into separate files
- CLI commands should call into module functions, not contain all the logic inline
- **Never enumerate struct fields in multiple places** - if adding a field requires changes in 5 places, refactor to use iteration, derive macros, or serde. Define fields once.

## Workflow

- After implementing a change, always git status/diff/commit/push as a WIP commit
- Don't wait for the user to ask for a commit
- Run the app with `./scripts/dev.sh` to test changes

## Tools

- Never use sed - always use the Edit tool for file modifications
- Don't use sed for normal reading/writing files - use Read/Write/Edit tools instead
- Never run `cargo clean` or similar destructive clean commands - they waste time rebuilding
- Use `uv run python` instead of `python` or `python3`
- Always use `./scripts/dev.sh` to start the dev server - never run cargo commands directly for the backend

## API Endpoints

Backend endpoints must work in both Tauri native mode and HTTP dev mode. To ensure consistency:

1. **Define shared types and logic in `handlers.rs`**
   - Input structs (e.g., `CreateCollectionInput`)
   - Output structs (e.g., `Collection`)
   - Handler functions that take `&AppState` and return `Result<T, String>`

2. **Create thin wrappers in both:**
   - `commands.rs` - Tauri commands that extract state and call handlers
   - `api.rs` - HTTP handlers that parse JSON input and call handlers

3. **Register in `lib.rs`** - Add the command to `invoke_handler`
4. **Register in `api.rs`** - Add the route to `create_router`

Example pattern:
```rust
// handlers.rs - THE SOURCE OF TRUTH
pub async fn get_collections(state: &AppState) -> Result<Vec<Collection>, String> { ... }

// commands.rs - thin wrapper
#[tauri::command]
pub async fn get_collections(state: tauri::State<'_, AppStateHandle>) -> Result<Vec<Collection>, String> {
    let state_guard = state.read().await;
    handlers::get_collections(&state_guard).await
}

// api.rs - thin wrapper with JSON handling
async fn rspc_get_collections(State(state): State<SharedState>) -> impl IntoResponse {
    let state_guard = state.read().await;
    match handlers::get_collections(&state_guard).await {
        Ok(collections) => rspc_ok(collections).into_response(),
        Err(e) => rspc_err::<Vec<Collection>>(e).into_response(),
    }
}
```

This ensures business logic is defined once in `handlers.rs`, preventing Tauri/HTTP drift.

## Database

- **Games database**: `~/.local/share/lunchbox/games.db` (read-only)
  - Contains: games, platforms, game_alternate_names
  - Created by the unified import CLI tool (`lunchbox-cli unified-build`)
- **User database**: `~/.local/share/lunchbox/user.db` (created on first write)
  - Contains: settings, favorites, collections, play_sessions
  - Only created when user saves data (no empty files)
- **IMPORTANT**: Never rebuild the games database without asking first - it takes a long time
