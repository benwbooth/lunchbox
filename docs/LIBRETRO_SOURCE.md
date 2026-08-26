# Libretro database source policy

Libretro Database is Lunchbox's first artifact-identification source, not its universal game
catalog. The upstream project itself describes its platform and DAT-source lists as
non-exhaustive. It aggregates preservation-oriented data used by RetroArch and is especially
valuable for exact ROM/disc hashes, filenames, serials, regions, and basic metadata.

It does not cover every platform or every kind of game. PC storefront releases, current consoles,
mobile catalogs, delisted digital titles, homebrew, mods, and obscure systems require additional
sources. Lunchbox therefore keeps provider records separate and owns its canonical IDs rather than
making a Libretro filename or record position the global identity.

## Pinned snapshot

The tracked manifest at `sources/libretro.json` pins:

- Git revision `4de37d2c8042d9d0f608c43006c9b35438c536ce`.
- The codeload archive URL, byte count, and SHA-256.
- 145 compiled RDB files totaling 180,071,816 bytes.
- A deterministic SHA-256 over the sorted extracted-file manifest.
- The upstream CC BY-SA 4.0 license and reviewed platform-name overrides.

At this revision the importer parses 752,037 records across all 145 RDB files. Of those, 749,873
have a usable title and 2,164 records have no usable title. All source observations remain in the
compact evidence pack; they do not automatically become canonical game/release entities. Reviewed
platform overrides map the source files to 137 Lunchbox platform identities. These are snapshot
measurements, not a promise of universal platform coverage.

## Identity and discrepancies

The importer stores lookup evidence without fuzzy cross-provider matching. SHA-1 and MD5 are strong
artifact evidence; CRC32 plus size is weaker evidence. Serials are stored independently. Canonical
game, release, and artifact entities are created only by a later exact or reviewed resolution step.

Some upstream records with identical SHA-1, MD5, and size disagree about CRC32. Lunchbox preserves
each source record and reports the disagreement in its audit. It neither discards the discrepancy
nor lets a non-cryptographic checksum silently define canonical identity.

## Reproducibility

`prepare-libretro` is safe to rerun. A valid cache is reused only after its archive and extracted
tree match the tracked manifest. Invalid or interrupted generated cache paths are replaced
atomically. Each compact evidence record stores the snapshot, RDB path, and ordinal needed to
reconstruct the exact upstream MessagePack record; columns, binary hash tables, serial lookups, and
auxiliary JSON retain every known RDB field without duplicating the raw payload. The build performs
a strict SQLite audit before publishing its temporary database.
