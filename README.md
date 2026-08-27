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
