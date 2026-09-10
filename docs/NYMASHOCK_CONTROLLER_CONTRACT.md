# Nymashock adapter boundary

The current `build/lunchbox.db` relationship identifies `nymashock` under
**BizHawk**, for `Sony Playstation`, emulator ID
`8f31c0f9-c6aa-515d-9be1-f3d4e791db83`.

This was inspected with a read-only join of `emulator_platforms`, `emulators`
and `platforms`; a nonempty `core_name` does not establish a libretro interface.
The controller coverage report now preserves that owner and labels the row as a
BizHawk core. It offers no RetroArch mode selector for this relationship.

Nymashock remains part of the original uncovered-core denominator. It is not
deleted, aliased to Beetle PSX, or counted as covered by another PSX core.
Implementation requires tracing BizHawk's controller definition, per-player
bindings, sync settings and launch/config persistence before adding an adapter.
No runtime checks or tests were performed in this implementation step.

## Native binding contract (source inspection)

Source revision: BizHawk `8c6b8958bbbe623eaaa36bc82af858b812893628`.
This section records adapter requirements, not implemented launch coverage.

- `src/BizHawk.Bizware.Input/SDL2/SDL2Gamepad.cs` assigns recognized
  GameControllers the prefix `X{SDL device index + 1} ` and raw joysticks
  `J{SDL device index + 1} `. Both use the same SDL joystick enumeration;
  numbering is not a separate ordinal among recognized controllers.
- `RefreshIndexes` runs after device addition and removal. Persisting `X1`
  as a hardware identity is therefore unsafe. Resolve the selected device in
  the launch environment, and account for index changes during hotplug before
  claiming stable routing. A Linux event-device index is not this SDL index.
- `SDL2InputAdapter.ProcessHostGamepads` emits analog names with the literal
  suffix ` Axis`, for example `X1 LeftThumbX Axis`. Digital names omit it,
  for example `X1 LeftThumb`. Recognized stick axes are `LeftThumbX`,
  `LeftThumbY`, `RightThumbX`, and `RightThumbY`.
- Axis samples are converted from SDL signed 16-bit values to approximately
  +/-10000. SDL Y remains negative upward in this adapter; the analog binding
  multiplier and core axis reversal must be composed, not independently guessed.
- `src/BizHawk.Client.Common/config/AnalogBind.cs` defines `Value`, `Mult`,
  `Deadzone`, `ButtonBindPositive`, and `ButtonBindNegative`. A string-only
  button binding cannot substitute for this analog object.
- `ConfigExtensions.cs` reads `CoreSyncSettings` using the core type name and
  deserializes the stored token directly into the requested settings type.
  Its serializer extracts the `o` token from an internal wrapper; the saved
  core settings value must not be wrapped in another `o` object. Preserve
  existing type metadata and unrelated settings when adapting a private config.

## Implementation state

`controller_bizhawk.rs` now contains native digital-pad, DualShock, and Dual Analog binding
builders, raw SDL joystick naming, explicit multitap topology, and private config
preparation. It selects Nymashock, disables core fallback, replaces the owned
controller deck, and disconnects unselected ports. Multitap settings are explicitly
set for the launch; memory-card and other unrelated settings remain untouched.
The source config's contents and symlink destination are rechecked during preparation.

The separate SDL2 probe inventories exact paths and mappings and can open selected
devices for counts and input samples. Classic and evdev retain separate numbering
and axis conversions. The shared mapping evaluator translates paired samples into
logical GameController changes. The UI records explicitly selected gestures with
the runtime hash, mapping and count context, and provides native runtime/player
configuration. These records now feed digital-pad, DualShock, and Dual Analog launch bindings.

Dual Analog (`dualanalog`, SCPH-1180) shares the sixteen digital controls and
four stick axes of DualShock, but has neither an Analog toggle nor rumble in
the pinned Mednafen `src/psx/input/dualanalog.cpp` definition. Saved settings,
recognized SDL calibration, direct raw classic/evdev translation, and normalized
session input preserve that distinction. The separate SCPH-1110 Analog Joystick
uses different native control names and is not aliased to this mode. These paths
are implemented but have not been tested or runtime-validated.

