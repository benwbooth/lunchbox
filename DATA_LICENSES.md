# Data licenses and redistribution

The Lunchbox source code is licensed under MIT. Data embedded in a generated database retains the
terms of its provider; the code license does not replace those terms.

## Currently imported redistributable sources

- **MAME CHD core (linked)** — `libchdman-rs` 0.289.0 wraps MAME's
  `chd_file` implementation (`src/lib/util/chd.cpp` plus the CD/DVD/HD format
  readers) and is statically linked into the application so that compressed
  disc images (CHD) can be unpacked into the cue/bin set an emulator expects.
  License: BSD-3-Clause (MAME's own terms), copyright the MAME development
  team and the `libchdman-rs` author. Upstream:
  <https://github.com/mamedev/mame> and
  <https://github.com/danifunker/libchdman-rs>. The pinned static archives are
  release assets of `libchdman-rs` v0.289.0, referenced by URL and SHA-256 in
  `flake.nix` and `.github/workflows/native-packages.yml`. Their outputs are
  compared against `chdman extractcd` for byte-identical cue/bin results.
  This is linked code only: no MAME ROM, BIOS or other provider data is
  redistributed with it.

- **DOSBox default controller mapper (derived runtime data)** — the baseline at
  `crates/lunchbox-app/data/controllers/dosbox-default-mapper.map` is generated
  from the pinned DOSBox-X `CreateDefaultBinds`/`DefaultKeys` tables
  (`src/gui/mapper.cpp`, commit
  `532909c4e84160a5ac2185fbf9c4c97dbe07f85d`) together with SDL's public
  scancode values (`include/SDL_scancode.h`, zlib license). It contains only
  functional keyboard/joystick event names and numeric scancodes; no game,
  firmware or provider data. DOSBox-X is GPL-2.0-or-later
  (<https://github.com/joncampbell123/dosbox-x>); the derived table is used to
  preserve the emulator's own default keyboard bindings when Lunchbox patches
  only the emulated joystick events. DOSBox Staging shares the same mapper
  grammar (<https://github.com/dosbox-staging/dosbox-staging>).

- **Lunchbox emulator catalog** — maintained in this repository and distributed under MIT.
- **Libretro Database** — distributed under CC BY-SA 4.0. The exact upstream revision, source URL,
  archive SHA-256, and license are recorded in `sources/libretro.json` and in every generated
  database's `source_snapshots` table. Upstream project: <https://github.com/libretro/libretro-database>.
  License: <https://creativecommons.org/licenses/by-sa/4.0/>.

Any redistributed database containing Libretro-derived records must retain attribution, identify
the pinned revision, link the license, indicate that Lunchbox normalized the data, and be shared
under terms compatible with CC BY-SA 4.0.

- **EmulationWiki recommendation order** — per-system emulator comparison tables published under
  Creative Commons Attribution Share Alike (version as published on the wiki; "Content is available
  under Creative Commons Attribution Share Alike unless otherwise noted"). Lunchbox imports only a
  bounded, hand-curated ranking snapshot (`sources/emulationwiki-recommendations.json`, one entry
  per supported emulator with the source page URL and retrieval date); each import records a
  `source_snapshots` row, and the app credits the wiki wherever rankings are displayed. Curated
  ranks and verdicts are Lunchbox's reading of the tables, not a copy of their prose. Upstream
  project: <https://emulation.gametechwiki.com/>.

## Approved for future redistributable use, but not imported

- **Wikidata structured data** — made available under CC0 1.0. Lunchbox may import a bounded,
  reproducibly pinned set of facts and external-ID links in a future build. No Wikidata records are
  present in the current artifact. Policy: <https://www.wikidata.org/wiki/Wikidata:Licensing>.

## Runtime-only metadata services

- **IGDB** — its API FAQ permits local caching and serving retrieved data to end users. Commercial
  integrations require a partnership and visible attribution. This review does not interpret that
  as permission to publish an IGDB database dump. <https://api-docs.igdb.com/>.
- **MobyGames** — API use is subscription-based; commercial use requires a commercial plan and
  attribution, and the service prohibits repackaging or reselling its data.
  <https://www.mobygames.com/api/subscribe/>.
- **RAWG** — API use requires attribution and its published terms prohibit data redistribution.
  <https://rawg.io/apidocs>.
- **RetroAchievements** — the API supports game/hash and achievement integrations and recommends
  caching static responses, but no permission to publish a bulk data copy is recorded here.
  <https://api-docs.retroachievements.org/>.
- **Minerva** — acquisition records remain subject to provider-specific terms and are fetched at
  runtime.

Local collection paths, hashes, and user-created provisional records are private user data. They
are stored only in a writable user database and are never included in the public artifact.

Runtime-only data may be cached in a user's local database subject to provider terms. It is excluded
from the public `lunchbox.db.7z` artifact.

## Sources requiring legal review

LaunchBox Games Database, TheGamesDB, the legacy OpenVGDB copy, and user library data are not
imported into the public artifact unless their applicable terms and intended use have been reviewed.
The current LaunchBox documentation describes a downloadable local metadata export but does not
provide the reuse and redistribution grant this project would need. A public API or downloadable
file is not itself a data license.

The complete reviewed provider registry is `sources/metadata-providers.json`; the rationale and
integration order are in `docs/METADATA_BACKBONE.md`.
