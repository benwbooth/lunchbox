# Data licenses and redistribution

The Lunchbox source code is licensed under MIT. Data embedded in a generated database retains the
terms of its provider; the code license does not replace those terms.

## Currently imported redistributable sources

- **Lunchbox emulator catalog** — maintained in this repository and distributed under MIT.
- **Libretro Database** — distributed under CC BY-SA 4.0. The exact upstream revision, source URL,
  archive SHA-256, and license are recorded in `sources/libretro.json` and in every generated
  database's `source_snapshots` table. Upstream project: <https://github.com/libretro/libretro-database>.
  License: <https://creativecommons.org/licenses/by-sa/4.0/>.

Any redistributed database containing Libretro-derived records must retain attribution, identify
the pinned revision, link the license, indicate that Lunchbox normalized the data, and be shared
under terms compatible with CC BY-SA 4.0.

## Sources not approved for redistribution

LaunchBox Games Database, IGDB, Minerva provider data, the legacy OpenVGDB copy, and user library
data are not imported into the public artifact unless their applicable terms and intended use have
been reviewed. Their provider rows exist so future ingestion cannot silently bypass that decision.
