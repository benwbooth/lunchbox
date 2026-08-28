# Native frontend parity roadmap

The native frontend will absorb every useful capability from Lunchbox's own
Electron implementation and then add the collection-management depth expected
from a polished desktop emulator frontend. The work is organized as complete
vertical slices rather than a reduced "first release".

The Electron source remains intact on `legacy/electron`. It is a behavioral
reference for Lunchbox-owned features, not a runtime dependency.

The source-backed inventory in [the legacy feature parity
ledger](LEGACY_FEATURE_PARITY.md) is the completeness checklist for this work.

## Completed foundation

- Native Qt 6 Quick window compiled into the executable.
- Rust `QAbstractListModel` with virtualized grid and list presentations.
- Asynchronous, read-only schema-v3 catalog loading.
- Asynchronous search, platform, local, and downloadable filtering with stale
  result rejection.
- Native filter menu for installed and Minerva-covered/not-installed games plus
  the legacy non-retail and adult classifications, with persistent content
  preferences and composability with search/platform selection.
- Read-only layering of the preserved discovery catalog, Minerva bundle
  coverage, and legacy installed-game state. The Minerva filter contains only
  covered games that are not installed and composes with platform selection.
- Responsive game selection and details with real catalog metadata, exact-name
  Minerva bundle resolution, bounded off-thread torrent inspection, and
  explicitly review-only title candidates. Candidate ranking strips catalog
  region/version tags before title comparison, normalizes common Roman-numeral
  sequels, then applies persisted region and latest/original-revision
  preferences only between equally strong title matches. The complete legacy
  region ordering is native, durable, editable with keyboard-focusable controls,
  and alias-normalized. The exact variant is always visible before download.
- Generic multi-disc optical downloads are represented as a single durable plan:
  numeric/alphabetic/Roman disc markers, primary-image selection, common CUE,
  CCD, MDS and GDI sidecars, exact-member confirmation, aggregate progress,
  preserved relative layout, safe retry, and last-step M3U publication are
  native.
- eXoDOS, eXoWin3x, and eXoWin9x related archives are represented as one
  durable acquisition plan with exact game-data/metadata/utility dependencies,
  legacy DOS language priority, role-aware confirmation, aggregate progress,
  a shared per-torrent archive cache, idempotent staging, and exact-primary-only
  collection identity. Prepared extraction is also native: stable-ID cache
  records, exact title metadata, nested shared utilities, safe ZIP paths,
  cooperative cancellation, atomic publication, reuse, and details-panel
  status/retry behavior are complete. Emulator launch remains separate
  emulator-management work.
- MAME, Hypseus Singe, and Daphne views of Minerva's shared Laserdisc
  Collection produce strict machine-layout plans. Canonical LibRetro MAME
  records provide exact-title set names; plans require the complete ROM/CHD or
  ROM/framefile/data/video/audio bundle, expose every member for review, select
  only those torrent files, and ingest into launchable MAME or
  Hypseus-compatible paths. Valid metadata has a bounded OS-native disk cache,
  while one decoded file index is reused across the active source views.
- Native qBittorrent settings and connection testing, operating-system
  credential storage, exact reviewed-file priority, optional whole-torrent
  mode, a restart-safe queue with pause/resume/cancel, and verified completed
  file ingestion through configurable link/copy policies. The native seeding
  policy can follow qBittorrent or durably pause a Lunchbox-owned torrent after
  all selected bundle members import, with automatic retry and no data removal.
- Unified native Find Artwork workflow for SteamGridDB and IGDB with
  provider-specific media categories, Twitch client-credentials authentication,
  operating-system credential storage, explicit exact-game review, durable
  stable provider links, individual artwork review, and one shared bounded,
  signature-validating, atomic publisher. IGDB commercial use remains gated on
  the distributor obtaining the required provider partnership.
