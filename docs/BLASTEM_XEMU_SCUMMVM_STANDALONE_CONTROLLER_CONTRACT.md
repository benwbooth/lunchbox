# BlastEm, xemu, and ScummVM standalone-native controller contract

This document records the source-backed native boundaries for these adapters.
Serialization and parser checks are not proof of runtime compatibility.

## Pinned upstream contracts

- BlastEm is pinned to the mirrored `libretro/blastem@b4d75247ebad8852fd9bc385b423df704c6c5af5` source. Its tern configuration stores
  controller bindings under `bindings/pads/<SDL device index>` with `dpads`,
  `buttons`, and signed `axes` nodes targeting `gamepads.<1-8>.<button>`.
  Configuration is under the private HOME-based `.config/blastem` tree;
  per-ROM saves and states remain under `.local/share/blastem/<ROMNAME>/`
  (`save.sram`, `save.eeprom`, `save.nor`, `slot_0.state` through
  `slot_9.state`, and `quicksave.state`). Cartridge play needs no BIOS.
- xemu is pinned to `mborgerson/xemu@fd0ae0c0a189d56e87f8e46073b15b287e4a1e1a`.
  Native `xemu.toml` stores SDL GUIDs in `input.bindings.port1..4`, disables
  `input.auto_bind`, and writes standard SDL gamepad button/axis indices under
  `input.gamepad_mappings[].controller_mapping`. The HDD qcow2 image contains
  per-game saves and QEMU snapshots; `eeprom.bin`, MCPX boot ROM, and BIOS
  flash paths remain explicit sidecars/configured firmware inputs.
- ScummVM native SDL is pinned to
  `scummvm/scummvm@3f6428df202e2c044ef53208acba0f665ea92096`. The `[keymapper]`
  and per-target INI domains store `keymap_engine-default_<action>` hardware
  IDs (`JOY_A`, D-pad IDs, and related SDL standard controls); `joystick_num`
  selects the resolved SDL device and `-c` selects a private config. Engine
  saves default under the configured save path with `<target>.NNN` or
  `<target>.sNN` names. ScummVM has no save-state facility and requires no BIOS,
  though some engines need extra data through `extrapath`.

## Hardening covered here

BlastEm now rejects unknown targets and shared physical inputs. xemu validates
that ports and SDL GUIDs are unique and within the native four-port range.
ScummVM rejects duplicate SDL mapping fields instead of silently overwriting
the earlier field. Existing tests continue to cover the native syntax and
required controls.

## Remaining runtime boundary

Focused tests cover pure parsing and serialization only. They do not establish
SDL/evdev discovery, device identity at launch, process startup, effective
configuration, firmware availability, gameplay input, HDD persistence, or
save/state round trips. Those claims require launch-time logs and artifacts
from the selected executable.
