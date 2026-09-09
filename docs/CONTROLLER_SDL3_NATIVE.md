# Native SDL3 controller input

Lunchbox uses SDL 3.4.12 or newer for Steam Controller (2026), also called SC2.
The imported `gamecontrollerdb.txt` does not activate SDL's native HID drivers;
the running library does. The first native integration targets Valve VID 28de,
PIDs 1302–1305. Other controllers retain their existing GilRs input path.

Sources:

- https://github.com/libsdl-org/SDL/blob/release-3.4.12/src/joystick/controller_list.h
- https://github.com/libsdl-org/SDL/blob/release-3.4.12/src/joystick/hidapi/SDL_hidapi_steam_triton.c
- https://github.com/libsdl-org/SDL/releases/tag/release-3.4.12

## Runtime and identity

The app starts its own executable with `--sdl3-input-stream`. SDL initializes,
pumps events and reads gamepads on that helper process's main thread; it does
not share SDL/Qt thread assumptions or emulator-process device numbering.
The helper opens only SC2 gamepads. SDL supplies serial, path, VID/PID, resolved
mapping, button state and axis state. No rumble or firmware commands are issued
by Lunchbox. SDL itself may change lizard mode while the gamepad is open.
Closing the parent's stdin pipe requests normal SDL shutdown/restoration;
forced termination is a timeout fallback.

`LUNCHBOX_SDL3_LIBRARY` selects a trusted runtime library. Nix builds embed the
store path; Flatpak builds embed `/app/lib/libSDL3.so.0`. A library beside the
executable is supported for portable packages, followed by the normal platform
library name. Missing/old runtimes produce a discovery warning and do not
disable GilRs. For a read-only identity/state diagnostic (opens then closes):

```text
lunchbox --sdl3-input-inspect
```

Stable serial-based keys are separate from model identity. Missing or duplicate
serials use session-local keys instead of merging identical controllers. The
UI keeps Steam Input virtual output separate and labels it as virtual; it never
guesses which native device produced a virtual controller's events.

## Mapping and limits

"Use SDL3 mapping" saves a `sdl3-gamepad` calibration with SDL-standard logical
buttons/axes and their physical-layout roles. It reuses the existing global
layout assignment algorithm. Manual edits and live button identification also
work through this input path. Initial held inputs are ignored until changed;
neutral completion waits for all active inputs to release, and disconnects
invalidate pending capture.

SDL logical codes have a separate namespace and cannot carry evdev measurements
or be saved as GilRs codes. Existing emulator launch adapters retain their
physical-input compatibility checks: SDL-native setup does **not** prove native
SC2 launch mapping in every emulator. Linux evdev-only adapters cannot consume
these calibrations without an explicit translation/transport implementation.

The preset includes 32 control roles: standard gamepad buttons and stick/trigger
directions, Steam/quick-access buttons, four grip buttons and two pad clicks.
The layout is a schematic, not an exact SC2 shell drawing. Touch coordinates,
gyro, haptics, pressure and grip/touch sensor semantics are not mapped to target
systems by this integration. Capacitive stick/grip sensor flags are ignored by
calibration so merely holding the controller cannot block a button's release.

Local live validation: SDL 3.4.12 opened the connected USB SC2 (28de:1302),
returned its serial and resolved native mapping. Unit tests cover code namespaces,
identity collisions, neutral/capture and default-layout validation. This is not
a hardware button-press, Bluetooth, Windows/macOS or all-emulator certification.

The broader working-tree controller suite currently reports 88 passes, 6
failures and 8 ignored tests. Failures concern Flycast 6/8-button selection,
an older DuckStation reason-string assertion, physical-pressure fixtures,
PCE/Activator menu routing and PCE-6 self-mapping. Those existing workflows are
not certified by this change and are outside this native-driver integration.

## Redistribution

SDL is zlib licensed. Packaged SDL binaries/source retain their upstream license
notice. No new Rust dependencies are required for the dynamic SDL3 runtime.
