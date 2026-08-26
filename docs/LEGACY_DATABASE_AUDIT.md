# Legacy database audit

The original databases remain local migration inputs and are not committed on the new branch.

## `lunchbox-games.db`

- 303,560 game rows and 192 platforms.
- 174,463 LaunchBox-source rows and 129,097 Libretro-source rows.
- No row contains both a LaunchBox database ID and a Libretro CRC.
- 10,088 exact platform/title duplicate groups contain 21,343 rows.
- Only eight exact title/platform groups cross the two source partitions.
- IDs are valid UUID strings, but the legacy importer generated random UUID v4 values on every clean rebuild.
- 82,007 rows carry an OpenVGDB release ID, but those links are unsafe.
- 13,958 OpenVGDB IDs span both source partitions and cover 66,385 rows.
- 8,363 OpenVGDB IDs map to more than two rows. Examples include a single ID attached to 100 unrelated Tetris releases and another attached to 74 Pac-Man releases across platforms.

Conclusion: this database is two mostly concatenated catalogs with title-based enrichment, not a trustworthy canonical identity graph. It may provide raw metadata assertions, but none of its internal IDs or OpenVGDB links are accepted automatically.

## `openvgdb.sqlite`

- 43 systems, 53,871 releases, 51,742 ROMs, and 39 regions.
- No orphan release-to-ROM references.
- Four duplicate non-empty CRC groups and four duplicate non-empty SHA-1 groups.

Conclusion: useful as a legacy artifact-identification source for its covered systems, subject to source-license verification. It is too narrow to be the main catalog.

## `db/emulators.db`

- 244 emulator rows, 493 platform mappings, and 199 distinct platform names.
- No orphan platform mappings.
- Two case-insensitive duplicate emulator-name groups: `Ares`/`ares` and `CaPriCe Forever`/`Caprice Forever`.
- Host operating systems are stored as semicolon-delimited strings with inconsistent ordering and one `Nintendo DS` value.

Conclusion: valuable authored data. The new builder imports its tracked CSV source, deduplicates emulator names case-insensitively, and normalizes host systems and packages into junction tables.

## `db/games.db` and `db/images.db`

Both contain no useful rows. They are not migrated.

## Minerva databases

`db/minerva.db` and the root `minerva.db` are byte-for-byte identical.

- 1,049 torrent records.
- 1,839 torrent/platform rows.
- 1,717 distinct raw Minerva platform strings.
- 1,470 platform rows have no Lunchbox platform mapping.

Conclusion: retain Minerva as a provider input, but remodel its rows as acquisition offers and resolve platforms separately. Do not use Minerva identifiers as game IDs.

The `inspect-existing` command regenerates a machine-readable audit from these local files.
