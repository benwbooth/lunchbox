# Dolphin native GameCube mapping

## Current status

Partial native Linux dispatch is connected for explicitly saved Dolphin 2606
GameCube ISO/GCM evdev setups. The launcher retains private configuration,
checks the executable version/hash and disc identity, confirms child directory
mounts and open input nodes, handles startup cancellation/failure, and checks
kernel topology during gameplay. No tests, builds or runtime probes were run.

The system-data directory is a user-declared input paired with the trusted
executable, not independently confirmed from the running child. Compressed
media, Wii, other input backends/host variants and runtime compatibility remain
incomplete. Settings trees must currently exist and contain no symlink entries.
Coverage is 93/94 RetroArch profiles (98.9%) and 5/249 standalone candidates with
partial dispatch (2.0%). This supersedes the incremental historical notes below.

## Implementation history

Source pin: Dolphin 2606, `6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4`.
This is separate from the existing libretro Dolphin contract and its mixed
trigger fallback/shortcut channels.

Implemented `controller_dolphin/standalone.rs`: a one-to-four-pad native
GCPadNew.ini writer and Dolphin.ini serial-port selection. The catalog now has
a separate 22-control native GameCube visual layout/profile. Calibrated physical
control IDs compose with explicitly resolved Dolphin backend input names and
ranges; the writer does not reinterpret evdev/SDL ordinals or accept arbitrary
controller expressions in place of native input resolution.

Native input names are quoted as individual controls. Each selected port has an
exact backend/id/name qualifier, all gameplay inputs and explicit per-input
ranges. Unrelated settings and other pad sections remain unchanged. Old main
and C-stick modifier expressions are cleared to avoid altering the newly
calibrated stick. Microphone, Triforce, rumble and Wii controls are not generated
by this standard GameCube writer.

Analog L/R travel and digital L/R clicks are distinct settings. A measured
continuous trigger can feed both native MixedTriggers inputs with equal range;
other duplicate input ownership is rejected. Binary controls cannot substitute
for proportional analog targets. Native trigger threshold/deadzone settings
remain preserved; runtime behavior has not been tested.

Primary sources inspected:

