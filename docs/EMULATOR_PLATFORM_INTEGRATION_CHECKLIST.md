# Emulator platform integration checklist

Source-only review: 2026-09-12. No build, test, emulator probe, or launch was
performed for this update.

This is the current checklist for the platform facts that controller setup,
firmware management, and save synchronization need. The detailed path, filename,
syntax, naming, and source evidence live in the linked
`emulator_details/records/<slug>.json` files; this page tracks whether those facts
exist and whether application code consumes them.

## Status definitions

- **Capture** means a source-cited record contains a controller configuration
  location/syntax pointer, BIOS/firmware location, and save location/naming for
  that platform. It is research evidence, not a runtime result.
- **Native** means a standalone controller writer/session/launch route is listed
  by `controller_coverage.rs`. Every native adapter is still partial and subject
  to the restrictions in its contract; none is promoted to runtime-verified here.
- **RetroArch** means the shared frontend has core mappings (93/94 core names),
  not a standalone adapter and not all-game/all-peripheral support.
- **Rule** means `sources/firmware-rules.json` has at least one runtime rule for
  that emulator. A captured firmware path without a rule is documentation-only.
- **Per-file** and **whole-image** are the classifications consumed by
  `platform_locations.rs`. Enumeration exists; upload/download, versioning,
  conflict handling, and restore do not.
- The current record and rule formats have no structured expected-checksum field.
  Consequently **0/34 records have machine-enforced BIOS/firmware checksum
  coverage**, even where prose evidence names a dump or firmware pack.

Platform columns: **L** native Linux, **F** Linux Flatpak, **M** macOS, **W**
Windows. `yes` means a record exists for that host; `--` is a capture gap.

## Current coverage

| Track | Current | Overall denominator | Percent | What remains |
|---|---:|---:|---:|---|
| Native controller source adapters | 28 | 249 catalog candidates | 11.2% | 221 standalone candidates lack a registered partial adapter |
| RetroArch core source contracts | 93 | 94 core names | 98.9% | One core contract plus runtime verification |
| Combined controller source entries | 121 | 343 | 35.3% | This combines two different partial-source denominators only |
| Platform records in active 40-emulator integration set | 34 | 40 | 85.0% | Six registered native adapters still lack records |
| Standalone catalog platform-record coverage | 33 | 249 catalog candidates | 13.3% | RetroArch's shared frontend record is tracked separately |
| Requested host cells captured | 111 | 136 (34 records x 4 hosts) | 81.6% | 25 host/emulator cells below are still absent |
| Records complete on all four hosts | 17 | 34 records | 50.0% | Fill every `--` platform cell |
| Linux capture | 34 | 34 records | 100.0% | Runtime verification remains |
| Flatpak capture | 18 | 34 records | 52.9% | 16 records lack a Flatpak entry |
| macOS capture | 26 | 34 records | 76.5% | 8 records lack a macOS entry |
| Windows capture | 33 | 34 records | 97.1% | DuckStation is Linux-only in the record |
| Captured records with controller source routing | 23 | 34 records | 67.6% | 22 native adapters plus the RetroArch frontend |
| Captured records with firmware runtime rules | 15 | 34 records | 44.1% | 14 named standalone runtimes plus shared RetroArch rules |
| Structured firmware checksum coverage | 0 | 34 records | 0.0% | Add expected identity fields and validation semantics |
| Save capture plus enumeration consumer | 34 | 34 records | 100.0% | States are also captured where supported; metadata/enumeration is not sync |
| End-to-end save synchronization | 0 | 34 records | 0.0% | Transport, identity, versioning, conflicts, atomic restore, UI |

## Captured emulator matrix

