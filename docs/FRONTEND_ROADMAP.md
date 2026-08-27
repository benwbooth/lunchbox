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
  preferences only between equally strong title matches. The exact variant is
  always visible before download.
- Generic multi-disc optical downloads are represented as a single durable plan:
  numeric/alphabetic/Roman disc markers, primary-image selection, common CUE,
  CCD, MDS and GDI sidecars, exact-member confirmation, aggregate progress,
  preserved relative layout, safe retry, and last-step M3U publication are
  native.
- eXoDOS, eXoWin3x, and eXoWin9x related archives are represented as one
  durable acquisition plan with exact game-data/metadata/utility dependencies,
  legacy DOS language priority, role-aware confirmation, aggregate progress,
  a shared per-torrent archive cache, idempotent staging, and exact-primary-only
  collection identity. Prepared extraction and emulator launch remain separate
  emulator-management work.
- Native qBittorrent settings and connection testing, operating-system
  credential storage, exact reviewed-file priority, optional whole-torrent
  mode, a restart-safe queue with pause/resume/cancel, and verified completed
  file ingestion through configurable link/copy policies. The native seeding
  policy can follow qBittorrent or durably pause a Lunchbox-owned torrent after
  all selected bundle members import, with automatic retry and no data removal.
- Native local-ROM folder scan with a platform selector, extension-aware
  discovery, optional SHA-1/MD5 identity verification, cancellable streaming
  progress, searchable/sortable review, bulk/per-file selection, idempotent
  persistence, lossless native paths, and immediate My Collection/Minerva
  availability updates. Unmatched and ambiguous files remain local-only.
- Stable-UUID favorites and named collections with native create/edit/delete,
  sidebar counts and navigation, per-game membership controls, asynchronous
  persistence with rollback, restart recovery, and composition with search and
  platform filters.
- Linux Nix package plus host-neutral Rust path handling for Linux, macOS, and
  Windows.
- Measurable shell-ready and catalog-ready probes.

## Collection management

- Extend the implemented folder chooser, platform assignment, scan preview,
  progress, cancellation, exact/ambiguous reporting, and idempotent rescans
  with named extension presets, resumable hash caches, duplicate/change audits,
  and a missing-file management view.
- Archive-member inspection, multidisc grouping, patches, alternate versions,
  and manual matching review.
- Editable game and release metadata, tags, playlists, custom
  fields, completion state, play count, play time, and last-played history.
- Extend the implemented stable-UUID favorites and named collections with smart
  rules, manual ordering, bulk membership changes, and portable export.
- Non-destructive merge/split/relink tools backed by the database identity event
  log.
- Fast bulk editing, audits, backup/restore, and portable collection export.

## Download-first experience

- Indexed, paged Minerva search without loading the multi-gigabyte hash catalog
  into the GUI process.
- Complete game-level offer selection beyond the implemented exact-file,
  whole-torrent, generic optical-set, and eXo archive-set paths: free-space
  enforcement, arcade laserdisc layouts, and broader duplicate
  avoidance. Extend the implemented primary-region choice into the legacy
  arbitrary fallback-order editor when variant-rich catalog browsing lands.
- Extend the implemented persistent qBittorrent queue, aggregate speed, safe
  record/history cleanup, and durable post-import pause policy with explicit
  retry/recovery controls and richer error history.
- BIOS availability audit and explicit acquisition workflows, with provenance
  and checksums retained for every imported artifact.

## Metadata and media

- Canonical-game metadata enrichment with provenance and per-field conflict
  review.
- Box art, backgrounds, screenshots, logos, manuals, music, and video with an
  asynchronous memory/disk cache and visibility-driven decoding.
- Configurable media-source priority, image replacement, missing-media audit,
  and background download queue.
- Responsive game details, related games, alternate versions, 3D box display,
  manuals, soundtrack playback, and video playback.

## Emulators and launching

- Host-aware emulator discovery for Linux, macOS, and Windows.
- Managed downloads, updates, version pinning, core management, BIOS checks,
  platform defaults, per-game overrides, and command preview.
- RetroArch, standalone emulator, DOSBox, ScummVM, arcade, and PC launch
  profiles.
- Controller detection and mapping, pre-launch validation, process supervision,
  pause/resume, clean shutdown, and play-stat recording.

## Full-screen interface

- A separate controller-first Qt Quick presentation backed by the same Rust
  models and services—not a second database or duplicated backend.
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
