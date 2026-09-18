# Pinball and OpenBOR download sources

The catalog carries `Pinball` and `OpenBOR` metadata from LaunchBox, but
neither has a Minerva torrent: Minerva's only pinball rows are two arcade
machines inside the TeknoParrot collection, and it has no OpenBOR rows at all.
Pinball tables come from [PleasureDome's NonMAME
sets](https://pleasuredome.github.io/pleasuredome/nonmame/); OpenBOR is
community-distributed. Neither is redistributable by Lunchbox, so they are
wired as **user-managed local providers**: the operator supplies the
`.torrent` files and Lunchbox indexes them.

## How coverage works

A catalog game is marked downloadable when its platform is covered by a
download source *and* the game is not installed. Coverage now comes from two
places:

1. The pinned Minerva catalog (`minerva.db`).
2. Local provider catalogs registered from a manifest
   (`registered_torrent_catalogs.managed_by_provider_id`).

Only platforms declared by a registered catalog count, and matching is an
exact normalized platform key, so registration is always an explicit user
action and no fuzzy title link is involved. Both bulk manifest imports and
single manual torrent registrations are recognised.

`crates/lunchbox-app/src/catalog.rs` — `load_registered_torrent_platforms()`
is unioned into `load_minerva_coverage()`'s platform set in the preview,
discovery, and availability paths.

## Pinball (PleasureDome)

All sets are TorrentZipped and publish a datfile on the same page for Rom
manager reproduction.

| Set | Page | Magnet | Size |
| --- | --- | --- | --- |
| Visual Pinball (2026-07-15) | [pinball](https://pleasuredome.github.io/pleasuredome/nonmame/pinball/) | `xt=urn:btih:45f473cb23ea636e5cd130061566f7305bfd1fad` | ~1.68 TB |
| Future Pinball (2026-07-15) | [pinball](https://pleasuredome.github.io/pleasuredome/nonmame/pinball/) | `xt=urn:btih:f00f650d985f3d069ac0e05f1cff764c57196bde` | ~223.2 GB |
| PinMAME 3.6.0-1227 ROMs (split) | [pinmame](https://pleasuredome.github.io/pleasuredome/nonmame/pinmame/) | `xt=urn:btih:343fbafa66de33ed1480c8b250aacd65d29b519d` | — |

Future Pinball does not use PinMAME. Visual Pinball tables that recreate real
machines need the separate PinMAME romset. PinMAME romset names
(`mm_109c.zip`) are not table titles, so those files must be associated per
game from the game page rather than by title; only the table sets resolve
platform-wide.

## OpenBOR

PleasureDome has no OpenBOR set. The community sources are
[ChronoCrash](https://www.chronocrash.com/) (official community, account
required, largest `.pak` library) and community uploads on the
[Internet Archive](https://archive.org/search?query=openbor); the engine itself
is on [SourceForge](https://sourceforge.net/projects/openbor/). Register an
OpenBOR `.torrent` under platform `OpenBOR`, or associate individual `.pak`
files per game.

## Setup

1. Save the sets' `.torrent` files into a directory, for example
   `~/lunchbox-providers/pleasuredome/`, keeping the relative paths used below.
2. Copy `packaging/pleasuredome-provider.example.json`, point each
   `torrent_path` at the saved file, and keep `platform` exactly `Pinball` or
   `OpenBOR`.
3. In Lunchbox open **Settings → Local provider catalogs → Import manifest…**
   and choose the JSON. Importing only indexes metadata; it never starts a
   download. A single magnet or `.torrent` registered manually from the torrent
   inbox with platform `Pinball` or `OpenBOR` covers the platform the same way.
4. The `Pinball` and `OpenBOR` cards now show the `DOWNLOAD` badge and each
   game's details pane lists the matching torrent. Removing or resyncing the
   manifest updates coverage.

Manifest contract (`crates/lunchbox-app/src/local_provider_manifest.rs`):
`format` must be `lunchbox-local-torrent-provider`, `schema_version` 1,
`authorization` must be `user-managed`, `torrent_path` must be a relative path
to a regular `.torrent` file inside the manifest directory, and there may be
1–256 offers.

## Source reference

`packaging/rom-sources/pinball.csv` and `openbor.csv` list the same sources in
the maintained ROM-source format (copies also live in the gitignored
`rom_sources/` reference directory). These files are documentation only; the
app does not read them.

No PleasureDome, Pinball, or OpenBOR payload is committed to this repository,
and no provider data is embedded in the public database artifact.