| Emulator record | L | F | M | W | Controller source | Firmware | Save model | Next concrete gap |
|---|:---:|:---:|:---:|:---:|---|---|---|---|
| [Altirra](../emulator_details/records/altirra.json) | yes | -- | -- | yes | capture only | capture only | whole-image | Flatpak/macOS decision; native controller adapter |
| [BizHawk](../emulator_details/records/bizhawk.json) | yes | -- | yes | yes | native | capture only | per-file | Flatpak capture; firmware rules as applicable |
| [BlastEm](../emulator_details/records/blastem.json) | yes | yes | yes | yes | native | capture only | per-file | Firmware rule/checksum policy; runtime verification |
| [Citron Neo](../emulator_details/records/citron-neo.json) | yes | -- | yes | yes | capture only | capture only | per-file | Flatpak capture, native adapter, Switch rules |
| [DeSmuME](../emulator_details/records/desmume.json) | yes | yes | yes | yes | native | capture only | per-file | Firmware rule/checksum policy; runtime verification |
| [Dolphin](../emulator_details/records/dolphin.json) | yes | yes | yes | yes | native | rule | per-file | Reconcile modern Windows root in the runtime adapter |
| [DOSBox Staging](../emulator_details/records/dosbox-staging.json) | yes | yes | yes | yes | capture only | capture only | whole-image | Native adapter; mounted-drive/image sync policy |
| [DuckStation](../emulator_details/records/duckstation.json) | yes | -- | -- | -- | native | rule | per-file | Capture Flatpak/macOS/Windows |
| [Eden](../emulator_details/records/eden.json) | yes | -- | yes | yes | capture only | rule | per-file | Flatpak capture; native controller adapter |
| [Emulicious](../emulator_details/records/emulicious.json) | yes | -- | yes | yes | capture only | capture only | per-file | Flatpak decision; native adapter and firmware rule |
| [Flycast](../emulator_details/records/flycast.json) | yes | yes | yes | yes | native | rule | per-file | Runtime verification and checksum policy |
| [Gearcoleco](../emulator_details/records/gearcoleco.json) | yes | -- | yes | yes | capture only | rule | per-file | Flatpak capture; native controller adapter |
| [Gopher64](../emulator_details/records/gopher64.json) | yes | -- | yes | yes | capture only | capture only | per-file | Flatpak capture; native adapter and firmware rule |
| [Hatari](../emulator_details/records/hatari.json) | yes | yes | yes | yes | native | capture only | whole-image | Disk-image conflict policy; firmware rule |
| [jgenesis](../emulator_details/records/jgenesis.json) | yes | -- | -- | yes | native | capture only | per-file | Flatpak/macOS capture; firmware rule |
| [Kronos](../emulator_details/records/kronos.json) | yes | -- | -- | yes | native | capture only | whole-image | Flatpak/macOS capture; backup-RAM atomicity |
| [MAME](../emulator_details/records/mame.json) | yes | yes | yes | yes | native | rule | per-file | Per-machine scope and checksum identity remain dynamic |
| [Mednafen](../emulator_details/records/mednafen.json) | yes | -- | yes | yes | native | rule | per-file | Flatpak capture; broader native module coverage |
| [melonDS](../emulator_details/records/melonds.json) | yes | yes | yes | yes | native | rule | per-file | DSi asset validation and runtime verification |
| [mGBA](../emulator_details/records/mgba.json) | yes | yes | yes | yes | native | rule | per-file | Runtime verification and checksum policy |
| [Nestopia UE](../emulator_details/records/nestopia-ue.json) | yes | -- | yes | yes | capture only | rule | per-file | Flatpak capture; native controller adapter |
| [openMSX](../emulator_details/records/openmsx.json) | yes | yes | yes | yes | native | rule | per-file | Runtime verification and checksum policy |
| [PCSX2](../emulator_details/records/pcsx2.json) | yes | yes | yes | yes | native | rule | per-file | Runtime verification and BIOS identity policy |
| [PPSSPP](../emulator_details/records/ppsspp.json) | yes | yes | yes | yes | native | capture only | per-file | Firmware rule policy if needed; runtime verification |
| [puNES](../emulator_details/records/punes.json) | yes | yes | -- | yes | capture only | rule | per-file | macOS capture; native controller adapter |
| [RetroArch](../emulator_details/records/retroarch.json) | yes | yes | yes | yes | RetroArch | rule | per-file | One core contract; per-core save/firmware identity |
| [RMG](../emulator_details/records/rmg.json) | yes | -- | -- | yes | capture only | capture only | per-file | Flatpak/macOS capture; native adapter |
| [RPCS3](../emulator_details/records/rpcs3.json) | yes | yes | yes | yes | native | capture only | per-file | Firmware installer/identity rule; runtime verification |
| [ScummVM](../emulator_details/records/scummvm.json) | yes | yes | yes | yes | native | capture only | per-file | Target/save identity and runtime verification |
| [simple64](../emulator_details/records/simple64.json) | yes | -- | -- | yes | capture only | capture only | per-file | Flatpak/macOS capture; archived-upstream decision |
| [Stella](../emulator_details/records/stella.json) | yes | yes | yes | yes | native | capture only | per-file | Runtime verification; firmware policy if applicable |
| [VICE](../emulator_details/records/vice.json) | yes | yes | yes | yes | native | rule | per-file | ROM-set identity and runtime verification |
| [xemu](../emulator_details/records/xemu.json) | yes | -- | yes | yes | native | capture only | whole-image | Flatpak capture; HDD/EEPROM atomic sync policy |
| [Yaba Sanshiro 2](../emulator_details/records/yaba-sanshiro-2.json) | yes | -- | -- | yes | native | capture only | whole-image | Flatpak/macOS capture; firmware rule/checksum policy |

