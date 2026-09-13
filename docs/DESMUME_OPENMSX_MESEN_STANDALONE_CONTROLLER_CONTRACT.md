# DeSmuME, openMSX, and Mesen standalone-native controller contract

This document records the native contracts used by the three adapters. A
successful writer or parser check is not a runtime compatibility claim.

## Pinned upstream contracts

- DeSmuME POSIX is pinned to
  `TASEmulators/desmume@b3915949700be824253a35affa7f7b8248e84e46`.
  The GTK frontend persists a GLib keyfile at `desmume/config`, with a
  `[JOYKEYS]` integer for each DS control. Codes use device, type, and index
  fields (axis, hat, button), and unmapped Debug/Boost/Lid entries are
  explicitly `0xFFFF`. POSIX battery files default to `<rom>.dsv` in the
  module directory; ten quick states are `<rom>.ds0` through `.ds9`, with
  rolling `.dst` backups. ARM BIOS images are built in; optional external
  firmware is the explicitly enabled `firmware.bin` path.
- openMSX is pinned to
  `openMSX/openMSX@25179d6b8d5ec69ad68252f3854721c9a02594eb`.
  `msxjoystick1_config` and `msxjoystick2_config` are TCL settings containing
  `UP`, `DOWN`, `LEFT`, `RIGHT`, `A`, and `B` event lists. Native events are
  `joyN buttonK`, `joyN hatK <direction>`, or signed `joyN +/-axisK`, with
  one-based joystick numbers. XML settings preserve the required settings DTD.
  Persistent cartridge data remains in `persistent/<hardware>/<untitledN>/`;
  states are `savestates/<name>.oms` with a neighboring screenshot, and C-BIOS
  system ROMs provide the normal no-proprietary-firmware fallback.
- Mesen 2 is pinned to
  `SourMesen/Mesen2@b9fa69ddc6d0a331fb103fdb5eef6904305703c2`.
  Linux controller mappings are embedded in `settings.json`: `Nes.Port1` uses
  `Type: NesController` and `Mapping1` fields containing UInt16 key-manager
  values; unused ports are explicitly `Type: None`. Save data defaults under
  `Saves/`, and quick states under `SaveStates/` as `<rom>_<slot>.mss` for
  slots 1-10 (slot 11 is automatic). Firmware is system-specific and uses the
  source's exact names and checksum allow-list; ordinary NES/SNES cartridges
  need none.

## Hardening covered here

DeSmuME now rejects unknown, duplicate, or physically shared JOYKEYS entries;
Mesen rejects shared physical key-manager inputs. Existing openMSX checks cover
the one-based event grammar, required controls, cardinal hats, and XML escaping.
These checks prevent ambiguous or silently unbindable native mappings.

## Remaining runtime boundary

Focused tests cover pure parsing and serialization only. They do not establish
SDL/evdev discovery, unique device selection, native process startup, effective
configuration, firmware availability, gameplay input, or save/state round trips.
Those claims require launch-time logs and artifacts from the selected runtime.
