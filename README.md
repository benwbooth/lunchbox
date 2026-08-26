# Lunchbox

Lunchbox is a cross-platform, acquisition-first game browser, collection manager, and emulator frontend. The application will use Rust with CXX-Qt and QML. This branch begins with the canonical data layer shared by the desktop and couch interfaces.

The previous Electron application is preserved in the `legacy/electron` branch.

## Database principles

- A game, a platform release, an exact ROM/disc artifact, and a downloadable offer are different entities.
- Provider evidence packs are distinct from the canonical catalog; importing a DAT does not declare
  every dump to be a separate canonical game.
- Lunchbox owns stable internal IDs; provider IDs are versioned links, never primary keys.
- Exact hashes and explicitly reviewed provider links may establish identity. Fuzzy title matches may only create review candidates.
- Raw provider assertions retain their provenance and do not overwrite user choices.
- Ambiguous legacy records remain quarantined until they can be resolved safely.

See [the database architecture](docs/DATABASE_ARCHITECTURE.md), [the pinned Libretro
source policy](docs/LIBRETRO_SOURCE.md), and [the legacy audit](docs/LEGACY_DATABASE_AUDIT.md).

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

All commands are implemented directly in Rust and are safe to rerun.
