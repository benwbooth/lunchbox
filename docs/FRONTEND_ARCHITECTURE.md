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
- The writable state database is selected with `--state-database` or
  `LUNCHBOX_STATE_DATABASE`, otherwise an OS-native application-data directory
  is used. Its WAL setup and schema migration are serialized before concurrent
  settings, queue, and catalog workers access it. Favorites use the stable game
  UUID as their primary key; the LaunchBox numeric ID remains metadata only.
  Favorite writes run on named workers, update the native views optimistically,
  and roll the UI back if SQLite rejects the change. Window shutdown is deferred
  until an accepted favorite write has left the in-process worker.
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
  match always wins. Persisted primary-region and latest/original-revision
  choices only order equally strong candidates. The native picker shows the
  parsed region and revision and visually identifies the resulting best match.
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
  blocking Qt. Emulator process launch is a separate vertical slice.
- Local collection scanning enumerates without following symlinks and hashes on
  a named worker. Progress is throttled before crossing the Qt bridge, and an
  atomic cancellation token is checked between every read chunk. The review
  view is virtualized; filtering and selection operate on compact Rust indices.
  Imported paths retain native bytes (`unix_bytes`, `windows_utf16le`, or
  `utf8`) instead of using a display string as identity.

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
`LUNCHBOX_MEDIA_READY_MS` with game and preferred-asset counts, and exit.
`--artwork-ui-probe` filters to a deterministic set of real cached covers and
leaves the native grid open for a virtual-display visual check.
`--media-fetch-probe --media-directory EMPTY_PATH` performs the same path from
an uncached visible delegate through the real LibRetro service, waits for Qt to
index the downloaded asset, prints `LUNCHBOX_MEDIA_FETCH_READY_MS`, and exits.
`--favorite-probe --state-database EMPTY_PATH` selects a real catalog game,
persists it through the native Qt action, prints `LUNCHBOX_FAVORITE_READY_MS`,
and exits only after the worker reports success. `--favorite-ui-probe` leaves
the same grid, star affordance, and details pane open for visual inspection.

`--collection-probe --state-database EMPTY_PATH` creates a named collection,
adds a real catalog game by stable UUID, applies the native collection filter,
and exits only after persistence succeeds. Reusing the state path verifies that
the collection and membership survive restart. `--collection-ui-probe` follows
the same route and leaves the collection-management surface open for inspection.

`--import-ui-probe --import-directory PATH` opens the local-import surface and
starts a real checksum scan. It exists for deterministic virtual-display visual
checks; it uses the same model, worker, database, and QML as an interactive
folder selection. Adding `--import-commit-probe` selects the readable results
and commits them to the chosen `--state-database`, enabling a fully unattended,
idempotent scan-to-collection integration test.

`--download-ui-probe` leaves the persistent native Downloads drawer open.
`--settings-ui-probe` leaves the native settings dialog open, including the
post-import seeding policy, for deterministic visual inspection.
`--settings-seeding-probe --state-database EMPTY_PATH` selects the pause policy,
saves it through the asynchronous CXX-Qt settings bridge, and exits only after
the save completes. Inspecting or reusing the state path verifies persistence
and restart recovery.
`--settings-release-probe --state-database EMPTY_PATH` similarly saves Japan
plus original-release preferences through the real bridge. The
`--release-candidate-ui-probe` route opens Super Mario Land, inspects its live
Minerva bundle, and scrolls the details pane to the ranked native candidate
cards for deterministic visual review against the preserved discovery catalog.
`--multidisc-ui-probe` opens the preserved Sony PlayStation Final Fantasy VII
record, inspects its live Minerva sources, and opens the exact multi-disc review
dialog when a plan is found. This route verifies real provider filenames,
aggregate size, playlist naming and dialog layout without starting a download.
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
`--download-history-probe --state-database FIXTURE_PATH` loads real terminal
queue records, clears them through the asynchronous Qt action, and exits only
after the refreshed model reports zero finished records. The store refuses to
delete active, paused, queued, completion-pending-import, or
post-import-action-pending records.

On the current Linux development host, the real preserved 303,560-game catalog
loads on its worker in roughly 0.5–0.8 seconds in release mode, while a combined
text plus Minerva filter completes in about 16 ms. The existing 8 GB media tree
indexes 6,979 preferred assets for 6,677 games in 106 ms without delaying
catalog readiness. A real empty-cache NES cover retrieval completed in 7.944
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
ingestion, optical and eXo related-file plans, eXo prepared installs, durable safe seeding policy,
local-folder scan/review/import, legacy
content filters, and durable favorites are now native. Existing game media is
now indexed and rendered natively. On-demand LibRetro retrieval and durable
negative caching are native.
Playlists and smart collections, alternate media providers, media rotation and
redownload, emulator installation and launch, the remaining settings,
archive-member and manual identity workflows, and the controller-first
full-screen interface follow on the same shared models.
