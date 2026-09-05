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
It does not apply emulator mappings. An optional player probe projects a verified
DuckStation revision's assignment rules; an actual-emulator oracle checks a
single startup against that projection.

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
- `--runtime-library`: absolute trusted dependency path to load before SDL,
  repeatable in dependency order. For a Nix-built helper inside DuckStation's
  x86-64 Flatpak, load `/usr/lib/x86_64-linux-gnu/libcap.so.2` followed by
  `/usr/lib/x86_64-linux-gnu/libudev.so.1`. Paths are runtime-specific; do not
  substitute host libraries. Each dependency is hashed and recorded.
- `--duckstation-player-probe 0a53bc47c`: explicitly opens **all** enumerated
  devices in SDL event order (including non-gamepads), retaining handles through
  binding queries. Requires SDL 3.2.20, Linux classic mode, and explicit target
  libudev. It rejects failed opens, duplicate/new/remapped/removed devices, and
  incomplete or changing topology. It never initializes haptics or requests rumble.

The JSON includes the library hash/version, optional mapping-database hash,
requested/effective hints, and device paths, GUIDs, raw SDL mapping strings, and
reported player-index hints. A negative player index is unavailable, not player
zero. An environment override of the selected mapping database is an error;
other overridden hints are reported. A snapshot is not proof of the emulator's
effective configuration: mirror its custom hints and DB selection explicitly.
Keep snapshots private; they can include device names and local paths.
Schema 2 adds optional `resolved` data per selected device; its absence never
means that the device has no mappings. Older schema-1 inventories still decode.
Schema 3 adds `runtime_libraries` and optional `player_probe`, with opened-device
names and player-index hints plus projected DuckStation slots. It is a snapshot,
not a guarantee that a later process sees the same topology.
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

## Actual emulator startup oracle

After capturing a fresh snapshot with all effective hints, target dependencies,
and the player probe, run:

```console
nix develop -c cargo run -p lunchbox-controller-probe --example duckstation_startup -- /absolute/path/to/snapshot.json
```

This Linux-only Rust oracle verifies the installed Flatpak revision and SDL/
dependency hashes, then starts the real emulator offscreen with a fresh private
configuration. It checks the logged configuration path and **every** device-open
record, including order, instance ID, opened name, and assigned player ID. It
stops only its own child and confirms the original settings file is unchanged.
No ROM/BIOS is launched or user input settings copied. Evidence is retained in
a unique `/tmp/lunchbox-duckstation-startup-*` directory on success or failure.

Flatpak reserves XDG paths at sandbox entry: the oracle sets its private XDG
directories **inside** the sandbox via `env` executing the binary directly,
without a shell. Passing `--env=XDG_CONFIG_HOME=...` to Flatpak did not isolate
this installed application; the oracle caught that and now rejects wrong-root
startup immediately. This proves fresh-root routing, not preservation of user
saves, BIOS paths, game overrides or input profiles in a production overlay.

Verified on 2026-09-05: the target libudev removes non-gamepad LED/mouse/pointer
interfaces, giving three devices on this host, with selected `/dev/input/js4`
and `/dev/input/js5` assigned DuckStation slots 1 and 2. Earlier six-device
snapshots lacked the runtime libudev and **must not be used for player mapping**.
Loading the same SDL library alone was not runtime parity: the Nix helper's
loader could not discover Flatpak's libudev without the explicit dependencies.
The actual-emulator check exposed this mismatch; it now passes with matching
dependencies. Repeat the check after runtime/backend/topology changes.

SDL ABI references:

- [Joystick paths before opening](https://wiki.libsdl.org/SDL3/SDL_GetJoystickPathForID)
- [Gamepad mapping ownership](https://wiki.libsdl.org/SDL3/SDL_GetGamepadMappingForID)
- [Reported player indices](https://wiki.libsdl.org/SDL3/SDL_GetJoystickPlayerIndexForID)
- [Resolved gamepad bindings](https://wiki.libsdl.org/SDL3/SDL_GetGamepadBindings)
- [Binding ABI and axis ranges](https://wiki.libsdl.org/SDL3/SDL_GamepadBinding)
- [SDL 3.2.20 input-event semantics](https://github.com/libsdl-org/SDL/blob/release-3.2.20/src/joystick/SDL_gamepad.c)
- [SDL 3.2.20 Linux classic numbering](https://github.com/libsdl-org/SDL/blob/release-3.2.20/src/joystick/linux/SDL_sysjoystick.c)
- [SDL event queue](https://wiki.libsdl.org/SDL3/SDL_PollEvent)
- [SDL3 event ABI](https://github.com/libsdl-org/SDL/blob/release-3.2.20/include/SDL3/SDL_events.h)

Linux runtime verification on 2026-09-05 used DuckStation's installed Flatpak,
its `/app/bin/libSDL3.so.0` (SDL 3.2.20), and its packaged controller database.
The helper distinguished the two generic Xbox pads by their `/dev/input/js4`
and `/dev/input/js5` paths despite identical reported GUIDs/names. Other joystick
devices were present too, demonstrating why enum order is not a player identity.
Windows/macOS compile/unit-test jobs are included in CI; runtime verification on
those systems remains outstanding.