The SCPH-1110 `analogjoy` encoder now has its own fourteen-button/four-axis
definition, including the native thumbstick, trigger, pinky, thumb-button, and
Up / Down axis names. `NymaCore.Controller.cs` uses `SettingsNameNeg/Pos`
(`up`/`down`) for absolute-axis suffixes, not the Fore/Back display labels;
`NymaCore.cpp` exports those fields from `sname_dir`. Its fixed SDL route checks that all required controls
exist without requiring nonexistent stick-click inputs. The separate
`playstation-analog-joystick` calibration layout records all fourteen buttons and
eight directional samples for the four axes, with actual joystick labels and
PlayStation wire identities for cross-layout assignment. Saved mode selection,
recognized SDL calibration, normalized-session launch input, and direct raw
classic/evdev translation are connected. Raw translation uses the distinct
fourteen-button names and four measured axes, without inventing stick-clicks,
toggle inputs, or force feedback. No validation claim is made for these routes.

The normal launch entry point now retains private config and optional normalized
virtual gamepads, with topology and bridge-health checks. Normalized sticks use
independent measured halves and recenter at the recorded neutral. This transport
does not forward force feedback. Unknown/ambiguous SDL mappings, unsupported HID
identity paths, and other controller peripherals remain unfinished. No tests or
runtime validation have been run; Nymashock is not yet counted as covered.

The digital rhythm-device encoder now distinguishes `dancepad` (ten buttons)
from `popnmusic` (Select/Start plus nine colored buttons). Contracts come from
the pinned Mednafen `src/psx/input/gamepad.cpp`. The encoder requires the exact
native button set and explicitly translated host sources; it does not introduce
opposite-direction suppression for dance-pad buttons. A calibrated SDL dispatch
variant is present. Dedicated calibration layouts now identify the eight dance
floor positions and nine Pop'n colored buttons, retaining PSX wire identities
for native-name translation. Layout resolution, saved mode selection, and
calibrated SDL and direct raw classic/evdev launch routing are now connected.
The mode selector makes device choices mutually exclusive. Normalized-session
input now uses the existing physical-code-preserving bridge and checks distinct
physical assignments before creating a virtual device. It does not convert the
floor layout into an exclusive directional axis. Direct and normalized routes
reject shared physical buttons/axes and conflicting hat positions
that cannot deliver simultaneous rhythm inputs; the logical route also rejects
ambiguous mappings. These paths have not been tested or runtime-validated.

### neGcon source requirements and encoder (launch not yet connected)

The pinned `negcon.cpp` defines eight digital controls: Start, four D-Pad
directions, R (the shoulder-name override), A, and B. Absolute axes are
`Twist Ccwise / Cwise`, `I`, `II`, and `L`. Nyma's default absolute-axis hook
uses 0..65535 with neutral 32768 for the twist; ButtonAnalog uses 0..65535
with neutral 0 for each pressure control. Nymashock's special 8-bit stick
override does not match these names. Pressure-input scaling must respect that
zero-neutral distinction before a neGcon route can be claimed.

The native encoder now requires that exact eight-button/four-axis set and
rejects digital substitutes for pressure inputs. BizHawk `Controller.cs` scales
host axis values by 10000, applies deadzone and multiplier, then scales around
the core axis neutral. A released host pressure value must therefore resolve to
zero; a multiplier alone cannot offset an arbitrary nonzero rest position.
SDL trigger axes supply the needed zero-based range when appropriately mapped;
there are only two standard trigger outputs, so the third pressure input needs
its own calibrated axis. A dedicated layout now captures both twist directions
and all three pressure measurements. The logical pressure translator uses
measured continuous-axis travel, rejects button-to-axis substitutions and rest
positions outside the deadzone, and scales the measured press to the endpoint.
Saved mode selection and calibrated SDL launch integration are connected.
Normalized input selects the neGcon layout and retains pressure measurements;
its logical capture permits both trigger and stick outputs for pressure. Twist
and the three pressure controls must use distinct physical axes. Direct raw
classic/evdev pressure translation is now connected, including when a normalized
virtual device is exposed by SDL only as a raw joystick. It converts measured
physical endpoints through the actual SDL backend, requires released pressure
inside the zero deadzone, and scales the full press to the target endpoint.
Off-center raw pressure still requires normalized input. This encoder and
translator have not been tested or runtime-validated.

