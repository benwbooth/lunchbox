# Target-runtime SDL controller probe

This small Rust helper dynamically loads an explicitly supplied, trusted SDL3
library and emits a JSON inventory. Run it as its own process on the main thread,
inside the emulator's runtime when that runtime differs from the host (Flatpak,
for example). Loading a native library executes code; this is not a scanner for
untrusted downloads.

By default it uses SDL3's enumeration/mapping APIs without opening devices.
With `--bindings-for-path`, it briefly opens only the explicitly selected gamepad
to query typed bindings and joystick control counts, then closes it. SDL may do
its normal driver initialization. The helper does not initialize haptics, request
rumble/LED changes, or write emulator settings or mappings.
It does not establish DuckStation's final player assignments or apply mappings.

Build with `nix develop -c cargo build --release -p lunchbox-controller-probe`,
or use the `lunchbox-controller-probe` Nix flake package/app. The binary accepts:

- `--sdl-library`: required absolute path to the target SDL 3.2+ library.
- `--mapping-db`: optional absolute path to the emulator's effective mapping DB.
- `--hint SDL_NAME=value`: repeatable application hints, using SDL normal priority.
- `--match-path`: repeatable exact runtime device paths; absent/ambiguous paths
  cause failure. Names, VID/PID, GUID, and enumeration position are not fallbacks.
- `--bindings-for-path`: repeatable exact gamepad paths to open for resolved
  bindings. All selections must be unique, present, and SDL gamepads before any
  are opened. The returned bindings retain axis ranges/inversion and hat masks;
  these are SDL raw indices, **not Linux evdev/joydev indices**.

The JSON includes the library hash/version, optional mapping-database hash,
requested/effective hints, and device paths, GUIDs, raw SDL mapping strings, and
reported player-index hints. A negative player index is unavailable, not player
zero. An environment override of the selected mapping database is an error;
other overridden hints are reported. A snapshot is not proof of the emulator's
effective configuration: mirror its custom hints and DB selection explicitly.
Keep snapshots private; they can include device names and local paths.
Schema 2 adds optional `resolved` data per selected device; its absence never
means that the device has no mappings. Older schema-1 inventories still decode.
For SDL 3.2.20 on Linux with the classic backend and an actual `/dev/input/jsN`
path, the helper additionally reads the kernel button/axis maps. The optional
`linux_classic` record maps physical evdev codes to SDL raw controls, excluding
hat axes from the axis sequence and retaining SDL's first-seen hat order. Counts
must agree with the opened target SDL gamepad. Other backends/versions do not get
this mapping by assumption. Axis rest/pressed range calibration is still needed
to turn physical axis directions into digital target actions correctly.

The Rust `duckstation::digital_binding` function translates a resolved SDL raw
digital action into a DuckStation binding suffix (without a player ID). It
handles mapped buttons, signed/inverted axes, trigger ranges, cardinal hats,
and available raw events. It rejects conflated outputs and unusable ranges.
It preserves SDL binding order and requires an explicit raw-axis suppression
contract, including the installed DuckStation revision's output-index quirk.
The classic adapter supplies the physical-code lookup for that verified backend;
other physical adapters, analog target controls, final player IDs, and launch
settings remain. No automated DuckStation launch is enabled by this helper.

SDL ABI references:

- [Joystick paths before opening](https://wiki.libsdl.org/SDL3/SDL_GetJoystickPathForID)
- [Gamepad mapping ownership](https://wiki.libsdl.org/SDL3/SDL_GetGamepadMappingForID)
- [Reported player indices](https://wiki.libsdl.org/SDL3/SDL_GetJoystickPlayerIndexForID)
- [Resolved gamepad bindings](https://wiki.libsdl.org/SDL3/SDL_GetGamepadBindings)
- [Binding ABI and axis ranges](https://wiki.libsdl.org/SDL3/SDL_GamepadBinding)
- [SDL 3.2.20 input-event semantics](https://github.com/libsdl-org/SDL/blob/release-3.2.20/src/joystick/SDL_gamepad.c)
- [SDL 3.2.20 Linux classic numbering](https://github.com/libsdl-org/SDL/blob/release-3.2.20/src/joystick/linux/SDL_sysjoystick.c)

Linux runtime verification on 2026-09-05 used DuckStation's installed Flatpak,
its `/app/bin/libSDL3.so.0` (SDL 3.2.20), and its packaged controller database.
The helper distinguished the two generic Xbox pads by their `/dev/input/js4`
and `/dev/input/js5` paths despite identical reported GUIDs/names. Other joystick
devices were present too, demonstrating why enum order is not a player identity.
Windows/macOS compile/unit-test jobs are included in CI; runtime verification on
those systems remains outstanding.
