# MAME libretro controller contract

Source revision: `4fc9a9312baaf34963847f884961ad9793fbbc1d` from
https://github.com/libretro/mame. Inspected sparse checkout:
`/tmp/lunchbox-mame-contract.LVMDf2`. No builds or native probes were run.

## Current implementation status

`packaging/mame-game-mouse-only.patch` targets the pinned core's
`src/osd/modules/input/input_retro.cpp`. Its compile-time
`LUNCHBOX_MAME_GAME_MOUSE_ONLY=1` mode suppresses mouse-generated UI movement,
left press/release/double-click pointer events and first-mouse UI default
assignments. It leaves polled game mouse axes/button state updates intact.
The macro defaults to zero: applying the patch alone does not enable the mode.
The opt-in Linux flake package `mame-game-mouse-only` pins the exact archive and
hash, applies the patch and explicitly enables that mode in its build recipe.
It installs only the core library, not a stock-frontend launcher. Package metadata
describes build intent and is not evidence about the loaded runtime. The package
has not been evaluated or built and the patch has not been applied. Saved UI sequences may still
bind mouse inputs, so launch must also isolate those and establish the actual
core's mode before forwarding clicks. The patch is not a completed isolation
contract by itself. Button-bearing launch now also requires inspected device-name
mode declarations, bounded private cfg/ctrlr UI-binding validation, exact source
identity checks and the owned frontend routing handshake. These paths are wired
in source, not verified by execution. The blanket button launch gate is removed;
64-bit Linux and the opt-in runtime requirements remain mandatory.

The relative preview API accepts an optional `buttons` list in its object request
alongside `assignments` (relative axes) and `sources`. Each button entry contains
the exact inspected `field`, `source_player`, and native `output_button` (1–5).
It composes native button/axis ownership and restricts saved source capabilities
without opening devices. Legacy axis-only object requests and native-only axis
arrays retain their prior meaning. Omitting `buttons` (or setting it to null)
preserves the setup's button assignments; an explicit empty list removes them.
Requests with sources must cover the complete combined axis/button set. They
can return a structurally validated `draft_configuration` for read-only review.
A nonempty button list yields `button_mapping_pending: true` when the platform
or saved core-mode requirement is unmet, with a `launch_blocked_reason` describing
that requirement. A false pending flag is not a runtime-readiness claim: fresh
inspection, configuration isolation and live routing must still succeed. Native-only
previews still have no applicable draft. Applying a draft is not saving or staging.
The returned XML and prepared settings are planning data, not launch permission.

The implemented digital-arcade path now composes calibrated physical layouts,
native active-field discovery and private launch configuration. It is not a
general MAME catalog profile or a claim that every arcade game is mapped.
General catalog coverage remains 91 of the documented 95 canonical cores
(95.8%), with 289 enabled profiles and 157 layouts. Specialized MAME/FBNeo
per-game paths do not increase that count.

### Presets and exact per-game priority

The application default is Automatic six/eight-button arcade geometry. When all
ordinary digital controls have explicit overrides and no default profile remains,
Automatic still selects the six-button arcade destination; extended switch
channels and stick-click outputs retain their prior fixed/eight-button selection.
This explicit-only fallback changes geometry, not native routing, and remains
runtime-unverified. Combined analog profiles also retain an explicitly selected
six-button, eight-button or Neo Geo layout when no default digital bindings remain.
Ordinary explicit Automatic buttons choose arcade geometry before analog
composition; pure analog Automatic continues to use fixed-channel geometry.
Six buttons use `1 2 3 / 4 5 6`; eight use `1 2 3 7 / 4 5 6 8`. Automatic chooses
eight only when the inspected player uses native button 7 or 8. Neo Geo is an
explicit A/B/C/D variation, not guessed from a display title or archive name.
Only active controls are required, and too-small presets fail instead of
discarding controls. The fixed native output channels remain unchanged.

Exact saved emulator/core/content setups take precedence over the default.
Older saved setups retain legacy fixed-channel geometry; changing a preset
invalidates assignment review. The application default can be disabled for
per-game-only setup. Typed optional profiles distinguish unused digital ports
from invalid snapshots or preset errors; the full field planner still rejects
unsupported fields before automatic player discovery.

