# mGBA native SDL controller mapping

Source pin: `26b7884bc25a5933960f3cdcd98bac1ae14d42e2` (0.10.5).

Primary contracts inspected without building or executing:

- [SDL input events](https://github.com/mgba-emu/mgba/blob/26b7884bc25a5933960f3cdcd98bac1ae14d42e2/src/platform/sdl/sdl-events.c): raw joystick events, GUID preference and profile precedence.
- [Input configuration](https://github.com/mgba-emu/mgba/blob/26b7884bc25a5933960f3cdcd98bac1ae14d42e2/src/core/input.c): SDLB sections, key/axis/hat serialization and incremental loading.
- [GBA key identity](https://github.com/mgba-emu/mgba/blob/26b7884bc25a5933960f3cdcd98bac1ae14d42e2/src/gba/input.c) and [SDL main](https://github.com/mgba-emu/mgba/blob/26b7884bc25a5933960f3cdcd98bac1ae14d42e2/src/platform/sdl/main.c): GBAInputInfo key ordering, including GB/GBC frontend use.

Implemented `controller_mgba.rs` plus native GBA and GB/GBC catalog profiles.
The writer generates native SDLB keys, signed-axis thresholds and integer hat
targets in both general and selected GUID-profile sections. Other profiles,
keyboard settings, hotkeys and non-input configuration are preserved. It sets
the native device preference only after the caller supplies a resolved GUID.

The physical translator consumes saved calibration, a fresh SDL2 backend/count
snapshot and sampled released state. It does not reinterpret evdev numbers as
SDL ordinals. Thresholds lie between measured released/pressed SDL values;
opposite thresholds must not overlap. Duplicate target/input ownership and
ambiguous same-GUID controllers are rejected. It performs no device capture.

Native loading is incremental: absent axis keys do not erase earlier/default
axis bindings. Unused axis targets are moved to a disabled high direction at
32767 on the first index beyond captured axes; live SDL axis events cannot
activate it. Hat bindings are explicitly cleared as a contiguous range so
loading cannot stop early and leave a later saved mapping active. Runtime
behavior of this configuration remains untested under the user's instruction.

Saved setup/settings integration is now implemented. Each record selects an
exact emulator/content identity, handheld layout, calibrated controller and
trusted runtime paths. Review and staging are source-only, with no device I/O.
The shared native setup dialog now renders source/destination diagrams and
mapping lines for mGBA, PPSSPP and DuckStation using ControllerMappingView.

The configuration owner retains a bounded private config.ini and checks the
original path/content before handoff. Linux Bubblewrap arguments overlay only
that file, preserving native save/state locations and other configuration files.
The pinned CLI's `-c` is a cheats file and `-C` sets scalar options; neither is
used as an invented controller-config path. Portable/native config resolution
is checked by the native launcher, including marker readability and path changes.

Native Linux SDL dispatch is now connected. It validates the saved executable
hash and 0.10.5 SDL CLI contract, captures fresh SDL inventory/backend state with
kernel topology ownership, resolves the selected physical runtime node, and
rechecks inventory before spawning. The child must load the inspected SDL2
library, see the private config.ini inode, and open the selected input node.
These checks plus the pinned unique-GUID selection algorithm establish the
configured handoff; they are not a gameplay/button-response oracle. Topology
changes remain session health failures. Preparation and startup have bounded,
cancellable waits; nothing was executed during implementation.

Next: continue ordinary standalone adapters; runtime tests remain deferred.
This contract is for the SDL
frontend; Qt behavior is not inferred from it. Sensors and rumble are separate
from these ordinary handheld gamepad profiles.

Coverage: 93/94 RetroArch profiles (98.9%, with Steem SSE a platform
exception), and 4/249 standalone candidates with partial dispatch (1.6%).
Overall all-mode completion is not
established. No tests, builds, device probes or emulator launches were run.
