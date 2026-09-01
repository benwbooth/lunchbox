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
- Rust `QAbstractListModel` with virtualized grid and a compact native data-list
  presentation. The selected presentation, visible column order, and every
  deterministic metadata sort survive restart; sorting stays in the catalog
  worker, applies identically to both presentations, and preserves explicit
  collection and recent-play order when Library order is selected. The list
  has a sticky sortable header, keyboard-accessible rows, a polished column
  manager, and all 19 legacy fields plus native Availability. Metadata uses an
  interned columnar store and one-key-per-row worker sorts rather than expanding
  every game object or repeatedly lowercasing values in the comparator. Every
  column also has a searchable exact-value multi-select with live counts,
  visible active-state affordances, All/None controls, and compact adaptive
  include/exclude state; facet scans and filtering stay on generation-guarded
  Rust workers and compose with the rest of the library filter stack.
- Intentional pointer-hover gameplay previews in the desktop cover grid. Exact
  cached-media lookup runs off-thread with generation guards,
  one shared Qt Multimedia player follows the active virtualized card, sound is
  opt-in, and cancellation prevents a recycled delegate from receiving another
  game's video. Recycled or merely visible games request artwork only. A game
  enters the bounded EmuMovies video queue only after a continuous 500-ms hover
  or when the user explicitly opens it; completion refreshes the exact card and
  selected supplemental-media state without reloading details. The enlarged
  card shows the live queue state and transfer percentage instead of presenting
  an inert cache-miss message.
- Native EmuMovies game-music discovery and playback. The Rust FTP service
  searches only individually downloadable HyperAudio trees, applies the same
  guarded platform/title matching as videos, rejects multipart archive packs,
  and atomically publishes supported audio plus a bounded title sidecar under
  the exact game cache. Game Details indexes provider-aware tracks and exposes
  previous/play/pause/next, seeking, title/source/count, cancellation, download
  progress, and direct EmuMovies setup routing through a dedicated QML card.
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
  Minerva bundle resolution, four-way bounded progressive torrent inspection,
  immediate per-source publication, primary-preservation-set ordering, and
  explicitly review-only title candidates. A durable Release Profile covers
  sort title, series, region, play mode, version,
  and release status. These values are searchable, sortable, available as list
  columns and exact filters, and visible in desktop and Couch Mode without
  changing canonical identity. Desktop details can browse an exact series with
  one action. A bounded native More Like This shelf ranks exact stable-UUID
  records using effective series, title/franchise terms, creators, genres,
  platform, play mode, and release proximity, while honoring content filters
  and preserving Installed/Minerva/Catalog availability. Candidate ranking
  strips catalog
  region/version tags before title comparison, normalizes common Roman-numeral
  sequels, then applies persisted region and latest/original-revision
  preferences only between equally strong title matches. The complete legacy
  region ordering is native, durable, editable with keyboard-focusable controls,
  and alias-normalized. The exact variant is always visible before download.
- Generic multi-disc optical downloads are represented as a single durable plan:
  numeric/alphabetic/Roman disc markers, primary-image selection, common CUE,
  CCD, MDS and GDI sidecars, exact-member confirmation, aggregate progress,
  preserved relative layout, safe retry, and last-step M3U publication are
  native. Local M3U libraries can also launch when their entries are still in
  ZIP, 7z, or RAR containers: a bounded pure-Rust worker requires one
  unambiguous launch entry per archive, stages its control files and companions,
  publishes a temporary exact playlist, grants every required Flatpak path,
  and removes the owned session after exit or cancellation. Arcade ROM-set
  archives retain their direct-file contract.
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
  mode, and a mandatory native download preflight. The review surface resolves
  the exact selected and whole-torrent byte counts, checks host-native free
  space plus a bounded reserve, distinguishes copy/link/leave-in-place storage,
  rejects existing destination files, and detects the same reviewed member set
  in Lunchbox jobs or qBittorrent before mutation. The same gate runs again at
  enqueue time. A restart-safe queue provides pause/resume/cancel and verified
  completed file ingestion through configurable link/copy policies. The native seeding
  policy can follow qBittorrent or durably pause a Lunchbox-owned torrent after
  all selected bundle members import, with automatic retry and no data removal.
  Connection testing detects qBittorrent's default save path, local clients use
  that base automatically, and all managed payloads live beneath the portable
  `lunchbox/roms` suffix; only container or remote clients need a host mapping.
  Failed jobs expose exact stable-ID recovery and newest-first durable event
  history. Every enqueue retains exact bounded torrent metadata with SHA-256 and
  v1-info-hash receipts before qBittorrent mutation. Recovery verifies existing
  Lunchbox ownership or recreates a removed owned torrent from those exact bytes,
  restores the combined reviewed file selection and persisted save path, then
  rechecks and resumes without changing provenance, plans, or native mappings.
  Active missing torrents self-heal on refresh; foreign torrents are never
  adopted or modified.
