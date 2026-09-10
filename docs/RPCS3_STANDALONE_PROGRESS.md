# Standalone RPCS3 controller mapping

## Shared native launch integration

Connected saved Linux RPCS3 setups to shared calibrated launch preparation,
retained session ownership, spawning, prelaunch checks and topology health checks.
Startup requires the generated profile log record, matching gamepad naming,
the expected loaded SDL library and opened physical controller descriptors.
Failure/cancellation terminates the child started by this session. Coverage/UI
now label RPCS3 partial, not missing. File boot targets are supported by this
implementation; directory targets, mapping database parity, nonstandard controls,
other backends and runtime correctness remain incomplete or unverified.
No tests/builds/probes/launches ran. Formatting and diff whitespace checks only.
Coverage: 93/94 RetroArch profiles (98.9%) and 13/249 partial standalone
integrations (5.2%). This is not tested compatibility or all-mode completion.

## Child-owned log and profile confirmation

Added Linux reading of the RPCS3.log descriptor actually opened by the verified
child, avoiding guessed cache paths and old standalone log files. Reads are
bounded and decode complete lines only. Generated-profile selection checks use
the unique temporary path and reject a later switch away from it. Native source
shows ordinary SDL notices are file logs, not reliable stdout output; pipe-only
capture would therefore be insufficient. Native session/dispatch remains pending.
No tests/builds/probes/launches. Coverage remains 93/94 RetroArch profiles (98.9%)
and 12/249 partial standalone integrations (4.8%).

## Native enumeration confirmation

Added bounded complete-line parsing for the pinned handler's opened-gamepad
records and comparison against prepared physical-path/native-name assignments.
Native instance IDs are not compared with helper-process IDs. Duplicate devices,
unexpected devices and naming differences fail explicitly; incomplete startup
enumeration remains pending. Wiring a fresh child-owned log and native session
into shared dispatch remains unfinished. No tests/builds/probes/launches.
Coverage stays 93/94 RetroArch profiles (98.9%) and 12/249 partial standalone
integrations (4.8%). RPCS3 is not counted yet.

## Gamepad-name capture

Extended SDL3 probe metadata with SDL_GetGamepadNameForID, separate from the
joystick name and backward-compatible when reading older snapshots. RPCS3 capture
now builds provisional same-name routing assignments for all enumerated gamepads
and supplies those to profile preparation. Older probes without gamepad names
fail explicitly. Native open success, names and ordering still require child
confirmation; these provisional assignments alone are not launch readiness.
No tests/builds/probes/launches ran. Coverage remains 93/94 RetroArch profiles
(98.9%) and 12/249 partial standalone integrations (4.8%).

## Physical capture session

Added launch-time selected-device SDL3 binding capture with saved identity to
physical-path resolution, retained topology and runtime hashes, cancellation,
and pre-handoff recapture. Profile preparation can consume these captured devices
alongside explicit RPCS3 routing assignments. Unlike PCSX2, this does not request
or reuse an SDL player-number projection: RPCS3 selects by gamepad name. Native
gamepad-name enumeration and child confirmation still need connecting before
dispatch is available. No tests/builds/probes/launches ran; capture code was not
invoked. Coverage remains 93/94 RetroArch profiles (98.9%) and 12/249 partial
standalone integrations (4.8%); RPCS3 is not counted yet.

## Runtime configuration root

Launch preparation now resolves the native Linux configuration root using pinned
Utilities/File.cpp precedence: executable-adjacent portable directory, present
XDG_CONFIG_HOME, HOME/.config, then cwd/config; non-portable roots append rpcs3.
Empty environment values retain native semantics. The directory/environment
selection is rechecked before launch, and existing configuration roots are
required rather than silently creating a new runtime setup. Device capture and
native-child confirmation remain pending. No tests/builds/probes/launches.
Coverage: 93/94 RetroArch profiles (98.9%), 12/249 partial standalone integrations
(4.8%). RPCS3 is not counted as integrated.

## Retained launch plan

Connected profile preparation to a retained native launch plan using the verified
--no-gui/--input-config arguments, executable/content identity checks and classic
SDL selection. The profile guard remains owned by the prepared launch. This
currently accepts file boot targets; directory boot handling remains pending.
Runtime input-root discovery, capture and native-child routing confirmation are
still required before shared dispatch can be connected. No tests/builds/probes
or launches ran. Coverage stays 93/94 RetroArch profiles (98.9%) and 12/249
partial standalone integrations (4.8%); RPCS3 remains uncounted.

## Native profile selection and ownership

