# Canonical database architecture

## Identity layers

Lunchbox separates four concepts that the legacy database placed in one row:

1. **Game**: an abstract creative work, such as *Tetris*.
2. **Release**: a platform, region, language, edition, and version of a game.
3. **Artifact**: exact content such as a ROM, disc image, executable, or multi-file set.
4. **Acquisition offer**: a provider-specific way to obtain an artifact or package.

This prevents regional ROMs, revisions, ports, remasters, similarly named games, and downloadable packages from being mistaken for one another.

## Stable IDs

Lunchbox entity IDs do not expose provider ownership. Source records use the unique key `(provider, record type, external ID)` and link to Lunchbox entities through an explicit `source_links` decision.

Deterministic IDs are used for reproducible reference data such as emulator and provisional platform records. Canonical game IDs will be persisted in the identity registry and survive provider deletions, merges, renames, and rebuilds. Merges retain redirects and an auditable identity event.

## Source evidence

Identity evidence is ranked as follows:

1. Exact cryptographic hash or trusted product/serial identifier.
2. An explicit cross-provider identifier supplied by a trusted source.
3. A reviewed match accepted by a user or curator.
4. A fuzzy title/platform candidate, which must remain pending.

Normalized titles alone never create accepted links.

Every imported Libretro evidence record points through `libretro_databases` to a pinned source
snapshot and carries its RDB path and ordinal. Source snapshots record the exact upstream Git
revision, archive digest, byte count, URI, and data license. These locators reconstruct the exact
MessagePack record from the verified source archive, so the public SQLite file does not carry a
second JSON copy of the same RDB data. Frequently queried fields, hashes, and serials use compact
provider-pack tables; remaining metadata is retained as JSON. Sources without a pinned retrievable
snapshot may instead materialize their JSON payload and payload SHA-256 in `source_records`.

An evidence record is not a canonical entity. Exact hash or reviewed decisions can later create or
link canonical games, releases, and artifacts without assigning a permanent Lunchbox UUID to every
provider observation. This keeps duplicate dumps visible and prevents an upstream DAT layout from
becoming Lunchbox's identity model.

## Provider roles

- Libretro, No-Intro, Redump, MAME, and similar DAT sources provide artifact evidence; they do not
  define the complete Lunchbox game catalog.
- Lunchbox is the source of truth for canonical identity, redirects, user overrides, and selected
  display values.
- IGDB is the preferred broad metadata-enrichment connector. Wikidata supplies open facts and
  external-ID crosswalks; MobyGames may fill demonstrated historical gaps under an appropriate
  license.
- Minerva contributes acquisition offers, not canonical identity.
- Storefronts contribute authoritative identifiers for their own releases.
- User-created entries and overrides remain first-class and are never overwritten.

The machine-readable `sources/metadata-providers.json` registry records each provider's role,
reviewed terms URL, decision, and redistribution policy. The Rust builder validates that registry
and seeds the `providers` table from it. A public database artifact must contain only redistributable
data.

See [the metadata-backbone decision](METADATA_BACKBONE.md) for the provider comparison, identity
rules, and integration order.

## Validation gates

Every generated database must pass:

- SQLite integrity and foreign-key checks.
- Source-link target validation.
- Duplicate release-fingerprint detection.
- Globally unique cryptographic artifact hashes. CRC32 collisions across artifacts are reported but
  are not treated as identity proof.
- Preservation of disagreeing CRC32 assertions for an artifact already identified by SHA-1 or MD5.
- Complete Libretro snapshot/path/ordinal provenance and declared-record-count validation.
- Exact byte-length validation for compact binary CRC32, MD5, and SHA-1 values.
- At most one selected value per entity field.
- No unresolved error-severity data-quality issue.
- Verified 7z extraction test and a compressed size below 100,000,000 bytes.