Known schema-3 snapshots are retained as stored data during settings load/save,
with their original schema and assignments unchanged. The setup list marks them
as needing reinspection; loading any setup clears assignment approval. Only
current-schema snapshots may be parsed as new inspection results, generate digital
profiles, pass assignment review/staging or authorize a saved launch. An exact
stale override blocks rather than falling through to automatic defaults. Generate
an inspection request from the retained draft, run the trusted inspection, use
its matching completed result, review, stage and save to replace the snapshot.
Unknown schemas and malformed stored identities are not silently accepted.

### Automatic preparation boundary

Observed JOYSTICKLEFT/JOYSTICKRIGHT fields now select a dedicated twin-digital
layout under Automatic or legacy fixed selection. The left stick uses native
hat switches; the right uses buttons 4/1/3/2 for up/down/left/right, preserving
independent digital inputs without assuming analog conversion. Explicit six/eight
button or Neo Geo presets fail for twin-stick fields. Up to four observed
numbered action buttons are allocated in native-number order to switches 5-8.
The calibration profile retains each native button number, and uses the same
snapshot-derived allocation as the native configuration. More than four actions
or an additional normal joystick cluster still requires another routing contract.
Start/Coin remain available when observed. Source identities come from pinned inpttype.ipp, and
the native switch wires from input_retro.cpp. Calibration/launch behavior has
not been tested. Deferred checks include both-stick simultaneity, absent fields,
player disconnection, incompatible presets, sparse/high-numbered action sets,
simultaneous actions/directions and output-capacity rejection.
Assignment planning requires physical identities for every observed twin-stick
control. A button or axis cannot supply different sticks or an action and a
direction. Opposite directions on one stick may share an axis only with opposite
raw signs. Optional catalog entries do not excuse missing observed controls.
Deferred checks also include cross-stick axis aliases and action/direction aliases.

The fallback is limited to the exact Arcade platform, native RetroArch and
self-contained `.zip`/`.7z` sets with exact machine short names, or split/nonmerged
sets when sibling dependency discovery is explicitly enabled. That setting
defaults off and searches only the selected archive's canonical parent directory.
Enabled preparation retains the expanded manifest for session staging and source
hash rechecks. Exact saved setups keep their explicit manifests. It resolves
original-content persistence before private command-file substitution, copies
declared native configuration/state inputs and assigns only inspected digital
players with usable saved calibration. Unselected ports are explicitly cleared.

Applicable RetroArch core/folder/game `.cfg` overrides are rejected before any
save-directory creation or native inspection unless override loading is disabled.
Includes, enabled external MAME INI/path settings and unresolved runtime/content
identities are not silently approximated. Reviewed saved setups supply their own
resolved persistence/manifest contract; that path does not implement arbitrary
frontend override merging.

One retained configuration snapshot serves persistence, inspection and launch.
It checks base configuration, effective options and applicable override absence
before/after inspection and before launch. Automatic setup also tracks save-path
canonical targets and native cfg presence/absence. Both frontend and options
sources retain their lexical filename, resolved target and bytes; a new
higher-priority option file or changed target requires renewed preparation, even
with identical bytes. Valid configuration-file symlinks work; broken links, read
errors, non-regular files and files over 8 MiB fail explicitly. The frontend's
primary `retroarch.cfg` must exist; absence is not treated as an empty file or
silently resolved to legacy/system defaults. Draft request generation uses these
same bounded source readers. Other adapters' base-file resolver is unchanged.

### Pinned frontend path evidence

RetroArch revision `69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576` was read directly
from Git objects; the sparse source checkout's working HEAD is instead
`9a4726b05089ea339a53a313ece920bd8748d006`. These are not interchangeable source
identities. The pinned `runloop.c:1083` / `runloop_init_core_options_path` selects
one effective source in game, folder, core, global order, rather than merging.

For omitted `rgui_config_directory`, pinned `platform_unix.c:1934` supplies
`config/` beneath the frontend configuration root. Explicit empty/default uses
the current config file's directory (`configuration.c:4337` and
`file_path_special.c:196`). Save/state settings are different: the dedicated
handlers at `configuration.c:4416` restore desktop defaults for `default` and
ignore an empty non-directory value, leaving initialized defaults in effect.
`platform_unix.c:1978` supplies `saves/` and `states/`; content-directory flags
override those roots, and sorting appends the content folder before `MAME`.
Automatic setup fails when a selected path cannot be resolved/created rather
than silently adopting RetroArch's directory-creation fallback.

Pinned `configuration.c:3642-3676` falls back from the missing primary config to
`$HOME/.retroarch.cfg`, then may initialize one from a build-specific system
skeleton. MAME's resolver requires an explicit primary file before preparation
instead of guessing that fallback's effective settings or persistence roots.

