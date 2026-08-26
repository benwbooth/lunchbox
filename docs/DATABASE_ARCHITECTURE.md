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

## Provider roles

- Libretro, No-Intro, Redump, MAME, and similar DAT sources identify exact artifacts.
- IGDB and other licensed metadata services enrich games and releases.
- Minerva contributes acquisition offers, not canonical identity.
- Storefronts contribute authoritative identifiers for their own releases.
- User-created entries and overrides remain first-class and are never overwritten.

The `providers.redistribution_policy` field records whether provider data can be shipped, must be fetched at runtime, or requires legal review. A public database artifact must contain only redistributable data.

## Validation gates

Every generated database must pass:

- SQLite integrity and foreign-key checks.
- Source-link target validation.
- Duplicate release-fingerprint detection.
- Globally unique exact artifact hashes.
- At most one selected value per entity field.
- No unresolved error-severity data-quality issue.
- Verified 7z extraction test and a compressed size below 100,000,000 bytes.
