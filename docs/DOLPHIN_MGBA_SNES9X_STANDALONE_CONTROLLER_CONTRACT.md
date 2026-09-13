# Dolphin, mGBA, and Snes9x standalone controller contract

This document records the source-backed native boundaries for this adapter
batch. Serialization success is not runtime success.

## Pinned contracts

- Dolphin standalone is pinned to `dolphin-emu/dolphin@6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4`.
  GameCube controller sections are `[GCPad1]` through `[GCPad4]` (and profiles
  use the same `Device`, group, control, and optional `Range` keys). Dolphin's
  user root splits XDG config and data trees; controller configuration belongs
  under `Config`, while memory cards, Wii data, and `StateSaves` remain in the
  data root. Region-specific IPL files are optional and are not interchangeable
  with a generic BIOS name.
- mGBA standalone SDL is pinned to
  `mgba-emu/mgba@26b7884bc25a5933960f3cdcd98bac1ae14d42e2`. Joystick controls
  use `gba.input.SDLB` (`keyN`, signed `axisN`, and `hatN<Direction>` fields),
  with per-GUID `gba.input-profile.<GUID>` sections and a GUID preference key.
  Saves default beside the ROM as `<basename>.sav`; states use `.ss1` through
  `.ss9`, with `savegamePath`/`savestatePath` overrides. External BIOS is
  optional because the pinned source has an HLE fallback.
- Snes9x GTK standalone-native is pinned to
  `snes9xgit/snes9x@7a8878f1306f65594c30b7d86dee41d972c2e495`; the adapter's
  GTK binding grammar is recorded against the existing 1.63 source pin
  `921f9f7b83660eb44ad263022a57a4a029057c37`.
  Native settings use `[Joypad 0]`-style sections and one-based joystick device
  numbers; bindings distinguish buttons from signed axis thresholds. SRAM and
  states default beside the ROM, with configured directory overrides. Ordinary
  SNES cartridges need no external BIOS; optional BS-X/STBIOS files remain
  explicit firmware inputs.

## Hardening covered here

The adapters reject malformed or aliasing native values, preserve physical
device identity separately from backend numbering, clear inherited mGBA SDL
bindings before writing calibrated controls, and keep Dolphin raw patch lines
out of settings merges. Snes9x threshold differences on one axis direction do
not create a second routing owner.

## Remaining runtime boundary

The focused tests cover pure parsing and serialization invariants only. They do
not establish actual SDL/evdev discovery, device uniqueness at launch, process
startup, effective native configuration, firmware availability, gameplay input,
or SRAM/save-state round trips. Those claims require launch-time logs and
artifacts from the selected Dolphin, mGBA, or Snes9x runtime.
