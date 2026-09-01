# Native frontend architecture

Lunchbox uses Qt 6 Quick for rendering and CXX-Qt for typed communication with
Rust. There is no handwritten C++, embedded web runtime, HTTP application
server, or shell-script launcher.

## Responsiveness contract

The desktop shell is created entirely from compiled-in QML. Database, network,
filesystem hashing, image decoding, emulator discovery, and torrent operations
must never run before the first shell-ready event or on the Qt GUI thread.

The initial implementation enforces this shape:

- Startup is deliberately two-stage. The catalog worker first publishes 240
  real, alphabetically ordered games plus complete platform counts and
  installed/Minerva coverage. That makes the shell browsable without waiting
  for the 303,560-row search and sort index; the full catalog then replaces the
  preview atomically while preserving the current shelf, search, platform, and
  filters. Neither phase performs network work.
- `LibraryModel` is a native `QAbstractListModel`; Qt views request only roles for
  visible rows.
- Both grid and list views reuse delegates and keep a bounded one-viewport
  cache.
- The data-list's 17 display fields use aligned numeric rows and an interned
  string pool. A metadata sort computes one effective normalized key per
  filtered row on the catalog worker, sorts those temporary keys, then drops
  them before publishing; the comparator does not allocate or repeatedly hash
  metadata overrides.
- Exact-value list filters use compact adaptive include/exclude sets, so Select
  All and Select None remain constant-space even for 303,560 titles. Facet
  scans and counts run on generation-guarded Rust workers, ignore their own
  column while honoring every other active filter, and publish at most 300
  searchable values to Qt at once.
- SQLite opens read-only on a named worker thread after the window is created.
- Search, platform, and availability filtering run on worker threads. A
  generation number discards stale results when the user types quickly.
- Platform navigation keeps a separate compact Rust index over 191 platform
  names, canonical database aliases, and bounded legacy abbreviations. Its
  search is immediate, the 180–400 px resize preview never touches storage, and
  debounced search/drag completion is coalesced through one SQLite worker. The
  desktop game-details pane follows the same model: its 340–900 px width is
  previewed without I/O while dragging, clamped to preserve the library area,
  and durably coalesced with the sidebar state. The divider supports mouse
  drag, keyboard arrows/Home/End, and double-click reset.
  The 216 legacy platform images are compiled into the application as Qt
  resources. Explicit aliases cover catalog spellings such as Atari 8-bit,
  Commodore CD32/CDTV/Plus-4, Naomi, PICO, Satellaview, Videopac+, and V.Smile,
  so platform navigation never depends on a runtime asset path.
  `--sidebar-ui-probe --state-database EMPTY_PATH` filters by the `snes` alias,
  resizes to 332 px, and saves the real state. A new process using the same path
  and `--sidebar-restored-ui-probe` succeeds only when both values and the exact
  Super Nintendo result restore; `--screenshot-output PATH` captures the native
  sidebar in either run.
- The initial query reads compact text, stable provider IDs, and state fields only. The preserved
  discovery catalog can be layered read-only over the canonical database;
  Legacy user-file state is joined in memory by exact provider game IDs or
  stable game UUIDs. Minerva platform coverage uses exact normalized mapped or
  provider names because its numeric platform IDs are not in the discovery
  database's internal ID namespace.
  Artwork is indexed by stable LaunchBox database ID on a separate named worker
  after the catalog becomes usable. Visible delegates resolve only against that
  compact in-memory index, and Qt decodes local images asynchronously at display
  size. Video, manuals, emulator inspection, torrent metadata, and hashing remain
  separately scheduled services rather than additions to startup.
- Pointer-wheel input uses one reusable QML physics component in both grid and
  list views. It amplifies high-resolution trackpad deltas, gives physical
  notches an immediate displacement, accelerates repeated input, accumulates a
  long kinetic tail, and eases to rest without blocking the GUI thread. Delegate
  recycling schedules only asynchronous artwork; it cannot enqueue gameplay
  videos while the user is moving through the library. Enlarged hover cards use
  real layout and font sizes rather than a transformed texture, clamp all four
  edges inside the game viewport and scrollbar inset, and reduce the requested
  2x expansion only when a smaller window cannot contain it. They therefore
  remain sharp and cannot hide behind the sidebar, page heading, window edge,
  or scrollbar.
- The horizontal `More Like This` shelf uses the same accelerated wheel
  physics, touch flicking, an always-visible draggable scrollbar, and bounded
  Left/Right/PageUp/PageDown/Home/End navigation. Related cards retain exact
  catalog IDs when focus or scrolling changes.
- The desktop shell is split into reusable QML controls for header actions,
  sidebar navigation, links, metrics, filters, metadata, status, wheel physics,
  and first-run setup. `Main.qml` remains the composition root instead of
  re-declaring those controls inline.
- A new profile opens a native first-run setup page before ordinary browsing.
  Required local-ROM and qBittorrent configuration is presented separately
  from optional SteamGridDB, IGDB, and EmuMovies accounts. Every missing-account
  action routes to the exact Settings section; secrets remain in the operating
  system credential store. Existing profiles migrate as already configured.
- Settings is a full-window application section with a persistent navigation
  rail and a bounded, centered content column. Directory selection crosses the
  CXX-Qt boundary into the Rust native-dialog backend, so Windows, macOS, KDE,
  and other Linux desktops receive their host picker instead of an application
  imitation. Header and navigation controls clip and elide labels at constrained
  widths. Deterministic overview and provider-section probes verify the complete
  window geometry and direct-section scrolling.
- Search-entry text and compact game-detail facts use Qt outline rendering,
  shaping, kerning, and unhinted advances. This avoids per-glyph grid fitting
  against uneven device-pixel phases on fractional-scale Wayland while keeping
  the host UI font on Windows, macOS, and Linux.
- Paths cross the Rust boundary as native paths. `--database PATH` and the
  `LUNCHBOX_DATABASE` environment variable are supported on Linux, macOS, and
  Windows without assuming `/bin/sh`, drive letters, or a path separator.
- `--games-database`, `--minerva-database`, and `--user-database` select the
  discovery, acquisition, and installed-state layers. Their environment
  equivalents use the `LUNCHBOX_` prefix. None of these paths is interpreted as
  a provider path or converted to a platform-specific string identity.
- `--media-directory` or `LUNCHBOX_MEDIA_DIRECTORY` selects an existing user
  media cache. Otherwise the cache is a `media` sibling of the OS-native state
  database. Local files become `QUrl::fromLocalFile` URLs at the Qt boundary;
  Rust never constructs a POSIX-only or Windows-only URL.
- Visible delegates may enqueue missing artwork only after the local index is
  ready. Two named Rust workers consume a bounded newest-viewport-first queue;
  no network request runs on the Qt thread or gates startup. The workers follow
  the configured provider order: LibRetro, SteamGridDB, IGDB, exact EmuMovies
  archive members, and ScreenScraper are eligible. Reviewed IDs are reused.
  Unlinked automatic retrieval accepts only one strict exact SteamGridDB title,
  one strict exact IGDB title/platform pair, or one strict exact ScreenScraper
  title inside a mapped provider platform. It never persists those transient
  matches as identity links, and ambiguous results remain explicit review work.
  LibRetro responses must be PNG files of at most 8 MiB and are published with a
  same-directory atomic rename.
  Provider misses are cached per stable LaunchBox ID and media type for seven
  days. A provider transport or authentication failure opens a two-minute
  in-process circuit breaker so one bad account cannot stall every visible card;
  explicit refresh bypasses that breaker. `--offline` or `LUNCHBOX_OFFLINE=1`
  makes the same UI strictly cache-only.
- The media index retains every valid candidate in deterministic media-type,
  provider, format, and path order. The details hero can rotate those candidates
  without rescanning the filesystem. Explicit refresh bypasses the miss cache
  and replaces only the owned LibRetro target after write and sync; overwrite
  fallback restores the prior file on publication failure and never removes
  alternatives from other providers.
- `MediaAuditModel` loads the same catalog and scans the same provider-ordered
  cache on a named worker. My Collection and Entire Catalog scopes are explicit;
  progress and cancellation cover the cache and game passes, while the result
  list retains only compact missing rows and stays virtualized in QML. Coverage
  is exact by media kind: an available fallback does not satisfy the audit.
  Positive exact game identities on LibRetro-supported platforms may enter a
  confirmed repair batch. Two workers receive at most eight scheduled rows at
  once, require the requested category instead of media-type fallbacks, and
  follow the same exact provider chain as visible games. Every success is
  bounded, signature validated, and atomically published only into the source
  provider's owned files. Local-only or unsupported records remain reviewable
  and route back to Game Details rather than receiving guessed identity or
  media.
- `FirmwareAuditModel` reads only present native local paths and resolves the
  same exact game-then-platform emulator preference used by Play. A named Rust
  worker groups only identical normalized platform/emulator/runtime/core
  choices before invoking ordinary emulator discovery and the canonical
  firmware rule service. Required packages, user-supplied dumps, HLE-capable
  optional packages, ready packages, missing runtimes, and inspection failures
  remain separate model states. The dedicated `FirmwareAuditView.qml` is a
  full-window, virtualized management surface with accelerated wheel and
  keyboard paging; it never mutates firmware directly and instead opens the
  representative exact game in the existing reviewed acquire/import/sync card.
- The writable state database is selected with `--state-database` or
  `LUNCHBOX_STATE_DATABASE`, otherwise an OS-native application-data directory
  is used. Its WAL setup and schema migration are serialized before concurrent
  settings, queue, and catalog workers access it. Favorites use the stable game
  UUID as their primary key; the LaunchBox numeric ID remains metadata only.
  Favorite writes run on named workers, update the native views optimistically,
  and roll the UI back if SQLite rejects the change. Window shutdown is deferred
  until an accepted favorite write has left the in-process worker.
