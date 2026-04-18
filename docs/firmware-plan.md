# Firmware / BIOS Plan

This document is the implementation checklist and status summary for firmware handling in Lunchbox.

The core design constraint is:

- firmware is not a `platform -> files` problem
- firmware is a `runtime -> rule set -> target path` problem

Examples:

- `Sony Playstation + DuckStation` needs DuckStation's BIOS directory
- `Sony Playstation + RetroArch SwanStation` needs the RetroArch `system` directory
- `Sega ST-V + MAME` needs zipped BIOS romsets in MAME `rompath`
- `MS-DOS + 86Box` needs a machine ROM set, not one loose BIOS file

## Current State

Implemented and verified:

- canonical Lunchbox-owned firmware store and DB schema
- provenance tracking for imported firmware packages
- Minerva-backed package ingest for supported sources
- user/manual import support through Lunchbox-managed import folders
- runtime sync / launch-scoped staging for supported emulators
- picker UI for firmware status, provenance, install/sync/repair, and open-folder actions
- structured metadata for:
  - `required` vs `optional`
  - HLE fallback support
  - target strategy (`runtime_dir`, `launch_scoped`, `mame_rompath`, `manual_import`)

Implemented auto-install / auto-sync runtime families:

- RetroArch / libretro
- DuckStation
- PCSX2
- Dolphin
- openMSX
- Flycast
- MAME
- RetroArch `mame`
- 86Box
- PCem
- Snes9x
- O2EM
- LoopyMSE
- mGBA

Implemented but manual-only by design for now:

- `melonDS` for `Nintendo - Nintendo DSi`
- `GeePee32`
- `M88kai`
- `Tsugaru`
- `UNZ`
- `Emu5 (Common Source Code Project)` for `Sord M5`
- `DEmul` for:
  - `Sammy Atomiswave`
  - `Sega Hikaru`
  - `Sega Naomi`
  - `Sega Naomi 2`

Excluded / intentionally not added:

- standalone `NP2kai`
  - the shipped emulator catalog currently exposes the RetroArch `np2kai` path, which is already supported
  - there is no separate standalone `NP2kai` adapter in the current Lunchbox catalog to wire firmware against

## Checklist

### Auto-install / auto-sync coverage

- [x] RetroArch / libretro firmware adapter family
- [x] DuckStation
- [x] PCSX2
- [x] Dolphin
- [x] openMSX
- [x] Flycast
- [x] MAME
- [x] RetroArch `mame`
- [x] 86Box
- [x] PCem
- [x] Snes9x Satellaview support
- [x] O2EM
- [x] LoopyMSE
- [x] mGBA e-Reader support
- [x] optional external BIOS support for `PCSX-ReARMed`

### Manual-only runtime coverage

- [x] `melonDS` DSi surfaced as manual-only with managed import folder
- [x] `GeePee32` surfaced as manual-only with managed import folder
- [x] `M88kai` surfaced as manual-only with managed import folder
- [x] `Tsugaru` surfaced as manual-only with managed import folder
- [x] `UNZ` surfaced as manual-only with managed import folder
- [x] `Emu5` / `Sord M5` surfaced as manual-only with managed import folder
- [x] `DEmul` arcade branch surfaced as manual-only with managed import folder

### Metadata / model work

- [x] split `required` vs `optional/HLE` into structured firmware metadata
- [x] add explicit firmware target strategy identifiers

### UI / repair / manual-import flow

- [x] show source package / provenance in the emulator picker
- [x] add install / sync / repair firmware actions
- [x] add per-runtime `open firmware directory` action
- [x] add Lunchbox-owned manual import folder flow for manual-only runtimes

## Remaining Work

There are no active firmware implementation blockers in the current pass.

What remains is future improvement work, not incomplete baseline support:

- optionally promote current manual-only runtimes to auto-install if we later verify a clean source package and exact standalone target semantics
- optionally add a standalone `NP2kai` adapter if the emulator catalog grows a real standalone entry
- optionally broaden firmware-package UI for non-required / HLE-backed firmware so optional packs are more discoverable without needing launch-time install

## Source Strategy

Primary package sources:

- Minerva `Internet Archive/chadmaster/RetroarchSystemFiles/Retroarch-System/*.zip`
- Minerva MAME merged / non-merged BIOS romsets where that is the correct asset class
- official GitHub ROM-set sources where Minerva is not the right fit (`86Box`, `PCem`)
- user import for manual-only or non-packageable cases

Separate asset classes kept distinct:

- RetroArch system packs
- MAME BIOS / device romsets
- 86Box / PCem machine ROM sets
- manual-only standalone emulator assets

Do not flatten these into one anonymous BIOS folder.
