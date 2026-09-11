# xemu standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

mborgerson/xemu fd0ae0c0a189d56e87f8e46073b15b287e4a1e1a:

- `ui/xemu.c` — `-config_path <file>` selects the configuration file (loaded
  instead of the user's `xemu.toml` under `SDL_GetPrefPath("xemu","xemu")` and
  auto-saved back at exit); `sys.files.dvd_path` mounts the XISO at boot.
- `ui/xemu-input.c` — `input.bindings.port1..4` hold SDL GUID strings (from
  `SDL_GUIDToString`); `input.auto_bind` must be false so an empty port does
  not steal a device; each connected gamepad reads its
  `input.gamepad_mappings[]` entry keyed by `gamepad_id`, and the nested
  `controller_mapping` maps SDL_Gamepad standard button/axis indices to Xbox
  functions with `invert_axis_*` flags, range-validated at runtime against
  `SDL_GAMEPAD_*_COUNT` (`xemu_input_bindings_set_in_range`).
- `config_spec.yml` — the standard-index defaults (a=0…dpad_right=14; axes
  leftx=0…righttrigger=5) and the complete schema.
- Numbering: the probe and the launched child both run with
  `SDL_JOYSTICK_LINUX_CLASSIC=1`, the documented SDL3 hint selecting the
  classic `/dev/input/js*` backend, so raw indices follow the joydev order the
  verified classic map translates. The calibration's evdev codes map to raw
  indices through that map, and SDL's own resolved gamepad bindings (queried
  from the same library in the same process) provide the standard indices
  xemu consumes.

## Integration

- `controller_xemu_native.rs` renders the private xemu.toml (sys.files mount
  trio, input.bindings GUID ports with the Duke driver, auto-bind disabled
  and per-controller controller_mapping tables) with unit tests;
  `controller_xemu_native/settings.rs` holds the saved launch setups
  including the user-declared MCPX boot ROM and flash image;
  `controller_xemu_native/session.rs` probes the trusted SDL3 runtime per
  player, matches each calibrated control's raw input against SDL's resolved
  gamepad bindings and composes the standard-index fields with stick-pair
  inversion; `controller_xemu_native/native_command.rs` owns the
  `-config_path` launch with the classic-hint environment.
- The probe's classic-interface capture gate is extended from exactly SDL
  3.2.20 to the SDL 3.2+ line (the hint is a documented SDL3 semantic and the
  count validation still fails loudly on disagreement); the DuckStation
  player projection keeps its exact 3.2.20 verification pin.
- Guided target `xemu:standalone-xbox` (four players) reuses the existing
  xbox layout for the Microsoft Xbox platform. Saved per-game setups are
  reused for another game of the same emulator.

## Limits

Standard Duke pads only. Same-GUID devices are rejected rather than
ambiguously bound; Steel Battalion, the S controller driver (`DRIVER_S`),
peripheral ports, hotkeys and the dashboard UI input are not covered. The
boot ROM and flash image are user-declared without content hashes. Runtime
behavior is unverified.
