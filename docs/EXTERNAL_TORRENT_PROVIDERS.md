# External torrent provider architecture

Lunchbox should broaden acquisition coverage without making any download source
the authority for game identity and without embedding access to an unlicensed
commercial-ROM service. Minerva remains the reviewed built-in source. The next
portable foundation is provider-neutral intake for lawful user-supplied torrents
and future sources whose operators document authorization and stable terms.

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
still import lawful torrent files or magnet links they obtained independently;
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

## Native source flow

1. A watched folder and file picker accept `.torrent` files using the existing
   bounded bencode parser; a separate field accepts validated magnet URIs.
2. The source worker extracts metadata off the GUI thread, rejects unsafe or
   oversized structures, deduplicates by info hash, and stores provenance.
3. The Qt source picker presents the offer alongside Minerva results and local
   collection state. Nothing downloads until the user reviews the exact files
   and canonical-game association.
4. The existing qBittorrent/download planner owns selection, progress,
   cancellation, import validation, and collection publication. Providers do
   not get filesystem or emulator-launch access.
5. A later adapter API can run in-process only for a small audited protocol or
   out-of-process with a versioned JSON contract and least-privilege storage.
   Credentials belong in the system keyring; databases contain references only.

This route supports newer platforms without binding the core application to one
Telegram channel. Platform support is still separate from download availability:
emulator/runtime compatibility, user-owned keys or firmware, legal content
ownership, safe import, and exact launch configuration remain independent gates.

## Delivery order

1. Manual `.torrent` and magnet intake with exact-file review and provenance.
2. Idempotent watched-folder intake with archive/move-on-success behavior and
   duplicate info-hash suppression.
3. A local provider manifest for user-managed, lawful catalogs.
4. A documented adapter SDK with capability, terms, authentication, rate-limit,
   and revocation metadata.
5. Individual source adapters only after legal/terms review and real integration
   tests against their authorized APIs.
