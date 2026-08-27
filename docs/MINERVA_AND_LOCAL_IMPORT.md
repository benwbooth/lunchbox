# Minerva offers and local collection import

## Separation of public and user data

The public `lunchbox.db.7z` contains the schema, redistributable reference data, and the pinned
Libretro evidence pack. It does not contain Minerva provider records or a user's paths and library.

A writable user copy can be augmented with two Rust commands:

```console
cargo run -p lunchbox-db -- import-minerva \
  --database /path/to/user/lunchbox.db \
  --catalog /path/to/minerva.db
```

```console
cargo run -p lunchbox-db -- scan-local \
  --database /path/to/user/lunchbox.db \
  --root /path/to/roms/game-boy \
  --platform "Nintendo Game Boy" \
  --extensions gb,gbc
```

Both commands are idempotent. They use Rust path APIs and command arguments; no shell scripts or
hard-coded Unix paths are part of the application.

## Minerva import behavior

The maintained `minerva.db` catalog describes 1,049 torrent bundles and their provider platform
labels. `import-minerva`:

- hashes the complete input database and records a source snapshot;
- imports each torrent as an acquisition offer with its URL, provider path, collection, byte size,
  ROM count, and platform assertions;
- accepts only exact normalized platform or platform-alias matches;
- retains unmatched and ambiguous provider strings in provenance without creating platforms or
  guessing identities;
- marks availability as `unknown` because catalog presence is not proof that a URL is currently
  reachable; and
- replaces only Minerva offer records on refresh, leaving all other providers and user data alone.

The separate local `hashes.db` currently contains 2,973,791 file rows and is approximately 1.375 GB.
It is a provider cache, not a redistributable canonical database, so it is not copied into the public
artifact. File-level Minerva search and selective-download materialization will query or index that
cache on demand in a later step.

## Local scan behavior

`scan-local` recursively inventories regular files without following symbolic links. An optional
comma-separated extension filter limits the scan. Every included file is read once while CRC32,
MD5, SHA-1, and SHA-256 are calculated together.

For each path, the scanner creates or refreshes:

- a content-addressed artifact using SHA-256;
- exact artifact hashes for later provider matching;
- a local source record with lossless native path bytes and a human-readable path;
- a provisional game using the file stem as an editable initial title; and
- a provisional release when the platform is explicitly selected or exactly inferred.

The root and relative path bytes are stored with an encoding tag:

- `unix_bytes` on Linux and macOS;
- `windows_utf16le` on Windows; or
- `utf8` on other targets.

This prevents Windows drive syntax, POSIX separators, or lossy display strings from becoming the
portable identity key.

If `--platform` is present, it must match one canonical platform or alias exactly. Without it, the
scanner may infer a platform only when a strong MD5 or SHA-1 match in the pinned Libretro evidence
pack resolves to exactly one platform. Multiple exact-hash platform candidates remain pending for
review. Titles and directory names are never fuzzy-accepted.

A completed rescan marks disappeared paths as `missing`; it does not delete their history or user
metadata. A changed file at the same path is relinked to its new content-addressed artifact.

Archives are currently inventoried as container artifacts. Their members are not silently treated
as games. Archive-member indexing needs format-aware extraction and will be implemented separately
so that hashes correspond to the actual emulated content rather than guessed filenames.