- Native local-ROM folder scan with a platform selector, extension-aware
  discovery, optional SHA-1/MD5 identity verification, cancellable streaming
  progress, searchable/sortable review, bulk/per-file selection, idempotent
  persistence, lossless native paths, and immediate My Collection/Minerva
  availability updates. ZIP and 7z members are reviewed and hashed independently;
  multi-member archives select nothing automatically, while explicit member
  imports are digest-revalidated and materialized atomically with exact source
  provenance. Unmatched and ambiguous files remain local-only unless the user
  explicitly links one through canonical/alternate-title search; reviewed links
  restore only against unchanged path, archive member, size, MD5, and SHA-1.
- Stable-UUID favorites plus manual and smart collections with native
  create/edit/delete, sidebar counts and navigation, exact per-game membership,
  preserved manual order, confirmed bulk visible-result changes, composable
  automatic rules, portable exact-ID import/export, asynchronous persistence
  with rollback, and restart recovery.
- Native local Library Audit with one prepared catalog index, cancellable
  off-thread scans, exact healthy/changed/duplicate/new/missing/unreadable/root-
  offline states, review-first filtering and sorting, and confirmed stale-row
  cleanup. Availability is synchronized only for roots that completed a full
  scan; cleanup cannot delete files and offline roots are never treated as
  missing.
- Stable-UUID play activity with durable per-launch sessions, exactly-once
  elapsed-time finalization, count/time/last-played statistics, editable
  completion state, and a live Recently Played view.
- Linux Nix package plus host-neutral Rust path handling for Linux, macOS, and
  Windows.
- Native Emulator Manager with host-scoped exact package identities, off-thread
  detection and lifecycle work, Linux Flatpak/isolated Nix/AppImage/GitHub/Wine
  transports, Windows winget, macOS Homebrew, ownership-safe uninstall, and
  exact host-specific Libretro core management. The Linux discovery/UI probe is
  exercised; package mutations and Windows/macOS behavior retain release-host
  gates.
- Measurable shell-ready and catalog-ready probes.
- A native **Couch Mode** browse/play slice backed by the same Rust catalog,
  details, media, favorite, filter, download, and launch services as desktop
  mode. It restores the exact selected stable UUID, virtualizes the horizontal
  cover shelf, works without a mouse by keyboard, enters real fullscreen, and
  returns to the complete desktop management surface without duplicating state.
- Native management of Libretro's complete official Slang and GLSL shader packs
  with cross-platform target discovery, verified cache reuse, safe extraction,
  explicit unmanaged-pack replacement, and transactional multi-target rollback.

## Collection management

- Extend the implemented folder chooser, platform assignment, scan preview,
  progress, cancellation, exact/ambiguous reporting, idempotent rescans, exact
  duplicate/change audit, and missing-file management view with named extension
  presets and resumable hash caches.
- Extend native ZIP/7z/RAR member inspection and manual matching review with
  multidisc grouping, patches, and alternate-version management.
- Extend the implemented stable-UUID game metadata overlay editor with release-
  specific editing, tags, custom fields, playlist-specific presentation, and a
  richer session-history inspector beyond the implemented activity summary.
- Non-destructive merge/split/relink tools backed by the database identity event
  log.
- Fast metadata bulk editing, collection audits, and whole-profile backup/restore.

## Download-first experience

- Indexed, paged Minerva search without loading the multi-gigabyte hash catalog
  into the GUI process.
- Add provider-neutral, review-first acquisition intake: watched-folder and
  manual `.torrent` import, magnet import, then authorized catalog adapters.
  External offers retain source provenance and never establish or merge a
  canonical game identity. A direct NSW Torrent Library adapter is not planned
  unless its operators provide documented rights, terms, and an authorized API;
  Lunchbox will not bundle a Telegram account session, scrape bots, or automate
  blocking/proxy evasion. See `EXTERNAL_TORRENT_PROVIDERS.md`.
- Complete game-level offer selection beyond the implemented exact-file,
  whole-torrent, generic optical-set, eXo archive-set, and arcade-laserdisc
  paths: free-space enforcement and broader duplicate avoidance. Reuse the
  implemented full region-order editor when variant-rich catalog browsing lands.
- Extend the implemented persistent qBittorrent queue, aggregate speed, safe
  record/history cleanup, and durable post-import pause policy with explicit
  retry/recovery controls and richer error history.
