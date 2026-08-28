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
artifact. The native frontend now inspects bounded torrent metadata on demand,
presents filename matches for explicit review, and sends only the reviewed file
to qBittorrent unless whole-torrent mode is selected. The much larger hash cache
can later accelerate indexed search without becoming the canonical identity
source.

## Local scan behavior

The desktop `Import ROMs` dialog provides the same operation without modifying
the read-only packaged catalog. It writes a per-user inventory to `state.db`,
uses the discovery catalog's Libretro SHA-1/MD5 evidence for unique exact game
links, and refreshes both `My Collection` and the Minerva/not-installed filter
as soon as selected results are committed. Ambiguous hashes and unmatched files
remain selectable local-only entries; they never inherit a catalog identity by
filename. A user can explicitly open `Match…`, search canonical and alternate
titles with an optional exact platform filter, inspect the stable catalog ID,
and choose one record. Search ordering is suggestion-only; identity is created
only by that click. A reviewed link is saved with manual provenance and restored
only when the lossless path, archive member, byte size, MD5, and SHA-1 remain an
exact match. Duplicate persisted keys fail closed instead of restoring an
ambiguous decision. The preview supports text/status filters, sorting, bulk or
per-row selection, live progress, and cancellation.

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

The desktop scanner handles ZIP and 7z members without extracting during review:
it enumerates contained paths, rejects unsafe, encrypted, nested-archive, and
metadata entries, then streams every safe recognized ROM member through the same
CRC32/MD5/SHA-1 pipeline. A sole member retains the original archive as its launch
path. An archive with multiple recognized members exposes separate rows but
selects none automatically, protecting multi-file arcade sets from accidental
splitting. When the user explicitly selects members, Lunchbox reopens the
archive, verifies the reviewed size and digests, atomically materializes each
member under a SHA-256 content directory, and preserves lossless source-archive
plus exact-member provenance in the state database. 7z decoding is implemented
in Rust and requires no host 7z executable. RAR member inspection remains
separate work.
The canonical `scan-local` data command still inventories archives as container
artifacts rather than silently treating members as games.
