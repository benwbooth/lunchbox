# Legacy Lunchbox feature parity ledger

The `legacy/electron` branch is the behavioral source for Lunchbox-owned
features. The native port does not embed its web frontend or HTTP server, but it
must retain the useful behavior behind them. This ledger is checked whenever a
native vertical slice is completed so that features are not silently lost.

## Catalog and collection experience

| Legacy behavior | Source | Native state |
| --- | --- | --- |
| Full discoverable catalog separated from writable user state | `backend/src/state.rs`, `backend/src/api.rs` | Implemented as read-only catalog layers |
| Platform sidebar, counts, search, grid/list views, zoom and artwork choice | `src/components/sidebar.rs`, `toolbar.rs`, `game_grid.rs` | Core sidebar/search/grid/list implemented; artwork and zoom remain in the ledger |
| Installed, non-retail and adult filters | `src/components/toolbar.rs`, `backend/src/api.rs` | Installed and Minerva-not-installed implemented; content filters remain in the ledger |
| Regional/version variants and alternate titles | `backend/src/api.rs`, `backend/src/db/schema.rs` | Behavior recorded |
| Favorites, playlists, normal collections and smart collections | `backend/migrations`, `src/components/sidebar.rs` | Behavior recorded |
| Play count, play time, last played, completion state and sessions | `backend/migrations`, `backend/src/handlers.rs` | Behavior recorded |
| ROM folder scan, checksum matching, import progress and cancellation | `backend/src/scanner`, `src/components/rom_import.rs`, `import_progress.rs` | Implemented with a native folder picker, optional exact checksums, platform assignment, searchable/sortable preview, bulk and per-row selection, cooperative cancellation, persistent lossless paths, and immediate collection refresh. Archive members and manual ambiguous-identity resolution remain in the ledger. |

## Minerva acquisition

| Legacy behavior | Source | Native state |
| --- | --- | --- |
| Detect Minerva catalog and resolve all bundles for a platform | `backend/src/handlers.rs` | Implemented with exact normalized mapped/provider names; Atari 800 and Arcade retain only their collection-bounded legacy rules. Cross-database numeric platform IDs are deliberately not joined. |
| Exclude already installed games by LaunchBox ID or stable game UUID | `backend/src/api.rs`, migration `0016_game_uid.sql` | Implemented in the native catalog layer |
| Fetch and cache torrent metadata, list files, rank title/region candidates | `backend/src/torrent/mod.rs`, `backend/src/handlers.rs` | Native details pane fetches bounded torrent metadata off-thread, caches the current bundle, lists real files, and ranks review candidates; persistent cache and region preferences remain in the ledger |
| Game-only and full-torrent modes | `backend/src/handlers.rs` | Implemented for reviewed exact-file selection and explicit whole-torrent mode |
| Related-file plans for multidisc, eXo, MAME laserdisc, Daphne and Hypseus | `backend/src/exo.rs`, `arcade.rs`, `handlers.rs` | Behavior recorded |
| qBittorrent Web API setup, connection testing and container path mapping | `backend/src/torrent/clients/qbittorrent.rs`, `state.rs` | Implemented with native settings, credential-store secrets, literal remote/container paths, bounded Web API requests, and an isolated `lunchbox` category |
| Queue progress, aggregate speed, pause, resume, cancel, delete and recent jobs | `src/components/minerva_download_queue.rs`, `backend/src/handlers.rs` | Persistent native queue, per-job progress, pause/resume, and non-destructive cancel implemented; aggregate speed, record deletion, seeding policy, and history cleanup remain in the ledger |
| Completion ingestion and symlink, hardlink, reflink, copy or leave-in-place policy | `backend/src/handlers.rs`, `state.rs` | Implemented with collision checks, idempotent recovery, exact installed identity, and cross-platform path handling |
| BIOS/firmware inventory and acquisition | `backend/src/firmware.rs`, settings UI | Behavior recorded |

## Game details and media

| Legacy behavior | Source | Native state |
| --- | --- | --- |
| Metadata, variants, notes, related actions and launch controls | `src/components/game_details.rs` | Native responsive details pane implements core metadata and Minerva source inspection; variants, editing, related actions and launch controls remain in the ledger |
| Box front, screenshots, title screens, fan art and clear logos | `backend/src/images`, `src/components/lazy_image.rs` | Behavior recorded |
| Source priority, fallback, rotation, redownload and missing-media state | `backend/src/images/source_selector.rs`, `download_service.rs` | Behavior recorded |
| Manuals, videos, video progress and local-file opening | `backend/src/api.rs`, `video_player.rs` | Behavior recorded |
| 3D box viewer | `src/components/box_3d_viewer.rs` | Behavior recorded |

## Emulators and launching

| Legacy behavior | Source | Native state |
| --- | --- | --- |
| Host-aware emulator catalog and detection | `backend/src/emulator.rs`, `db/emulators.db` | Reference catalog is loaded; host state remains in the ledger |
| Install, update, uninstall and launch | `backend/src/emulator.rs`, `src/components/emulator_updates.rs` | Behavior recorded |
| Platform and per-game emulator preferences | migrations `0008` and handler APIs | Behavior recorded |
| Runtime launch profiles and template overrides | migrations `0013` and `0014` | Behavior recorded |
| RetroArch cores, standalone emulators, DOS/eXo and arcade layouts | `backend/src/emulator.rs`, `exo.rs`, `arcade.rs` | Behavior recorded |
| Controller discovery, per-player mapping and InputPlumber profiles | `src/components/controller_mapping.rs`, `backend/src/controllers.rs` | Behavior recorded |
| Firmware validation before launch | `backend/src/firmware.rs` | Behavior recorded |

## Settings, platform support and presentation

| Legacy behavior | Source | Native state |
| --- | --- | --- |
| OS-native data/media/program/save directories | `backend/src/state.rs` | Native path handling established |
| Region priority and all provider/torrent settings | `src/components/settings.rs`, `backend/src/state.rs` | Behavior recorded |
| Credentials in the system keyring | `backend/src/keyring_store.rs` | Implemented with the operating system credential store; no password column exists in SQLite |
| Linux, macOS and Windows packaging | `.github/workflows/release.yml` | Nix Linux package works; native macOS/Windows runtime packaging remains in the ledger |
| Keyboard/controller-first full-screen presentation | Product requirement beyond the Electron desktop UI | Tracked in `FRONTEND_ROADMAP.md` |
| Loading minigame and GPU presentation | `src/components/minigame` | Behavior recorded |

## Port rule

A row moves to implemented only with real service behavior, native Qt UI,
persistent state where applicable, error and cancellation handling, tests, and a
measured release runtime. A disabled control, empty dialog, compile-only bridge,
or silent no-op never counts as parity.
