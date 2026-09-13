# Mednafen, MAME, and ares standalone-native controller contract

This document records the native contracts used by these adapters. Pure writer
checks do not establish runtime compatibility.

## Pinned upstream contracts

- Mednafen native paths and input expressions are pinned to
  `libretro-mirrors/mednafen-git@f0ee9d595db68ad5247ba5ac6a8367fdced9c3fc`.
  Native joystick bindings use a complete 128-bit inventory ID and expressions
  such as `joystick 0x<id> button_N` or `abs_N` with polarity and fixed-point
  scale. Configuration layers are flat `key value` files under the private
  Mednafen base. Saves default under `sav/` with per-system extensions; generic
  gzip states are `mcs/<rom>.mc0` through `.mc9`; firmware remains an explicit
  per-system `firmware/` input.
- MAME 0.280 controller configuration is pinned to the recorded
  `mamedev/mame@ec9abd86c6` contract and its version-10 XML format. Native
  controller presets use `<mapdevice>` stable IDs and `JOYCODE_`/`KEYCODE_`
  sequences, while machine defaults are loaded from `cfg/default.cfg` and
  `<shortname>.cfg`. NVRAM is under `nvram/`, states under `sta/<shortname>/`
  as `.sta` files, snapshots under `snap/`, and ROM/BIOS sets under the
  configured `rompath`/`hashpath` trees.
- ares desktop is pinned to
  `ares-emulator/ares@af4cbb04f067682a8a3cf42695ff78bed634b38d`.
  `settings.bml` stores `VirtualPad<n>/<input>` mappings as semicolon-separated
  device/group/input assignments with `Lo`, `Hi`, and `Rumble` qualifiers.
  Saves default beside the ROM (or configured `Paths/Saves`), and states use
  individual `.bs<slot>` files plus `.bsu`/`.blu` undo files. Firmware is
  manifest-identified and matched by SHA-256 rather than one universal name.

## Hardening covered here

Mednafen now has focused checks for 128-bit identity, fixed-point scaling, and
input bounds. MAME tests cover stable panel naming, duplicate ownership, and
token validation. Existing ares coverage exercises GUID/slot identity,
calibrated SDL translation, private settings preservation, and profile limits.

## Remaining runtime boundary

Focused tests cover pure parsing and serialization only. They do not prove
native provider discovery, unique device selection at launch, process startup,
effective configuration, firmware availability, gameplay input, or save/state
round trips. Those claims require launch-time logs and artifacts from the
selected executable.