### Optional dependency discovery

Advanced inspection accepts explicit ROM search directories. Request generation
resolves their exact canonical identities without running native code. When the
user starts the trusted inspection, the same selected core emits
`-noreadconfig -nowriteconfig -nodtd -listxml` metadata for the exact machine and
missing reachable parents/devices. Runtime/core fingerprints surround discovery,
staging and field inspection. The XML build identity and repeated records must
remain consistent. This does not import or redistribute a third-party database.

The parser bounds XML size, nodes, depth and machine counts, rejects DTDs and
other directives, and follows exact `romof` and `device_ref` identities. Optional
and undumped images do not create mandatory local-file requirements. Discovery
uses at most 32 queries within the inspection's 120-second total deadline.
Native CLI success intentionally ends core loading with a nonzero result in the
pinned wrapper; process exit 0 or 1 is therefore accepted only alongside a
complete XML document containing the requested machine. Runtime confirmation of
this behavior remains pending.

Each required ROM-bearing set needs its own exact ZIP/7z, and disks use exact
set/name.chd paths, including the same filename under an exact ROM-parent folder.
The selected disk is staged under the requesting set's directory. Parent cycles,
missing metadata, ambiguous locations and conflicts with declared sources fail.
Pinned driver.cpp:32-38 constructs the ancestor search path; infoxml.cpp emits
that relationship as romof, and romload.cpp:160 uses those paths for CHD lookup.
Native fallback to a differently named parent disk compares hashes. Discovery
now retains valid native SHA-1 metadata for good dumps and admits ancestor disk
filenames with the same SHA-1. Missing hashes and bad/undumped records never
authorize that substitution. Alternative paths are bounded at 4096 per disk;
ambiguous files still fail. This identifies candidate files from native metadata;
the native loader must still validate their actual CHD content.
The expanded manifest preserves every explicitly declared file, including optional
images. Required dependency lookup first honors an exact declared staging
destination, allowing its selected absolute regular-file source outside the
search roots. Multiple declared alternatives, duplicate destinations or an
invalid explicit source fail instead of selecting another candidate. This source
selection uses the declared destination; it does not infer identity from the
source file's name. The remaining search applies only to undeclared dependencies.
The expanded manifest also preserves optional
images omitted from mandatory metadata requirements. An identical discovered
source/destination pair is included once; a different source at that destination
fails. Retained optional files receive the same staging and fingerprint checks.
The receipt records requested and expanded manifests; applying it requires the
unchanged original draft, then copies the expanded manifest for review/staging.
Empty search directories preserve the explicit-manifest workflow. Automatic
launch can opt into the same discovery using the archive's directory. Discovery's
output transport is currently implemented for native Unix runtimes.

### Remaining coverage and verification

Snapshot schema 4 distinguishes positively identified native internal signals
from unknown fields. In pinned `luaengine_input.cpp:244`, the native `port.fields`
table inserts every non-internal field under `field.name` before adding aliases.
Name collisions can replace values but cannot remove keys. After a class-getter
failure, absence of the exact successfully read name therefore proves native
exclusion. A present name, failed name getter or other ambiguity remains unknown;
the getter failure alone never establishes internal status. The name table is
used only for this one-way omission proof, not for enumerating fields or matching
identities. Mask-based discovery is retained, and earlier snapshots require
reinspection. Internal signals remain in the snapshot and review, separate from
mapped bindings and preserved DIP/configuration settings; no generated native
field override is written for them.

Unknown fields, including internal names colliding with user-mappable fields,
remain launch blockers. Analog/peripheral inputs, conditional mode changes and
nonstandard controls require further contracts.
The dependency resolver now consumes metadata from the selected core during
opt-in inspection. Merged archives, CHD parent-data dependencies beyond declared
machine metadata, complete
guided new-setup creation and runtime validation of discovery remain unfinished.
No all-game completion percentage is established.

Scoped Rust formatting and static source review were performed. Builds, tests,
hardware inspection and emulator runs remain deferred. Independent acceptance
review remains pending. These changes remain uncommitted and are not accepted
as runtime-verified.

Deferred step-190 acceptance cases include exact native XML production and CLI
exit behavior, missing parents/devices, conflicting repeated records, optional
and undumped images, malformed/truncated/oversized output, cancellation while
an output writer remains open, duplicate roots/archives, declared-source
conflicts, runtime changes, and applying an expanded receipt only to its matching
draft. No such cases have been executed.
Step 191 additionally requires optional-file retention, identical-pair coalescing,
duplicate-declaration rejection and conflicting-source rejection checks; these
also remain deferred.
Step 193 adds deferred checks for matching and mismatching ancestor hashes,
missing/malformed hashes, bad dumps, repeated conflicting disk records and
alternative-path limits. No runtime coverage is claimed from these source edits.

