# Lunchbox

Lunchbox is a cross-platform, acquisition-first game browser, collection manager, and emulator frontend. The application will use Rust with CXX-Qt and QML. This branch begins with the canonical data layer shared by the desktop and couch interfaces.

The previous Electron application is preserved in the `legacy/electron` branch.
The new native desktop shell is now runnable and is described in [the frontend
architecture](docs/FRONTEND_ARCHITECTURE.md).
The ongoing vertical-slice plan is tracked in [the native frontend parity
roadmap](docs/FRONTEND_ROADMAP.md).

## Native Qt frontend

Run the Rust/CXX-Qt application against the current development database:

```console
nix develop
cargo run -p lunchbox-app -- --database build/lunchbox.db
```

The window paints before catalog work begins. SQLite loading and collection
filtering run on worker threads, while virtualized Qt views consume a native
`QAbstractListModel`. Use `--startup-probe` to measure shell construction without
waiting for database loading.

To exercise the preserved acquisition-first catalog, layer the local legacy
game catalog, Minerva bundle index, and optional user state over the canonical
database:

```console
cargo run --release -p lunchbox-app -- \
  --database build/lunchbox.db \
  --games-database lunchbox-games.db \
  --minerva-database minerva.db \
  --user-database ~/.local/share/lunchbox/user.db
```

The `Minerva` view contains only games whose platform has an exact Minerva
bundle mapping and which are not already present in the user database. Selecting
a platform keeps that availability filter active. The native Filters menu makes
the same distinction explicit alongside Installed, Hide Non-Retail, and Hide
Adult controls. Content classifications follow the bounded legacy rules, and
the content-filter preferences survive restart in the writable state database.
Select a game to open its
native details pane; Minerva sources can be inspected against the real torrent
contents, where title matches are presented as review candidates rather than
silently accepted identities. A reviewed file can be sent to qBittorrent from
that pane, and the native Downloads drawer persists progress and exposes
pause, resume, and non-destructive cancel controls.

Configure qBittorrent and both sides of its filesystem mapping in the native
Settings dialog. Native paths are chosen with the operating system folder
picker; qBittorrent/container paths are deliberately retained as verbatim
client strings. Passwords use the operating system credential store and never
the SQLite state database. Empty credentials remain valid for qBittorrent
instances whose Web UI authentication bypass is intentionally enabled.

On completion, Lunchbox verifies and materializes the reviewed file using the
selected symbolic-link, hard-link, reflink, copy, or leave-in-place policy. It
never overwrites different existing content, and only records the exact game as
installed after the destination exists. That installed state immediately keeps
the game out of the active Minerva/not-installed view; the catalog refreshes
and reapplies the current search and platform filters after ingestion.

`Import ROMs` opens the native local-collection workflow. It recursively scans
supported files off the GUI thread, streams progress, can be cancelled during
large-file hashing, and presents a searchable, sortable review table. Unique
SHA-1/MD5 matches are selected by default; filenames never establish identity.
Users can also include ambiguous or unmatched files, which remain visible as
honest local-only entries in `My Collection` without suppressing an unrelated
Minerva game. Imports and rescans are idempotent, retain missing-path history,
and store lossless OS-native path bytes alongside display paths.

The equivalent catalog environment variables are
`LUNCHBOX_GAMES_DATABASE`, `LUNCHBOX_MINERVA_DATABASE`, and
`LUNCHBOX_USER_DATABASE`. `--state-database` or
`LUNCHBOX_STATE_DATABASE` can select an alternate writable settings, queue, and
native collection database for testing or portable deployments.

The preserved game catalog is a local runtime input and is not added to the
published artifact because redistribution permission for its LaunchBox-derived
records has not been established.

## Database principles

- A game, a platform release, an exact ROM/disc artifact, and a downloadable offer are different entities.
- Provider evidence packs are distinct from the canonical catalog; importing a DAT does not declare
  every dump to be a separate canonical game.
- Lunchbox owns stable internal IDs; provider IDs are versioned links, never primary keys.
- Exact hashes and explicitly reviewed provider links may establish identity. Fuzzy title matches may only create review candidates.
- Raw provider assertions retain their provenance and do not overwrite user choices.
- Ambiguous legacy records remain quarantined until they can be resolved safely.

See [the metadata-backbone decision](docs/METADATA_BACKBONE.md), [the database
architecture](docs/DATABASE_ARCHITECTURE.md), [the pinned Libretro source
policy](docs/LIBRETRO_SOURCE.md), [Minerva and local collection
import](docs/MINERVA_AND_LOCAL_IMPORT.md), and [the legacy
audit](docs/LEGACY_DATABASE_AUDIT.md).

## Reproducible workflow

Enter the development environment:

```console
nix develop
```

Prepare the exact pinned Libretro snapshot. This downloads into an ignored local cache, verifies
the archive size and SHA-256, extracts only `rdb/*.rdb`, and verifies the complete extracted tree:

```console
cargo run -p lunchbox-db -- prepare-libretro \
  --manifest sources/libretro.json \
  --cache source-cache
```

Build the canonical database from the maintained emulator catalog and verified Libretro RDBs:

```console
cargo run -p lunchbox-db -- build \
  --database build/lunchbox.db \
  --emulators emulator_details/all_emulators.csv \
  --libretro-manifest sources/libretro.json \
  --libretro-rdb-dir source-cache/libretro-database-4de37d2c8042d9d0f608c43006c9b35438c536ce/rdb
```

Audit it strictly:

```console
cargo run -p lunchbox-db -- audit --database build/lunchbox.db --strict
```

Create and verify a maximal-compression 7z artifact. The command fails if the archive is 100,000,000 bytes or larger:

```console
cargo run -p lunchbox-db -- compress \
  --database build/lunchbox.db \
  --output artifacts/lunchbox.db.7z
```

Audit preserved local databases without modifying them:

```console
cargo run -p lunchbox-db -- inspect-existing \
  --legacy-catalog lunchbox-games.db \
  --openvgdb openvgdb.sqlite \
  --emulators db/emulators.db \
  --minerva db/minerva.db \
  --output reports/existing-databases.json
```

Import Minerva bundle offers into a writable user database:

```console
cargo run -p lunchbox-db -- import-minerva \
  --database /path/to/user/lunchbox.db \
  --catalog /path/to/minerva.db
```

Hash and import a local collection. `--platform` may be omitted when strong Libretro evidence
resolves each file to exactly one platform:

```console
cargo run -p lunchbox-db -- scan-local \
  --database /path/to/user/lunchbox.db \
  --root /path/to/roms/game-boy \
  --platform "Nintendo Game Boy" \
  --extensions gb,gbc
```

All commands are implemented directly in Rust and are safe to rerun.
