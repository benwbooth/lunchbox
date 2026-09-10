# DeSmuME standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

TASEmulators/desmume b3915949700be824253a35affa7f7b8248e84e46 (posix
frontends):

- `frontend/posix/shared/ctrlssdl.cpp` — joypad key codes are 4-hex-digit
  values `(device & 15) << 12 | type << 8 | index` with type 0=Axis, 1=Hat,
  2=Button. Axis halves are `2*axis` (negative) and `2*axis+1` (positive);
  hat directions are `4*hat + (0=right,1=left,2=up,3=down)`; buttons use the
  plain SDL button index. The device digit is the SDL enumeration index, and
  axis keys engage at |value| >> 14 (16384).
- `frontend/posix/shared/desmume_config.cpp` — the GLib keyfile holds a
  `[JOYKEYS]` section with one integer per `key_names[]` entry at
  `$XDG_CONFIG_HOME/desmume/config`; a private XDG_CONFIG_HOME isolates every
  configuration read and write from the user's own files.

## Integration

- `controller_desmume_native.rs` encodes the joypad codes and renders the
  private keyfile (the twelve mapped DS controls plus explicitly disabled
  0xFFFF entries for Debug, Boost and Lid) with unit tests;
  `controller_desmume_native/settings.rs` holds the saved launch setups;
  `controller_desmume_native/session.rs` probes the trusted SDL2 runtime and
  translates the calibrated controls through the classic physical map with
  16384 threshold checks; `controller_desmume_native/native_command.rs` owns
  the XDG-isolated single-player launch.
- Guided target `desmume:standalone-nds-native-buttons` (one player, Nintendo
  DS) reuses the DS buttons-only layout already registered for melonDS, so
  the same calibration drives either emulator. Saved per-game setups are
  reused for another game of the same emulator.

## Limits

Single joypad only; the touchscreen remains a mouse input; microphone, lid,
Debug and Boost are explicitly disabled; hotkeys, the SDL1/cli frontend and
the Windows/Cocoa frontends are not covered. Runtime behavior is unverified.