### Pointer-device encoder boundary

The native encoder now distinguishes the mouse's two relative Motion axes and
two buttons from GunCon and Justifier absolute X/Y aim and four buttons each.
Both guns retain an explicit Offscreen Shot input. Justifier is rejected on
multitap B-D slots, matching the pinned core's stated limitation.

`MainForm.cs` transforms `WMouse X/Y` through the current display geometry and
video buffer size to signed frontend coordinates; these source names do not
carry an ` Axis` suffix. The encoder permits them for absolute aim, but rejects
them as relative mouse motion. Mouse relative axes use Nyma's -127..127 range,
not screen position. SDL axis sources can express explicitly configured analog
aim or relative velocity. Dedicated controller-driven layouts now identify
mouse buttons/motion and each gun's aim, trigger, auxiliary and off-screen
controls. Saved mode selection and controller-driven launch routing are connected
for recognized SDL, direct raw classic/evdev and normalized input. X/Y require
distinct physical axes, and Justifier retains the multitap-slot restriction.
Controller-driven relative mouse motion has a separate saved gain, defaulting
to 100 percent, applied after calibration/deadzone on both logical and raw
routes. The core's -127..127 per-frame range still clamps the final result;
this gain neither changes lightgun aim nor turns cursor position into deltas.
An explicit desktop-cursor option now binds gun aim to BizHawk's transformed
`WMouse X/Y`, with zero deadzone and unit scale, while leaving buttons on the
selected controller. It skips controller-axis calibration for aim, supports raw
and recognized button routes, and permits only one desktop-cursor player.
Physical mouse-delta capture and device-specific lightgun calibration remain
unfinished; controller motion or one desktop cursor is not presented as their replacement. None of these paths
has been tested or runtime-validated.

## Runtime library boundary

The pinned `ExternalProjects/SDL2/CMakeLists.txt` emits a no-SONAME `libSDL2.so`
under `dll/`. `Assets/EmuHawkMono.sh` changes directory to the installation and
replaces `LD_LIBRARY_PATH` with `dll`, the installation, and a distro library
directory. An isolated probe launched directly from Lunchbox does not execute
those wrapper instructions. Preparation now rejects a selected SDL library that
differs from an existing bundled `dll/libSDL2.so`, and rechecks that choice after
probing. This is not complete proof of equivalent wrapper environments: custom
and Nix launchers, dependent libraries, loader overrides and changed SDL hints
still need a full runtime invocation contract before a coverage promotion.

An opt-in direct-Mono invocation now avoids executing the external wrapper. It
requires an executable ELF Mono binary and supplies the same explicit library
search environment to probe and emulator. Separate unmanaged-library and existing
data directories support split installations; an explicit source config enters
the same private-copy path and cannot conflict with a launch `--config` option.
Preparation changes a copy of the launch plan and commits it only after success.
This does not install assets, copy a wrapper's private data, reproduce theme setup,
or establish that an arbitrary configured Mono/assembly combination matches the
pinned native input contract. Runtime validation remains deferred.
# Explicit managed assembly paths

Direct Mono runtime settings now accept optional `managed_paths` alongside
native `library_paths`. They extend the existing installation/dll MONO_PATH in
declared order; native and managed search paths are kept separate. Missing fields
retain the earlier behavior. Additional paths require direct Mono mode, bounded
absolute paths without search separators, and the shared search-directory count
limit. The runtime editor exposes the setting without scanning or installing
packages. This provides configuration for split managed assemblies, not proof
that a particular package/runtime works or that transitive libraries are pinned.
