# External torrent provider architecture

Lunchbox should broaden acquisition coverage without making any download source
the authority for game identity and without embedding access to an unlicensed
commercial-ROM service. Minerva remains the reviewed built-in source. The first
portable foundation is now provider-neutral intake for lawful user-supplied
`.torrent` files and v1 magnet links; future sources still require documented
authorization and stable terms.

## Research finding: NSW Torrent Library

As of 2026-08-28, NSW Torrent Library's public entry point is the Telegram bot
[`@Switch_library_bot`](https://telegram.me/Switch_library_bot), with its own
website linked from the bot. The visible workflow offers bot commands and
`.torrent` attachments, while its Windows client advertises an embedded torrent
client and alternate-network/proxy recovery. No public, documented catalog API,
redistribution license, or authorization for a third-party frontend was found.
Its catalog prominently includes current commercial Nintendo Switch releases.

The third-party open-source project
[`AstroSoup/loadster`](https://github.com/AstroSoup/loadster) demonstrates one
possible implementation shape: it runs a Telegram user account through TDLib,
downloads torrent attachments, records deduplication/resume state in SQLite, and
feeds an embedded torrent client. That is useful architectural evidence, but it
is not a suitable Lunchbox integration contract. It would require users to
entrust Telegram session credentials to Lunchbox and would couple the product to
bot scraping, mutable message formats, access-control evasion, and an
unlicensed-content index.

Therefore Lunchbox will not ship a direct NSW adapter, Telegram userbot, bot
scraper, bundled account, network-blocking bypass, or copy of its index/torrents.
That decision can be revisited only if the source publishes documented rights,
terms acceptable to a redistributable client, and an authorized API. Users may
still import lawful torrent files or v1 magnet links they obtained independently.
Lunchbox is responsible for review and safe handling, not for deciding that an
external source had permission to distribute its content.

## Provider-neutral contract

Every provider or manual intake path should produce the same bounded offer:

- provider ID, display name, terms URL, authorization mode, and catalog version;
- provider-owned offer ID plus info hash or torrent bytes/magnet URI;
- source title/platform labels and an immutable provenance record;
- exact file inventory when metadata is available, including size and paths;
- availability, transport requirements, and actionable errors;
- optional explicit links to canonical game UUIDs, accepted only from a
  reviewed mapping table rather than fuzzy title matching.

External labels are evidence for a candidate, never canonical identity. Exact
provider mappings may select a stable game UUID. Otherwise the existing review
surface can show platform/title/release evidence and require the user to choose.
Import success records the selected canonical UUID, provider/offer identity,
info hash, chosen files, and hashes. It does not merge game records or rewrite
catalog metadata.

## Implemented exact-game source flow

1. The exact game's details pane accepts one native `.torrent` or pasted v1
   magnet link; the selected catalog UUID, title, platform, and positive
   LaunchBox database ID are locked before inspection begins.
2. A Rust worker reads at most 16 MiB off the GUI thread or asks configured
   qBittorrent for bounded magnet metadata, then parses the v1 metadata,
   bounds file counts, names, paths, and total sizes, and rejects traversal,
   duplicates, case ambiguity, controls, and filenames that are not portable
   across Linux, macOS, and Windows.
3. The native Qt review dialog exposes the source filename, torrent name,
   SHA-1 info hash, complete file inventory, exact selection, and whether
   Settings requires the entire torrent. Nothing downloads until Queue is
   explicitly activated.
4. The existing qBittorrent/download planner owns selection, progress,
   cancellation, import validation, and collection publication. SQLite records
   the source filename, torrent SHA-256, info hash, reviewed file index and
   actual qBittorrent path, exact game association, and queue job. Providers do
   not get filesystem or emulator-launch access.
5. A later adapter API can run in-process only for a small audited protocol or
   out-of-process with a versioned JSON contract and least-privilege storage.
   Credentials belong in the system keyring; databases contain references only.

## Implemented reusable source flow

1. **Torrent Sources** in the main navigation accepts one native `.torrent` or
   a pasted v1 magnet link and an exact platform. It does not create catalog
   games from torrent labels and adding it queues no payload.
2. Local metadata uses the same bounded parser as exact-game intake. Magnet
   syntax is bounded and must contain one unambiguous hexadecimal or Base32 v1
   `btih`; qBittorrent retrieves the metadata under Lunchbox's isolated category,
   exports the resulting `.torrent`, and Lunchbox verifies its info hash before
   showing the review. The temporary metadata-only qBittorrent entry is removed
   after registration and the magnet URI is not stored.
3. The Qt review shows the complete portable inventory and total size. SQLite
   stores the bounded reviewed torrent metadata, source provenance, exact
   platform, verified identities, and a normalized member-title index. Re-adding
   the same info hash updates the source idempotently.
4. Game details query that index only by the exact normalized canonical title
   and exact-linked alternate titles. Matching members appear in the ordinary
   Get review; filename similarity never establishes catalog identity.
5. Queueing always selects only the explicitly reviewed member from a registered
   source, regardless of the global whole-torrent preference. The existing
   durable queue then owns progress, pause/resume, retry, cancellation, storage
   reservation, import validation, and post-import seeding policy.

This route supports newer platforms without binding the core application to one
Telegram channel. Platform support is still separate from download availability:
emulator/runtime compatibility, user-owned keys or firmware, legal content
ownership, safe import, and exact launch configuration remain independent gates.

## Delivery order

1. **Complete:** exact-game manual `.torrent` and v1 magnet intake with
   exact-file review, durable provenance, and deterministic Rust/Qt tests.
2. **Complete:** reusable `.torrent` and v1 magnet registration with bounded
   metadata resolution, complete-inventory review, exact platform/title lookup,
   and selective per-game queueing.
3. Idempotent watched-folder intake with archive/move-on-success behavior and
   duplicate info-hash suppression.
4. A local provider manifest for user-managed, lawful catalogs.
5. A documented adapter SDK with capability, terms, authentication, rate-limit,
   and revocation metadata.
6. Individual source adapters only after legal/terms review and real integration
   tests against their authorized APIs.
