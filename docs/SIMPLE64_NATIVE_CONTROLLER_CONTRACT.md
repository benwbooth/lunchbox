# simple64 native Linux controller contract

This source-level adapter targets the archived `simple64/simple64` tree at
`d8c969c7b932e3d76e6a25549d76348839dbaefd` (the pinned record abbreviates this
as `d8c969c7b`). It is intentionally marked runtime-unverified.

## Upstream format

`simple64-input-qt/main.cpp` opens `input-profiles.ini` and
`input-settings.ini` below the Mupen64Plus user config directory returned by
`ConfigGetUserConfigPath()`. A profile section uses these exact keys:

`A`, `B`, `Z`, `Start`, `L`, `R`, `DPadL`, `DPadR`, `DPadU`, `DPadD`,
`CLeft`, `CRight`, `CUp`, `CDown`, `AxisLeft`, `AxisRight`, `AxisUp`, and
`AxisDown`.

Values are comma-separated SDL2 values. Type `1` is an SDL
`GameControllerButton` (`button,type`); type `2` is an SDL
`GameControllerAxis` (`axis,type,sign`). The plugin polls these with
`SDL_GameControllerGetButton` and `SDL_GameControllerGetAxis`. The adapter
therefore resolves calibrated Linux controls through the exact probed SDL2
mapping and refuses raw-joystick or keyboard fallbacks.

`input-settings.ini` has `[Controller1]` through `[Controller4]`. Each uses
`Profile`, `Gamepad`, and `Pak`. The adapter gives each selected pad a unique
`Lunchbox-Player-N` profile and an exact `device_index:device_name` Gamepad
selection. Unselected ports are explicitly `Gamepad=None`, preventing
simple64's `Auto` scan from stealing a device. `Pak=Memory` is retained for
selected ports; pak selection itself is outside this mapping contract.

## Isolation and roots

simple64-gui reads `simple64-gui.ini` beside its executable. Since upstream has
no command-line config-directory switch, the launch session stages the
executable directory with symlinked runtime companions, copies the executable,
and writes only a private `simple64-gui.ini` whose `configDirPath` points to a
private core config directory. The declared baseline `mupen64plus.cfg`,
`input-profiles.ini`, and `input-settings.ini` are copied and patched there;
unknown profile/settings sections remain intact.

The GUI's `configDirPath` override changes configuration/input lookup only.
The adapter deliberately does not set `XDG_DATA_HOME`, and the core's native
save/state paths consequently remain `$XDG_DATA_HOME/mupen64plus/save` (or the
normal platform default), including `.eep`, `.sra`, `.fla`, `.mpk`, and `.st0`
through `.st9` naming. The user's GUI INI and config directory are never
written.

Every launch input is hash-checked: the selected executable, staged executable,
probe helper, SDL library, ROM, private GUI INI, and generated input INIs. The
SDL2 probe must report the declared library and SHA-256, version 2, exact
device paths, and an unchanged ordered inventory before launch. A Linux input
topology guard also rejects unplug/replug or device-number changes.

## Covered and deferred

The contract covers one through four physical SDL2 GameController-backed N64
pads, including A/B/Z, Start/L/R, D-pad, C-button axes and the proportional
control stick. It does not author keyboard profiles, raw joystick profiles,
voice/VRU, Transfer Pak contents, 64DD IPL media, or pak policy beyond the
preserved `Memory` default. No runtime launch or gameplay probe has been
completed yet.

## Parent integration

The parent integration must add `mod controller_simple64_native` and route the
simple64 native setup through settings persistence, the guided setup/runtime
surface, native target routes, controller coverage, and the launch dispatch
(`native_command::prepare`, `spawn`, health and cancellation). It must add the
`simple64:standalone-n64` catalog profile with transport
`simple64-native-settings`, target layout `n64`, four players, and the exact
source URL for the pinned tree. The platform record is already
`emulator_details/records/simple64.json`; this adapter does not alter shared
registration or launch/settings files.
