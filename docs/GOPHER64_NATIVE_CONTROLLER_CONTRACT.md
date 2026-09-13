# Gopher64 standalone controller contract

Source-level integration; the focused writer tests pass, but no Gopher64 binary
or physical controller was exercised.

## Pinned functional contract

gopher64/gopher64 0bb9fbba638f5cebe3b8a3c1c245abcfec8b0132
(v1.1.36-12):

- `src/ui/config.rs` writes `config.json` with an `input` object.
  `input_profiles` maps names to profiles containing nineteen pairs of optional
  externally tagged input objects, `input_profile_binding[4]` chooses the
  profile per port, and `controller_assignment[4]` stores exact device paths.
- `src/ui/input_profile.rs` fixes indices 0–17 as D-pad right/left/down/up,
  Start, Z, B, A, C right/left/down/up, R, L, and analog-stick
  right/left/down/up. Index 18 is the hotkey activator. SDL3 gamepad bindings
  serialize as `ControllerButton` or `ControllerAxis` objects.
- `src/ui/input.rs` matches the saved assignment against
  `SDL_GetJoystickPathForID`, opens the device through SDL3's gamepad API when
  `dinput` is false, and reads the standard gamepad button/axis IDs stored in
  the selected profile.
- `src/ui.rs` selects `$XDG_CONFIG_HOME/gopher64/config.json` separately from
  `$XDG_DATA_HOME/gopher64/{saves,states}`. A `portable.txt` beside the
  executable instead forces `portable_data` and takes precedence over XDG.

## Integration

- `controller_gopher64_native.rs` registers the four-port N64 layout and
  renders exact nineteen-entry profiles. It copies the declared `config.json`
  plus existing `cheats.json` and `retroachievements.json` companions to a
  private `XDG_CONFIG_HOME`, replacing only profile selection, device
  assignment, and enabled-port fields. Unknown JSON and Transfer Pak/VRU fields
  are preserved in the private copy.
- At launch, the trusted target SDL3 library is probed with
  `SDL_JOYSTICK_LINUX_CLASSIC=1`. Exact physical paths are resolved, the
  calibrated evdev controls are projected through the runtime's classic map and
  resolved SDL gamepad bindings, and only standard SDL3 gamepad button/axis
  outputs are accepted. Analog stick pairs must resolve to opposite halves of
  two distinct proportional axes.
- The child receives the same SDL backend hint and private config home.
  `XDG_DATA_HOME` is not changed, so ordinary native save and save-state files
  remain in Gopher64's persistent data root and remain visible to the save-sync
  resolver.
- Guided target `gopher64:standalone-n64` uses the existing N64 layout for up
  to four players. Per-game runtime declarations can be reused only when their
  non-player identity fields agree.

## Limits

Native Linux only. Portable mode is rejected because it bypasses the private
config root. The adapter preserves existing Transfer Pak and VRU configuration
but does not author those peripherals, keyboard hotkeys, cheats, or
RetroAchievements credentials. Custom launch arguments and SDL3 logical-only
calibrations are not covered. Runtime controller behavior is unverified.
