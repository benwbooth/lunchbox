# bsnes standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

bsnes-emu/bsnes `7d5aa1e656b9171524d01b1b22917197d8121cb4` (v115 line, May 2026):

- `target-bsnes/input/input.cpp` — mapping assignments are
  `0x{device-id}/{group}/{input}[/{Lo|Hi}]`, up to four per control joined by
  `;`. Digital axis/hat qualifiers engage at ±16384 in SDL joystick units.
  Node paths are `{System}/{Port}/{Device}/{Input}` with spaces removed, so the
  SNES gamepad controls live under `SuperFamicom/ControllerPort{1,2}/Gamepad/`.
- `nall/hid.hpp` — Joypad group ids are Axis=0, Hat=1, Trigger=2, Button=3.
  SDL-joypad devices use generic vendor `0x0000`/product `0x0003` ids, so a
  device id is `sdl_joystick_index << 32 | 3` (for example `0x3`, `0x100000003`).
- `ruby/input/joypad/sdl.cpp` — the SDL joypad driver (the driver official
  Linux builds link, including the dev.bsnes.bsnes Flatpak v115): device index
  is SDL's enumeration index; hat input `2*hat+0` is the X axis (Left=Lo,
  Right=Hi) and `2*hat+1` the Y axis (Up=Lo, Down=Hi); buttons/axes follow SDL
  joystick order. The alternative udev driver has a different id scheme
  (CRC32 of the USB devpath) and is not covered.
- `target-bsnes/bsnes.cpp` — `--settings=<file>` selects a private settings
  file, `--fullscreen` toggles fullscreen and a bare existing path argument
  loads the game. `Settings::load()` parses, applies defaults and immediately
  rewrites that file, so Lunchbox always passes a fresh private file and the
  user's own `settings.bml` is never read or written.
- `sfc/interface/interface.cpp` — port devices None/Gamepad; both controller
  ports default to Gamepad, so an unused port 2 is explicitly set to None.

## Integration

- `controller_bsnes.rs` encodes the assignment grammar and renders the private
  `settings.bml`; `controller_bsnes/settings.rs` holds the saved launch setups;
  `controller_bsnes/session.rs` probes SDL2 numbering at launch through the
  trusted helper and the SDL2 library the bsnes build links, translates
  calibrated physical controls to SDL joystick inputs and owns the private
  settings tempdir; `controller_bsnes/native_command.rs` owns the launch.
- Guided target: `bsnes:standalone-snes` (transport `bsnes-native-settings`),
  two players, SNES platform. Saved per-game setups are reusable for another
  game of the same emulator at launch.
- Limits: standard Gamepad targets only. Mouse, Super Multitap (3–5 players),
  Super Scope, Justifier(s), Satellaview expansion input, hotkeys and turbo
  buttons are not covered. Saves, states and recent-game history keep bsnes's
  own defaults from the private file. Runtime startup confirmation (like
  SameBoy's) is not implemented; topology and hash guards run before spawn.
- The probe additionally gained a generic read-only `--evdev-catalog` mode that
  dumps raw evdev capability bitmaps and sysfs identity; it is not used by this
  contract but is available for verifying udev-driver builds later.