- Play activity uses the same stable game UUID boundary. A `game_activity` row
  stores aggregate count, elapsed time, first/last timestamps, and completion
  state; immutable-identity `play_sessions` rows retain emulator, start/end,
  elapsed seconds, and outcome. The supervisor creates both records only after
  a real emulator process exists, measures with a monotonic clock, and applies
  duration in the same transaction that closes a still-running session. This
  makes finalization idempotent. Recently Played is a compact in-memory order
  map refreshed off-thread and composed with search/platform filtering.
- qBittorrent paths and host paths are separate types at the service boundary:
  host paths use Rust `PathBuf` and native folder dialogs, while remote or
  container paths remain literal strings. The app never rewrites a Windows
  path as a POSIX path or assumes the client runs on the GUI host.
- A successful qBittorrent test reads the client's configured default save
  path. When that path is also a real local directory it becomes the native
  base automatically; container or remote clients retain an explicit host-side
  base mapping. Lunchbox appends `lunchbox/roms` to both bases for every managed
  game download, so the default is isolated and identical on Linux, Windows,
  and macOS without exposing the internal suffix as another setting. Settings
  renders both resulting paths as one default managed mapping so the exact
  qBittorrent and host destinations are visible before a download is queued.
- The persisted seeding policy either leaves qBittorrent's existing rules alone
  or records a durable `pause_pending` action after successful ingestion. A
  shared torrent is stopped only after none of its selected jobs remains in a
  transfer or import state. Failures remain pending across refresh and restart;
  history cleanup cannot remove them. Lunchbox calls qBittorrent's stop/pause
  endpoint and never changes the client's global share-limit action or deletes
  torrent data as part of this policy.
- Minerva candidate matching treats provider filenames as review evidence, not
  identity. Region and revision tags are removed before title scoring, common
  multi-character Roman-numeral sequels are normalized, and a stronger title
  match always wins. The complete persisted region ordering and
  latest/original-revision choice only order equally strong candidates; aliases
  and duplicates are normalized before persistence. The native picker shows the
  parsed region and revision and visually identifies the resulting best match.
- Torrent sources are inspected by a bounded four-worker pool and each source's
  exact candidates are published to Qt as soon as that source finishes; a slow
  derivative set cannot hold back a ready primary result. Standard No-Intro or
  Redump preservation sets sort before headerless, RetroAchievements,
  FinalBurn Neo, aftermarket, private, and source-code variants. The GET action
  is enabled per ready source while the remaining sources continue in the
  background. A source enters either download UI only after inspection finds at
  least one matching game member; empty torrents remain model-internal and are
  never rendered as choices. Desktop groups candidates by preservation source, while Couch
  Mode exposes one controller-first ranked list. The Couch projection uses
  Rust-provided flat indices that map back to the exact source and torrent-
  member index; it never copies candidate identity into QML or re-ranks by
  display text. Selecting a row runs the same native storage/duplicate
  preflight and the same guarded queue operation as Desktop Mode. Setup errors
  route to the qBittorrent settings section, and returning from review preserves
  the exact Couch selection.
- Bounded torrent bytes are retained in a validated, URL-keyed OS cache for 24
  hours. The currently inspected torrent also has one decoded file index shared
  by its source views, so a large multi-platform torrent is not downloaded or
  expanded repeatedly. All fetch and decode work remains off the Qt thread.
- A reviewed reusable torrent is different from that transient inspection
  cache: Lunchbox durably owns its exact bounded `.torrent` bytes, SHA-256 and
  v1 info-hash receipts, source provenance, platform association, and normalized
  member index in SQLite. qBittorrent is a replaceable transfer executor rather
  than the catalog authority. A removed live client job therefore cannot erase
  discovery data; queueing or recovery recreates only a Lunchbox-owned transfer
  from the retained bytes and exact reviewed selection.
- Generic optical multi-disc matches become one versioned download plan. Disc,
  disk, CD, side, part, volume and card markers support numeric, alphabetic and
  common Roman forms; the planner chooses one runnable primary per disc and
  includes CUE, CCD, MDS and GDI companion formats. The confirmation dialog
  exposes every exact torrent member and total size before qBittorrent changes.
  One persisted queue row selects and monitors the complete set with
  size-weighted progress. Import preflights every source and conflicting target,
  preserves the common relative layout, and publishes the M3U and installed-game
  record last. Retries are idempotent and never overwrite different content.
- Completed acquisitions persist a normalized installed-game row plus one
  lossless native-path membership for every materialized plan member and
  generated playlist. A separate receipt stores the exact file type, length,
  SHA-256 identity, and whether Lunchbox created the path. Receipts are shared
  across games by exact encoded path, so uninstall deletes an owned file only
  at its last reference. The entire last-reference deletion set is rehashed
  before any file changes; a modified path refuses the operation. Missing paths
  are accepted as an idempotent retry, while imported, pre-existing, and
  leave-in-place paths are association-only. Empty-directory pruning cannot
  cross the configured ROM root. Prepared eXo cleanup additionally requires an
  exact immediate child of the stable game cache boundary and preserves shared
  bootstrap assets. Game Details exposes the result as a distinct Game Files
  card and uses `QUrl::fromLocalFile` plus Qt's native URL handler for Open
  Folder, with no platform shell or path rewriting.
- eXoDOS, eXoWin3x, and year-bucketed eXoWin9x matches become versioned related-
  archive plans. The planner reproduces the legacy exact layouts for the primary
  game, DOS `Content/GameData`, collection metadata, and shared DOS/Win9x
  utilities; unrelated language and extras directories are never accepted as
  dependencies. DOS primaries retain the legacy default, English, German,
  Polish, then Spanish priority. One confirmation dialog and durable queue row
  expose the exact roles and aggregate size. The completed layout is staged
  idempotently below `ROM/.lunchbox-pc-archives/INFO_HASH`, so shared dependencies
  can be reused by the later prepared-install pipeline without appearing as
  separate installed games; only the exact primary archive is recorded.
- Minerva's Laserdisc Collection is planned by machine layout, not fuzzy title
  score. Exact normalized game titles are joined to the pinned canonical
  LibRetro MAME records to obtain reviewed set names. MAME plans require one
  exact ROM ZIP plus its CHD. Hypseus plans require a package ROM, framefile,
  data, video, and audio; Daphne's `vldp_dl` or `mpeg2` source shape is
  normalized to the same validated Hypseus `vldp` contract and retains matching
  RAM files. Incomplete groups fail closed. Persisted plans validate unique
  torrent indices, unique safe target paths, and exact role cardinality before
  queueing. qBittorrent selects only reviewed members. Ingestion preflights the
  full set and stages the real launch layout; leave-in-place mode uses native
  symlinks so qBittorrent remains the owner of large media files.
- An installed eXo primary can be prepared from its exact native path on a named
  worker. The source archive set is fingerprinted and expanded below
  `ROM/.lunchbox-pc-cache/installs/COLLECTION/GAME_UID/SIGNATURE`; title metadata
  and DOS game data are selected by their exact legacy subtrees. Shared utility
  archives are fingerprinted separately, including nested `EXTDOS` and
  `EXTWin9x` ZIPs, so MT-32, DOSBox, 86Box, and PCBox parent assets are reused.
  ZIP paths must remain relative and contained. Cancellation removes only the
  app-owned UUID staging tree; a complete tree is published by same-filesystem
  rename and recorded by stable game UUID only after its launch config exists.
  Refreshing an unchanged game reuses the verified cache. The details panel
  shows preparation state, throttled progress, cancellation, and retry without
  blocking Qt.
- Prepared-PC launch classification is pure Rust and treats legacy BAT/BSH
  files as bounded data formats; Lunchbox never executes them. Exact supported
  patterns choose DOSBox, ScummVM, DOSBox-X, 86Box, or PCBox-compatible plans,
  while traversal and unsupported helper-process commands fail closed. The
  canonical emulator catalog supplies host/package metadata. Discovery covers
  native executables, Windows application directories, macOS app bundles, and
  Linux Flatpaks off the Qt thread. Launches use `Command` argument vectors and
  native `PathBuf`/`OsString` values with no shell. Flatpak access is limited to
  the prepared install root; Win9x parent disks are reflinked or copied into
  persistent writable children, and temporary MT-32 exception files are
  removed after process exit. Qt reports detection, starting, running, and exit
  state. The unattended `--exo-launch-probe` exercises the real discovered
  emulator and waits for its process to exit.
- Generic local-ROM discovery joins the canonical platform or exact alias to
  host-compatible emulator records and exact core slugs. It discovers native
  standalone programs and native or Flatpak RetroArch/core pairs off-thread,
  sorts recommended options deterministically, and persists a stable emulator
  ID plus runtime/core—not a mutable display name. A game override wins over a
  normalized platform default. Multiple imported files remain distinct native
  `PathBuf` values. Launch plans pass the selected ROM and exact core as
  `OsString` arguments, grant Flatpak only the containing directory, and never
  invoke a command shell. Standalone MAME preserves ZIP/7z sets and passes an
  exact `-rompath` plus set name. Hypseus accepts only a present TXT framefile
  below a `vldp` or `singe` bundle with a ROM directory and complete support
  tree, then passes exact framefile/home/data/ROM paths. Unsupported standalone
  arcade machines fail closed instead of receiving a generic archive path;
  only the legacy-known archive-oriented FinalBurn Neo, Flycast, and Supermodel
  executables retain the direct-file contract.
- Launch customization is stored by exact emulator ID, runtime kind, core name,
  and one of stable-game, normalized-platform, or all-platform scope. Extra
  arguments and command templates resolve independently in game, platform,
  global order, so overriding one field does not erase inherited behavior in
  the other. The portable bounded parser treats quotes only as argument
  grouping and preserves ordinary Windows backslashes. It never expands a
  variable, command substitution, glob, pipe, or redirection. A blank template
  inserts extra arguments at the reviewed emulator-specific position; a
  non-empty template replaces the built-in argv and may consume only the
  values available to that exact launch plan. `%f`, RetroArch, MAME, Hypseus,
  Altirra, DOSBox, ScummVM, 86Box, and PCBox placeholders materialize directly
  as native `OsString` path or literal values after the existing native,
  Flatpak, or Wine path adapter has been selected. Prepared launches recheck
  the discovered emulator ID before applying a profile. The inline Qt editor
  exposes built-in and effective templates, independent inheritance sources,
  three save scopes, clear, validation errors, and no-shell semantics. A second
  native manager loads the complete global/platform matrix on a worker,
  deliberately represents standalone and each exact RetroArch core as separate
  stable-ID rows, and filters a virtualized list by scope, customization, or any
  platform/emulator/core/template text. Its focused editor uses the same store,
  inheritance resolver, context-placeholder validator, and direct-argv rules;
  game-scoped values remain beside Play where their game context is visible.
