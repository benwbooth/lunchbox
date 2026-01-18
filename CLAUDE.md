# Project Guidelines

## Code Style

- Keep code straightforward - don't go crazy with abstractions
- Refactor to maintain clean code standards when it makes sense
- Never put stubs in to be done later - just do the thing
- No TODOs, no "coming in a future update", no "not yet implemented" - implement it now or break it into smaller steps
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

## API Endpoints

- All backend endpoints must be added to BOTH:
  - `src-tauri/src/commands.rs` (Tauri IPC for native mode)
  - `src-tauri/src/api.rs` (HTTP API for browser dev mode)
  - `src-tauri/src/lib.rs` (register the command)
- Never add an endpoint to only one place - both must stay in sync
- Frontend uses rspc-style calls that work with both backends

## Database

- **Games database**: `~/.local/share/lunchbox/games.db` (read-only)
  - Contains: games, platforms, game_alternate_names
  - Created by the unified import CLI tool (`lunchbox-cli unified-build`)
- **User database**: `~/.local/share/lunchbox/user.db` (created on first write)
  - Contains: settings, favorites, collections, play_sessions
  - Only created when user saves data (no empty files)
- **IMPORTANT**: Never rebuild the games database without asking first - it takes a long time
