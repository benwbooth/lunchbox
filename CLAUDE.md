# Project Guidelines

## Code Style

- Keep code straightforward - don't go crazy with abstractions
- Refactor to maintain clean code standards when it makes sense
- Never put stubs in to be done later - just do the thing
- No TODOs, no "coming in a future update" - implement it now or break it into smaller steps
- Keep main.rs thin - modularize command implementations into separate files
- CLI commands should call into module functions, not contain all the logic inline
- **Never enumerate struct fields in multiple places** - if adding a field requires changes in 5 places, refactor to use iteration, derive macros, or serde. Define fields once.

## Workflow

- After implementing a change, always git status/diff/commit/push as a WIP commit
- Don't wait for the user to ask for a commit

## Tools

- Never use sed - always use the Edit tool for file modifications
- Don't use sed for normal reading/writing files - use Read/Write/Edit tools instead
- Never run `cargo clean` or similar destructive clean commands - they waste time rebuilding

## Database

- `games.db` - Game metadata (games, platforms, images, etc.)
- `lunchbox.db` - User data (settings, play stats, favorites, collections, etc.)
