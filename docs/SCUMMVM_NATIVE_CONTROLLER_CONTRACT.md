# ScummVM standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

scummvm/scummvm 3f6428df202e2c044ef53208acba0f665ea92096:

- `common/config-manager.cpp` — the ini `[keymapper]` domain and per-target
  game domains persist `keymap_<keymap-id>_<action-id> = <hw input ids>`
  entries (`backends/keymapper/keymap.cpp` loadMappings/saveMappings with
  the `keymap_` prefix, space-separated ids).
- `engines/metaengine.cpp initKeymaps` — engines without custom keymaps get
  the `engine-default` game keymap (LCLK, RCLK, PAUSE, MENU, SKIP, SKLI,
  PIND, RETURN, UP, DOWN, LEFT, RIGHT) with joystick defaults `JOY_A`,
  `JOY_B`, `JOY_LEFT_SHOULDER`, `JOY_Y`, `JOY_X` and the D-pad ids; game
  keymaps load from the active target's own ini domain.
- `backends/keymapper/hardware-input.cpp` + `backends/events/sdl/
  sdl2-events.cpp mapSDLControllerButtonToOSystem` — joystick hardware ids
  `JOY_A/B/X/Y/BACK/GUIDE/START/LEFT_STICK/RIGHT_STICK/LEFT_SHOULDER/
  RIGHT_SHOULDER/UP/DOWN/LEFT/RIGHT` map 1:1 onto SDL's standard gamepad
  button order.
- `base/commandLine.cpp` — `joystick_num` (default 0) selects the SDL
  device; `-c <file>` selects the configuration file.
- `backends/platform/sdl/posix/posix.cpp` — the configuration resolves to
  `$XDG_CONFIG_HOME/scummvm/scummvm.ini`, so a private XDG_CONFIG_HOME
  isolates reads and writes.

## Integration

- `controller_scummvm_native.rs` parses the device's SDL gamecontroller
  mapping string into standard-field → raw-backing pairs (unit-tested),
  maps the JOY ids and renders the private ini (app section with
  `joystick_num=0`, target section with the game path and every
  `keymap_engine-default_*` entry); `controller_scummvm_native/settings.rs`
  holds the saved launch setups; `controller_scummvm_native/session.rs`
  probes the trusted SDL3 runtime, matches each calibrated control's raw
  backing to its standard field and requires the selected controller to be
  SDL device zero; `controller_scummvm_native/native_command.rs` owns the
  `-c <ini> <target>` launch with the private XDG_CONFIG_HOME.
- Guided target `scummvm:standalone-scummvm-default-actions` (one player)
  registers a default-actions layout for the ScummVM platform. Saved
  per-game setups are reused for another game of the same emulator.

This commit also completes the settings-model exposure for the eight most
recent adapters (VICE, Hatari, DeSmuME, openMSX, Mesen2, BlastEm, xemu,
ScummVM): their cxx-bridge declarations and implementations had been lost
to partially failed wiring scripts, leaving QML entries calling unexposed
methods. All eight now declare, implement and surface their setup editors.

## Limits

Single SDL device zero; engines with custom keymaps, the GUI/global
keymaps, keyboard remapping and touch devices are not covered. Runtime
behavior is unverified.