## Registered controller adapters that still lack platform records

These six adapters are included in the 28/249 controller count but excluded from
the 34-record platform percentages. Their controller work is not a substitute
for the four-host configuration/firmware/save capture.

| Emulator | Controller source | Platform record | Next concrete gap |
|---|---|---|---|
| ares | native | missing | Capture L/F/M/W config, firmware, saves, and states |
| bsnes | native | missing | Capture L/F/M/W config, firmware, saves, and states |
| FCEUX | native | missing | Capture L/F/M/W config, firmware, saves, and states |
| Mesen / Mesen2 | native | missing | Capture L/F/M/W config, firmware, saves, and states |
| SameBoy | native | missing | Capture L/F/M/W config, firmware, saves, and states |
| Snes9x | native | missing | Capture L/F/M/W config, firmware, saves, and states |

## Review findings that must not be counted as completion

1. `platform_locations.rs` embeds all 34 records, but
   `ControllerCoverageDialog.qml` still hard-codes only 27 record slugs. Altirra,
   Citron Neo, DOSBox Staging, Eden, Emulicious, Kronos, and Yaba Sanshiro 2 are
   therefore not exposed through that dialog even though their records load.
2. The resolver retains documentation-only paths as prose and does not select a
   single host platform before resolving entries. `%APPDATA%` and
   `%LOCALAPPDATA%` always return unresolved, including in a Windows build;
   macOS-style `~/Library/...` entries can resolve under a Linux home; and
   Flatpak `~/.var/app/...` entries resolve without consulting the discovered
   sandbox roots. The comments promise stricter platform behavior than the code
   currently enforces.
3. The save consumer performs a bounded local file walk over every resolved
   save/state entry. It does not yet establish game identity, hash/version files,
   synchronize remotely, resolve conflicts, or restore atomically.
4. Six whole-image models require coordination semantics that per-file copying
   cannot provide: Altirra, DOSBox Staging, Hatari, Kronos, xemu, and Yaba
   Sanshiro 2.
5. Citron Neo has captured Switch key/firmware paths but no matching firmware
   rules. Eden has rules; that does not make the Citron record operational.
6. Controller coverage counts source routing only. The 28 adapters are partial,
   and the requested no-testing phase means none of this review establishes
   runtime behavior on Windows, macOS, Linux, or Flatpak.
