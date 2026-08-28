# Native frontend architecture

Lunchbox uses Qt 6 Quick for rendering and CXX-Qt for typed communication with
Rust. There is no handwritten C++, embedded web runtime, HTTP application
server, or shell-script launcher.

## Responsiveness contract

The desktop shell is created entirely from compiled-in QML. Database, network,
filesystem hashing, image decoding, emulator discovery, and torrent operations
must never run before the first shell-ready event or on the Qt GUI thread.

The initial implementation enforces this shape:

- `LibraryModel` is a native `QAbstractListModel`; Qt views request only roles for
  visible rows.
- Both grid and list views reuse delegates and keep a bounded one-viewport
  cache.
- SQLite opens read-only on a named worker thread after the window is created.
- Search, platform, and availability filtering run on worker threads. A
  generation number discards stale results when the user types quickly.
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
  no network request runs on the Qt thread or gates startup. LibRetro responses
  must be PNG files of at most 8 MiB and are published with a same-directory
  atomic rename. Provider misses are cached per stable LaunchBox ID and media
  type for seven days. `--offline` or `LUNCHBOX_OFFLINE=1` makes the same UI
  strictly cache-only.
- The media index retains every valid candidate in deterministic media-type,
  provider, format, and path order. The details hero can rotate those candidates
  without rescanning the filesystem. Explicit refresh bypasses the miss cache
  and replaces only the owned LibRetro target after write and sync; overwrite
  fallback restores the prior file on publication failure and never removes
  alternatives from other providers.
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
- Bounded torrent bytes are retained in a validated, URL-keyed OS cache for 24
  hours. The currently inspected torrent also has one decoded file index shared
  by its source views, so a large multi-platform torrent is not downloaded or
  expanded repeatedly. All fetch and decode work remains off the Qt thread.
- Generic optical multi-disc matches become one versioned download plan. Disc,
  disk, CD, side, part, volume and card markers support numeric, alphabetic and
  common Roman forms; the planner chooses one runnable primary per disc and
  includes CUE, CCD, MDS and GDI companion formats. The confirmation dialog
  exposes every exact torrent member and total size before qBittorrent changes.
  One persisted queue row selects and monitors the complete set with
  size-weighted progress. Import preflights every source and conflicting target,
  preserves the common relative layout, and publishes the M3U and installed-game
  record last. Retries are idempotent and never overwrite different content.
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
  Updating, Updated, or Failed state with the underlying error. Partial source
  failures are warnings; direct downloads and Libretro buildbot cores without a
  reliable comparable version are not presented as updates.
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
  `utf8`) instead of using a display string as identity.
- Library maintenance runs on the same bounded scanner contract but prepares
  the catalog match index once for the entire audit. Each root either completes
  atomically for availability purposes or is reported as offline; a partial or
  failed scan can never manufacture missing records. Persisted SHA-1, then MD5,
  then size/time evidence classifies changes, while exact current SHA-1 groups
  identify duplicates without merging them. The CXX-Qt model owns compact
  filter indices and selection state. Confirmed cleanup accepts stable UUIDs
  only and its SQL predicate can remove only included rows already marked
  `missing`; filesystem paths are not part of the mutation API.

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
exiting. Unlike the shell probe, this waits for the asynchronous catalog worker.
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
`--favorite-probe --state-database EMPTY_PATH` selects a real catalog game,
persists it through the native Qt action, prints `LUNCHBOX_FAVORITE_READY_MS`,
and exits only after the worker reports success. `--favorite-ui-probe` leaves
the same grid, star affordance, and details pane open for visual inspection.

Game metadata edits are nullable local overlays keyed only by stable game UUID
in the writable state database. The immutable catalog title and platform remain
the inputs to provider lookup, acquisition, artwork, launch history, and ROM
identity; only presentation, search, sorting, smart title rules, details, and
couch-mode labels consume the effective overlay. Empty editable fields can be
cleared explicitly, while saving values identical to the catalog removes those
columns and restoring the catalog deletes the row. `--metadata-ui-probe
--state-database EMPTY_PATH --screenshot-output ABSOLUTE_PATH` first proves the
custom title is absent, edits the exact Super Mario Bros. UUID through the real
native dialog, persists it off-thread, refreshes the library, proves the custom
title is searchable, and asserts that the canonical title is still unchanged.

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