Added a retained uniquely named temporary YAML profile in the runtime's existing
global input directory, with location/content checks and automatic cleanup.
Existing user profiles are never overwritten. Native CLI selection is emitted as
--no-gui --input-config NAME, as required by pinned rpcs3.cpp; system_utils.cpp
locates profiles beneath input_configs/global. Runtime root resolution and actual
launch/session dispatch remain pending. No temporary profile was instantiated,
and no tests/builds/probes/launches ran during implementation.
Coverage remains 93/94 RetroArch profiles (98.9%) and 12/249 partial standalone
integrations (4.8%); RPCS3 is not counted yet.

## Calibrated native profile preparation

Connected saved source selections to measured SDL3 gamepad bindings and the
seven-player YAML writer. Preparation requires unique physical paths, recognized
gamepads, verified classic-axis translation and four distinct paired stick axes.
Axis-backed triggers remain proportional; unsupported translations fail instead
of becoming guessed button numbers. Device names are supplied by routing and
must still be verified against the native child before accepting a launch.
Pinned pad_config.cpp confirms input-profile overrides are resolved by name under
the native input-config directory, not as arbitrary absolute YAML paths; launch
isolation must respect that lookup and its per-title fallback behavior.
No tests/builds/probes/launches. Coverage remains 93/94 RetroArch profiles (98.9%)
and 12/249 partial standalone integrations (4.8%); RPCS3 dispatch remains pending.

## Saved setups and visual review

Connected seven-player RPCS3 setup persistence, bounded JSON review/staging and
the shared controller editor's source/destination layout view. Validation requires
all 24 standard pad destinations, distinct calibrated sources, player/controller
identity uniqueness and absolute runtime paths. The editor explicitly labels
native dispatch pending; saving configuration does not imply launch readiness.
No tests/builds/probes/launches. Coverage remains 93/94 RetroArch profiles (98.9%)
and 12/249 partial standalone integrations (4.8%); RPCS3 is not counted yet.

## Native device routing

Added fresh-handler device-name projection from all successfully opened SDL
gamepads in native enumeration order. The pinned handler names controllers by
exact gamepad name plus a one-based same-name occurrence, not GUID or SDL player
index. Physical paths and instances remain attached to each assignment, and
duplicate identities fail explicitly. Native-child verification is still required;
this helper does not establish launch readiness. No tests/builds/probes/launches.
Coverage remains 93/94 RetroArch profiles (98.9%) and 12/249 partial standalone
integrations (4.8%); RPCS3 is not counted yet.

## Measured SDL translation

Added measured digital/proportional SDL3 translation through existing range and
alias checks into RPCS3's positional vocabulary. Native polling confirms Y+
means up, opposite SDL Y signs; the adapter reverses only the serialized Y sign.
Triggers require positive SDL activation. Raw joystick/hat and unsupported
full-range mapped outputs fail explicitly instead of inventing RPCS3 keys.
Physical capture/setup/dispatch remain pending. No tests/probes/launches.
Coverage unchanged: 93/94 RetroArch profiles (98.9%), 12/249 partial standalone
integrations (4.8%).

## Native YAML writer

Added fresh seven-player cfg_input YAML generation with exact handler device
identifiers, explicit standard mappings, null unused players and bounded analog
settings matching native SDL defaults. Unspecified PS/pressure/limiter actions
are blanked so native default combinations cannot silently compete with gameplay
buttons. All strings are quoted. Source pad_config_types.cpp confirms handler
names; sdl_pad_handler.cpp confirms analog defaults. Physical discovery, saved
setup UI and dispatch remain pending. No tests/builds/launches. Coverage remains
93/94 RetroArch profiles (98.9%), 12/249 partial standalone integrations (4.8%).

Source pinned to RPCS3/rpcs3 `54014a7de4b2ccec98c9c0cb7dbebec0606c5cd6`.
Inspected Emu/Io/pad_config.h/.cpp and Input/sdl_pad_handler.cpp.

Added standard DualShock visual-to-native field routes, seven-player YAML node
names, exact SDL input vocabulary and single-input ownership validation.
Optional PS/pressure/limiter actions stay explicit; native default Start+Back PS
combo must not leak into generated profiles. SDL raw ordinals are not supported
by this handler's mapping strings. Physical translation, YAML config, setup UI
and native dispatch remain pending. No tests/builds/probes/launches.

Coverage: 93/94 RetroArch profiles (98.9%); 12/249 partial standalone integrations
(4.8%). RPCS3 is not counted as integrated yet.