Deferred acceptance checks for steps 188-189 must cover a schema-3 settings
load/save round trip without version or assignment changes, strict rejection at
inspection/profile/staging/launch boundaries, unchanged current-schema checks,
and replacement through the completed-inspection workflow. Configuration checks
must cover identical-byte symlink retargeting, missing/broken/unreadable/oversized
main files, preserved option precedence and the final pre-launch recheck. None
of these cases has been executed in this implementation phase.

The step-by-step sections below are historical checkpoints; their pending-work
statements are not the current implementation status.

## Step 153: source-defined standard digital sequences

The current wrapper is not interchangeable with SAME CD-i's older input backend.
`src/osd/modules/input/input_retro.cpp` 207..214 initializes numbered buttons
1..6 from RetroPad B/A/Y/X/L/R. 1350..1361 registers these native button items;
1390..1407 registers L3/R3 as BUTTON7/8. L2/R2 are separate trigger channels,
including analog magnitudes read at 779..799; they must not be labeled as buttons
7/8. Four stick axes and two negated trigger axes are registered at 1310..1350.

The wrapper creates eight native joystick devices (1537..1554), each corresponding
to its fixed frontend port, and polls all eight. `Input_Binding` has game-specific
button permutations but is called only with `buttons_profiles` enabled. The
required `mame_buttons_profiles = disabled` leaves the initial mapping in a fresh
core instance; this must not be applied halfway through a reused instance that
has already changed the static button map.

`controller_mame::digital_controller_xml` now writes a version-10 controller
configuration with standard type defaults for explicitly selected ports. It
maps eight buttons, four hat directions and Start/Select, including arcade
STARTn/COINn aliases. Unselected ports receive NONE for those same types. Fixed
native tokens are backed by `src/emu/input.cpp` and `src/emu/inpttype.ipp`.
The writer accepts only a nonempty subset of ports 1..8, not arbitrary XML or
input sequence text. `digital_options` supplies the fixed button-profile option,
disables the extra four-way remapper, auto-state loading, configuration writes
and external MAME INI path overrides.

`src/emu/ioport.cpp` 2280..2320 distinguishes default/controller type sequences
from system field overrides. A per-game field override can supersede this writer's
defaults. Therefore this is not yet an enabled MAME launch profile. Per-game
input discovery and field override handling, configuration preservation, owned
controller-file routing, core/media identity, analog/peripheral modes and UI
integration remain necessary. No existing user configuration was replaced.

## Step 154: preserve per-game settings while replacing digital sequences

`merge_digital_configuration` now replaces only standard sequences on saved
fields whose type belongs to the source-defined digital control set. It requires
the exact machine short name and one selected system/input section, and retains
tag, mask, default value, toggle behavior, nonstandard sequences, unrelated
fields, video settings and comments. A controlled field without a standard
sequence gets one; self-closing field nodes are expanded without rewriting their
attributes. Multiple distinct fields of the same type are handled independently.

Parsing is bounded by 8 MiB, 100,000 elements and 64 levels. DTDs, duplicate
attributes, duplicate standard sequences for one field, duplicate selected
systems/input sections and malformed structure fail instead of choosing an
arbitrary interpretation. Generated tokens are fixed by the typed port contract;
the writer accepts no arbitrary sequence text. Missing source configuration
produces a minimal selected-system document, leaving controller defaults to
supply standard bindings.

This returns bytes for an owned launch directory and does not modify the user's
file. Native driver fields absent from the saved configuration may have their
own explicit defaults, so per-game driver discovery remains necessary. Owned
file routing and launch integration are also still pending. Only formatting and
source inspection were performed; no tests, builds or emulator execution.

## Step 155: active native field snapshot adapter

The Rust inspection module now generates a native autoboot API adapter and
parses its bounded response. The adapter checks `machine.system.name`, walks
each port through its 32 input-mask bits, deduplicates native identities, and
exports port tag, type token, mask, masked default value and analog capability.
The output path belongs to an isolated inspection session supplied by the
future caller. It schedules that inspection machine's exit after success or
failure. Nothing is executed by generating the adapter text.