- Extend the implemented runtime-scoped firmware inventory, exact Minerva and
  official-source acquisition, local import, per-file integrity manifests, and
  repair workflow with a whole-library missing-firmware audit and native
  Windows/macOS release-host verification.

## Metadata and media

- Canonical-game metadata enrichment with provenance and per-field conflict
  review.
- Extend the implemented exact Minerva manual acquisition, durable visible
  progress/cancellation, validated atomic cache publication, and native cached
  manual/video bundle with an authorized EmuMovies API adapter, video
  acquisition, and soundtrack browsing.
- Whole-library missing-media audit and a background bulk download queue.
  Durable media-source priority, per-game candidate rotation, immediate
  off-thread reindexing, and safe explicit LibRetro replacement are native.
- Extend the responsive game details and native video/fullscreen/manual/3D-box
  surface with related games, alternate versions, and soundtrack playback.

## Emulators and launching

- Host-aware canonical-catalog discovery and prepared eXo process launch are
  native for Linux, macOS, and Windows path models. Linux Flatpak launch is
  runtime-verified; Windows and macOS still need native release-host gates.
- eXoDOS/eXoWin3x DOSBox and ScummVM exception plans plus eXoWin9x DOSBox-X,
  86Box, and PCBox-compatible profiles are native. Writable VM child disks and
  process lifecycle reporting are included.
- Generic local-ROM RetroArch and standalone discovery/launch are native, with
  multi-file selection and exact per-game/platform stable-ID/core defaults.
  Linux Flatpak RetroArch plus real core/content loading is runtime-verified.
- Exact game/platform/all-platform launch profiles are native for ordinary ROM
  and prepared eXo plans. Their contextual editor exposes independently
  inherited extra arguments and replacement templates; the bounded compiler
  preserves native/Flatpak/Wine paths and never invokes a shell. The native
  Launch Commands manager adds a worker-loaded, virtualized, searchable matrix
  for exact global/platform standalone and RetroArch-core rows with
  built-in/customized filtering and focused save/clear. Complete Windows/macOS
  runtime-host certification.
- Standalone MAME set-name/ROM-path and Hypseus framefile/support/bundle
  profiles are native. Derived Arcade Laserdisc and Arcade Pinball labels map
  to the canonical Arcade emulator catalog without being collapsed in the UI.
  Linux Flatpak MAME launch is runtime-verified with the ROM-free Pong driver.
  Known archive-oriented FinalBurn Neo, Flycast, and Supermodel executables
  retain the legacy bounded direct-file contract.
- Add explicit version pinning, lifecycle cancellation/recovery, and command
  preview to the native manager. Available-version-only presentation is native:
  it checks every exact installed backend, presents current-to-available
  versions, and applies explicitly selected rows sequentially with per-row
  outcomes. Firmware/BIOS validation before game launch is also native.
- Dedicated Daphne runtime, TeknoParrot, and non-eXo PC launch profiles.
- Finish the native controller slice with attached-hardware InputPlumber
  mutation/restore certification, Windows and macOS host adapters, pre-launch
  validation presentation, pause/resume integration, and controller-first
  focus. Linux inventory, durable per-player action/profile/target/order state,
  the visual custom-profile designer, pre-spawn activation, RAII restoration,
  process supervision, and play-stat recording are native.

## Couch Mode

- Extend the implemented native Qt Quick Couch Mode browse/play presentation,
  which already shares the Rust models and services rather than creating a
  second database or duplicated backend.
- Theme packages, platform/game wheels, details and game menus, attract mode,
  startup/shutdown/pause screens, screensaver, background music, sound packs,
  marquee monitor support, parental lock, and couch-safe settings.
- Complete keyboard and gamepad navigation with deterministic focus restoration
  and no mouse requirement.

## Definition of done for every slice

A slice is complete only when its UI is functional against real data, expensive
work is cancellable and off the GUI thread, errors are actionable, keyboard and
controller behavior is defined, state survives restart, unit and runtime tests
cover the critical path, and performance is measured in a release build. Empty
buttons, placeholder services, silent no-ops, and compile-only claims do not
count as progress.
