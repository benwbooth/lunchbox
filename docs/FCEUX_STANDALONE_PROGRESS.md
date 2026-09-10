# FCEUX Qt standalone controller mapping

Current status: native Linux saved-setup dispatch is now connected to mapping
preparation, child startup, prelaunch verification and health checks. This is
partial support: ROM-selected device overrides remain unresolved and no tests
or launches have run. Coverage is now 93/94 RetroArch profiles (98.9%) and 7/249
standalone candidates with partial dispatch (2.8%). Older checkpoints below
describe the implementation sequence, not the current adapter count.

Source pin: FCEUX 2.6.6, `34eb7601c415b81901fd02afbd5cfdc84b5047ac`.
Primary source: `src/drivers/Qt/sdl-joystick.cpp`.

Implemented native profile binding encoding for SDL buttons, axes and cardinal
hats. Axis validation preserves the pinned native asymmetric activation values
(positive >=16363, negative <=-16383). Profile text accepts only single-digit
hat indices despite a wider packed field; the writer rejects unsupported indices.

The standard NES profile writer now emits all eight controls in bank zero,
explicitly clears three alternate banks and turbo buttons, and rejects duplicate
native input ownership. It validates GUIDs, safe profile basenames and every
record against the native 256-byte line reader. Output is a new private profile
without inherited gamepad hotkeys; original profile files are not edited.

Native NES catalog integration now reuses the existing visual NES layout with
FCEUX-specific outputs and transport validation. Supplied SDL2 snapshots can be
translated from saved Linux calibrations into complete native profiles, checking
released buttons/hats and measured axis endpoints against native thresholds.

Native fresh-start GUID allocation is now implemented for all four ports,
including ordered unused-slot matching, game-controller fallback and keyboard
fallback suppression. Selected players must resolve to their calibrated physical
paths. Untouched earlier ports are included because they can consume devices.
The projection consumes supplied slot inventory; capture integration remains open.

Configuration selection now writes private per-player DeviceType, DeviceGUID
and Profile assignments in the native flat format. The native assignment parser
preserves value whitespace/quotes literally and exposes all four effective GUIDs
for allocation checks. Auxiliary cfg.d files load after the main config on Linux;
the launch owner must include those layers before validating effective routing.
This writer does not yet resolve per-game NES port-type overrides.

Saved FCEUX setups now persist exact emulator/content identity, native base
directory, trusted runtime/helper paths and one to four distinct physical-player
assignments. The no-I/O review model projects saved calibration into the shared
NES visual layout and rejects incomplete native mappings. The shared native UI
now loads, reviews and stages these setups with source/destination layout views.
It labels launch integration as pending; staging is bounded and validates the
complete list before modifying saved settings. No devices are opened by review.

Main/auxiliary layer snapshots now capture bounded UTF-8 configuration in native
unsorted cfg.d enumeration order, merge effective settings, and recheck root,
membership/order, canonical paths and bytes. Replacement generation applies
selected profile assignments to every captured layer so auxiliary overrides
cannot undo them. These are replacement bytes only; private mounting is pending.

Remaining: native inventory integration, private configuration
and launch integration.
Private config staging now retains a temporary writable replacement for each
captured layer and provides canonical-target bubblewrap mount arguments. It
rejects aliased layer targets and rechecks source and staged bytes before handing
off mounts. Only config files are overlaid; the base directory and saves are not
redirected. Generated profile mounts and launch dispatch remain to be connected.
Generated profile staging now mirrors the existing input directory into a private
writable mount, preserves untouched profiles, adds unique GUID/profile files,
and rechecks original and generated data before mount handoff. Copying is bounded
and symlink entries are rejected explicitly. This requires an initialized native
input directory. The session still needs to combine config/profile mounts and
native device capture with dispatch; no launch support is claimed yet.
Combined mapping preparation now checks the requested player/path set against
native GUID allocation, preserving untouched ports from the same captured config
snapshot used for staging. It owns both config and profile mounts and revalidates
files and supplied device routing before mount handoff. Device inventory capture,
ROM input handling and launch dispatch remain pending.
The Linux native session now connects the existing cancellable SDL2 helper
capture protocol to saved physical-controller identities, calibrated profile
generation, native 32-slot GUID allocation, and combined private file staging.
It retains kernel topology and SDL routing snapshots for prelaunch rechecks.
This is implementation only: no helper or device capture was executed. Native
command construction, child handoff and ROM input handling remain pending.
Native command preparation now validates saved emulator/content identity and
trusted executable hash, retains runtime/content hashes, combines both private
mount sets, and selects the saved base using child-only FCEUX_CONFIG_DIR. Pinned
Qt config.cpp confirms that variable takes precedence; FCEUX_HOME alone would
incorrectly append .fceux. Child handoff, ROM input handling and dispatcher
integration remain pending; command preparation has not been executed.
Native spawn ownership now includes cancellable bounded child startup, executable
and SDL-library identification, private config/profile mount inode checks, and
selected-controller open-descriptor checks. Failed startup kills/reaps the owned
child. These checks are not gameplay evidence and have not been executed.
ROM input handling and application launch dispatch remain pending.
The configuration writer now selects standard NES pads, disables expansion
input and inherited automatic input presets, selects Four Score when players
three/four are requested, and disables the inherited A+B+Start+Select exit chord.
Source inspection of Qt fceuWrapper.cpp confirms automatic presets load BEFORE
ParseGIInput; generating a preset therefore cannot force ROM input metadata to
standard gamepads. No private preset workaround was added. These settings are
applied to all captured config layers by the existing replacement generator.
Per-game input selection can override user port types and needs to be resolved.
No tests, builds, device probes or emulator launches were run.

Coverage remains 93/94 RetroArch profiles (98.9%) and 6/249 standalone candidates
with partial dispatch (2.4%). FCEUX is not counted as a seventh adapter yet.
