# Project Guidelines

## Code Style

- Keep code straightforward - don't go crazy with abstractions
- Refactor to maintain clean code standards when it makes sense
- Never put stubs in to be done later - just do the thing
- No TODOs, no "coming in a future update" - implement it now or break it into smaller steps
- Keep main.rs thin - modularize command implementations into separate files
- CLI commands should call into module functions, not contain all the logic inline

## Tools

- Never use sed - always use the Edit tool for file modifications
