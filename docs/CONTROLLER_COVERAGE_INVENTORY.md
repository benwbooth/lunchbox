# Controller coverage inventory

This is the remaining-work inventory for the original goal: calibrate a physical
controller once, select its layout, and apply the composed mapping automatically
at emulator launch. It is not a declaration that every listed emulator is covered.

## Authoritative baseline

Inspected repository base: `bfb60c18b1a14045cf607ff7eaa8edcd7c23ca5f`.
Database: `build/lunchbox.db`, SHA-256
`d73196b008177613ec2688cd7e1fefe4ac8e9b71add0b65bd57a89c4720a085e`.

| Inventory | Observed baseline |
| --- | --- |
| Emulator definitions linked to platforms | 249 |
| Platforms with an emulator relationship | 200 |
| Platform/emulator/core-list rows | 511 |
| Distinct core names after splitting semicolon lists | 98 |
| Described physical/target layouts | 16, in six families |
| Controller output profiles in committed catalog | 16 |
| Profiles enabled for automatic RetroArch launch | 12, across nine cores |

The database contains 253 platform records in total; 200 is the number linked
through `emulator_platforms`, not the total. Emulator definitions and core names
are discovery metadata, not necessarily installed executables. The in-progress
Beetle PSX work adds four output profiles to the working tree; those are not
included in the committed baseline counts above.

The policy-v2 layout report evaluates all 256 ordered pairs of the current 16
layouts. Of these, 146 cover all required target controls under the current rules;
110 explicitly remain partial. Neither 256 evaluated pairs nor 146 complete
layout assignments establishes runtime coverage of the emulator catalog.

Reproduce the database relationship count with a read-only SQLite query:

```sql
SELECT count(DISTINCT emulator_id), count(DISTINCT platform_id), count(*)
FROM emulator_platforms;
```

For the raw baseline core inventory, select nonempty `core_name` values, split
each on `;`, trim, and deduplicate. Discovery now additionally applies the explicit
[core identity aliases](RETROARCH_CORE_IDENTITIES.md) before deduplication, so its
canonical count can differ from the raw baseline. Do not count a semicolon list
as one core or confuse a core's marketing name with its library filename.

## Work grouped by reusable capability

These are implementation batches, not a new list of supported adapters. The named
core/platform relationships below are present in the inspected database. Hardware
classifications without a linked input source remain planning classifications;
their exact controls, modes and output IDs must be verified before activation.

| Batch | Representative catalog targets/cores | Shared work needed |
| --- | --- | --- |
| Existing digital layouts, more contracts | NES: `nestopia`, `mesen`; GB: `sameboy`; SNES: `bsnes_hd_beta`, `mesen-s`; Mega Drive: `blastem`, `picodrive` | Reuse existing layouts. Verify each core's output IDs, devices, mode options and player topology; add contracts, not physical-controller pair tables. |
| Missing small digital layouts | Atari 2600/`stella`, Atari 7800/`prosystem`, Neo Geo Pocket/`mednafen_ngp`, Lynx/`gearlynx`, Neo Geo/`geolith`, 3DO/`opera`, PC-FX/`mednafen_pcfx` | Describe face/control variants and console/menu actions. Separate game controls from console switches. Reuse button assignment once semantics are known. |
| Multiple directional clusters | Virtual Boy/`beetle_vb`; WonderSwan/`mednafen_wswan` and `beetle_cygne` | Model digital clusters and orientation explicitly. Generalize the directional capability bridge, keeping direction/cluster identity and core rotation options consistent. |
| Keypad/auxiliary panels | ColecoVision/`gearcoleco`, Intellivision/`freeintv`, Jaguar/`virtual_jaguar`, Atari 5200/`atari800` | Add keypad roles and an explicit strategy for insufficient buttons. Do not silently drop required keys or invent independent hardware inputs. |
| Analog/trigger variants | PSP/`ppsspp`, Dreamcast/`flycast`, GameCube/`dolphin`, PS2/`pcsx2` | Describe stick/trigger capabilities, ranges, full/half presses and optional functions from source. Shared assignment alone does not implement required signal conversion. |
| Relative and absolute pointing | Coleco spinner/roller modes, DS/`desmume` and `melonds_ds`, Wii/`dolphin`, ScummVM | Distinguish relative motion, absolute position, touch and sensors. Extend calibration/transport where needed; these are not interchangeable scalar buttons. |
| Keyboard-oriented systems | C64/`vice_x64sc`, Amiga/`puae`, MSX/`bluemsx`, DOS/`dosbox_pure`, ZX Spectrum/`fuse` | Separate joystick modes from keyboard needs; represent per-content requirements and explicit virtual-keyboard/layer behavior. |
| Multi-system arcade | `mame`, `fbneo`, `flycast` arcade modes | Resolve per-machine input descriptions and player/control topology. A single generic six-button profile does not cover every arcade machine. |
| Standalone and other hosts | Database-linked native emulators, Windows/macOS/Wine paths | Reuse the mapping plan, but implement each configuration/driver transport. The current automatic launch boundary accepts Linux native/Flatpak RetroArch only. |