This route deliberately does not enumerate `port.fields` by display name:
`src/frontend/mame/luaengine_input.cpp` 244..264 constructs that table with names
as keys, which can collide. Its `port:field(mask)` binding calls
`ioport_port::field` at `src/emu/ioport.cpp` 1567..1573, selecting the first
enabled field intersecting the mask. Consequently the snapshot describes
currently visible enabled fields, not disabled conditional alternatives or
multiple simultaneously overlapping fields. DIP, slot and runtime condition
changes require renewed discovery; this is not an exhaustive static inventory.

Rust rejects oversized reports, wrong machine/version, invalid masks/tags/type
tokens and duplicate identities. The native `TYPE_OTHER(type,player)` fallback
token is retained as discovery data, not treated as a supported digital action.
The parser checks shape and identity only; runtime/content hash binding, process
isolation, deadlines, source-state preservation and launch integration remain
pending. The native Lua bridge is emitted by Rust as required by this core's
introspection API; no separate executable tooling or test run was introduced.

## Step 156: connect discovered identities to native overrides

`plan_digital_fields` now consumes a validated active-field snapshot and produces
both the owned controller XML and merged per-game configuration in memory.
After the global type-default section, a machine-specific controller section
uses each controlled field's exact tag/type/mask/masked-default identity.
`ioport_manager::load_controller_config` applies these as field defaults rather
than merely changing the global type fallback, covering driver-provided explicit
sequences that never appeared in a saved configuration.

Selected ports get their native joystick sequence; unassigned ports get NONE.
Analog fields unexpectedly labeled with a controlled digital type fail. All
other snapshot fields remain in an explicit unhandled list rather than being
silently counted as mapped. The planner returns separate mapped and disabled
field counts and rejects a selection with no mapped fields. Serialization is
deterministic by field identity and XML-escapes native port tags.

The snapshot is revalidated at planning time, but its runtime/content provenance
still belongs to the future isolated runner. Conditional refresh, preservation
of any pre-existing controller-profile layer, owned file staging and launch/UI
integration remain pending. No test, build, native adapter or emulator was run.

## Step 157: honor native joystick slot remapping

The active snapshot format is now version 2 and includes the joystick class's
enabled state and each device's exact ID/devindex. Rust requires all eight
`RetroPad0` through `RetroPad7` identities, unique native indices within the
source-defined 255-slot range, and enabled joystick input for digital planning.
No display-name substring is used to select an identity.

The planner now translates frontend ports through those observed indices when
emitting type defaults, machine-specific fields and saved-config overrides.
This addresses native controller-profile device maps: `input.cpp` 1342..1397
applies maps and `inputdev.cpp` 636..655 swaps device indices. Native Lua exposes
`input.device_classes.joystick.devices`, device `id` and `devindex` in
`luaengine_input.cpp` 582..650. The snapshot's list position is deliberately not
used as the native slot.

Existing controller-file bytes still need to be preserved by the staging layer,
and the inspection must run with that same controller layer. Routing metadata
alone does not prove matching configuration provenance. This step has not run
the adapter, core, build or tests.

## Step 158: preserve the original controller-profile layer

The field planner now accepts the original controller-profile contents as well
as the original game configuration. It keeps all original controller-file bytes
and appends the generated default and machine-specific sections immediately
before the closing root. An empty root is expanded while retaining its original
attributes. Device maps, remap tables, source/parent-specific sections, unrelated
bindings and comments remain ahead of the calibrated sections.

This ordering follows `configuration_manager::load_xml` at `config.cpp`
194..269: applicable controller-system nodes are loaded in file order. Original
first-applicable device maps therefore remain first; generated field overrides
are applied afterward using the observed native indices from the same original
profile. The source is bounded and parsed before any output is returned, with
duplicate attributes, invalid roots, DTDs and malformed structure rejected.
Repeated applicable system blocks are preserved because MAME supports them.

Each plan is reproduced from original declared inputs, not from a previously
generated session file. The isolated runner must still prove that inspection
and launch use the same original configuration and runtime/content. No user
configuration has been overwritten and no native code or tests were executed.

## Step 159: MAME-specific inspection command construction

The existing FBNeo helper requires refreshed descriptors after one frame, which
does not provide MAME's autoboot field snapshot. `inspection_command` now emits
the MAME-specific `.cmd` payload for a separate supervised invocation. It takes
only an exact machine name, a caller-owned private root and whether an original
controller profile was staged. It routes cfg, ctrlr, ROMs, NVRAM, disk differences,
states, snapshots and input recordings into that tree, sets the generated
`inspect.lua` as the zero-delay autoboot script and disables INI loading/writing,
automatic states and cheats. The final token is the machine name.