- Provider-neutral manual `.torrent`/v1-magnet intake from an exact game's
  details pane:
  a bounded off-thread parser, portable-path validation, complete native file
  review, explicit exact-file or whole-torrent handoff, immutable catalog
  association, and durable source/torrent/job provenance all reuse the existing
  qBittorrent, import, and collection pipeline. No external source label can
  establish game identity.
- Provider-neutral reusable-source intake from a native `.torrent` or pasted
  v1 magnet link: bounded qBittorrent metadata resolution, verified info hash,
  complete-inventory review, exact platform/title indexing, no payload at
  registration, and selective per-game queueing through the normal Get review.
  Magnet URIs are not persisted.
- Versioned local provider manifests for bulk user-managed lawful catalogs:
  native JSON selection, portable relative paths, optional exact hash receipts,
  aggregate parser bounds, immutable same-version offers, transactional
  import/resync/removal ownership, and zero payload queueing. Explicit manual
  source review takes ownership so provider removal cannot erase that source.
- Unified native Find Artwork workflow for SteamGridDB, IGDB, authorized
  ScreenScraper WebAPI v2, EmuMovies FTP, and explicit Web
  artwork with
  provider-specific media categories, Twitch client-credentials authentication,
  operating-system credential storage, explicit exact-game review, durable
  stable provider links, individual artwork review, and one shared bounded,
  signature-validating, atomic publisher. A reviewed ScreenScraper game link may
  fill later visible-media misses without another title search. The Web source opens a focused search
  in the system browser and quarantines only a user-reviewed HTTPS URL or local
  file; it never scrapes or silently chooses a result. IGDB commercial use
  remains gated on the distributor obtaining the required provider partnership.
- Native local-ROM folder scan with a platform selector, extension-aware
  discovery, optional SHA-1/MD5 identity verification, cancellable streaming
  progress, searchable/sortable review, bulk/per-file selection, idempotent
  persistence, lossless native paths, and immediate My Collection/Minerva
  availability updates. Named scan profiles preserve an exact native root,
  platform, normalized extension scope, and checksum policy across restart;
  scoped rescans never mark out-of-scope records missing. ZIP, 7z, and RAR
  members are reviewed and hashed independently;
  multi-member archives select nothing automatically, while explicit member
  imports are digest-revalidated and materialized atomically with exact source
  provenance. Unmatched and ambiguous files remain local-only unless the user
  explicitly links one through canonical/alternate-title search; reviewed links
  restore only against unchanged path, archive member, size, MD5, and SHA-1.
- Stable-UUID favorites plus manual and smart collections with native
  create/edit/delete, sidebar counts and navigation, exact per-game membership,
  preserved manual order, confirmed bulk visible-result changes, composable
  automatic rules, portable exact-ID import/export, asynchronous persistence
  with rollback, and restart recovery. Couch Mode has a controller-first
  virtualized collection picker with live counts and rules, exact-ID routing,
  cold-start restoration, and safe deletion/stale-ID fallback. The desktop
  collection overview reports exact full-shelf and visible totals, platforms,
  installed and Minerva-ready games, favorites, activity, completion, and play
  time from the catalog worker, with direct Edit, Export, and Manage actions.
  Manual entries also support collection-specific display titles and notes in a
  dedicated native editor. These exact-ID child records survive reordering,
  cascade on removal, apply only in the active manual shelf, and round-trip in
  backwards-compatible portable collection files without changing canonical
  metadata, matching, downloads, or launch behavior.
- Native local Library Audit with one prepared catalog index, cancellable
  off-thread scans, exact healthy/changed/duplicate/new/missing/unreadable/root-
  offline states, review-first filtering and sorting, and confirmed stale-row
  cleanup. Availability is synchronized only for roots that completed a full
  scan; cleanup cannot delete files and offline roots are never treated as
  missing.
