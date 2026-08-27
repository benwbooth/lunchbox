# Metadata backbone decision

Reviewed: 2026-08-26

## Decision

No third-party database is the canonical Lunchbox catalog. Lunchbox owns the canonical game,
release, artifact, and acquisition-offer records and their stable IDs. External IDs are durable,
versioned links to provider assertions.

The practical provider stack is:

1. **Local files and Minerva offers define what the user can actually play or acquire.** They create
   provisional Lunchbox games and releases when no accepted identity already exists.
2. **Libretro supplies exact artifact evidence.** It can identify many ROMs, but it is not a game
   catalog and does not cover every platform.
3. **IGDB is the preferred broad metadata-enrichment service.** Its assertions remain attributed
   provider data and are never used as Lunchbox primary keys.
4. **Wikidata supplies redistribution-safe facts and external-ID crosswalks.** It is CC0, but its
   game records are too uneven to provide the polished metadata and media experience alone.
5. **MobyGames is an optional licensed historical-gap service.** It has unusually broad platform
   and release coverage, but its commercial API terms and no-repackaging restriction make it
   unsuitable for the public database artifact.
6. **RAWG is an optional modern-discovery fallback.** RetroAchievements is an optional exact-hash
   and achievements service. Neither is a universal catalog.

This means “primary source” has two precise answers:

- The **source of truth for identity and user-visible selections is Lunchbox**.
- The **default external metadata lookup is IGDB**, subject to its authentication, attribution, and
  partnership terms.

The public `lunchbox.db.7z` will not contain an IGDB, MobyGames, RAWG, LaunchBox, or TheGamesDB bulk
copy unless a later written license explicitly permits that distribution.

## Why a single external database is the wrong boundary

A large record count does not make a provider canonical. Providers disagree about whether a port,
regional release, compilation, edition, remaster, DLC, or ROM revision is a separate “game.” Their
IDs can be deleted or merged, their coverage changes, and their media has different rights from
their structured metadata.

Lunchbox therefore stores provider assertions beside one another. An accepted link says that a
provider record refers to a particular Lunchbox entity; it does not transfer ownership of that
entity's ID or silently replace user edits.

This also avoids preloading an imaginary catalog of every game ever made. The acquisition-first
product only needs rich records for:

- games present in the user's local collection;
- games currently offered by Minerva or another acquisition provider; and
- games the user explicitly searches for or adds.

Those records can be enriched lazily and cached locally. The shipped database remains small,
auditable, and legally redistributable.

## Candidate assessment

| Provider | Current official coverage signal | Useful role | Constraint and decision |
| --- | --- | --- | --- |
| [IGDB](https://api-docs.igdb.com/) | Broad game, release, platform, company, localization, external-ID, artwork, and video endpoints | Preferred metadata enrichment | Requires Twitch application credentials. The FAQ permits local caching and serving data to end users, but commercial projects require a partnership and user-facing attribution. Do not publish a bulk snapshot. |
| [MobyGames](https://www.mobygames.com/stats/) | 333,000 games and 347 platforms at review time, plus detailed releases, product codes, covers, and credits | Best optional historical/release gap source | [API plans](https://www.mobygames.com/api/subscribe/) distinguish hobby and commercial use, require attribution, and prohibit repackaging/reselling the data. Runtime or server-side licensed use only. |
| [Wikidata](https://www.wikidata.org/wiki/Wikidata:WikiProject_Video_games/Statistics/instance_of) | 175,558 direct video-game instances at review time, with many external identifier properties | Open facts and cross-provider ID graph | Structured data is [CC0](https://www.wikidata.org/wiki/Wikidata:Licensing), but field completeness and entity modeling are inconsistent. Safe to redistribute; insufficient alone. |
| [RAWG](https://rawg.io/apidocs) | Advertises 500,000+ games across 50 platforms | Optional modern discovery and store links | Requires attribution and explicitly prohibits data redistribution. Platform breadth is much narrower than MobyGames. Runtime only. |
| [TheGamesDB](https://api.thegamesdb.net/) | API exposes games, platforms, media, unique IDs, and ROM-hash lookup | Possible secondary metadata and media source | The current API is real, but an official current license grant for redistributing its data and media was not located. Do not integrate until clarified. |
| [LaunchBox Games Database](https://feedback.launchbox-app.com/en/help/articles/8890692-getting-started-with-the-launchbox-games-database) | Community metadata and many media classes oriented toward LaunchBox and Big Box | Product-behavior reference only | Official documentation describes LaunchBox's local metadata export, but the reviewed material contains no grant to clone or redistribute the database. Do not ingest without permission. |
| [RetroAchievements](https://api-docs.retroachievements.org/v1/get-game-list.html) | API returns games and supported hashes for its systems | Exact supported-artifact evidence and achievements | Coverage follows the RetroAchievements ecosystem and its custom per-system hashing rules. Runtime integration, not a catalog backbone. |
| [Libretro Database](LIBRETRO_SOURCE.md) | The pinned Lunchbox pack contains 752,037 source observations from 145 RDBs | Redistributable ROM/disc evidence | Strong for file recognition, weak as a universal work/release catalog. Keep it separate from canonical entities. |

Published counts are not directly comparable: each provider defines “game,” release, platform, and
add-on differently. They are coverage signals, not quality rankings.

## Identity and duplicate rules

The following rules are mandatory for every connector:

1. A provider record is unique by `(provider, record type, external ID)`.
2. A provider link may have only one accepted canonical target at a time. Superseded decisions are
   retained as identity events or redirects.
3. Exact strong hashes, trusted serials/product codes, explicit provider cross-links, or a reviewed
   decision may establish identity.
4. Normalized title, platform, year, developer, or publisher similarity may rank candidates but may
   never accept or merge them automatically.
5. Regional releases, ports, revisions, editions, DLC, and compilations remain distinct releases or
   related works as appropriate; they are not collapsed because their titles match.
6. Conflicting field values are stored with provider and snapshot provenance. A deterministic
   selection policy or user override chooses the displayed value without deleting alternatives.
7. User-created unmatched records are valid first-class records. Later enrichment links to them
   instead of replacing their IDs.

## Integration order

1. Import Minerva offers and local collection scans into the four-layer schema without fuzzy
   accepted links.
2. Resolve exact artifacts through the existing Libretro evidence pack and later approved
   No-Intro/Redump-style evidence.
3. Implement an IGDB connector that stores raw provenance, accepted external IDs, and a local cache;
   keep credentials outside Git and the Nix store.
4. Add a bounded Wikidata crosswalk for accepted games and releases, pinned by source snapshot and
   query hash. Do not import every Wikidata video-game item merely to inflate catalog size.
5. Measure unresolved local/Minerva records by platform. Only then enable MobyGames or another
   provider for demonstrated gaps.
6. Continuously audit duplicate candidates, conflicting identifiers, orphaned links, and provider
   drift. Provider changes generate review work; they never silently rewrite canonical identity.

This sequence gives the UI rich data where it matters while preserving local imports, download-first
browsing, legal redistribution, and stable IDs.
