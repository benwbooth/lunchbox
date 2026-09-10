# openMSX standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

openMSX/openMSX 25179d6b8d5ec69ad68252f3854721c9a02594eb:

- `src/input/MSXJoystick.cc` — each MSX joystick reads the TCL setting
  `msxjoystick1_config`/`msxjoystick2_config`: a list of key/value pairs with
  keys UP, DOWN, LEFT, RIGHT, A, B whose values are lists of event specs.
- `src/events/BooleanInput.cc` — event specs are `joyN buttonK`,
  `joyN hatK up|right|down|left` and `joyN +axisK`/`joyN -axisK` with
  0..255 indices; `joyN` numbers host joysticks from one
  (`JoystickId::parse`).
- `src/config/SettingsConfig.cc` — settings persist as
  `<!DOCTYPE settings SYSTEM 'settings.dtd'>` with nested `<settings>`
  elements holding `<setting id="...">value</setting>`; `-setting <file>`
  loads that file instead of the user's settings.xml and auto-saves back to
  it on exit.
- `src/CommandLineParser.cc` — `-command <tcl>` executes TCL at startup
  (used to plug `msxjoystick2` into joyportb).
- `src/file/FileOperations.cc` — `OPENMSX_HOME` relocates the user
  directory, keeping auto-loaded user scripts and savestates away from this
  launch.
- `src/input/JoystickManager.cc` — the per-joystick dead zone defaults to
  25 percent (≈8192 in SDL axis units), used for the axis travel checks.

## Integration

- `controller_openmsx_native.rs` renders the event specs, per-joystick dicts
  and the private settings XML with unit tests; `controller_openmsx_native/
  settings.rs` holds the saved launch setups; `controller_openmsx_native/
  session.rs` probes the trusted SDL2 runtime per player and translates the
  calibrated controls into dict bindings; `controller_openmsx_native/
  native_command.rs` owns the OPENMSX_HOME-isolated `-setting` launch with the
  second-port plug command.
- Guided target `openmsx:standalone-openmsx-native-joystick` (two players)
  registers a native MSX joystick layout (up/down/left/right plus triggers A
  and optional B) covering the MSX/MSX2/MSX2+/Spectravideo platforms. Saved
  per-game setups are reused for another game of the same emulator.

## Limits

MSX mice, JoyMega, the Arabic/trackball pads, Arkanoid paddles, keyboard
bindings, hotkeys and TCL-level remapping of other devices are not covered.
Runtime behavior is unverified; the dead-zone resource is assumed at its
default.