`--import-ui-probe --import-directory PATH` opens the local-import surface and
starts a real checksum scan. It exists for deterministic virtual-display visual
checks; it uses the same model, worker, database, and QML as an interactive
folder selection. Adding `--import-commit-probe` selects the readable results
and commits them to the chosen `--state-database`, enabling a fully unattended,
idempotent scan-to-collection integration test.
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
`--controller-probe` route performs a non-Qt real-host inventory and prints the
native controller, InputPlumber service, managed-device, target, and warning
counts. `--controller-ui-probe --state-database EMPTY_PATH` opens the same
asynchronous inventory in the Qt settings dialog, scrolls to its polished
player-order/remap surface, and prints readiness only after the real provider
result has crossed the CXX-Qt bridge. On Linux, saved mappings are activated
immediately before emulator spawn and an RAII launch session restores captured
InputPlumber intercept, profile, target, and device-order state after normal
exit, launch failure, or process-supervision error. Applying those mutations
still requires a release gate with attached InputPlumber-managed hardware.
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
The `--release-candidate-ui-probe` route opens Super Mario Land, inspects its live
Minerva bundle, and scrolls the details pane to the ranked native candidate
cards for deterministic visual review against the preserved discovery catalog.
`--multidisc-ui-probe` opens the preserved Sony PlayStation Final Fantasy VII
record, inspects its live Minerva sources, and opens the exact multi-disc review
dialog when a plan is found. This route verifies real provider filenames,
aggregate size, playlist naming and dialog layout without starting a download.
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
the native launch-command editor, and scrolls it into view. Supplying
`--screenshot-output ABSOLUTE_PATH` captures the rendered details pane through
Qt itself and prints `LUNCHBOX_SCREENSHOT_READY`, avoiding host-specific
window-manager screenshot tooling.
`--launch-profile-manager-ui-probe` loads all exact catalog rows on the native
worker, filters to the platform-scoped standalone MAME arcade template, opens
its focused editor, and uses the same absolute screenshot argument to capture
the virtualized manager body before exiting.
`--activity-ui-probe` opens that persisted identity in the native Recently
Played view with its statistics and completion picker visible.
`--media-bundle-probe --media-directory PATH --state-database EMPTY_PATH`
opens the preserved Super Mario Bros. identity, discovers its exact cached
video and manual below the configured media root, waits for Qt Multimedia to
decode and play the real video, seeks to 12 seconds, persists the checkpoint
through the CXX-Qt model, and exits only after the SQLite worker returns.
`--media-bundle-ui-probe` leaves the same polished media card open for visual
inspection. The card uses Qt Multimedia for cross-platform playback and Qt's
native URL handler for manuals; Rust never invokes a shell or constructs an
OS-specific opener command.
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

`--bigbox-ui-probe --screenshot-output PATH` opens the native couch-mode shell
at a 1920x1200-class test size, filters the real catalog to the preserved Super
Mario Bros. family, restores the exact stable UUID for LaunchBox database ID
140, loads its real details and cached media, captures the full presentation,
and exits only after the selected row, details identity, and filtered-row lookup
agree. Pass `--media-directory PATH` to exercise an existing media cache. The
interactive Couch Mode control enters true fullscreen and Escape restores the
desktop window.

`--firmware-probe` opens an imported Coleco ADAM identity, runs normal emulator
discovery, prints the exact resolved rule counts and runtime path, and exits
only after firmware state reaches Qt. `--firmware-ui-probe` leaves that state
open so the selected installed MAME runtime's five required packages, Minerva
source, native target, and acquisition controls can be reviewed visually.
`--download-history-probe --state-database FIXTURE_PATH` loads real terminal
queue records, clears them through the asynchronous Qt action, and exits only
after the refreshed model reports zero finished records. The store refuses to
delete active, paused, queued, completion-pending-import, or
post-import-action-pending records.

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
durable safe seeding policy, local-folder scan/review/import, legacy
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
SteamGridDB and IGDB are native behind one Find Artwork workflow. Settings
stores the SteamGridDB key and the IGDB/Twitch client-credentials payload in the
native credential store, never SQLite. A provider search returns candidates
only; a human must choose the exact game before that provider's stable ID is
recorded against the positive LaunchBox catalog ID. SteamGridDB then presents
static backgrounds, covers, or logos; IGDB presents covers, artwork, or
screenshots using an in-memory cached Twitch application token. Both providers
use the same 16 MiB, HTTPS-only (except loopback fixtures), PNG/JPEG/WebP
signature-validating publisher and atomically replace only their own media kind.
IGDB attribution and the non-commercial/commercial-partnership boundary are
visible in Settings.

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

Richer playlist presentation, the remaining alternate media providers,
specialized machine launch profiles, the remaining settings, broader
manual-provider coverage, multidisc/patch archive management, and the
controller-first full-screen interface follow on the same shared models.