- Emulator lifecycle data is part of the canonical database, with package
  records keyed by stable emulator ID, host, manager, and exact package ID.
  A maintained reviewed-source overlay supplements the legacy catalog for
  isolated Nixpkgs installs, GitHub/AppImage releases, and bounded official
  downloads. The manager groups transport alternatives deterministically and
  performs detection and mutation on named workers. It invokes Flatpak, Nix,
  winget, and Homebrew with direct argument vectors; Snap and command shells
  are not supported. Downloaded programs live below the operating system's
  application-data directory, have version manifests, and can be removed only
  when a matching ownership receipt exists. Altirra runs through Wine with a
  per-emulator prefix and absolute content paths mapped explicitly to Wine's
  `Z:` drive. RetroArch cores use exact catalog slugs and host-specific
  Libretro buildbot filenames, extract only the expected archive member, and
  replace files atomically. The same receipts are consumed by launch discovery.
- Update discovery is intentionally independent of the manager's preferred
  install source. Exact manager/package/path tuples preserve alternate
  backends, including distinct user and system Flatpak installations. Named
  workers compare Flatpak update inventories, Nixpkgs versions, managed
  GitHub/AppImage release manifests, exact winget results, and Homebrew's JSON
  inventory. Only proven updates enter the Qt review model. Rows start selected,
  run sequentially only after an explicit request, and retain Pending,
  Updating, Updated, or Failed state with the underlying error. A durable pin
  is keyed by that same exact host/manager/package/path identity and records the
  detected installed version. Pinned updates remain visible as `PINNED`, are
  excluded from selection, batches, and scheduled prompts, and can be unpinned
  in place. The pin controls only Lunchbox's update manager; it never claims to
  hold an external Flatpak, Nix, winget, or Homebrew installation. Pin writes
  run on a named worker. `--emulator-update-pin-ui-probe` exercises a real
  discovered update through pin, visible non-actionable state, durable SQLite
  write, unpin, and restored actionability without applying the update. Partial
  source failures are warnings; direct downloads and Libretro buildbot cores
  without a reliable comparable version are not presented as updates.
- Emulator install, update, and uninstall mutations share one stable,
  state-adjacent cross-process lease. Lunchbox records the exact emulator,
  host, manager, package, install path, action, and start time in SQLite before
  invoking a package manager, then terminalizes that row exactly once and
  retains the newest 200 operations. On restart, it marks abandoned running
  rows interrupted only when the lease is free, immediately rescans the exact
  installation state, and presents the latest result in the manager instead of
  guessing whether a package transaction succeeded. Update batches support
  **Stop after current**: Lunchbox never kills an external Flatpak, Nix,
  winget, or Homebrew transaction, and later selected rows remain pending for
  review. `--emulator-lifecycle-recovery-ui-probe` seeds a real interrupted
  journal row in an isolated state database and verifies its recovered Qt
  presentation unattended.
- Firmware remains a `runtime -> rules -> target` system, matching the legacy
  architecture instead of assuming one BIOS directory per platform. Canonical
  rules bind an exact emulator or RetroArch core and platform to one reviewed
  source package, required/HLE policy, install mode, and target strategy.
  Acquisition reuses the native qBittorrent client for exact Minerva members
  and uses bounded HTTPS for official and GitHub archives. Local imports and
  downloaded packages enter an isolated staged store only after safe-path,
  size, file-count, and duplicate-entry validation. SQLite records the archive
  digest and a complete per-file manifest. Repair verifies both before copying
  or merging into a host-native runtime directory; it never invokes a command
  shell. Manual-only dumps use a dedicated native directory and retain their
  instructions without pretending that Lunchbox can acquire them.
- Local collection scanning enumerates without following symlinks and hashes on
  a named worker. Progress is throttled before crossing the Qt bridge, and an
  atomic cancellation token is checked between every read chunk. The review
  view is virtualized; filtering and selection operate on compact Rust indices.
  Imported paths retain native bytes (`unix_bytes`, `windows_utf16le`, or
  `utf8`) instead of using a display string as identity. Reusable profiles are
  stored by stable ID in the writable state database and preserve those same
  native root bytes plus an exact platform hint, canonical sorted extension
  set, and checksum policy. Profile reads and writes stay on named workers.
  Editing loaded controls detaches them into a visible one-time scope rather
  than silently changing the saved profile.
  Successfully read CRC32/MD5/SHA-1 records are committed incrementally to the
  writable state database, so a cancelled scan resumes from every completed
  regular file or archive. Reuse requires the same lossless path, stable native
  filesystem identity, container size, and OS modification/change metadata;
  malformed cache rows and changed-during-read files fail closed. Warm hits do
  not write SQLite. A successful complete walk prunes disappeared paths in one
  transaction, while cancellation deliberately skips pruning. An explicit
  extension scope limits both loose files and archive members. Archive
  containers are still inspected safely, all recognized members contribute to
  the physical-member count, and only in-scope collection records can become
  missing after a scoped rescan.
- Library maintenance runs on the same bounded scanner contract but prepares
  the catalog match index once for the entire audit. Each root either completes
  atomically for availability purposes or is reported as offline; a partial or
  failed scan can never manufacture missing records. Persisted SHA-1, then MD5,
  then size/time evidence classifies changes, while exact current SHA-1 groups
  identify duplicates without merging them. The CXX-Qt model owns compact
  filter indices and selection state. Confirmed cleanup accepts stable UUIDs
  only and its SQL predicate can remove only included rows already marked
  `missing`; filesystem paths are not part of the mutation API.
- Manual external-torrent intake has two explicit modes and is never another
  title matcher. Exact-game mode receives the already-selected stable UUID and
  immutable catalog fields, accepts one native `.torrent` or pasted v1 magnet,
  and queues an explicit reviewed file. Collection mode receives a platform
  scope plus the same source choices. Both modes resolve magnet
  metadata through configured qBittorrent under a bounded deadline, verifies
  the exported info hash, and queues the complete reviewed inventory into a
  managed per-platform directory. The parser caps input at 16 MiB and 100,000
  files, validates checked total size, and rejects traversal, duplicate or
  case-ambiguous paths, control characters, Windows-reserved components, and
  names that cannot be used consistently on Linux, macOS, and Windows. QML
  receives only compact reviewed metadata; magnet URIs and trackers are not
  persisted. Both modes use `manual_torrent` durable jobs and separate receipts
  binding source type/label, SHA-256, info hash, actual paths, and resulting job.
  A completed collection job exposes its exact downloaded directory to the
  ordinary local-import review; only that matcher and user confirmation can
  establish canonical game identities. Watched-folder state and
  network-provider adapters remain separate future transports.
- RetroArch shader management is a Rust service surfaced through the existing
  Settings CXX-Qt model. It discovers host-native shader roots, resolves the
  official Libretro Slang and GLSL archives into a SHA-256-verified cache, and
  validates safe paths, duplicate entries, symbolic links, file counts, and
  bounded compressed/unpacked sizes while honoring cancellation. Each pack is
  staged in its target filesystem and receives an ownership receipt before a
  multi-target transaction replaces only `shaders_slang` or `shaders_glsl`.
  Earlier targets roll back if any later activation fails; custom siblings are
  out of scope. Folder opening passes a path directly to the host opener and
  never constructs a shell command.

## Performance budgets

These are release gates, not estimates:

| Event | Budget |
| --- | ---: |
| Native shell ready, warm release build | 250 ms |
| Search/filter feedback | 100 ms |
| Input-to-highlight response | 16 ms |
| Database or filesystem work on GUI thread | 0 ms |
| Network work required for startup | 0 requests |

Run the shell-only probe without waiting for the catalog:

```console
cargo run --release -p lunchbox-app -- --startup-probe
```

It prints `LUNCHBOX_SHELL_READY_MS=<milliseconds>` and exits. A normal
development launch finds `build/lunchbox.db`; an explicit path is preferred for
testing:

```console
cargo run -p lunchbox-app -- --database build/lunchbox.db
```

Use `--catalog-probe` to print database loading time and row counts before
exiting. A normal run prints `LUNCHBOX_CATALOG_PREVIEW_READY_MS` as soon as real
preview content is interactive. Unlike the shell probe, `--catalog-probe` waits
for the complete asynchronous catalog replacement and prints
`LUNCHBOX_CATALOG_READY_MS` with the complete row count.
Use `--filter-probe` to load the catalog, apply a representative Minerva search,
print `LUNCHBOX_FILTER_READY_MS`, and exit.
Use `--filter-ui-probe` to leave the native filter surface open for an
unattended visual check against a real catalog.