Three verified examples show why simply adding more face-button diagrams is not
enough:

- Stella's documented controller interface includes console selection/reset and
  difficulty/color actions in addition to joystick fire/directions. Treat console
  controls separately from the physical joystick. [Stella controller tables](https://docs.libretro.com/library/stella/#controllers).
- Virtual Boy has two digital directional clusters and an option governing the
  right analog path. WonderSwan's rotate action changes the active directional
  pad. These need explicit cluster/mode semantics, not a generic four-face rule.
  [Beetle VB](https://docs.libretro.com/library/beetle_vb/#controllers),
  [Beetle Cygne](https://docs.libretro.com/library/beetle_cygne/#controllers).
- Gearcoleco documents keypad controls and spinner modes using mouse movement.
  This crosses digital-keypad and relative-motion transports.
  [Gearcoleco options and inputs](https://docs.libretro.com/library/gearcoleco/#core-options).

These documentation checks are planning evidence. No exact new core binding or
device number is accepted merely from an image-only table or a platform name.

## Cross-cutting gaps before all-core completion

1. **Core identity normalization.** Eight confirmed Beetle aliases now resolve to
   upstream Mednafen library names in catalog loading and filename discovery; see
   [the identity table and test boundary](RETROARCH_CORE_IDENTITIES.md). Remaining
   names still require identity evidence before adding aliases. Recognition alone
   does not establish a controller contract.
2. **Effective device and option layers.** Port type, remaps, overrides, per-game
   options, controller modes and multiplayer topology must agree with the plan.
   Resolve these as adapter/mode data; do not hide unresolved layers behind a
   successful layout assignment.
3. **Full one-calibration launch test.** Exercise physical discovery, persisted
   calibration, prepared content, device selection, generated configuration and
   emulated input readback in one path. The [saved-calibration GBA
   oracle](CONTROLLER_RETROARCH_ORACLE.md#saved-calibration-launch-path) now covers
   real supported virtual-pad discovery, private settings persistence, the ROM
   plan builder, production `prepare`, preferred-player selection, generated
   Flatpak grants and arguments, and emulated input readback. Physical GUI capture,
   automatic core selection and the desktop launch action remain unverified in
   that complete path; the oracle is not all-core or physical-Brawler evidence.
4. **Coverage accounting.** Track layout representability, assignment completeness,
   contract provenance, launch-adapter support and runtime evidence separately for
   every catalog core/platform/mode. A missing required capability remains visible.

The next implementation batch should combine additional contracts for already
represented digital layouts with the missing small-digital/dual-cluster layout
descriptions. Larger input types remain in scope and need their own explicit
capability transforms and transport work; they are not excluded by the current
16-layout test matrix.