- Native whole-library BIOS & Firmware Readiness audit. It inspects present
  local games off the Qt thread, preserves game-level emulator overrides over
  normalized platform defaults, deduplicates only identical
  platform/emulator/core choices, and keeps required, manual-dump, optional,
  ready, missing-runtime, and inspection-error states distinct. Its full-window
  virtualized surface filters and sorts exact packages and routes each row back
  to the existing reviewed per-game acquire/import/sync workflow.
- Portable whole-profile backup and review-first restore for the complete
  writable SQLite state plus receipt-owned Couch Mode themes. Online snapshots,
  bounded versioned ZIP manifests, exact SHA-256 receipts, database/migration/
  theme validation, next-launch activation, and rollback are native; keyring
  credentials and large content stores stay deliberately outside the archive.
- Stable-UUID play activity with durable per-launch sessions, exactly-once
  elapsed-time finalization, count/time/last-played statistics, editable
  completion state, and a live Recently Played view. Game Details also exposes
  a bounded newest-first session inspector with localized timestamps, exact
  emulator identity, duration, outcome badges, keyboard-accessible outcome
  filters, an empty state, and a virtualized list that refreshes after launch.
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
  mode. It restores the exact selected stable UUID and offers persistent
  virtualized cinematic-wheel and horizontal-cover-shelf presentations, works
  without a mouse by keyboard or direct gamepad input, enters
  real fullscreen, and returns to the complete desktop management surface
  without duplicating state. Its off-thread cross-platform input service has
  hotplug state, analog hysteresis, hold-repeat, paging, semantic action routing,
  focus restoration, and controller-family button hints. Couch-native Details
  and Game Menu overlays expose exact metadata, activity, launch/download state,
  favorite mutation, and explicit desktop handoff without losing selection.
  Its virtualized platform picker browses every catalog platform with live game
  counts, applies exact platform identity, and durably restores the selected
  shelf and platform without blocking Qt. Its collection picker similarly
  browses every manual and smart shelf, exposes live count and description/rule
  context, and restores by stable collection ID. Its polished Attract Mode can
  start manually or after a persisted idle delay, rotates through the exact
  current shelf with controller/keyboard/pointer escape routes, and exposes
  validated idle and cycle settings without creating a second catalog.
  Launching from Couch Mode now opens a real launch-status surface backed by
  the shared asynchronous supervisor: startup, active-process, emulator, and
  finalized-session state remain visible without losing the selected UUID, and
  the user can return to browsing or hand off to Desktop Details at any phase.
  Linux backend startup and deterministic real-QML routing through both overlays
  are exercised; attached-controller Linux, Windows, and macOS release-host
  certification remains.
- Native management of Libretro's complete official Slang and GLSL shader packs
  with cross-platform target discovery, verified cache reuse, safe extraction,
  explicit unmanaged-pack replacement, and transactional multi-target rollback.
- Native Missing Media Audit for My Collection or the entire catalog. It scans
  exact artwork categories off-thread with cancellation, keeps hundreds of
  thousands of results virtualized, separates exact LibRetro repairs from
  records requiring provider review, and uses a confirmed bounded two-worker
  repair queue with no cross-category substitution. Every selected exact record
  is journaled transactionally before provider work starts, a stable
  cross-process lease prevents two Lunchbox instances from mutating one batch,
  and restart recovery preserves completed results while offering explicit
  Resume or Discard controls for the remaining records. Unresolved provider
  misses and failures remain visible.

## Collection management

- Extend the implemented folder chooser, platform assignment, scan preview,
  progress, cancellation, exact/ambiguous reporting, idempotent rescans,
  resumable native-metadata-validated hash cache, exact duplicate/change audit,
  named extension-scoped scan profiles, sequential cross-root **Scan all**,
  bounded durable scan history, and missing-file management view with optional
  background schedules. The Library Audit now retains the newest 100 complete
  exact-summary baselines, rejects cancelled/partial work from history, compares
  every status count against the immediately prior complete run, and presents a
  virtualized keyboard/pointer history view without retaining another copy of
  every ROM path or hash.
- Extend native ZIP/7z/RAR member inspection and manual matching review with
  multidisc grouping, patches, and alternate-version management.