Use `--media-probe` to wait for the asynchronous media index, print
`LUNCHBOX_MEDIA_READY_MS` with game and cached-candidate counts, and exit.
`--artwork-ui-probe` filters to a deterministic set of real cached covers and
leaves the native grid open for a virtual-display visual check.
`--media-fetch-probe --media-directory EMPTY_PATH` performs the same path from
an uncached visible delegate through the real LibRetro service, waits for Qt to
index the downloaded asset, prints `LUNCHBOX_MEDIA_FETCH_READY_MS`, and exits.
`--media-rotation-ui-probe` opens the preserved Super Mario Bros. record after
the media index is ready, selects its second real cached hero candidate, and
prints `LUNCHBOX_MEDIA_ROTATION_READY` with the count, index, and provider. The
same open details surface exercises previous/next rotation and the explicit
LibRetro refresh action; successful replacement prints
`LUNCHBOX_MEDIA_REFRESH_READY` only after the refreshed file has re-entered the
index.
`--media-audit-ui-probe --media-directory EMPTY_PATH --screenshot-output PATH`
audits exact box-front coverage across the real 303,560-game catalog, filters
the virtualized result to exact provider-repair candidates, selects one row,
captures the complete dialog, and emits `LUNCHBOX_MEDIA_AUDIT_UI_READY` from
Rust after the model and QML checks agree.
The optimized Nix package's 1920x1200 software-rendered probe on 2026-08-28
reported the shell ready in 492 ms and the 303,560-game catalog ready in 702 ms;
the complete audit, filter, selection, capture, and clean exit took 2.8 seconds.
The empty-cache run classified 114,258 rows as exact LibRetro repairs and
189,302 as explicit provider review, proving that unsupported platforms and
media categories are not silently presented as downloadable.
Confirmed Missing Media repairs are also durable. SQLite stores the batch and
each exact stable game UUID before a request enters the provider queue, records
each outcome before Qt advances, and permits only one active batch. A stable
state-adjacent file lease serializes work across Lunchbox processes. After an
unclean exit, only abandoned `running` items return to `pending`; successful,
unavailable, and failed results remain terminal. Startup checks this journal
off the GUI thread and presents a compact Resume/Discard banner without making
network requests. `--media-repair-recovery-ui-probe --state-database EMPTY_PATH
--screenshot-output PATH` seeds one genuinely running exact record, exercises
the production recovery transaction, renders the restored Missing Media view,
and emits `LUNCHBOX_MEDIA_REPAIR_RECOVERY_UI_READY` only when Rust and QML agree
on its identity and state.
`--favorite-probe --state-database EMPTY_PATH` selects a real catalog game,
persists it through the native Qt action, prints `LUNCHBOX_FAVORITE_READY_MS`,
and exits only after the worker reports success. `--favorite-ui-probe` leaves
the same grid, star affordance, and details pane open for visual inspection.

Game metadata edits are nullable local overlays keyed only by stable game UUID
in the writable state database. The immutable catalog title and platform remain
the inputs to provider lookup, acquisition, artwork, launch history, and ROM
identity; only presentation, search, sorting, smart title rules, details, and
Couch Mode labels consume the effective overlay. Cooperative play is retained
as an exact yes/no/unspecified value from the discovery catalog, can be
overridden locally, and participates in smart collection rules using the
effective value. Empty editable fields can be
cleared explicitly, while saving values identical to the catalog removes those
columns and restoring the catalog deletes the row. The native editor separates
high-frequency presentation values, detailed catalog values, and ordered custom
fields into focused tabs instead of one unbounded form. Custom fields are keyed
to the same stable UUID, limited to 32 entries, preserve explicit order, require
case-insensitively unique non-empty names, and expose both names and values to
text search without entering provider matching or ROM identity.

The Release Profile is part of that same overlay and carries the legacy sort
title, series, region, play mode, version, and release-status fields. An explicit
sort title controls only requested title ordering; clearing it falls back to the
effective display title instead of resurrecting a catalog sort value. The other
fields participate in text search, deterministic list sorting, configurable
columns, and exact-value filters. Desktop details expose an exact **Browse this
series** action, while desktop and Couch Mode suppress empty facts. Release
status deliberately excludes internal catalog lifecycle values such as
`canonical`, `deprecated`, and `merged`. None of these values merge records,
provide a fuzzy identity signal, or change provider and ROM matching.

Community vote counts and catalog provenance remain immutable source facts, not
profile overrides. The current discovery database has rating counts for 174,463
records, catalog video links for 75,647, Wikipedia links for 45,150, exact Steam
app IDs for 22,853, and a source label for all 303,560 records. Rust publishes
only populated values. Catalog URLs are length-bounded and parsed before crossing
the Qt bridge; only credential-free public HTTP(S) hosts are accepted, while
localhost names, IP literals, file URLs, malformed values, and synthesized links
are rejected. A Steam Store URL is derived only from a positive exact app ID.
Desktop Game Details and the metadata editor open those actions through Qt's
system browser. Couch Mode exposes the vote count and provenance without moving
the living-room UI into an external browser.

`--metadata-ui-probe --state-database EMPTY_PATH --screenshot-output
ABSOLUTE_PATH` first proves the custom title is absent, edits the exact Super
Mario Bros. UUID through the real native dialog, adds and reorders two custom
fields, persists the complete profile off-thread, refreshes the library, proves
title, custom-field-value, and exact-series filtering, captures the editor, and
asserts that sort title, series, region, play mode, version, and release status
round-trip alongside the existing fields. It also asserts that
the catalog's exact no-co-op value becomes a durable local yes-co-op value while
the canonical title is still unchanged. The same probe asserts the real record's
1,150-vote count, LaunchBox source, exact Video and Wikipedia URLs, and absent
Steam ID before it captures the source card. A second process using
`--metadata-restored-ui-probe` and the same state path must rediscover the exact
title, Release Profile, co-op value, field order, names, values, and catalog
source facts before it exits successfully.

User tags are normalized, stable-UUID records in the same writable state
database. Metadata, tags, and custom fields validate before one transaction, so
validation or storage failure cannot publish only part of an edit. Tag names
collapse whitespace, deduplicate case-insensitively while preserving the chosen
spelling, and are bounded to 32 tags per game and 50 characters per tag.
Orphaned tag rows are removed after the last membership disappears. Search
includes tag names; the Filters surface and details chips apply an exact tag
filter; smart collections can use the same exact rule. `--tags-ui-probe
--state-database EMPTY_PATH
--screenshot-output ABSOLUTE_PATH` edits the exact Super Mario Bros. UUID through
the real worker, proves the two durable tags entered the global index, applies
one through the native filter, asserts a single result and the details chips,
and captures the real details pane.

`--collection-probe --state-database EMPTY_PATH` creates a named collection,
adds a real catalog game by stable UUID, applies the native collection filter,
and exits only after persistence succeeds. Reusing the state path verifies that
the collection and membership survive restart. `--collection-ui-probe` follows
the same route and leaves the two-pane collection-management surface open for
inspection; `--screenshot-output ABSOLUTE_PATH` captures it and exits.
`--smart-collection-probe` creates an automatic Minerva-available shelf, derives
its count from the real catalog, and exits only after the Qt bridge observes the
expected result. `--smart-collection-ui-probe` opens the same durable rules in
the native editor and can capture it with `--screenshot-output`.

Manual collection members can carry an optional collection-specific display
title and notes. The presentation is stored in a child row keyed by the exact
collection ID and stable game UUID; it never changes canonical metadata,
matching, downloads, or launch identity. Reordering updates membership rows in
place so retained overrides survive, while member or collection deletion
cascades them. The two-pane manager marks customized rows and opens a dedicated
native editor from the pencil action or a double click. Portable version-1
collection files add optional `playlist_title` and `playlist_notes` fields, so
older exports remain readable and exact-ID import/export preserves the new
presentation. `--collection-presentation-ui-probe --state-database EMPTY_PATH
--screenshot-output ABSOLUTE_PATH` creates the exact Super Mario Bros. member,
saves its scoped title and notes through the CXX-Qt worker, validates the
effective manager title, opens the populated editor, captures it, and exits.

The desktop collection overview is published with the generation-guarded
catalog result rather than scanned on the Qt thread. Its seed contains the exact
collection ID, ordered member IDs or smart-rule result, favorites, and durable
play activity. The worker resolves stable IDs through the catalog index, ignores
stale IDs, and reports full-shelf and visible counts, unique platforms,
installed and Minerva-ready games, favorites, played/completed games, and
saturating accumulated play time. `--collection-summary-ui-probe
--state-database EMPTY_PATH --screenshot-output ABSOLUTE_PATH` creates and
selects an exact one-game manual playlist; rerunning with the same state path
proves cold restoration. `--smart-collection-summary-ui-probe` validates and
captures the same surface against the real 281,400-member Minerva shelf, keeping
the large summary and filter off the GUI thread.

`--import-ui-probe --import-directory PATH` opens the local-import surface and
starts a real checksum scan. It exists for deterministic virtual-display visual
checks; it uses the same model, worker, database, and QML as an interactive
folder selection. Adding `--import-commit-probe` selects the readable results
and commits them to the chosen `--state-database`, enabling a fully unattended,
idempotent scan-to-collection integration test.
`--hash-cache-ui-probe` additionally requires explicit `--games-database`,
`--state-database`, and `--import-directory` paths. It seeds the cache through
the real scanner before Qt starts, then requires the ordinary model-driven scan
to report at least one reused file and zero content reads before exiting.
Repeating it against the same paths is idempotent.
`--import-profile-ui-probe` creates one deterministic named profile through the
production store, loads it through the ordinary CXX-Qt model, and exits only
after QML observes its name, exact platform, canonical extension scope, checksum
policy, active index, and native directory. Reusing the same state path proves
idempotent seeding and restart loading.
`--import-profile-batch-ui-probe` creates two extension-disjoint profiles over
one explicit root, prepares the real checksum index once, scans them
sequentially, and opens the production history dialog only after both compact
run snapshots are durable. Reusing the same state proves history survives a
restart and the second batch reuses the native-metadata-validated hash cache;
`--screenshot-output` captures the rendered 860x660 dialog before exit.
`--archive-import-probe --games-database PATH --import-directory PATH` succeeds
only when a ZIP, 7z, or RAR archive with one safe recognized ROM member receives an
exact SHA-1/MD5 catalog identity. `--archive-import-ui-probe` follows the same
real scan through the Qt review table; adding
`--screenshot-output ABSOLUTE_PATH` captures the native archive/member
presentation and exits. Inspection is streamed and never
extracts or mutates the source archive. Multiple safe ROM members receive
separate review rows and remain unselected until the user acts; selected members
are revalidated and atomically materialized into content-addressed launch paths.
Encrypted members, multi-volume RARs, unsafe paths, and archives without a
recognized ROM fail closed rather than acquiring a guessed identity. The unattended
`--multi-archive-import-probe` requires explicit database, import-directory, and
state paths; it exercises member review, materialization, provenance, and an
idempotent repeated commit through the packaged binary.
`--manual-match-probe` additionally requires `--manual-match-query` and
`--manual-match-platform`. It starts from exactly one unmatched ROM, requires a
unique canonical title/platform candidate, applies the same explicit-review
operation as the UI, commits it, rescans from disk, and succeeds only when the
reviewed stable identity and manual provenance restore from exact path/member,
size, MD5, and SHA-1 evidence. `--manual-match-ui-probe` opens the real resolver,
performs its asynchronous canonical/alternate-title search, and optionally
captures the populated review surface with `--screenshot-output`.

