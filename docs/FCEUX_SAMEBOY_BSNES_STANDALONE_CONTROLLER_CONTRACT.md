# FCEUX, SameBoy, and bsnes standalone-native controller contract

This is the source-backed boundary for the three standalone adapters in this
batch. A valid serialized configuration is not evidence of a working runtime.

## Pinned upstream contracts

- FCEUX Qt/SDL paths are recorded against `TASEmulators/fceux@a62b868e9247c4aafd66f597cdfa8d2609704087`, while the existing native
  controller grammar is pinned to the adapter's FCEUX 2.6.6 source
  `34eb7601c415b81901fd02afbd5cfdc84b5047ac`.
  GamePad profiles use SDL logical names (`A`, `B`, `Select`, `Start`,
  directions, and Turbo buttons) in `input/<SDL GUID>/<profile>.txt`; the
  `fceux.cfg` `SDL.Input` keys select the profile/device. The native packed
  values distinguish buttons, signed axes, and one-digit cardinal hats, and
  measured axes must cross FCEUX's fixed thresholds. Saves remain under the
  base `sav/` directory by default and quick states under `fcs/` (`.fc0` through
  `.fc9`, plus resume/autosave names). FDS firmware is the explicit 8192-byte
  `disksys.rom`; ordinary NES play needs no external firmware.
- SameBoy SDL v1.0.3 is pinned to
  `LIJI32/SameBoy@208ba4afabffab9edde416f2dbb8ae459e34adb8`. Its `prefs.bin`
  is a binary `configuration_t`, not an INI: joystick button and axis arrays
  use byte indices with `255` as the disabled sentinel, while hats are handled
  by the frontend. Saves and states default beside the ROM (`.sav`/`.ram` and
  `.s0` through `.s9`, with legacy `.snN` loading); bundled open-source boot
  ROMs mean no mandatory external BIOS.
- bsnes desktop is pinned to
  `bsnes-emu/bsnes@7d5aa1e656b9171524d01b1b22917197d8121cb4`. Native mappings
  are BML assignments in `settings.bml`, using `0x<device-id>/<group>/<input>`
  with optional `Lo`, `Hi`, or `Rumble` qualifiers. SDL device IDs are the
  emulator's enumeration index encoded with the generic joypad product id;
  unused controller ports are explicitly `None`. Saves default beside the ROM
  (`.srm`, `.rtc`, `.psr`, or `.sav` as applicable), and states are `.bsz`
  archives containing `.bst` members. Firmware is manifest-declared rather than
  one universal BIOS file.

## Hardening covered here

The adapters reject packed-value aliasing and threshold mismatches, reserve
SameBoy's disabled byte before writing button arrays, and reject bsnes mappings that assign one physical
input to multiple gameplay controls. These checks preserve native identity and
avoid silently producing an unbindable or ambiguous mapping.

## Remaining runtime boundary

Focused tests cover pure parsing and serialization only. They do not prove
SDL/evdev discovery, unique device selection at launch, process startup,
effective frontend configuration, firmware presence, gameplay input, or
save/state round trips. Those claims require launch-time logs and artifacts
from the actual FCEUX, SameBoy, or bsnes executable.
