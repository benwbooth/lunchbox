# blueMSX standalone-native controller boundary

The adapter emits the native Windows profile only when the caller supplies an
explicit DirectInput runtime slot. SDL/Linux profile writing remains
unsupported because those source-backed frontends do not provide a native
joystick backend.

## Verified native format and roots

Pinned source: `ducasp/blueMSX@aef17c6c7e4a6cb93c58f6425569c4586a887be`.
`Src/Emulator/Properties.c` reads and writes `bluemsx.ini` through the native
INI API, under the `config` root.  Controller-related properties include
`joystick.POV0isAxes`, `joy1.type`, `joy1.autofire`, `joy2.type`,
`joy2.autofire`, and `keyboard.configFile`.  `propertiesSetDirectory()` first
uses `bluemsx.ini` in the current directory and otherwise uses the supplied
alternate directory.

The Windows frontend sets its alternate root to a writable current-directory
or temporary directory and creates `Keyboard Config` beneath it.  Its native
keyboard profiles are `<root>/Keyboard Config/<name>.config`; the profile
stores input-event mappings, while `Win32keyboard.c` discovers DirectInput
joysticks at runtime and stores button indices.  The SDL/Linux frontends use
the same property tree, but their SDL keyboard implementation has profile
save/load disabled and its joystick hooks return zero devices.

## Writer boundary

`joy1.type`/`joy2.type` select emulated port type, not a physical host device.
The Windows profile writer emits `[Keymapping-1]`/`[Keymapping-2]` entries
using the exact `joy1-*`/`joy2-*` event names and `J<n> UP`, `J<n> BT <m>`
DirectInput tokens used by `Win32keyboard.c`. The `n` slot must be explicitly
resolved by the caller; blueMSX does not persist a stable physical identifier.
The writer rejects missing, out-of-range, or ambiguous slots rather than
silently choosing one.

This does not claim Windows startup, effective input, firmware availability,
save-RAM, savestate, or cross-platform parity.  A future writer needs a
version-pinned device identity contract and isolated runtime validation.