`--library-audit-fixture --audit-fixture-root EMPTY_PATH --state-database
EMPTY_PATH` creates a marker-guarded deterministic collection and succeeds only
when the real scanner reports one healthy, one missing, one changed, two exact
duplicates, and one untracked ROM. `--library-audit-ui-probe` opens that state
through the production CXX-Qt model and can capture the complete maintenance
surface. `--library-audit-cleanup-ui-probe` additionally selects the missing
row, invokes the production cleanup action, re-audits, and succeeds only when
that row is gone and all other states remain. The service tests verify byte-for-
byte that cleanup leaves every existing fixture ROM untouched and that an
offline root never becomes missing or removable.

`--download-ui-probe` leaves the persistent native Downloads drawer open.
`--manual-torrent-ui-probe --screenshot-output=PATH` opens Super Mario Bros. by
its exact preserved UUID, creates and parses a real two-file `.torrent` through
the production worker, verifies the locked association, v1 info hash, safe file
inventory, native `.torrent` import entry point, loaded-qBittorrent picker, and
second-file selection, then captures the native review dialog. The same dialog
can enumerate exportable v1 torrents already loaded in the configured
qBittorrent instance. Listing and metadata review are read-only. Confirming a
game download or collection source imports the torrent into Lunchbox management
by assigning its `lunchbox` category; Lunchbox also persists the validated
`.torrent` metadata and exact member inventory so a later on-demand selection
can recreate a missing qBittorrent entry.
Every actual enqueue, including Minerva and a one-game manual import, also
retains the exact bounded metadata in writable state with SHA-256 and v1-info-
hash receipts before qBittorrent is mutated. This lifecycle copy is independent
of whether the user registers the torrent as a reusable platform source.
The core test suite separately drives the production queue function through a
mock qBittorrent Web API and verifies exact-member priority plus durable source
and game provenance without contacting any content provider. The reusable
torrent-source tests additionally prove that registration is idempotent, queues
no payload, resolves only exact platform/title keys, and forces selective
downloads even when the global whole-torrent preference is enabled.
`--settings-ui-probe` leaves the native settings dialog open, including the
post-import seeding policy, for deterministic visual inspection.
`--settings-seeding-probe --state-database EMPTY_PATH` selects the pause policy,
saves it through the asynchronous CXX-Qt settings bridge, and exits only after
the save completes. Inspecting or reusing the state path verifies persistence
and restart recovery.
`--settings-release-probe --state-database EMPTY_PATH` similarly saves Japan
plus original-release preferences through the real bridge. The
`--settings-media-priority-ui-probe --state-database EMPTY_PATH` route moves
the last provider to first place through the same CXX-Qt editor, saves the
complete validated provider set, starts the real off-thread media reindex, and
exits only after verifying the resulting nine-row order; adding
`--screenshot-output ABSOLUTE_PATH` captures the populated priority surface.
The saved order governs all cached artwork, video, and manual candidates and
falls back to the complete default only when an older state database has no
valid order. The
`--profile-backup-ui-probe --state-database EMPTY_PATH --screenshot-output
ABSOLUTE_PATH` route opens the native profile card, requires both actions to be
ready after asynchronous settings initialization, captures the rendered card,
and exits with explicit readiness evidence. The underlying
`.lunchbox-profile` is a bounded, versioned ZIP containing a consistent online
SQLite backup, a deny-unknown-fields manifest, and only the exact files of
receipt-owned installed Couch Mode themes. Size and SHA-256 receipts cover every
payload entry; duplicate, extra, missing, unsafe, symlink, corrupt, foreign-key,
migration-incompatible, and mismatched-theme payloads fail closed. Export
extracts and validates the finished archive before atomic publication. Restore
repeats that validation into an owned staging directory, remains inert until the
user quits, then activates before QML or application models open. The current
WAL is checkpointed first; state and theme stores are renamed aside and restored
if migration or activation validation fails. A staged restore can be cancelled
off-thread without changing the live profile. ROMs, torrent data, media caches,
emulator binaries, save files, discovery catalogs, and operating-system
credential-store values are excluded. Absolute native paths are preserved
without Windows/Unix rewriting and therefore remain visible to Library Audit for
explicit cross-host repair.

The
`--controller-probe` route performs a non-Qt real-host inventory and prints the
native controller, InputPlumber service, managed-device, target, and warning
counts. Linux inventory reads the native joystick metadata; Windows and macOS
use the same GilRs backend as Couch Mode and expose synthetic `gilrs://` source
identifiers instead of assuming `/dev` or drive-letter paths.
`--controller-ui-probe --state-database EMPTY_PATH` opens the same asynchronous inventory in the Qt
settings dialog, scrolls to its polished player-order/remap surface, and prints
readiness only after the real provider result has crossed the CXX-Qt bridge.
Launch-time mapping controls are enabled only when the Linux InputPlumber
provider is available and its managed-device inventory is reachable. On Linux,
saved mappings are activated immediately before emulator spawn and an RAII
launch session restores captured InputPlumber intercept, profile, target, and
device-order state after normal exit, launch failure, or process-supervision
error. Applying those mutations still requires a release gate with attached
InputPlumber-managed hardware; Windows and macOS launch-remapping adapters
remain planned.
The adjacent Rust-backed profile designer exposes the legacy Xbox,
PlayStation, and generic controller diagrams without adding a web view. It
stores only explicit target-to-source differences, validates the complete
profile before it can enter settings state, and removes exact player,
platform, game, and default references when a profile is deleted. The
`--controller-profile-ui-probe` opens the editor against a fresh state database,
applies the two-button preset, and reports its real button and mapping counts.
`--alternate-title-probe --games-database PATH --minerva-database PATH` loads
Pokémon Silver by its stable catalog UUID, proves its alternate names came from
the same exact LaunchBox database ID, and succeeds only when one of those names
matches a real current Minerva torrent member. `--alternate-title-ui-probe`
opens the same record with its localized names rendered as native metadata
chips; adding `--screenshot-output ABSOLUTE_PATH` captures the details pane and
exits. Alternate names expand download search only: they never merge catalog
records, replace UUIDs, or bypass explicit candidate review.
`--variant-probe --games-database PATH` exercises the preserved Super Mario
Bros. release family without starting Qt. It requires one current row, unique
full-title presentation entries, and an exact UUID round trip to the Europe
release. `--variant-ui-probe` performs that switch through the production
CXX-Qt accessors, verifies the exact destination UUID, scrolls to the native
release carousel, and optionally captures it with `--screenshot-output`.
Grouping is bounded to one exact platform plus contiguous recognized trailing
region, language, release-state, or revision tags. It is navigation only:
records, metadata, LaunchBox IDs, and local state are never merged or copied.
The desktop `More Like This` shelf follows the same identity boundary. The
details worker reads at most 6,000 deterministic catalog candidates selected by
exact series, developer, publisher, or platform metadata, adds exact IDs with
local metadata overrides, applies effective adult/non-retail preferences, and
ranks at most 12 results by series, normalized title/franchise terms, creators,
genres, platform, play mode, and release-year proximity. Same-platform release
siblings already represented by the Releases carousel are excluded. The final
availability pass uses the native installed-identity set and exact Minerva
platform coverage; recommendation metadata never establishes identity.
`--related-games-ui-probe --screenshot-output PATH` opens the preserved Super
Mario Bros. UUID, validates a bounded non-self result with visible relationship
and availability data, captures the real Qt shelf, then selects its first card
and exits only after the destination stable UUID has loaded.
The `--release-candidate-ui-probe` route opens Super Mario Land, inspects its live
Minerva bundle, and scrolls the details pane to the ranked native candidate
cards for deterministic visual review against the preserved discovery catalog.
`--multidisc-ui-probe` opens the preserved Sony PlayStation Final Fantasy VII
record, inspects its live Minerva sources, and opens the exact multi-disc review
dialog when a plan is found. The reusable review component runs the same native
free-space, destination-collision, and existing-download preflight as every
ordinary single-file GET action. Queue remains disabled until it succeeds, and
enqueue repeats the complete gate immediately before qBittorrent mutation.
With `--screenshot-output`, the probe waits for that asynchronous result,
captures the rendered ready or actionable blocked state, and exits without
starting a download. This route verifies real provider filenames, aggregate
size, playlist naming, storage disclosure, and dialog layout.
`--laserdisc-ui-probe` opens the preserved Arcade Laserdisc Dragon's Lair
record, resolves the live MAME/Hypseus/Daphne source views, and opens the first
strict machine-layout review dialog. It exercises the real canonical MAME
set-name evidence and current Minerva torrent metadata without queueing data.
`--exo-archive-ui-probe` opens the preserved MS-DOS Prince of Persia record and
the live eXo related-archive review dialog. It verifies the canonical default
game archive, same-name game data, Linux metadata, shared utilities, aggregate
size, role labels, and layout without starting a download.
`--exo-prepare-probe --state-database FIXTURE_PATH` opens the fixed
`exo-prepare-probe` installed identity, runs the real preparation action through
the CXX-Qt model, and exits only after the cache is verified ready. Reusing the
same state and archive paths verifies idempotent cache reuse.
`--exo-prepare-ui-probe` leaves that game's details panel open for visual review
of the PC-install card and prepared path.
`--exo-launch-probe` uses the same prepared identity, canonical emulator
catalog, asynchronous Qt actions, and real process supervisor. It launches the
detected emulator, prints the exact program/argument-vector evidence, waits for
exit, and then closes. On the current Linux host this route discovered and ran
`com.dosbox_x.DOSBox-X` through Flatpak against a fresh prepared cache; DOSBox-X
loaded the exact generated config and exited successfully.
`--initialize-state --state-database FIXTURE_PATH` idempotently creates or
migrates the Rust-owned local state without starting Qt. `--rom-launch-probe`
opens an exact imported-file identity, discovers the installed runtime/core,
launches it through the normal CXX-Qt action, bounds the emulator session, and
exits only after process cleanup and the finalized activity row has returned to
Qt. It prints both started and finalized play-count/time evidence. On the
current Linux host it ran Flatpak RetroArch with `fceumm_libretro.so`, loaded
the owned Faxanadu NES file, created a real Wayland/OpenGL session, and persisted
one terminated session with measured elapsed time. `--rom-launch-ui-probe`
leaves the same `Play Locally` controls open for visual inspection;
`--launch-profile-ui-probe` waits for that exact runtime/core discovery, opens
the native launch-command editor, verifies its selected runtime and compiled
argument vector, and scrolls it into view. Supplying
`--screenshot-output ABSOLUTE_PATH` captures the rendered details pane through
Qt itself and prints `LUNCHBOX_SCREENSHOT_READY`, avoiding host-specific
window-manager screenshot tooling.
`--launch-profile-manager-ui-probe` loads all exact catalog rows on the native
worker, filters to the platform-scoped standalone MAME arcade template, opens
its focused editor, rejects an unavailable placeholder, restores the valid
inherited draft, and uses the same absolute screenshot argument to capture the
virtualized manager body before exiting.
`--activity-ui-probe` opens that persisted identity in the native Recently
Played view with its statistics and completion picker visible.
`--activity-history-ui-probe --state-database EMPTY_PATH` refuses the default
profile, idempotently seeds four exact Super Mario Bros. sessions into the
isolated SQLite state, and opens the real Game Details model against the full
discovery catalog. It verifies newest-first completed/interrupted/failed order,
the failed-only filter, aggregate play count, and bounded native accessors.
Adding `--screenshot-output ABSOLUTE_PATH` captures the 920-by-760 dialog after
Qt renders its localized timestamps, emulator names, durations, badges, filter
counts, and virtualized rows, then prints
`LUNCHBOX_ACTIVITY_HISTORY_UI_READY` and exits.
`--media-bundle-probe --media-directory PATH --state-database EMPTY_PATH`
opens the preserved Super Mario Bros. identity, discovers its exact cached
video and manual below the configured media root, waits for Qt Multimedia to
decode and play the real video, seeks to 12 seconds, persists the checkpoint
through the CXX-Qt model, and exits only after the SQLite worker returns.
`--media-bundle-ui-probe` leaves the same polished media card open for visual
inspection. The card uses Qt Multimedia for cross-platform playback and Qt's
native URL handler for manuals; Rust never invokes a shell or constructs an
OS-specific opener command.
The desktop grid uses the same provider-ordered supplemental-media cache after
a 500-ms pointer-hover intent delay. Keyboard focus and visibility never request
gameplay video. `LibraryModel` resolves the exact stable UUID to the catalog's
own positive database ID, performs the disk scan on a named worker, and accepts
the result only when both its generation and requested UUID still match. The
virtualized delegates contain only their
`VideoOutput`; one root-level `MediaPlayer` and `AudioOutput` follow the active
card. The player loops muted by default and disables both its audio track and
device sink until the user opts into sound, so AAC-bearing previews still load
their video without touching the host audio backend. The card exposes source
provenance and an opt-in sound control, and every leave, focus change, open, ID change, or
delegate destruction cancels the pending generation. A bounded serial
EmuMovies worker only queues identities explicitly opened or hovered for the
full delay, waits for credential initialization before starting FTP, and
republishes a completed path to both the hover player and the selected-game
media model. Definitive provider misses are remembered,
while credential and transient FTP failures remain actionable and retryable.
Queue, lookup, transfer progress, cached readiness, exact provider misses, and
transient failures are retained per stable game identity. Grid and details
cards therefore never display an unrelated global media result, and the
hover-card action retries only that exact title or routes a missing account to
EmuMovies Settings.