Absolute quoted paths avoid the wrapper's relative-option path rewriting.
Control characters, quote/backslash/search-path delimiters, overlong paths,
excessive argument counts and command bodies beyond 4095 bytes are rejected.
These limits follow this pinned wrapper's `CMDFILE[4096]` and 128-argument
capacity, not SAME CD-i's older 511-byte command contract. `retro_init.cpp`
789..943 appends explicit command options after wrapper defaults.

The runtime supervisor, declared-input copies/hash binding and RetroArch
system/save configuration still need implementation. This constructor neither
creates the private tree nor starts a process. No commands, native code, builds
or tests were executed.
# Explicit trigger switches

Selected-player routing applies action-capacity and twin-stick direction
collision checks only to explicitly selected ports. Unselected ports receive
NONE for the complete known BUTTON1-BUTTON16 range, as well as the existing
known direction/start/coin defaults. Unknown and peripheral fields are not
silently exempted. Disabled fields remain separate from mapped coverage.

The ordinary digital planner preserves existing native bindings for non-analog
miscellaneous SERVICE and SERVICE1-SERVICE4 fields. These exact maintenance
tokens have native keyboard defaults in pinned `src/emu/inpttype.ipp`; no
gameplay button is allocated automatically. Per-field explicit assignments still
override this policy. Power, door, tilt, reset and unknown inputs are not included
in this exception. Review reports preserved service fields separately and does
not count them as calibrated mappings or verified native-keyboard delivery.

Explicit per-field switch assignments also accept frontend LeftTrigger and
RightTrigger. At the pinned native source revision
`4fc9a9312baaf34963847f884961ad9793fbbc1d`, `input_retro.cpp` registers L2 as
RZAXIS and R2 as ZAXIS, negates pressure travel, and falls back to full travel
for pressed digital L2/R2 when pressure is zero. `input.cpp::code_to_token`
serializes modifier before class; `inputdev.cpp::read_as_switch` compares
negative-axis travel against the native threshold. The resulting switch tokens
are `RZAXIS_NEG_SWITCH` and `ZAXIS_NEG_SWITCH`, not BUTTON7/BUTTON8.

Generic defaults remain unchanged. For an exact inspected non-twin-stick game,
native BUTTON9/BUTTON10 now route through these two switch channels. Unselected
ports receive NONE. Twin-stick allocation supports six observed actions among
native buttons 1-16: native switches 5-8 followed by the two trigger switches.
It preserves all eight direction inputs, allocates actions in native-number
order, and retains those action numbers on the diagram. Larger sets still need
explicit routing or another transport; they are not silently dropped.

For non-twin games, sparse native Buttons 11-16 can use action channels not
occupied by observed Buttons 1-10. Their native-number ordering determines the
allocation; conventional observed low-button routes are preserved. The ten-action
capacity is checked before allocation. These remaps require fixed-channel geometry
(selected by Automatic), not a numbered arcade preset that would mislabel actions.
Resolved field routes expose the real native action identity and captured label.
The fixed/twin channel diagrams have optional digital switch controls, including
their derived analog variants. Automatic geometry uses the fixed-channel family
when requested. Explicit smaller arcade presets may reject these extra controls.
If an analog pressure assignment already owns the same output, its measured
physical binding is shared and remains proportional; a switch-only assignment
does not claim measured pressure. Native threshold behavior and runtime input
delivery remain untested. When inspected button 9/10 and an explicit analog
field share a trigger, the combined profile retains one proportional binding;
the same physical travel also crosses the native switch threshold. This does
not create an independent extra analog channel.

## Explicit stick-direction switches

The eight X/Y/RX/RY positive/negative switch tokens are available for explicit
standard or increment/decrement assignments. They use the existing frontend
LeftStick/RightStick directional outputs; automatic action allocation does not
consume them. Fixed and twin panels have dedicated derived switch variants,
including mixed analog variants. Existing proportional bindings on a shared
channel remain measured; switch-only requirements stay digital.

Pinned RetroArch revision `69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`,
`input/input_driver.c::input_joypad_analog_axis`, supplies full signed travel from
button bindings when the analog result is zero. Simultaneously pressing both
directions cancels to neutral. MAME applies its native axis-switch threshold.
These eight directions share four bipolar axes, not eight independent simultaneous
action channels. Physical travel and digital fallback can also interact; no
independent analog/switch ownership is implied. Runtime remains unverified.
