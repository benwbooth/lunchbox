# Hatari standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

hatari/hatari `11964da62914bf232ca84eacf0cedf1d25223e08`:

- `src/configuration.c` — `[Joystick0]`/`[Joystick1]` sections hold
  `nJoystickMode` (1 = JOYSTICK_REALSTICK), `nJoyId` (the SDL device index
  bound to the emulated port) and `nJoyBut1/2/3Index` (SDL button indices for
  the fire buttons; -1 disables a slot). `[ROM]` holds `szTosImageFileName`.
- `src/sdl/joy_ui.c` `JoyUI_ReadJoystick` — directions are hardcoded to SDL
  axis 0 (X) and axis 1 (Y) with hat 0 overriding the axes; buttons come from
  the configured indices. Axes beyond 0/1 and hats beyond 0 cannot drive
  directions and are rejected by this contract instead of silently ignored.
- `src/joy.c` — digital thresholds X ≤ -16384 / X ≥ 16383 and
  Y ≤ -16384 / Y ≥ 16383 in SDL axis units.
- `src/options.c` / `src/paths.c` — `-c <file>` reads additional
  configuration values and the configuration home resolves from `$HOME`, so a
  private HOME plus `-c` isolates every read and write from the user's own
  hatari.cfg while native saves stay inside the session directory.

## Integration

- `controller_hatari_native.rs` renders the private additional configuration
  (real-stick mode, device binding, fire slots, TOS selection) with unit
  tests; `controller_hatari_native/settings.rs` holds the saved launch setups
  (including the declared absolute TOS image, existence-checked at launch);
  `controller_hatari_native/session.rs` probes the trusted SDL2 runtime per
  player and enforces the axis-0/1 and hat-0 constraints with ±16384
  rest/travel checks; `controller_hatari_native/native_command.rs` owns the
  HOME-isolated launch.
- Guided target `hatari:standalone-hatari-native-joystick` (two players,
  Atari ST) registers a native ST joystick layout (up/down/left/right/fire
  plus optional fire2/fire3). Saved per-game setups are reused for another
  game of the same emulator.

## Limits

The ST mouse, analog paddles, joypad emulation (Jaguar pad), CAB hardware,
keyboard emulation and autofire toggles are not covered. Runtime behavior is
unverified; the TOS image is user-declared without a content hash.