Both the shared hover player and the details player retain the legacy bounded
local-playback recovery policy. A decoder or file-publication race clears the
failed source and reloads that same exact local URL up to two times without
showing a transient error. Loading or playing resets the budget, changing games
cancels a pending retry, and only an exhausted exact source reaches the visible
playback error state. `RetryingMediaPlayer.qml` keeps the multimedia and retry
wiring outside the composition root, while `MediaRetryController.qml` owns the
tested reload policy. Deterministic QML tests cover reload, exhaustion, success
reset, and source replacement.

The box-art video surface remains behind the opaque cover until its
`QVideoSink` delivers a decoded frame; `PlayingState` alone is not treated as
render readiness. Keeping the requested sink visible behind the cover lets
backends that stop producing frames for hidden or zero-opacity outputs complete
the handoff. The artwork therefore remains the presentation during expansion,
decoder startup, stop, and collapse instead of exposing Qt's black clear
frame. Once the 500-ms intent gate has elapsed, a selected card also reuses the
same exact local video URL already resolved by the game-details model while the
hover cache lookup completes. A successful cache lookup clears any stale
per-session EmuMovies miss, and a download completion forces a new cache scan
even when an older scan is still in flight. `HoverPreviewPresentation.qml`
owns and tests the resolved-source and first-frame handoff outside the
composition root.

`--hover-preview-ui-probe --preview-video-fixture PATH --media-directory
EMPTY_PATH` requires an explicit non-empty video fixture of at most 64 MiB,
publishes it idempotently under the stable Super Mario Bros. cache identity,
loads the complete catalog, forces the grid presentation, focuses that exact
UUID, and exercises the ordinary delay, worker lookup, and shared player. It
puts the card against a viewport edge and prints
`LUNCHBOX_HOVER_PREVIEW_UI_READY` only after Qt reports a video track, the
fixture duration, `PlayingState`, a decoded-frame handoff, and a genuinely
enlarged card wholly inside the live grid bounds. The first-frame assertion is
a rendered-host gate; a headless backend whose video clock remains at zero is
expected not to claim success. The probe deliberately does not use
Qt Quick's item grab: an Xvfb clock can remain at zero without a working
hardware backend, and a grab cannot prove that a video texture itself rendered.
Visual acceptance of the live texture remains a release-host check.
Grid covers decode once at the largest possible hover-card size instead of
rebinding `Image.sourceSize` to animated geometry. Qt retains that texture while
loading, and the cover stays opaque underneath the video layer so stopping a
preview cannot expose the artwork mat. The independent
`--hover-artwork-transition-ui-probe --preview-artwork-fixture PATH
--media-directory EMPTY_PATH` gate publishes a bounded real image fixture,
samples the ordinary card during both the 150-ms expansion and collapse, and
prints `LUNCHBOX_HOVER_ARTWORK_TRANSITION_UI_READY` only while the image remains
`Ready`, fully opaque, and at the same decode size in both directions. It exits
before the 500-ms video intent gate, keeping image-transition regressions
independent from host codec availability.
`--emumovies-auto-ui-probe --media-directory EMPTY_PATH` runs the same complete
grid-hover and playback gate without seeding a fixture. It loads the saved
operating-system credential, proves the tile has not started a preview halfway
through the 500-ms intent delay, then queues the still-hovered Super Mario Bros.
identity through the production EmuMovies FTP worker, downloads into the empty
media root, and exits only after Qt plays the newly published video. Visibility
alone requests artwork and cannot enqueue gameplay video. Opening a game is the
other intentional path and promotes that exact identity immediately. This is an
opt-in live service probe and never prints credential values.

Game music is a separate provider-aware supplemental-media path. The Rust
client deliberately searches only `/Official/Music/_HyperAudio`, where tracks
are individually downloadable, and never exposes the large multipart platform
packs as per-game actions. Platform aliases and the guarded exact/regionless/
bounded-fuzzy title matcher select one game folder; only MP3, FLAC, Ogg, Opus,
M4A, AAC, and WAV entries survive enumeration. Each selected remote path maps
to a deterministic SHA-256-derived filename below the exact game's
`emumovies/` directory, and a bounded control-character-free `.title` sidecar
preserves the display title. Downloads use the existing cancellable FTP stream,
per-file lock, guarded partial cleanup, and atomic publication. `GameDetailsModel`
indexes all cached tracks by provider/format priority, while
`GameSoundtrackCard.qml` owns discovery, setup routing, progress, track choice,
seek, and Qt Multimedia playback without adding another presentation block to
`Main.qml`. `--emumovies-soundtrack-probe --media-directory EMPTY_PATH` uses
saved credentials to discover and download the exact Super Mario Bros. track,
then requires the ordinary supplemental-media scanner to index it.
`--soundtrack-ui-probe` opens that same exact catalog identity and exits only
after Qt reports playable audio through the native Game Music card.
`--manual-candidate-probe --minerva-database PATH` reads the live Minerva
catalog and its bounded cached torrent metadata without starting Qt or
downloading game content. It exits successfully only when the fixed Cooking
Pico title resolves to one exact ZIP member and prints its info hash, index,
byte size, score, and reviewed path. `--manual-download-ui-probe` opens that
same catalog identity with the polished missing-manual action visible. The
normal Find action runs off the GUI thread, records a generic durable media
transfer, reuses qBittorrent's exact-file ownership checks, shows byte progress,
and can cancel one selection while leaving other manuals in the shared torrent
active. Completion requires a non-empty readable ZIP inside the configured
native torrent root; publication uses a unique same-directory temporary file,
flush, sync, and atomic rename under the configured media root.
`--box-3d-ui-probe` waits for the media index, opens the real Super Mario Bros.
catalog record, obtains its exact LibRetro front cover through the normal bounded
Rust queue, and switches the details hero to the native Qt Quick 3D box. The
probe prints `LUNCHBOX_BOX3D_READY` only after the exact local cover URL reaches
the viewer; the resulting frame is suitable for visual inspection under Xvfb.
`--arcade-launch-probe` exercises the same chooser and supervisor with a
persisted MAME standalone default. On the current Linux host it launched the
installed `org.mamedev.MAME` Flatpak with the exact collection/runtime ROM path
list and MAME's ROM-free `pong` driver, then exited successfully;
`--arcade-launch-ui-probe` leaves that state open for review.

