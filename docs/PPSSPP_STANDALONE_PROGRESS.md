# PPSSPP standalone controller mapping

Source pin: `e49c0bd8836a8a8f678565357773386f1174d3f5`, v1.19.3.

Implemented the standalone SDL key protocol and native controls.ini renderer,
plus a PSP visual mapping profile that is explicitly separate from RetroArch.
The shared physical-layout planner composes saved control IDs with resolved SDL
inputs; it does not invent device order or translate raw evdev numbers as SDL
logical button numbers. The writer requires all sixteen PSP gameplay inputs,
preserves independent bipolar analog axes, rejects duplicate input ownership,
and retains existing hotkeys and unrelated INI sections.

Source contracts inspected: `SDL/SDLJoystick.cpp` (logical buttons and axis
events), `Common/Input/InputState.h` and `.cpp` (device/key encoding),
`Common/Input/KeyCodes.h` (numeric constants), and `Core/KeyMap.cpp` (PSP setting
names and controls.ini loading). The SDL backend uses device index plus ten,
not SDL instance ID; the native loader erases absent gameplay defaults once a
ControlMapping section is present. SDL A/B/X/Y map to numbered keys 2/3/4/1.

Global/per-game input file preparation is now implemented in
`controller_ppsspp/configuration.rs`. It retains a private global settings copy,
patches controls.ini and an existing exact-game INI, and preserves unrelated
game settings. Original file presence is tracked as well as bytes, so creating
a game-specific file after preparation invalidates a formerly global selection.
It requires an explicitly resolved source search directory and initialized
global controls, rather than inventing platform defaults. No native source file
is written. `--appendconfig` is deliberately not used: Config::LoadAppendedConfig
does not call KeyMap::LoadFromIni and also saves settings.

Measured physical-to-SDL2 composition is now implemented in `physical.rs`, using
the shared SDL2 mapping evaluator and backend maps. It consumes a target-runtime
snapshot and explicit released state, translates measured button/axis/hat
gestures, rejects ambiguous logical outputs, and checks that analog mappings
cover the full measured interval. It does not execute or fabricate a capture.
Split/multi-route SDL analog outputs remain rejected pending a continuous-route
contract; no button is promoted to a proportional analog input.

The SDL2 helper now accepts `--sdl2-mapping-db` to load the exact frontend VFS
database after SDL initialization, matching PPSSPP's order. Its contents enter
the effective runtime fingerprint and are rechecked after capture. Existing
BizHawk calls without this option retain their prior behavior. The existing
sampled-state output is reused, not replaced with assumed zero state.

`session.rs` composes a fresh snapshot, measured physical resolver and native
configuration owner. It requires the selected device's real sampled state and
generic SDL index and retains the mapping context for subsequent observation
checks. Its returned preparation is not an actual emulator startup confirmation.

The native Linux configuration owner constructs Bubblewrap arguments for a
SYSTEM-only directory overlay. It deliberately does not change XDG_CONFIG_HOME:
the pinned Linux frontend derives the complete memory-stick root from that
variable, so changing it would also redirect save data. SYSTEM auxiliary files
are retained in a bounded private copy; symlinks/special files require explicit
resolution. This is configuration isolation, not a security sandbox. The
launcher checks the actual native search directory before applying it.

Saved setup persistence and an advanced settings editor are now connected.
Records bind one physical controller to an exact emulator/content/game identity
and explicit native SYSTEM, SDL, mapping-database, helper and Bubblewrap paths.
Review uses the existing PSP visual mapping plan and rejects incomplete native
calibration. DuckStation and PPSSPP share editor presentation only; staging and
stored records stay separate. Neither staging nor review opens hardware.

Native preparation now owns a kernel input topology guard spanning SDL2
enumeration, exact runtime-node resolution and physical-state capture. It checks
the saved executable hash, retains helper/SDL/database/Bubblewrap hashes, prepares
the SYSTEM overlay, and provides a fresh pre-spawn observation check. Helper
execution is shared with DuckStation, bounded and cancellable; no helper was run
during implementation. Preparation does not yet prove the child's SDL library,
asset mapping database, configuration search path or device order; child-side
checks below complete the launch dispatch contract.

Native Linux SDL2 launch dispatch is now connected for saved setups. The owner
retains the private SYSTEM directory, confirms its mount inode through the
owned child's /proc root, checks the loaded SDL2 path, and compares the pinned
frontend's complete ordered startup mapping log with the helper inventory.
The native SYSTEM location must match Linux's XDG/HOME-derived search path;
custom command arguments, VR/Qt variants and SDL fallback mappings are excluded.
Startup collection is bounded and cancellable; readers drain without retaining
gameplay output afterwards. Controller topology remains part of session health.
This is source implementation, not a tested/native compatibility claim.

Next: continue ordinary standalone adapters. Runtime verification remains
deferred by the user. Existing keyboard hotkeys are preserved, not newly mapped.

Coverage: 93/94 RetroArch profile entries (98.9%); 3/249 standalone candidates
have partial dispatch (1.2%). Overall all-mode completion remains unestablished.
No tests, builds, device probes or emulator launches were performed.
