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
  Minerva platform coverage and legacy user-file state are joined in memory by
  exact provider IDs, stable game UUIDs, or exact normalized platform names.
  Artwork, video, manuals, emulator inspection, torrent metadata, and hashing
  remain separately scheduled services rather than additions to startup.
- Paths cross the Rust boundary as native paths. `--database PATH` and the
  `LUNCHBOX_DATABASE` environment variable are supported on Linux, macOS, and
  Windows without assuming `/bin/sh`, drive letters, or a path separator.
- `--games-database`, `--minerva-database`, and `--user-database` select the
  discovery, acquisition, and installed-state layers. Their environment
  equivalents use the `LUNCHBOX_` prefix. None of these paths is interpreted as
  a provider path or converted to a platform-specific string identity.

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

On the current Linux development host, the real preserved 303,560-game catalog
loads on its worker in roughly 0.5–0.8 seconds, while a combined text plus
Minerva filter completes in 13 ms. Warm shell-ready probes remain below the
250-ms release budget.

The Nix application package extracts the verified database artifact at build
time and points the executable at the unpacked database. Users do not pay a
decompression cost at application startup.

## Incremental feature migration

The Electron implementation remains on `legacy/electron` as the specification
for Lunchbox-owned behavior. Features should move as vertical slices: model and
service, native UI, error and cancellation behavior, tests, then a measured
runtime gate. The next slices are local-folder import, Minerva browsing and
download queue, game details/media, emulator installation and launch, settings,
and the controller-first full-screen interface.