- [GCPadEmu.cpp](https://github.com/dolphin-emu/dolphin/blob/6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4/Source/Core/Core/HW/GCPadEmu.cpp) and its header: native group/control names and pad sections.
- [ControlGroup.cpp](https://github.com/dolphin-emu/dolphin/blob/6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4/Source/Core/InputCommon/ControllerEmu/ControlGroup/ControlGroup.cpp): expression/range INI keys.
- [MixedTriggers.cpp](https://github.com/dolphin-emu/dolphin/blob/6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4/Source/Core/InputCommon/ControllerEmu/ControlGroup/MixedTriggers.cpp): separate thresholded click and continuous travel.
- [SI_Device.h](https://github.com/dolphin-emu/dolphin/blob/6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4/Source/Core/Core/HW/SI/SI_Device.h): standard GC controller serial device value 6.

The evdev translator is now implemented in `standalone/evdev.rs`. It counts
all supported EV_KEY codes for native Button N aliases, enumerates ordinary
EV_ABS codes below ABS_MISC (including hats), and reproduces Dolphin's centered
half-axis and Full Axis normalization. Measured peak travel determines range.
Both sticks require opposite pairs on four independent physical axes. A
directional hat cannot supply a proportional target. Arbitrary off-center rest
needs an explicit offset expression and is rejected, rather than silently
losing part of its travel.

The explicit `capture_node` function reads evdev properties, capabilities,
pressed keys and current axis metadata/values through read-only ioctls, and
checks node identity afterwards. It consumes no events and was not invoked.
Merged native device membership and its evdev/id/name qualifier still need
frontend inventory resolution. Motion/pointing nodes have separate namespaces;
multiple ordinary nodes are rejected when their indexed aliases could collide.

Additional sources: native [evdev.cpp](https://github.com/dolphin-emu/dolphin/blob/6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4/Source/Core/InputCommon/ControllerInterface/evdev/evdev.cpp)
and [CoreDevice.cpp](https://github.com/dolphin-emu/dolphin/blob/6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4/Source/Core/InputCommon/ControllerInterface/CoreDevice.cpp).

Saved setup persistence and visual review are now connected. Records bind an
exact disc ID/revision and content/emulator identity to native user/system paths,
a trusted runtime, and one to four physical controllers with distinct ports and
evdev qualifiers. Review checks complete native calibration, including analog
endpoint measurements. It opens no devices. The shared native dialog now uses
one adapter selector so records cannot be staged under competing boolean modes.

Per-game profile writers are also implemented: selected [Profile] settings can
be remapped while preserving unrelated profile settings, and selected
[Controls] PadProfileN keys can point to distinct session-owned basenames.
Native InputConfig loads these profiles instead of the corresponding GCPadN
section; merely patching GCPadNew.ini would not handle them. Basenames exclude
commas, paths and extensions because native InputProfile appends .ini and treats
directories/multiple choices specially. The launcher must stage the files and
resolve/apply the final effective game layer; this step does not claim that
runtime precedence has already been connected.

The game-profile writer now pins both legacy [Controls] and modern
[GCPad.Controls] PadProfileN selections to the same private profile. Selected
ports also receive standard GC device value 6 in [Controls] PadTypeN and both
[Core]/[Main.Core] SIDeviceN aliases. This prevents conflicting existing aliases
from selecting another device based on section order. Source basis is the
pinned 2606 Core/ConfigLoaders/GameConfigLoader.cpp mapping and its independent
LoadControllerConfig profile loader. This is writer implementation only, not
launch integration or runtime verification.

Configuration preparation now captures the native source files and resolves
selected players' effective global, legacy profile, embedded modern game-layer,
and modern profile settings in system-then-user priority. It produces private
replacement bytes for Dolphin.ini, GCPadNew.ini, unique per-player profile files,
and the highest-priority user revision INI. Unrelated settings are retained;
the original files are not written. The snapshot can be rechecked before launch.
Private mount materialization now copies the Config and GameSettings trees into
a retained temporary directory, applies replacements there, and prepares Linux
bubblewrap directory mounts over the original locations. Save directories and
the native user root are not relocated. Preparation rechecks source bytes,
directory membership and canonical roots. Copies are bounded; symlink entries,
overlapping roots and missing Config/GameSettings directories are rejected.
These helpers have not been executed or connected to launch dispatch yet.

Native inventory qualification now reproduces Dolphin's ordered evdev device
registration: interesting-device filtering, nonempty unique-ID plus physical-path
node merging, alphabetically first merged name, and lowest available same-name
device index after re-registration. This handles duplicate names without using
them as physical identities. It consumes supplied observations; the complete
native-order enumeration capture and launch integration remain to be connected.
Source basis: the pinned evdev.cpp and ControllerInterface.cpp AddDevice.

Linux inventory capture is now implemented using libudev's native enumeration
order, effective read/write permission checks, read-only evdev identity queries,
and the existing capability/state capture. Device identities and capabilities
are checked against node replacement. It performs no force-feedback writes.
The capture function has not been invoked. Launch dispatch, launch-time inventory
rechecks and child startup confirmation remain to be connected.

A retained native session now connects saved calibrations, physical kernel-node
identity, evdev inventory qualification, calibrated pad resolution and private
configuration materialization. Preparation requires each saved qualifier to
match its calibrated physical node. Before-spawn verification rechecks ordered
native routing, merged-node membership and capabilities while allowing normal
button/axis state changes; a separate health check observes kernel topology.
The session constructor is implemented but not yet called by launch dispatch.

Native command preparation now binds the selected executable to its saved hash,
checks the raw ISO/GCM disc's actual ID/revision, resolves the calibrated session,
and builds a bubblewrap command using explicit --user/--exec/--batch arguments.
Movie, state-load, NAND and arbitrary configuration arguments are rejected rather
than silently discarded. Runtime files and content are retained for rechecks.
The command is not yet dispatched: startup must confirm the pinned runtime and
its compiled system-data path before this adapter is counted as launch support.

Startup support now includes child mount-identity checks for both private settings
directories and descriptor checks for the evdev nodes used in native routing.
Launch preparation also queries --version and requires the pinned 2606 release
in addition to the saved executable hash. These checks are code only and have
not been run. The startup coordinator and dispatch are still pending; system
data-path confirmation must account for native initialization/logging order.

Next: native launch dispatch and startup integration. Native
Dolphin is not counted as a fifth dispatch adapter yet.

Coverage: 93/94 RetroArch profiles (98.9%, with Steem SSE a platform exception);
4/249 standalone candidates have partial dispatch (1.6%). Overall all-mode
completion remains unestablished. No tests, builds, device captures or emulator
launches were run.
