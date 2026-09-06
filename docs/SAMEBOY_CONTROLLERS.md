# SameBoy automatic controller contract

Source contract: upstream SameBoy libretro branch
[`8230189896a8bb6598574d302ba0ad3658f98ab4`](https://github.com/LIJI32/SameBoy/tree/8230189896a8bb6598574d302ba0ad3658f98ab4).
This is the source revision reported by the tested official 1.0.3 core, not the
older `libretro/SameBoy` fork. The core source uses the Expat (MIT) license; the catalog records
input-interface facts and this repository's own mapping policy, not copied core
implementation or boot-ROM bytes.

The existing Game Boy layout is composed with the user's saved physical layout
and native input capture. No new Brawler64-specific pair table is needed.

| Game Boy control | RetroPad semantic | libretro JOYPAD ID |
| --- | --- | --- |
| A | East | 8 |
| B | South | 0 |
| Select | Select | 2 |
| Start | Start | 3 |
| Up | DPadUp | 4 |
| Down | DPadDown | 5 |
| Left | DPadLeft | 6 |
| Right | DPadRight | 7 |

[`GB_update_keys_status`](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c#L125-L159)
uses these IDs in both individual and bitmask callback paths. The core advertises
joypad subclass 257 for Game Boy and SGB, but its device setter only logs the
request. Both default device 1 and explicit device 257 therefore have contracts;
other requested devices are not silently converted into a supported mode.
See [device descriptions](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c#L428-L434)
and [setter/library identity](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c#L1260-L1276).

## Model-dependent ports

`player_topology` describes an exact core option, its default, and a bounded
value-to-port-count table. For SameBoy it resolves `sameboy_model`:

- Explicit Super Game Boy, Super Game Boy PAL, and Super Game Boy 2: four ports.
- Auto, Auto (SGB), Game Boy, Pocket, Color 0/A/B/C/D/E, Advance, and Player: one
  frontend port. The exact option string for Color E is `Game Boy Color`.

The `Auto (SGB)` one-port result deliberately matches the pinned frontend:
the polling branch tests the requested `model[0]`, not the detected hardware
model. It does not establish that auto-detected SGB multiplayer works.
See [model parsing](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c#L723-L772),
[polling branch](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c#L1389-L1400),
and [all option values/default](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro_core_options.inc#L76-L103).

Launch preparation validates the prepared core/content against the actual command,
selects the effective game/folder/core/global options file, and preserves the chosen
model in a private snapshot. These files are alternatives, not layers to merge: a
game file that omits the model uses Auto even if a lower-priority file selects SGB.
Unrelated options remain in the snapshot. The model is never changed merely to
accommodate connected controllers. Disabled/unfilled ports are cleared, and
extra calibrated pads cannot create nonexistent handheld ports.

Unknown option values, duplicate/include-based settings, unresolved custom
configuration/command arguments, and existing core/game remap/override files
produce a visible preparation error rather than guessed wiring. The two-content
Game Boy Link subsystem remains a separate, unimplemented launch path. This
contract does not add GBA game support, cartridge sensors/camera, physical rumble,
standalone SameBoy, or non-Linux transport.

## Verification boundary

App tests cover all fifteen model strings, default selection, invalid topology
metadata, option-file precedence, private snapshots, both device IDs, Brawler64
button composition, preference ordering with five candidate controllers,
disabled-port gaps, and rejection of unsupported/substituted launch arguments.

The [real-core diagnostic](../crates/lunchbox-controller-probe/LIBRETRO_INPUT.md#game-boy--sameboy)
checks emulated JOYP register reads in ordinary Game Boy mode. It is not evidence
of physical GUI calibration, RetroArch configuration processing, SGB multiplayer
protocol behavior, or the complete desktop launch action. Those remain distinct
runtime gates.