`--library-view-ui-probe --state-database EMPTY_PATH --screenshot-output PATH`
loads the complete discovery catalog, switches to the native compact list,
chooses Publisher/Title/Rating/Availability/Platform, and selects ascending
Publisher order through the same CXX-Qt invokables used by the View popup,
column manager, and sortable headers. It exits only after the worker publishes
at least 250,000 retail-filtered rows, the first virtualized delegate exposes a
real publisher, and the rendered view has been captured. Starting a new process
with the same state database and `--library-view-restored-ui-probe` proves that
the presentation, exact column order, sort field, and direction survive a cold
start. The complete chooser includes Title, Platform, Developer, Publisher,
Year, Release Date, Genre, Players, Rating, ESRB, Co-op, Variants, Type, Series,
Region, and Notes from the legacy data-list plus native Availability. The
current 303,560-record fixture yields 280,466 visible rows after the default
non-retail filter; sorting is performed before the model reset on the catalog
worker rather than in QML or per delegate.

`--list-filter-ui-probe --state-database EMPTY_PATH --screenshot-output PATH`
opens the real Publisher facet over all 280,466 retail-filtered rows, selects
one exact publisher through the same adaptive multi-select controls used by the
header, and waits for the catalog worker to publish the facet's exact game
count. The captured native dialog exposes bounded search, full-catalog counts,
All/None actions, and active selection state. Column filters combine with AND,
compose with search/platform/tag/availability/content filters, and deliberately
remain session-only like the legacy implementation; column order and sort
continue to persist independently.

`--retroarch-shader-ui-probe --shader-target EMPTY_PATH --screenshot-output
PATH` opens Settings against an isolated target, runs the real Rust installation
worker, and exits only after both packs report `Managed`. Supplying
`--shader-archive-directory PATH` makes the same probe deterministic with local
fixtures; omitting it exercises the official Libretro sources and verified
cache. The real-source Linux probe installed 6,564 files (101.1 MiB), and a
second isolated target reused archives with unchanged SHA-256 digests.

`--couch-mode-ui-probe --screenshot-output PATH` opens the native Couch Mode shell
at a 1920x1200-class test size, filters the real catalog to the preserved Super
Mario Bros. family, restores the exact stable UUID for LaunchBox database ID
140, loads its real details and cached media, captures the full presentation,
starts the real gamepad backend, and exits only after the selected row, details
identity, filtered-row lookup, and backend readiness agree. Pass
`--media-directory PATH` to exercise an existing media cache. The interactive
Couch Mode control enters true fullscreen and Escape restores the desktop
window. Its hidden shelf is inert: selection and details loading begin only
after Couch Mode becomes active, so constructing or filtering the desktop
cannot silently replace the user's current game. Couch Mode owns only transient
overlay state: Details and Game Menu consume the same `GameDetailsModel`, stable
UUID, favorite persistence, launch, and download/setup handoff used by desktop
mode. Details is scrollable without a mouse. Game Menu provides the real dynamic
primary action, favorite toggle, explicit Desktop Details handoff, an exact
regional/version release-and-media picker when alternate records exist, Attract
Mode entry, persistent cinematic-wheel/cover-shelf switching, and Return to
Browsing; closing or switching panels never changes the selected release. The
release picker is virtualized, highlights the current
record, exposes current/installed/Minerva/catalog-only state plus cached-media
context, and routes a chosen row by its exact stable UUID without leaving Couch
Mode.

When Play is started from Couch Mode, the same `GameDetailsModel` launch
signals drive a full-screen launch-status surface. `launch_busy` presents the
startup phase while the worker builds the exact command and writable runtime;
`game_running` presents the active emulator/process state and exact emulator
name; and the final launch status remains visible until the user returns to the
shelf or hands the record to Desktop Details. The overlay is controller-safe,
does not duplicate launch state, and cannot claim pause/resume: those controls
remain a separate process-host adapter requirement for Linux, Windows, and
macOS. `--couch-launch-ui-probe --screenshot-output PATH` runs the real local-ROM
launch probe through Couch Mode, captures the active-process surface after the
supervisor reports the child, and exits only after the normal activity
finalization path returns to Qt.

`--couch-variant-ui-probe --screenshot-output PATH` opens that release picker for
the real Super Mario Bros. release family, captures its 1920x1200 surface, and
chooses a non-current regional record. It exits only after the Couch shelf and
the shared details model both report the target stable UUID and loading has
settled.

`--install-management-ui-probe --state-database EMPTY_PATH
--install-management-fixture-root EMPTY_DIRECTORY --screenshot-output PATH`
idempotently seeds one exact owned file for the real Super Mario Bros. catalog
record, captures the native Game Files card, accepts the real confirmation,
and exits only after the receipt, file, and local state have been removed.
Starting a new process with the same state database and
`--install-management-restored-ui-probe` proves that the removal survives a
cold catalog load. The fixture refuses to replace unexpected existing bytes.

The Couch Mode `Platforms` shelf opens a virtualized picker over the same 191
platform rows already owned by `LibraryModel`; it does not copy or reload the
catalog. Each row exposes the exact catalog name and live game count. Selecting
a row applies the ordinary exact platform filter, while a coalesced Rust worker
persists the shelf plus platform to the writable state database. Invalid or
stale saved combinations fall back safely to All Games. The fresh
`--couch-platform-ui-probe` and cold-start
`--couch-platform-restored-ui-probe` exercise keyboard/controller-equivalent
movement, exact SNES selection, a 6,008-row real-catalog result, persistence,
and the restored 1920x1200 picker capture. Xvfb capture uses
`QT_QPA_PLATFORM=xcb` to keep the test on the isolated display.

The Couch Mode `Collections` shelf is another presentation over the existing
manual and smart collection model. Its virtualized picker shows the exact name,
description or rule summary, kind, and live game count without copying members.
Selection and persistence use `collection:<stable UUID>` rather than display
text. A renamed collection therefore restores correctly, while a deleted,
missing, or otherwise stale ID falls back to All Games and is never guessed by
title. Deleting the currently saved Couch Mode collection also coalesces that
safe fallback through the ordinary state worker. The fresh
`--couch-collection-ui-probe` creates an exact one-game manual shelf, exercises
controller-equivalent selection, captures the 1920x1200 picker, and verifies
the routed filter and persisted ID. A new process using the same state database
and `--couch-collection-restored-ui-probe` proves cold restoration to the same
collection and Super Mario Bros. UUID.

Attract Mode is a presentation state over the active virtualized shelf, not a
second model, query, or demo catalog. It can be entered from Game Menu or by an
idle timer, advances with the persisted cycle interval, wraps exact rows, and
uses the same stable UUID, details, artwork, favorite, launch, and Minerva state
as normal Couch Mode. Directional input moves immediately; accept/menu opens
the exact game's menu; details and favorite retain their usual semantics; back
returns to the unchanged shelf. The native state database validates and stores
enable, 30--3,600 second idle, and 5--120 second cycle values through the same
coalesced Rust save worker as shelf/platform navigation. An idempotent schema
migration gives older profiles safe defaults without changing their saved
navigation. Settings exposes curated living-room choices.
`--couch-attract-ui-probe` and `--couch-attract-restored-ui-probe` exercise idle
entry, wrapped next/previous identity, browsing return, settings persistence,
and fresh/cold 1920x1200 captures against a 56-row real-catalog filter;
`--couch-attract-settings-ui-probe` captures and validates the desktop controls.

Couch Mode has two presentations over the same virtualized `LibraryModel`. The
cinematic wheel keeps the selected game's hero, metadata, and actions on the
left while a controller-first vertical wheel occupies the right; the cover
shelf retains the horizontal box-art presentation. The validated `wheel` or
`shelf` identity is saved in `couch_mode_preferences`, migrated safely for old
profiles, exposed in full-window Settings, and switchable from the header,
keyboard `V`, or the Game Menu. Switching recenters the same exact stable UUID
instead of selecting by row or title. `--couch-view-style-ui-probe` navigates the
real wheel vertically, switches both ways, verifies identity preservation, and
captures the final 1920x1200 surface. Reusing its state database with
`--couch-view-style-restored-ui-probe` proves cold restoration.

Couch Mode appearance is selected by exact theme ID and stored with its existing
navigation and Attract Mode preferences. Three built-in themes are always present.
Settings exposes a virtualized palette preview and installs or removes declarative
`.lunchbox-theme` ZIP packages on a Rust worker. The closed schema controls eight
palette tokens, an optional signature-checked PNG/JPEG/WebP background, hero scrim,
and card radius. It accepts no QML, scripts, commands, fonts, plugins, symlinks, or
undeclared files. Package size, expanded size, entry count, text, color, path, and
image format are bounded before an atomic publish. Exact SHA-256 ownership receipts
guard updates and removal, so Lunchbox never replaces or recursively deletes a
foreign directory. Invalid or missing selected themes fall back to the built-in
default with an actionable warning. The full versioned format is documented in
`COUCH_MODE_THEMES.md`.

`--couch-theme-ui-probe --state-database EMPTY_PATH --screenshot-output PATH`
creates a deterministic real package, installs it through the production validator,
persists its exact ID, starts the 303,560-game catalog, and enters Couch Mode at
1920x1200. It exits only after Rust and QML agree on four available themes, the exact
`ultraviolet-circuit` identity, background and accent colors, and 24-pixel radius,
then captures the full themed surface. Reusing the same state exercises idempotent
installation and restart restoration.

`GamepadInput` is a CXX-Qt Rust service backed by GilRs. It owns a named worker
thread, performs blocking device reads there rather than on the GUI thread,
publishes hotplug snapshots through the Qt event queue, and disables unused
force feedback. D-pad and left-stick input share one QML navigation-action path
with keyboard input; 0.68/0.38 analog engage/release thresholds prevent center
jitter, while a 380 ms initial delay and 90 ms repeat interval make long shelves
fast without accidental double moves. Directional input, five-item paging,
accept, back, favorite, details, category focus, and shelf-home are mapped.
Focus is restored on entry, every handled action, and application reactivation.
Controller-family inference changes the visible Xbox, PlayStation, Nintendo, or
generic button hints without changing semantic actions. The worker remains
listening after Couch Mode closes so re-entry and hotplug status are immediate;
the inactive QML view ignores its actions.