- Durable per-game tags are native: stable IDs, case-insensitive normalization,
  atomic metadata/tag/custom-field saves, clickable details chips, exact library
  filtering, text search, and exact smart-collection rules all share the same
  indexed state.
- Ordered per-game custom fields are native: bounded names and values,
  case-insensitively unique names, keyboard/pointer reordering, details-pane
  presentation, name/value search, and cold-start recovery all preserve the
  exact stable game UUID.
- Non-destructive merge/split/relink tools backed by the database identity event
  log.
- Exact-view bulk library editing is native: the reviewed current-filter UUID
  snapshot can apply or restore one supported categorical metadata override, or
  add/remove a case-insensitive set of tags without replacing unrelated tags.
  The complete scope and per-game 32-tag limit are validated before one
  transaction, duplicate UUIDs and memberships are idempotent, orphaned tags are
  cleaned after removals, and the live library refreshes off-thread. Extend the
  collection-wide summary with cross-collection comparison/reporting tools.

## Download-first experience

- Indexed, paged Minerva search without loading the multi-gigabyte hash catalog
  into the GUI process.
- Extend the implemented provider-neutral exact-game and reusable-source intake,
  bounded watched-folder `.torrent` discovery, exact-hash opt-in source archival,
  v1-magnet intake, and local user-managed provider manifests with authorized
  catalog adapters.
  External offers must retain source provenance and never establish or merge a
  canonical game identity. A direct NSW Torrent Library adapter is not planned
  unless its operators provide documented rights, terms, and an authorized API;
  Lunchbox will not bundle a Telegram account session, scrape bots, or automate
  blocking/proxy evasion. See `EXTERNAL_TORRENT_PROVIDERS.md`.
- Extend game-level offer selection beyond the implemented exact-file,
  whole-torrent, generic optical-set, eXo archive-set, and arcade-laserdisc
  paths with authorized multi-provider offer comparison. Native free-space,
  destination-collision, and broader duplicate checks are already mandatory
  before queueing and are repeated immediately before qBittorrent mutation.
- Extend the implemented persistent qBittorrent queue, aggregate speed, safe
  record/history cleanup, durable post-import pause policy, exact retained-
  metadata recreation, active-job self-healing, and per-job event history with
  opt-in retry schedules for other transient client or network failures.
- Extend the implemented runtime-scoped firmware inventory, exact Minerva and
  official-source acquisition, local import, per-file integrity manifests,
  repair workflow, and whole-library readiness audit with scheduled optional
  maintenance and native Windows/macOS release-host verification.

## Metadata and media

- Canonical-game metadata enrichment with provenance and per-field conflict
  review.
- Extend the implemented exact Minerva manual acquisition, durable visible
  progress/cancellation, validated atomic cache publication, native cached
  manual/video bundle, cached grid preview, and ported legacy EmuMovies FTP
  artwork/video/manual client. Direct EmuMovies transfers now expose throttled
  progress, cooperatively abort FTP, and remove unpublished partial files.
  Selected/opened games and sustained hover requests use a protected foreground
  artwork lane and preempt a less relevant automatic video lookup/transfer at
  its folder-listing, index, or byte-stream checkpoint. Interrupted work remains
  queued, while reselecting it before cancellation reasserts it at the front.
  Remaining work includes an unattended member-account cancellation gate. With
  saved credentials, only an opened game or a pointer hover that survives the
  500-ms intent gate enters the bounded automatic video queue; visibility and
  keyboard traversal remain artwork-only.
- Extend the implemented whole-library missing-media audit and durable bounded
  bulk repair queue with multi-provider bulk review, supplemental video/manual
  categories, and scheduled unattended maintenance. Durable media-source priority, per-game candidate
  rotation, immediate off-thread reindexing, and safe explicit LibRetro
  replacement are native.
- Extend the responsive game details and native video/fullscreen/manual/3D-box/
  related-game/game-music surface with richer franchise curation.

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
  Archive and multi-disc preparation stays off the Qt thread and has explicit
  cancellation in both Desktop and Couch Mode; cancellation is never recorded
  as a play session or shown as a launch failure.
