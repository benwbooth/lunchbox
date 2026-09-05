# Target-runtime SDL controller probe

This small Rust helper dynamically loads an explicitly supplied, trusted SDL3
library and emits a JSON inventory. Run it as its own process on the main thread,
inside the emulator's runtime when that runtime differs from the host (Flatpak,
for example). Loading a native library executes code; this is not a scanner for
untrusted downloads.

It uses SDL3's enumeration/mapping APIs without opening gamepads or joysticks,
does not initialize haptics, and does not write emulator settings or mappings.
It does not establish DuckStation's final player assignments or apply mappings.

Build with `nix develop -c cargo build --release -p lunchbox-controller-probe`,
or use the `lunchbox-controller-probe` Nix flake package/app. The binary accepts:

- `--sdl-library`: required absolute path to the target SDL 3.2+ library.
- `--mapping-db`: optional absolute path to the emulator's effective mapping DB.
- `--hint SDL_NAME=value`: repeatable application hints, using SDL normal priority.
- `--match-path`: repeatable exact runtime device paths; absent/ambiguous paths
  cause failure. Names, VID/PID, GUID, and enumeration position are not fallbacks.

The JSON includes the library hash/version, optional mapping-database hash,
requested/effective hints, and device paths, GUIDs, raw SDL mapping strings, and
reported player-index hints. A negative player index is unavailable, not player
zero. An environment override of the selected mapping database is an error;
other overridden hints are reported. A snapshot is not proof of the emulator's
effective configuration: mirror its custom hints and DB selection explicitly.
Keep snapshots private; they can include device names and local paths.

SDL ABI references:

- [Joystick paths before opening](https://wiki.libsdl.org/SDL3/SDL_GetJoystickPathForID)
- [Gamepad mapping ownership](https://wiki.libsdl.org/SDL3/SDL_GetGamepadMappingForID)
- [Reported player indices](https://wiki.libsdl.org/SDL3/SDL_GetJoystickPlayerIndexForID)

Linux runtime verification on 2026-09-05 used DuckStation's installed Flatpak,
its `/app/bin/libSDL3.so.0` (SDL 3.2.20), and its packaged controller database.
The helper distinguished the two generic Xbox pads by their `/dev/input/js4`
and `/dev/input/js5` paths despite identical reported GUIDs/names. Other joystick
devices were present too, demonstrating why enum order is not a player identity.
Windows/macOS compile/unit-test jobs are included in CI; runtime verification on
those systems remains outstanding.