`--couch-gamepad-ui-probe --screenshot-output PATH` uses the guarded native test
seam to inject the same semantic actions that the worker publishes. Against the
real 303,560-game catalog it proves right movement, exact stable-UUID restoration
after left movement, action/shelf focus movement, opening Game Menu, traversing
all six actions including the release picker and Attract Mode, returning without identity loss,
opening Details, and switching between panels before the final 1920x1200
Details capture. Pass
`--couch-gamepad-menu-screenshot-output PATH` instead to stop on and capture the
verified Game Menu. Xvfb runs set `QT_QPA_PLATFORM=xcb` so Qt cannot escape to an
ambient Wayland session. The ordinary Couch Mode probe separately starts the
real GilRs backend and waits for it to report ready. No controller was attached
to the current Linux host, so attached-device input and hotplug remain explicit
Linux, Windows, and macOS release-host gates rather than inferred from the
deterministic routing probe.

`--firmware-probe` opens an imported Coleco ADAM identity, runs normal emulator
discovery, prints the exact resolved rule counts and runtime path, and exits
only after firmware state reaches Qt. `--firmware-ui-probe` leaves that state
open so the selected installed MAME runtime's five required packages, Minerva
source, native target, and acquisition controls can be reviewed visually.
`--firmware-audit-ui-probe --state-database EMPTY_PATH --screenshot-output
PATH` seeds an isolated present Coleco ADAM game plus the exact standalone MAME
preference, then runs the production whole-library audit. It exits only after
the real installed Flatpak MAME runtime resolves all five canonical missing
Minerva packages through Rust, those rows reach the virtualized Qt view, and
the complete 1920x1200 management surface is captured. The seed is idempotent,
refuses a non-fixture directory or non-isolated writable state, and never
creates firmware receipts.
`--download-history-probe --state-database FIXTURE_PATH` loads real terminal
queue records, clears them through the asynchronous Qt action, and exits only
after the refreshed model reports zero finished records. The store refuses to
delete active, paused, queued, completion-pending-import, or
post-import-action-pending records.

`--download-recovery-ui-probe --state-database EMPTY_PATH --screenshot-output
PATH` seeds one deterministic failed exact-file job through the production
settings store, opens its native history surface, and exits only after the
CXX-Qt model reports one retryable job and the three newest-first transitions
`FAILED`, `DOWNLOADING`, and `QUEUED`. State transitions and explicit recovery,
import, and post-import outcomes are retained against the stable queue job ID;
deleting that terminal record intentionally cascades only its event history.
Retry re-reads the exact durable job and combines every active or failed member
of the same torrent. If the info hash still belongs to the isolated `lunchbox`
category it restores the reviewed priorities; if it was removed, Lunchbox adds
the exact retained metadata at the persisted client save path. It then requests
a data recheck and resume while preserving acquisition provenance, download
plans, and native mappings. Active jobs use this same path automatically when
refresh observes that the owned torrent disappeared. A foreign same-hash
torrent remains untouched. An older manual job without retained bytes fails
actionably; no display title or network search can substitute identity.

On the current Linux development host, the real preserved 303,560-game catalog
loads on its worker in roughly 0.5–0.8 seconds in release mode, while a combined
text plus Minerva filter completes in about 16 ms. The existing 8 GB media tree
currently contains 6,983 valid cached candidates for 6,679 games; the latest
packaged release probe indexed them in 102 ms without delaying catalog
readiness. A real empty-cache NES cover retrieval completed in 7.944
seconds and atomically stored a validated 608,533-byte PNG. The latest three
headless Xvfb shell-ready probes measured 273--276 ms; the 250-ms release target
remains a performance gate rather than being reported as met by this run. A
real UI-driven favorite write completed in 28--30 ms, and a second release launch
loaded the persisted UUID, count, card star, and details action.

The Nix application package extracts the verified database artifact at build
time and points the executable at the unpacked database. Users do not pay a
decompression cost at application startup.

## Incremental feature migration

The Electron implementation remains on `legacy/electron` as the specification
for Lunchbox-owned behavior. Features should move as vertical slices: model and
service, native UI, error and cancellation behavior, tests, then a measured
runtime gate. Catalog details, Minerva bundle inspection, persistent
qBittorrent setup, exact-file selection, queue control, completed-file
ingestion, optical and eXo related-file plans, eXo prepared installs and launch,
durable safe seeding policy, exact owned-torrent recovery with durable event
history, local-folder scan/review/import, legacy
content filters, and durable favorites are now native. Existing game media is
now indexed and rendered natively, and exact Minerva manual acquisition has
durable transfer, progress, shared-torrent cancellation, and safe publication
through the same details card. On-demand LibRetro retrieval and durable
negative caching, generic local-ROM launch, multi-file selection, and exact
per-game/platform emulator defaults are native. Exact game/platform/global
launch argument profiles and replacement templates are native for ordinary ROM
and prepared-PC plans, with contextual and searchable cross-emulator editors
plus a shell-free placeholder compiler.
Cached artwork rotation and safe explicit LibRetro refresh are also native.
SteamGridDB, IGDB, ScreenScraper, EmuMovies FTP, and explicit Web artwork are native behind one Find Artwork
workflow. Settings
stores the SteamGridDB key, IGDB/Twitch client-credentials payload, and optional
ScreenScraper WebAPI v2 credentials in the
native credential store, never SQLite. A provider search returns candidates
only; a human must choose the exact game before that provider's stable ID is
recorded against the positive LaunchBox catalog ID. SteamGridDB then presents
static backgrounds, covers, or logos; IGDB presents covers, artwork, or
screenshots using an in-memory cached Twitch application token. Both providers
use the same 16 MiB, HTTPS-only (except loopback fixtures), PNG/JPEG/WebP
signature-validating publisher and atomically replace only their own media kind.
IGDB attribution and the non-commercial/commercial-partnership boundary are
visible in Settings. ScreenScraper setup is explicitly authorization-gated and
links a positive provider game ID only after review. The ordinary visible-media
worker follows the saved provider order through LibRetro, SteamGridDB, IGDB,
exact EmuMovies archive title members, and ScreenScraper. It reuses reviewed
stable IDs, but may also consume one unambiguous strict exact title result from
SteamGridDB, one strict exact title/platform result from IGDB, or one strict
exact title result from a platform-constrained ScreenScraper search. Those
transient matches are not persisted as identity links. Explicit refresh
bypasses fresh provider-miss markers while every provider remains isolated to
its own cache directory.

Web artwork deliberately does not reproduce the legacy DuckDuckGo first-result
scraper. The system browser receives a category-specific Google Images query;
Lunchbox accepts
only one user-reviewed HTTPS image URL or local file. A generation-guarded Rust
worker downloads or copies the candidate into an opaque, per-candidate process
quarantine, rejects files over 16 MiB and non-PNG/JPEG/WebP signatures, and
exposes only that validated local copy to QML. Confirmation promotes the exact
quarantined bytes atomically into the selected game, category, and `websearch`
provider directory. Cancellation and stale completions cannot publish or
replace another candidate.

EmuMovies preserves the legacy Rust backend rather than routing through a new
web API. `EmuMoviesModel` is only the CXX-Qt boundary: it loads forum credentials
from the native keyring, starts the existing blocking FTP archive/video/manual
operations on named Rust workers, and generation-guards every completion before
refreshing the shared media index. Its progress callback is also a cooperative
cancellation contract: the worker checks it at discovery boundaries and every
64-KiB data chunk, sends FTP `ABOR` for an active transfer, and keeps the Qt model
busy until the transport acknowledges completion. Artwork archives, videos, and
manuals stream into same-directory guarded temporary files; only a completed
transfer is atomically renamed, while every cancelled or failed partial is
removed. `EmuMoviesTransferStatus.qml` presents this state, determinate progress,
and cancellation consistently in Settings and Find Artwork without expanding
`Main.qml` with another inline status surface. The port retains legacy platform aliases,
title normalization, archive matching, direct-file matching, cache indices,
timeouts, and the LaunchBox-ID-to-MAME parent/clone resolver. That resolver is
generated only from a user-owned `Arcade.xml` supplied by
`LUNCHBOX_ARCADE_XML` or the ignored local migration-data directory; public
packages do not redistribute LaunchBox metadata and safely fall back to the
catalog title when the local lookup is absent.

`--steamgriddb-ui-probe` exercises this complete QML workflow against an API
fixture selected with `LUNCHBOX_STEAMGRIDDB_API_URL`: search, exact-game review,
artwork enumeration, rendered preview, screenshot, validated publication, and
the durable provider link. `LUNCHBOX_STEAMGRIDDB_API_KEY` is accepted only as a
runtime override for unattended verification; interactive configuration remains
in the credential store.

`--igdb-ui-probe` exercises Twitch token acquisition, Apicalypse game search,
exact-game review, provider-link persistence, IGDB artwork enumeration, rendered
preview, screenshot, and validated publication against the deterministic Rust
fixture server in `crates/lunchbox-app/examples/igdb_fixture.rs`. The credential
and endpoint environment variables exist only to make unattended verification
independent of a live account; interactive credentials remain in the keyring.

`--web-artwork-ui-probe --media-directory EMPTY_PATH --screenshot-output PATH`
creates a valid local PNG before Qt starts, runs it through the ordinary
quarantine worker, captures the populated 1920x1200 Find Artwork dialog, and
confirms only after QML observes the validated candidate. It exits successfully
only after the exact bytes appear at
`lb-140/websearch/box-front.png` through the production atomic publisher.

Playlist-entry-specific presentation, the remaining alternate media providers,
specialized machine launch profiles, the remaining settings, broader
manual-provider coverage, multidisc/patch archive management, and the
controller-first full-screen interface follow on the same shared models.