- Exact game/platform/all-platform launch profiles are native for ordinary ROM
  and prepared eXo plans. Their contextual editor exposes independently
  inherited extra arguments and replacement templates; the bounded compiler
  preserves native/Flatpak/Wine paths and never invokes a shell. The native
  Launch Commands manager adds a worker-loaded, virtualized, searchable matrix
  for exact global/platform standalone and RetroArch-core rows with
  built-in/customized filtering and focused save/clear. Both surfaces now show
  a live, numbered direct-process argv preview from the runtime compiler,
  including independent per-field inheritance and contextual validation.
  Complete Windows/macOS runtime-host certification.
- Standalone MAME set-name/ROM-path and Hypseus framefile/support/bundle
  profiles are native. Derived Arcade Laserdisc and Arcade Pinball labels map
  to the canonical Arcade emulator catalog without being collapsed in the UI.
  Linux Flatpak MAME launch is runtime-verified with the ROM-free Pong driver.
  Known archive-oriented FinalBurn Neo, Flycast, and Supermodel executables
  retain the legacy bounded direct-file contract.
- Exact installation-scoped version pinning is native. Pinned updates stay
  visible and reversible but are excluded from Lunchbox batches and startup
  prompts; external package managers remain explicitly unaffected.
  Available-version-only presentation checks every exact installed backend,
  presents current-to-available versions, and applies explicitly selected rows
  sequentially with per-row outcomes. Durable lifecycle journaling,
  cross-process mutation serialization, restart reconciliation, compact recent
  activity, and stop-after-current batch cancellation are native. Lunchbox
  deliberately does not claim provider-native cancellation in the middle of an
  external package-manager transaction; Windows and macOS mutation remain
  release-host gates.
  Firmware/BIOS validation before game launch is also native.
- Dedicated Daphne runtime, TeknoParrot, and non-eXo PC launch profiles.
- Finish the native controller slice with attached-hardware InputPlumber
  mutation/restore certification, Windows and macOS launch-remapping adapters,
  pre-launch validation presentation, pause/resume integration, and
  controller-first focus. Cross-platform GilRs gamepad inventory now feeds the
  settings surface with stable path-agnostic identities; Linux InputPlumber
  remains the explicit launch-remapping provider. Durable per-player
  action/profile/target/order state, the visual custom-profile designer,
  pre-spawn activation, RAII restoration, process supervision, and play-stat
  recording are native.

## Couch Mode

- Extend the implemented native Qt Quick Couch Mode browse/play presentation,
  which already shares the Rust models and services rather than creating a
  second database or duplicated backend.
- Declarative theme packages are native: three built-in looks plus validated,
  atomically installed user packages control the shared palette, optional
  background image, hero contrast, and corner treatment without executable QML
  or scripts. Exact selection survives restart and verified ownership receipts
  guard updates and removal.
- The controller-first manual/smart Collections picker is native, virtualized,
  and restart-safe by exact stable ID. The Couch Game Menu now includes a
  virtualized exact release picker with regional/version ordering, current/
  installed/Minerva/catalog-only state, cached-media context, and stable-ID
  routing. The launch action now presents a controller-safe startup/now-playing/
  session-complete surface from the shared launch supervisor, including the
  exact emulator and actionable Desktop Details/browsing handoffs. Exact
  Minerva acquisition is now Couch-native: progressively available candidates
  stay in preservation-source order with the best candidate first, controller
  review exposes storage/install results, one-time setup routes explicitly, and
  confirmation queues through the shared Rust service without leaving Couch
  Mode. Continue with deeper video, manual, and artwork acquisition controls
  inside the implemented Details and Game Menu overlays. Cached per-game background music is now native with a
  stable-selection delay, launch/picker blocking, now-playing controls, and
  durable enable/volume settings. Continue with startup/shutdown/pause screens, sound packs,
  marquee monitor support, parental lock, and richer couch-safe settings.
- Extend the implemented keyboard/direct-gamepad navigation into every future
  Couch Mode surface. Certify attached-controller input, hotplug, and focus
  restoration on Linux, Windows, and macOS release hosts.

## Definition of done for every slice

A slice is complete only when its UI is functional against real data, expensive
work is cancellable and off the GUI thread, errors are actionable, keyboard and
controller behavior is defined, state survives restart, unit and runtime tests
cover the critical path, and performance is measured in a release build. Empty
buttons, placeholder services, silent no-ops, and compile-only claims do not
count as progress.
