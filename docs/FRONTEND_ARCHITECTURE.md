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
- The initial query reads compact text and state fields only. The preserved
  discovery catalog can be layered read-only over the canonical database;
  Legacy user-file state is joined in memory by exact provider game IDs or
  stable game UUIDs. Minerva platform coverage uses exact normalized mapped or
  provider names because its numeric platform IDs are not in the discovery
  database's internal ID namespace.
  Artwork, video, manuals, emulator inspection, torrent metadata, and hashing
  remain separately scheduled services rather than additions to startup.
- Paths cross the Rust boundary as native paths. `--database PATH` and the
  `LUNCHBOX_DATABASE` environment variable are supported on Linux, macOS, and
  Windows without assuming `/bin/sh`, drive letters, or a path separator.
- `--games-database`, `--minerva-database`, and `--user-database` select the
  discovery, acquisition, and installed-state layers. Their environment
  equivalents use the `LUNCHBOX_` prefix. None of these paths is interpreted as
  a provider path or converted to a platform-specific string identity.
- The writable state database is selected with `--state-database` or
  `LUNCHBOX_STATE_DATABASE`, otherwise an OS-native application-data directory
  is used. Its WAL setup and schema migration are serialized before concurrent
  settings, queue, and catalog workers access it.
- qBittorrent paths and host paths are separate types at the service boundary:
  host paths use Rust `PathBuf` and native folder dialogs, while remote or
  container paths remain literal strings. The app never rewrites a Windows
  path as a POSIX path or assumes the client runs on the GUI host.
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

`--import-ui-probe --import-directory PATH` opens the local-import surface and
starts a real checksum scan. It exists for deterministic virtual-display visual
checks; it uses the same model, worker, database, and QML as an interactive
folder selection. Adding `--import-commit-probe` selects the readable results
and commits them to the chosen `--state-database`, enabling a fully unattended,
idempotent scan-to-collection integration test.

On the current Linux development host, the real preserved 303,560-game catalog
loads on its worker in roughly 0.5–0.8 seconds, while a combined text plus
Minerva filter completes in 16 ms. Warm shell-ready probes remain below the
250-ms release budget.

The Nix application package extracts the verified database artifact at build
time and points the executable at the unpacked database. Users do not pay a
decompression cost at application startup.

## Incremental feature migration

The Electron implementation remains on `legacy/electron` as the specification
for Lunchbox-owned behavior. Features should move as vertical slices: model and
service, native UI, error and cancellation behavior, tests, then a measured
runtime gate. Catalog details, Minerva bundle inspection, persistent
qBittorrent setup, exact-file selection, queue control, and completed-file
ingestion, local-folder scan/review/import, and legacy content filters are now
native. Game media,
emulator installation and launch, the remaining settings, archive-member and
manual identity workflows, and the controller-first full-screen interface
follow on the same shared models.
