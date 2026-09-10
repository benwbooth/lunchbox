# FBNeo controller contract

## Calibrated Arcade Gun launch connection (2026-09-08)

Follow-up: preview, save validation and launch now share the same deterministic
two-axis reservation. Over-capacity mixed setups are rejected before device
preparation. Player preparation also checks the initial fragment's missing and
external target lists before opening a bridge, rather than postponing rejection
until session attachment. These are validation corrections, not new coverage.
Formatting/whitespace checks only; tests and runtime checks remain deferred.

Saved `absolute_sources` select a validated physical absolute-device record for
an already inspected Arcade Gun (1029) port. Both native X/Y queries must exist.
The physical assignment dialog can apply/remove a saved calibration; applying
replaces only the selected port's previous aim assignments in the text draft.
Button assignments remain separate. Native descriptors and queries are retained.

Launch composes `AbsoluteReader` and an optional calibrated gamepad reader into
one owned virtual joystick. Two unused output-axis codes prevent physical channel
collisions; final bindings use the virtual device's actual joydev numbering.
The existing session owner retains, health-checks and drops the bridge. Short
button transitions are published individually. Source errors destroy the output.
Neither mouse nor keyboard events are emitted by the virtual publisher.

Startup waits up to ten seconds for both aim axes to produce a real report and
for any gamepad source to synchronize. Cancellation tears down the worker.
Stationary initialization is not guessed from independent X/Y ioctls. Stale
queued aim events, changed physical bounds and out-of-calibration samples fail
the bridge. Hardware offscreen/reload semantics are not inferred from those
conditions. Aim-only ports need no dummy gamepad; mixed ports keep their existing
physical calibration and button checks. Saved source identity is checked at open.

This connects one absolute-aim route, not lightgun offscreen/reload protocols,
touch/pointer modes, perspective correction or every physical gun. Source was
inspected and Rust formatting/whitespace checks passed. No builds, tests, input
devices or emulator sessions were run. Independent review remains unavailable.

## Absolute coordinate conversion checkpoint (2026-09-08)

The absolute calibration preview now also shows native libretro coordinates.
The rectangular transform calculates them directly from raw readings over
`-32767..=32767`, with exact calibrated endpoints and integer nearest rounding.
It does not double-round the unsigned preview or emit `-32768` for ordinary aim.
Swapped/reversed axes, saved hardware bounds and the separate
`outside_calibrated_area` flag retain their existing semantics.

This follows the pinned FBNeo `CinpDirectCoord` at
[retro_input.cpp lines 548–556](https://github.com/libretro/FBNeo/blob/a251c76229f1637e433b93e29845039752771b6d/src/burner/libretro/retro_input.cpp#L548-L556):
the callback value is offset by `0x7fff`, and `-32768` produces the special `-1`
offscreen case. Hardware offscreen input is separate from calibration overflow.
The preview does not imply viewport/perspective calibration, device routing or
runtime absolute-pointer support. No new core is counted. Formatting and
whitespace checks only; no tests, builds or device operations were run.

## Native keyboard and mapped-key composition (2026-09-08)

`keyboard_passthrough_port` explicitly selects the frontend's native keyboard
state for one inspected keyboard port. It is not a physical-device selector,
exclusive keyboard capture or calibrated key remapping. It defaults to absent.
The UI confirms the draft change before removing supported gamepad-to-key
assignments. Other native inputs and unsupported keyboard keys remain required.

The implementation follows the pinned frontend's non-Android Linux key table,
not merely the RETROK enum. `RETROK_COLON` and `RETROK_QUOTEDBL`, among others,
have no entries in that table. Such keys remain in the calibrated requirements
and can use explicit keyboard-channel bindings alongside passthrough. This lets
full keyboards use native input without assigning every key to a gamepad channel,
while avoiding silent loss of the Spectrum punctuation inputs.

Source evidence:

- [FBNeo native keyboard mapping](https://github.com/libretro/FBNeo/blob/a251c76229f1637e433b93e29845039752771b6d/src/burner/libretro/retro_input.cpp#L2587-L2707).
- [RetroArch Linux key table](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/input/input_keymaps.c#L1222-L1388).
- [udev keyboard state](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/input/drivers/udev_input.c#L3647-L3651)
  and its `RETRO_DEVICE_KEYBOARD` branch at lines 3823–3824.

Save and launch both compute remaining gamepad requirements after the explicit
passthrough selection. Fully supplied keyboard-only or relative/keyboard ports
need no dummy gamepad. Fresh inspection is still mandatory. Launch retains the
native keyboard device, selects the udev input driver and clears frontend keyboard
hotkeys; remaps remain disabled unless explicit mapped keys also need a private
remap. Independent simultaneous keyboards are not supported by this shared-state
path. Actual availability of every physical key remains a runtime prerequisite.

Source inspection, Rust formatting and whitespace checks only; no tests, builds,
UI sessions, keyboard capture or runtime execution. No whole-core completion or
overall percentage is established by this checkpoint.

## Keyboard save-to-launch integration (2026-09-08)

This checkpoint supersedes the disconnected-plan notes below. Optional
`keyboard_bindings` now retain exact native key addresses and explicit frontend
channels alongside physical source assignments. Validation requires every
inspected key, one keyboard owner, unique channels and an explicit physical
assignment for every key. Foreign native addresses and conflicting shared-channel
gestures are rejected. Old setups default to no keyboard bindings.

The editor offers 16 named button channels and the existing calibrated source
picker. Advanced JSON can supply eight more analog directions, subject to the
ordinary measured-axis pairing checks. The 24-channel ceiling is enforced; this
does not represent a full computer keyboard or independent simultaneous keyboards.

Save and launch use the same translated physical requirements while retaining
native inspection evidence unchanged. After fresh inspection matches the saved
contract, launch writes the retained core-library fallback `.rmp` under a private
remap directory and includes its hash in session input checks. Automatic remaps
and remap binds are enabled only for this private keyboard session; controller
sorting is disabled. Keyboard hotkeys are cleared in addition to gamepad hotkeys.
Ordinary sessions keep their existing remap-disabled behavior and keyboard hotkeys.

No tests, builds, UI runs or runtime/device checks were performed. Formatting and
whitespace checks passed. This is one implemented integration path, not a new
fully covered core, a runtime acceptance result or an overall completion percent.

## Relative-input implementation checkpoint (Step 589)

Source now includes exact saved relative-device selection, per-port restriction,
mixed/gamepad-free preparation, fresh-inspection binding, owned endpoint bootstrap
and frontend command-bridge handoff. The temporary relative launch/attachment gates
are removed. The path requires 64-bit Linux and the opt-in frontend routing
extension; owned route/focus confirmation precedes input publication. Unsupported
conversions remain blocking. No build or runtime verification is claimed.

Private FBNeo config clears mouse-button hotkey and pad fallback bindings and
disables mouse/pointer menu navigation for relative sessions. Pinned RetroArch
[`configuration.c`](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/configuration.c)
declares the menu flags at 1964–1965 and resets an explicitly configured mouse
binding to NO_BTN before parsing at 6676–6691. The private `nul` values preserve
that unbound state. These settings do not disable raw libretro mouse callbacks.
This is source evidence, not proof of complete frontend/core UI isolation.

Step 207 adds `exclusive_source` (serde default false) to relative-device
settings. The form exposes explicit opt-in with a whole-node capture warning.
`open_saved` requests capture on its verified source descriptor after creating
the virtual output; failure propagates and drops the session. Neutralization
closes the sole physical descriptor immediately, including on pump failure,
normal shutdown and group cleanup. This relies on Linux's
[EVIOCGRAB definition](https://raw.githubusercontent.com/torvalds/linux/v6.12/include/uapi/linux/input.h)
and [evdev release and ioctl implementation](https://raw.githubusercontent.com/torvalds/linux/v6.12/drivers/input/evdev.c):
a nonzero scalar requests capture, and file release ungrabs that client.
The physical source is not the virtual output: desktop exclusion of the latter,
focus policy and exact frontend routing remain required before enabling launch.
Capture lifecycle and failure cleanup have not been exercised.

Step 206 adds `RelativeMouseGroup`: full-list validation before opening selected
player devices, cleanup on partial-open failure, complete-set endpoint readiness,
and group-wide shutdown on endpoint/pump failure. It does not make publications
transactional: packets sent before a later source fails cannot be rolled back.
The caller must enforce a readiness deadline and exact frontend routing.
RetroArch revision `69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`,
`input/drivers/udev_input.c:677-695,1110-1135,3392-3420`, resolves configured mouse
indices through `pointers` and rebuilds that table after hotplug. Therefore
sorting event nodes or saving an inferred index cannot prove stable routing.
This group is not yet wired into emulator launch; no native checks were run.

Step 203 adds a portable saved relative-device record and an advanced JSON editor
in automatic controller settings. Up to 16 entries retain normalized absolute
event/physical-identity paths, selected relative axes, distinct mouse-button maps
and motion calibration. The full list is validated before replacing staged
settings. Old settings default to an empty list; main Save persists changes.
`RelativeMouseSession::open_saved` validates a record and reuses the exact-device
opener, so stale paths must be reselected rather than matched by name. Saving
does not open devices or enable FBNeo launch. Guided device discovery and exact
frontend routing are still required. Round-trip, invalid-list atomicity, stale
identity rejection and editor behavior remain untested.

Step 202 adds an explicit one-to-one mouse-button map to relative sessions.
Only mouse-button codes are accepted, duplicate source/output identities fail,
and existing constructors keep identity mapping. The virtual publisher advertises
mapped buttons; packet state and shutdown releases use those output identities.
This supports left-handed/button-reordered transport without synthesizing chords.
Button permutations, simultaneous holds, invalid maps and failure releases remain
deferred checks. This does not enable FBNeo mouse launch or a calibration UI.

Step 201 adds optional X/Y swapping to relative-motion calibration, applied
before output-axis sensitivity/inversion. Fractional remainders belong to the
output axes. The publisher's allowed axes follow the same permutation, including
a source exposing only one motion axis. Wheel axes and buttons are unchanged.
Omitted swap_xy defaults false. Rotation combinations, single-axis publication
and unchanged wheel behavior remain deferred acceptance checks.

Step 200 adds an explicit relative-motion calibration contract to the owned
session: independent X/Y sensitivity of 1–1000 percent and optional inversion.
Its default preserves physical counts. Fractional movement carries between
complete kernel reports using signed integer remainders; buttons and wheel
detents are untouched. Shutdown clears residual motion. It is a count-to-count
transform with no velocity or elapsed-time inference. The session pump applies
it before publication and stops on overflow. No settings UI or FBNeo launch
integration is claimed. Deferred checks include fractional accumulation,
direction reversal, axis independence, wheel/button preservation, overflow and
shutdown. No builds, tests or native execution were performed.

Current addition (step 199): the relative-input transport now captures and
publishes conventional horizontal/vertical wheel detents (REL_HWHEEL=6,
REL_WHEEL=8), alongside X/Y and mouse buttons. It advertises only selected wheel
capabilities and preserves complete reports and signed counts. High-resolution
wheel events are not converted or double-counted. This remains transport
infrastructure; frontend mouse routing and desktop interaction policy are still
unfinished. Pinned RetroArch udev_input.c:882-893 accepts only wheel event values
of +1/-1 and stores boolean wheel flags, so forwarding a multi-detent count is
not yet a complete RetroArch wheel adapter. That downstream conversion/polling
contract remains required. No builds, tests or device execution were performed.

Source: `libretro/FBNeo` revision
`a251c76229f1637e433b93e29845039752771b6d`, particularly
`src/burner/libretro/retro_input.h` and `retro_input.cpp`.
The main `finalburnneo/FBNeo` tree is not the libretro input implementation.

The new Rust device model records Classic, Modern, six-button panel, mouse-ball,
arcade-gun, full-mouse and touchscreen identities, including their distinct
libretro subclasses. Plain joystick, keyboard, pointer and lightgun identities
are also represented. Native input supports up to six players.

This is not a generic arcade mapping. `AnalyzeGameLayout` and input setup use
`BurnDrvGetInputInfo` to determine the loaded driver's controls. Button order
also changes with device selection. Per-driver topology, effective options,
input descriptors, macros and physical capability requirements must be
resolved before enabling a profile. No FBNeo launch profiles are enabled by
this step, and no builds, tests or emulator runs have been performed.

The Rust device model now records native generic FIRE01–FIRE10 output order,
four-column positional rows and the three-button straight-line layout for
Classic, Modern and six-button-panel devices. These are virtual libretro
button IDs, not physical controller indices. They remain separate because
driver-specific fighting-game and Neo Geo routing can override generic order.
No game is assigned one of these mappings without resolving its native layout.

The standard fire-button resolver now preserves native branch priority:
Modern Neo Geo uses Y/B/X/A, otherwise fighting layouts use top-row punches
and bottom-row kicks, otherwise generic numbered fire ordering applies.
FIRE09/FIRE10 are R3/L3. Inputs beyond a branch's supported range stay unmapped
instead of receiving a guessed generic fallback. Driver classification and
specialized nonstandard mapping functions remain pending.

Standard non-fire routing now covers select/coin, start, D-pad directions and
X/Y/Z axes. A sole Z-labelled axis maps to left-stick X; with multiple axes it
maps to right-stick Y, matching the native per-player axis-count rule. Matching
uses native case-sensitive prefixes. Unknown controls stay unresolved; these
rules are not applied to specialized driver input paths automatically.

Metadata analysis now follows the native player-name/info precedence, counts
per-player axes, and detects the standard fighting layout from player-one
punch/kick descriptions or the CPS2 real-fire-button rule. Hardware and input
facts must come from the selected driver, not fuzzy title matching. This does
not yet resolve specialized layouts, macros or runtime descriptor collection.

Effective-descriptor records now preserve the full port/device/index/id
address and every label sharing it. Grouping is bounded and does not collapse
keyboard, mouse, gun and analog addresses into pad buttons. Shared-address
actions remain visible as simultaneous outputs rather than falsely independent
controls. UI consumption is not connected yet.

The existing isolated libretro diagnostic helper now captures
`SET_INPUT_DESCRIPTORS` notifications into its reports, copying bounded labels
and retaining the latest complete descriptor list rather than native pointers.
Capture errors fail report construction. The helper has not been executed;
an FBNeo-specific content/options/device inspection workflow is still needed.

The application can now decode the helper's descriptor-array format through
the shared typed record, with byte/entry/label bounds and strict field parsing.
It preserves record order and shared addresses. This parser does not accept a
descriptor list as proof of core, content, options or selected-device identity;
the inspection report still needs that provenance and launch integration.

Loaded-device coercion now follows `retro_set_controller_port_device`: MSX
and Spectrum force joystick on ports 0/1 and keyboard on port 2; other active
drivers accept the native pad/pointer/gun/mouse set and replace unsupported
IDs with Classic. Storage still depends on the driver's controller count.
Descriptor refresh may be deferred until controller setup has completed, so
the requested ID and a startup descriptor list alone do not prove the
effective post-selection mapping. Inspection sequencing remains pending.

Helper reports now include a descriptor-notification count captured under the
same lock as the descriptor list. Zero notifications can be distinguished from
an explicitly empty list, and later inspection code can require an update
after selection. The count alone does not prove which device caused an update.

The topology model now mirrors `SetControllerInfo`: fixed joystick/joystick/
keyboard choices for MSX/Spectrum, at least two player ports for NES, and the
nine standard choices for other advertised player ports. Internal controller
slots are tracked separately. In this revision, the standard list terminator
is placed at `nMaxPlayers`, hiding appended Mahjong keyboard entries; these
are not presented as implemented keyboard coverage merely because slots were
allocated. Actual advertised-list capture remains part of inspection work.

The isolated helper now captures `SET_CONTROLLER_INFO` into report fields,
preserving per-port order and device labels/IDs. Null means no notification;
an empty array means an explicitly terminated empty list. Traversal is bounded
to 16 ports, 64 choices per port and 1024-byte labels, with malformed capture
failing report construction. No helper was run. FBNeo content loading, option
resolution and post-selection inspection are still not connected.

Application-side selection validation now checks captured port counts and
ordered device identities against driver-derived topology, rejects duplicate
identities and invalid labels, and requires every requested device to be
advertised on its port. Display labels are retained rather than used as device
identity. Passing this check does not substitute for post-selection descriptor
refresh or core/content/options provenance; launch integration remains pending.

An owned option registry now parses FBNeo's legacy option definitions, whose
first listed value is the native default, and applies exact validated overrides.
Registration replaces definitions atomically while retaining previously
returned C strings. Effective-value export rejects overrides absent from the
final driver registration. Callback integration remains pending; no core has
been initialized and existing diagnostic option behavior is unchanged.

Step 115 adds an isolated option-environment adapter for legacy registration,
lookup, update queries and API-version negotiation. Native strings are copied
with explicit bounds; returned value pointers remain registry-owned. Callback
errors are retained and invalidate effective-option export even if the core
ignores a rejected registration. The adapter advertises version zero and treats
pre-initialization overrides as fixed, not as runtime edits. It is not yet wired
to an inspection executable or FBNeo launch. Existing diagnostics are unchanged;
only formatting was run, with builds, tests and core execution deferred.

Step 116 adds `inspect_content_controllers` to the helper library. It accepts
an exact expected core name/hash, real content, explicit sibling dependencies,
system files, options and requested devices. Files are copied into a disposable
directory with duplicate basenames rejected; save output has a private directory.
The report identifies the primary content and hashes every staged input. The
option adapter now owns callback state until after core teardown, while existing
diagnostics retain their previous defaults. Device choices are checked before
selection and after one neutral-input frame, which must emit a fresh descriptor
notification (FBNeo `InputMake` calls `RefreshControllers` before polling).
This does not prove that an advertised device was honored: application-side
driver/topology reconciliation remains required. Only full-path content is
accepted, and dependency discovery, nested system assets, helper CLI timeouts,
application consumption and actual launch integration remain to be implemented.
No native core, build or test was run in this step.

Step 117 adds the `lunchbox-libretro-content --request <absolute-json-path>`
binary. The request uses schema version 1 and requires `core`, `core_sha256`,
`core_name`, `content`, `content_dependencies`, `system_files`, `options` and
`devices`. Options are `{ "key": "...", "value": "..." }` records; devices
are `{ "port": 0, "device": 1 }` records. Lists make duplicate option keys and
ports detectable. Paths must be absolute UTF-8, the core hash must be a complete
SHA256, and unknown request fields are rejected. Input JSON is limited to 1 MiB.

The supervisor writes its validated request inside a private workspace and
starts a fresh worker, whose temporary directories also live under that root.
It enforces `--timeout-seconds` (1–300, default 30), kills/reaps a timed-out
worker before workspace cleanup, and reads a bounded result envelope only after
successful worker exit. Native stdout/stderr are discarded so core logging
cannot corrupt the report or fill pipes; normal Rust errors are carried in the
result envelope. A crash produces an explicit worker-exit error rather than a
partial success report. The final JSON report limit is 8 MiB. This is process
isolation and cleanup, not a security sandbox for untrusted native code.

The new command has not been built or executed. Application-side report
provenance validation, dependency discovery and actual FBNeo launch integration
remain pending; coverage counts are unchanged.

Step 118 adds strict typed report decoding and request-bound validation. Both
the supervisor and the application-facing FBNeo parser use it. It rehashes the
current core and every explicitly staged source, compares dependency names,
sizes and hashes without permitting extra/duplicate files, checks the primary
content's canonical basename, and requires exact requested options and devices.
The report now includes the descriptor counter before its refresh frame, which
must be smaller than the final counter. Controller choices and descriptors have
bounded counts and valid labels. The FBNeo parser further requires a selection
for every advertised port and exact source-derived topology agreement. This
detects mismatched/stale reports but does not authenticate a dishonest core,
eliminate file-change races after validation, or prove native device activation.
Automatic helper invocation and mapping-plan integration are not yet connected.
No build, test or core execution was performed.

Step 119 adds application-side `inspect_runtime`. Its inputs are an explicitly
resolved trusted helper, runtime environment, validated request, source-derived
topology, cancellation flag and bounded timeout. It invokes the worker protocol
directly so cancellation does not kill a supervisor while leaving its native
child running. The application owns the temporary root and kills/reaps the
worker before cleanup on error, timeout or cancellation. Native output is
discarded; the bounded result envelope must pass the complete report parser.
This function must run in a background launch task and is not yet invoked by
the actual launch flow. Runtime selection, request construction and dynamic
mapping-plan integration are still pending; no coverage was newly enabled.
No helper execution, build or test was performed.

Step 120 addresses a source-confirmed descriptor limitation. In
`GameInpAnalog2RetroInpAnalog`, analog-button pressure gets a joypad descriptor,
but `CinpJoyAxis` first queries analog index 2 and only then uses a digital
fallback. `GameInpDigital2RetroTouchEvent` emits no descriptor at all, while
`CinpTouch` queries pointer count. The helper now records distinct input-query
addresses and total calls during its single idle refresh frame, with limits of
4096 addresses and 65536 calls. Capture failures invalidate the report.

The application can construct mapping targets from the union of descriptor and
query addresses. Targets retain exact addresses, all shared labels, whether
they were queried, and transport encodings. Analog-pressure labels may reference
their source joypad descriptor without merging those independent addresses.
Query-only addresses remain unlabeled rather than acquiring invented game
actions. Unknown encodings remain explicit. Input-query presence does not prove
physical input behavior, and absence does not prove an input is unused: branches
requiring held inputs and touch-count action labels still need native contracts.
These targets are not yet connected to the launch/UI mapping flow. No native
core, build or test was run.

Step 121 adds `controller_launch::prepare_fbneo_inspection`, binding requests
to the prepared native RetroArch core/content paths and validated arguments.
It verifies the executable, rejects unresolved configuration roots/includes,
and reuses the existing effective option-file precedence (game, folder, core,
global). FBNeo's exact `fbneo-` namespace is selected from global files without
rejecting valid hyphenated option keys. Duplicate FBNeo keys are rejected. The
option baseline, validated report and typed targets are returned together for
the eventual per-launch mapping writer. Explicit dependencies, system assets,
library identity and driver topology still must be supplied by their resolvers;
this does not guess those facts or automatically enable an unproven profile.
The final launch/UI caller is not yet connected. No build, test or native-core
execution was performed.

Step 131 adds import of an existing request/report pair plus explicit application
context (`emulator_id`, helper, hardware/player/keyboard topology and per-port
physical controller IDs). It checks the report against current input files and
the supplied topology, then constructs expected files/options/descriptors/queries
directly from that validated observation. Assignments begin empty: nothing is
invented to make the draft saveable or launch-ready.

SettingsModel exposes a background import action and busy/status/draft properties.
Request/report/context text are bounded at 1 MiB/8 MiB/64 KiB. Hashing stays off
the UI thread; completion only publishes a draft, without replacing the editor,
staging settings or executing native code. The import-dialog controls remain to
be connected. No build, test, core or input-device execution was performed.

Step 132 connects the existing-report import dialog to the background action.
Separate request/report/context tabs explain the required topology and physical
controller identities. Inputs are read-only while validation runs. Successful
completion never overwrites the setup editor automatically: “Use imported draft”
requires unchanged import inputs and no existing un-staged editor changes. The
new draft then goes through assignment editing, staging and normal Save settings.
Import controller maps now reject duplicate numeric port keys instead of silently
keeping the last JSON value. No core is launched by report import.

Direct guided inspection and dependency discovery remain to be implemented;
this path consumes an already generated helper report. QML was not rendered or
executed, and no build, test or native execution was performed.

Step 133 connects explicit native inspection to the editor. SettingsModel checks
the bounded request/context, expected topology and physical-controller selection
before invoking the supervised helper on a background task. It uses the same
native report validator and unassigned-draft importer as the existing-report
path. A shared busy state prevents overlapping import/inspection jobs. Native
inspection exposes cancellation; the worker is reaped and its private workspace
cleaned before completion. A cancellation arriving while the completion callback
is queued prevents that draft from being accepted.

The UI confirms the exact helper/core/hash/content before native code runs and
explicitly distinguishes process isolation from a security sandbox. Closing its
inspection dialog requests cancellation. Successful inspection only publishes a
draft for the existing explicit “Use imported draft” action, assignment editing,
staging and Save settings. This advanced path still requires manually supplied
dependency and source-topology inputs; their discovery and remaining external
input adapters are incomplete. No native inspection, UI rendering, build or test
was run during this implementation step.

Step 122 adds exact RetroArch field translation from input addresses and a
per-port fragment writer for explicit physical assignments. Digital buttons,
axis halves, analog pressure and lightgun buttons retain distinct binding-part
identities. The writer uses current `JoydevMap` numbering, requires normalized
axis/pressure codes for continuous input, clears inherited keyboard/button/axis
fields in its owned pad/gun channels, and rejects duplicate or colliding output
assignments. It returns missing assignments and external-adapter targets rather
than silently dropping them or treating a partial fragment as launch-complete.

Field names are grounded in RetroArch revision
`69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`, `configuration.c`'s
`input_config_bind_map`. Its `input_driver.c` analog-button branch additionally
requires identity remap IDs; pressure fragments expose this as a requirement
for the session writer, not as a property already established by the fragment.
Absolute coordinates, relative mouse movement, keyboard keys and touch counts
still need dedicated input adapters. The final launch/UI caller and transport
ownership wiring remain incomplete. No build, test or core execution was run.

Step 123 adds `prepare_fbneo_player` for 64-bit Linux and exact source-control
IDs in the physical calibration. It validates current joydev numbering and
recorded evdev measurements, builds a gamepad frame using the existing measured
pressure/bipolar normalizers, retains the bridge in `PreparedFbneoPlayer`, and
renders against the resulting virtual device's numbering. It rechecks source
identity after bridge startup and exposes health checking to the session owner.
Axes cannot mix pressure and bipolar roles. Full axes require both measured
halves with consistent metadata/endpoints; polarity remains an explicit bind
property so inverted physical axes are preserved. Hats cannot become full
continuous sticks through this path.

The fragment writer now checks that a bipolar pair uses opposite gestures on
one physical axis. A pressure bind may cover its same-ID joypad fallback through
RetroArch's shared bind field; such aliases are reported separately rather than
requiring a second physical control or erasing the original input addresses.
The identity-remap requirement remains mandatory. The final launch/UI caller
does not yet own these prepared players, and pointer/keyboard/touch transport
is still pending. No virtual device was created and no build, test or core was
run during implementation.

Step 124 adds retained FBNeo session state to `CalibratedLaunch`. Its normal
health and pre-launch checks now cover prepared FBNeo player bridges, current
core/content/dependency hashes, the original base configuration and private
generated config hashes. `attach_fbneo_session` requires every inspected port
to have a complete prepared player, freezes the inspected effective options,
disables automatic remaps/overrides and remap application, sets six frontend
ports, disconnects unused ports and attaches the private config to a cloned
launch plan before replacing the caller's plan. Config files and transports
are retained with the launch object. Automatic state restoration is rejected.

The preparation path also checks that dependencies resolve beside the launched
content, supplied system assets match its configured system directory, and
canonicalization does not rename the content's driver basename. Nested assets
and renamed symlinks still need explicit handling. Pressure sessions remain
gated until identity-remap initialization is established; this owner does not
convert that requirement into an unsupported success claim. Missing coordinate,
keyboard and touch adapters remain explicit. The main UI/settings entry point
is not yet connected, and no profile was enabled. No build, test, native core
or virtual input device was executed.

Step 125 resolves the previously unconditional pressure-attachment gate using
the inspected RetroArch source initialization path. `configuration.c` calls
`input_remapping_set_defaults(false)` during defaults initialization;
`input_driver.c` initializes button remap IDs from `keybind->id`, axis IDs from
their index and port IDs from their port. `runloop.c` only loads automatic
remaps when `auto_remaps_enable` is true. The session already starts a fresh
process with a validated argument list and overrides/remaps disabled; it now
also uses a private empty remap directory and rejects wrappers by requiring a
direct ELF runtime. That executable's hash is retained and rechecked at launch.

Pressure fragments can therefore attach under these explicit initialization
conditions. Generated options set analog deadzone to zero and sensitivity to
one so the measured normalizer's range is not transformed a second time.
Generated device modes are checked against CLI overrides before attachment.
This is a source-grounded implementation, not a tested runtime result. The
UI/settings caller and external coordinate/keyboard/touch adapters still remain
unfinished, and no profile was enabled. No build, test or native execution ran.

Step 126 connects the previously separate preparation functions to
`prepare_with_cancellation` through explicit `controller_mapping.fbneo_launches`
settings. Each setup identifies the emulator, canonical core/content paths and
hashes, helper, dependencies, native topology, per-port devices, stable physical
controller IDs and source-control assignments. It retains the reviewed effective
options, descriptor groups, query addresses and complete input-file manifest.
Duplicate setups, ports, controllers and assignments are rejected; referenced
physical controls must have saved calibration.

On a matching native FBNeo launch, the application checks hashes, runs fresh
inspection, compares it with the reviewed contract, resolves each exact physical
controller, starts its owned transport and attaches the retained session. A
changed core/game/dependency/options/input contract requires renewed review.
No title matching or automatic per-game fallback is introduced. Missing setups
retain the existing unsupported-contract behavior. The settings/UI editor for
creating these reviewed setups is still pending, as are keyboard, pointing and
touch adapters. No catalog profile has been enabled and no runtime verification
has occurred; only source edits and formatting were performed.

Step 127 adds SettingsModel invokables for setup summaries, full setup retrieval,
validated staging and exact-key removal. Keys encode emulator/core/content as a
JSON tuple, avoiding ambiguous concatenation and unstable row-index identity.
An edit is parsed with an 8 MiB bound, validated with the whole cloned controller
mapping, and only then installed as staged settings. Persistence remains the
normal Save settings action. Removal does not touch game files or calibration.
These methods do not run native inspection or create input devices. The QML
editor still needs to be connected; no coverage count changed and no tests ran.

Step 128 registers `ControllerFbneoSetupDialog.qml` and exposes it from the
controller setup page. This advanced editor lists setup summaries, retrieves
full reviewed JSON, stages validated edits and confirms exact-key removal. It
keeps un-staged text when closed and blocks switching records until those text
edits are staged or discarded. UI copy distinguishes staging from the main
Save settings action and explains that changing compound identity creates a
separate setup. No inspection or input capture runs from this editor.

This is an advanced contract editor, not a completed guided setup wizard: it
does not discover dependencies, obtain native topology, produce the reviewed
contract or walk through physical assignments. Those flows and the remaining
external input adapters are still pending. QML was not executed or rendered;
no build, test or native inspection ran in this step.

Step 129 adds a settings-model assignment-review endpoint. It previews stored
descriptor/query records through the same target builder used by live reports,
without fabricating a runtime report. For each target it returns binding parts,
existing source assignments, calibrated physical-control choices and whether an
external adapter is required. Continuous choices require an analog physical
control with recorded native axis identity and measurement. Duplicate draft
target assignments are rejected. Incomplete assignments may be previewed so
the editor can fill them, but preview explicitly does not establish launch
readiness or replace staging/runtime validation. Guided QML controls remain to
be attached to these rows. No build, test or execution was performed.

Step 130 connects the assignment-review rows to an “Edit assignments” dialog.
Each target shows exact native identity, driver labels, controller identity and
calibrated source selectors for its required binding parts. Unknown/external
targets remain visible without a misleading pad selector. Pressure aliases and
missing saved calibration are explained explicitly. Selecting or clearing a
source edits only that exact address/part in the JSON draft; it does not stage
or persist settings automatically. Draft staging continues to use whole-mapping
validation, and launch still revalidates the native contract. The dialog retains
scroll position while rows refresh and closes with its parent editor.

Initial inspection and setup creation still require a completed workflow; this
editor operates on supplied reviewed input records. External input adapters and
runtime verification remain outstanding. No build, test, rendering or QML/native
execution was performed.

Step 134 adds explicit dependency-list editing in the inspection dialog. Content
dependencies and system files remain separate; primary content is not duplicated
in the content list. A request snapshot prevents edits from overwriting newer
request JSON, and an explicit apply action preserves all other request fields.
Unapplied list edits block inspection, report import and draft acceptance. Empty
lines are omitted, but path whitespace is retained; paths with line breaks must
use the JSON editor. The existing Rust request/staging validation remains the
authority for actual files, canonical basenames and collisions. This does not
discover archives, firmware, driver topology or input adapters. Source reviewed
only; no build, test or UI/native execution was performed.

Step 135 enables the legacy lightgun Pause address (device 4, index 0, ID 5).
The pinned RetroArch revision 69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576
maps both LIGHTGUN_PAUSE and LIGHTGUN_START to RARCH_LIGHTGUN_START in
input_driver_lightgun_id_convert (input/input_driver.c), whose configuration
field is gun_start (configuration.c). One assignment therefore satisfies both
addresses if both appear in the inspected target set. Explicit assignments to
both must resolve to identical native physical gestures; semantic channel
collision checks do not allow different button/axis suffixes to evade this.
Neither coordinates nor offscreen status are synthesized by this button route.
The editor exposes both original identities with a shared-channel explanation.
Formatting and source inspection only; runtime behavior remains unverified.

Step 136 neutralizes gamepad frontend hotkeys in the private session config:
all meta-bind button/axis fields from the pinned RetroArch configuration.c
input_config_bind_map, menu/quit gamepad combinations, and turbo enable. It
leaves keyboard shortcut fields and the user's base file unchanged. Without
this override, inherited frontend actions could consume the same normalized
controls that were assigned to game inputs. The setup dialog explains this
session policy. No runtime verification was performed.

Keyboard-adapter investigation: the same RetroArch revision loads keymapper
entries for 16 pad buttons plus 8 analog directions in configuration.c, then
input/input_driver.c evaluates them only for a keyboard-selected mapped port.
MAPPER_GET_KEY is shared key state rather than per-port state. Thus this route
does not represent arbitrary full keyboards or independent same-key keyboard
ports. It remains unimplemented, not an accepted general keyboard adapter.

Step 137 connects inspected system files to the actual launch. Each canonical
basename is copied with create-new semantics into the session's private system
directory; copied bytes and SHA256 must match the corresponding system/ entry
from the inspection report. Copies join the prelaunch hash checks and share the
session lifetime. An empty list deliberately produces an empty system directory.
The private config sets system_directory and systemfiles_in_content_dir=false:
the latter prevents RetroArch's GET_SYSTEM_DIRECTORY path (runloop.c at pinned
69a4f0ea) from replacing the explicit path with the content directory. Original
files and base configuration remain unchanged. This does not yet resolve the
separate content-directory dependency search contract. Source review and
formatting only; no builds, tests or native execution.

Step 138 also stages the primary content and explicit content dependencies.
All staged copies use the same canonical basename/group contract and match
the inspection report's byte count and SHA256, with bounded copying. The exact
original content argument must occur once; its replacement and prepared-content
metadata point to the session copy. Original identity remains in the inspection
for provenance checks. Dependencies no longer need to share the original folder,
and system files no longer need to match the global RetroArch system directory.

Before substitution, savefile and savestate destinations are resolved using the
original content parent, explicit settings, and content-then-core sorting from
RetroArch runloop_path_set_redirect. The private config pins those destinations
and disables further sorting/content-directory redirection. Destinations must
already exist; their canonical identities are rechecked before launch. Unknown
platform defaults and missing directories are reported rather than guessed or
created. This preserves persistent save routing without changing original files.
This source-only implementation still requires runtime validation; no builds or
tests were run. Keyboard, relative/absolute pointer and touch adapters remain
unfinished, and unresolved platform save defaults still require implementation.

Step 139 supersedes step 138's explicit-six-booleans restriction. RetroArch
69a4f0ea config.def.h defines both per-core sorting defaults as true and both
per-content sorting/content-directory defaults as false, without platform
conditionals. The resolver now uses those defaults only when flags are omitted;
explicit configuration still wins and malformed booleans still fail. Save/state
directory defaults are a separate platform-dependent contract and remain
unresolved. Source inspection and formatting only; no builds or tests.

Step 140 resolves directory defaults for the existing native desktop Linux
contract. Pinned platform_unix.c uses XDG_CONFIG_HOME/retroarch or
HOME/.config/retroarch as its base and appends saves/states for the default
directories. This matches the root already resolved by retroarch_base; custom
HOME/XDG overrides and non-native runtimes are rejected earlier. Missing,
empty and default directory settings use these paths before applying sorting.
An explicit nonexistent directory is rejected with a targeted diagnostic:
configuration.c ignores invalid directories, so honoring their text would not
preserve native behavior. Final persistent destinations must still exist.
Special CWD-based/embedded builds are not covered by this desktop contract.
Source review and formatting only; no build, test or native execution.

Step 141 exposes saved valid Linux calibration IDs in the import context UI,
with layout labels and exact zero-based port assignment. This is a read-only
settings inventory, not live device enumeration. Choosing an ID updates the
context draft and invalidates previous imported-result acceptance through the
existing text-change guard. Duplicate physical IDs across ports are rejected.
Connection, calibration suitability and requested topology are still checked
by the existing import/launch validation. No builds, tests or UI execution.

Step 142 adds a Devices tab backed by controller_topology, not a duplicated
QML device table. It uses the supplied hardware/player counts and preserves
each exact native port/device identity. Selecting a device updates just that
port in the request's device array and invalidates prior import acceptance.
Stale application context disables selection until refreshed; pending dependency
list edits also disable it. Unsupported external adapters remain labeled as
unfinished rather than hidden. The physical-controller port selector is bounded
to 0..5 to match FRONTEND_PORTS. This does not discover topology, execute a core,
or prove the advertised contract matches a particular binary. Source review
and formatting only; no build, test or UI execution.

Step 143 passes the existing launch cancellation token through session staging.
The bounded manifest copy checks it between 64-KiB chunks, before each file and
around hashing; a final check precedes plan replacement. Error unwinding drops
the temporary directory and prepared transports, while the original launch plan
is not mutated until success. Individual file-hash calls remain synchronous;
this does not promise immediate cancellation during a hash or blocked file I/O.
No builds, tests or runtime execution were performed.

Step 144 implements Arcade Gun's analog-backed absolute coordinates. In pinned
FBNeo retro_input.cpp, GIT_DIRECT_COORD advertises RETRO_DEVICE_ANALOG for
RETROARCADE_GUN while CinpDirectCoord queries device 1029. Pinned RetroArch
input/input_driver.c masks device with RETRO_DEVICE_MASK before dispatch, so
these reads share left analog X/Y channels. Subclass coordinates retain their
absolute-coordinate encoding but expose negative/positive measured axis parts.
Either native address can supply each part; duplicate alias assignments must
resolve to the same gesture, and pairs are validated after canonicalizing the
alias so split-address halves cannot bypass same-axis checks. Normalized bipolar
transport supplies -32767..32767, not a synthesized offscreen sentinel. The UI
describes this as absolute stick positioning, not physical lightgun calibration
or relative mouse movement. No builds, tests or runtime execution.

Step 145 source analysis fixes the Mouse Ball/Full Mouse adapter boundary.
Both RETROMOUSE_BALL (773) and RETROMOUSE_FULL (514) take GIT_MOUSEAXIS
in retro_input.cpp's generic pointer logic (870..896). That constructor
advertises RETRO_DEVICE_MOUSE (660), and CinpMouseAxis polls device 2, index 0
(595), regardless of the selected subclass. Full Mouse also polls native mouse
buttons; Mouse Ball retains other driver-defined buttons. These addresses must
not be aliased to analog axes merely because 773 has ANALOG as its base device.

The existing GamepadFrame/GamepadBridge intentionally admits only gamepad keys
and absolute axes, not EV_REL or mouse buttons. A new relative-input path must
preserve deltas, accumulate only between frontend polls, handle synchronization
loss without replaying stale movement, retain exact physical source identity,
and resolve RetroArch's actual per-port mouse selection. A gamepad-to-relative
velocity mode, if offered, must be an explicit distinct transform with defined
time units, not implicit reinterpretation of absolute samples. No adapter was
added in this analysis step; no builds, tests or native execution were performed.

Step 146 adds `controller_axis::relative::RelativeReader` as capture
infrastructure, not an enabled FBNeo adapter. It accepts an explicit event node
and canonical physical identity, validates only selected relative X/Y and mouse
buttons, and preserves complete kernel reports. Deltas use checked i64 addition
in physical counts. Initialization drains queued history before snapshotting
buttons. Synchronization loss stops the session instead of fabricating lost
motion; the future transport owner must publish `neutral()` on errors/teardown.
There is no device grab, permission change, discovery by fuzzy name, velocity
conversion, or emulator launch integration. Per-frontend-poll accumulation,
virtual mouse publication and actual RetroArch per-port mouse-index resolution
remain to be connected. No tests, builds or native execution were performed.

Step 147 implements `VirtualRelativeMouse` and `RelativeMouseSession` in the
same module. The session owns source capture and uinput output; a bounded pump
forwards each complete source packet once. The publisher validates selected
controls and i32 kernel delta range before writing a SYN_REPORT-terminated
packet, and destroys its owned device after any publication failure. Buttons
are released on source failure and shutdown. UI_GET_SYSNAME identifies the
owned device without guessing a frontend index. REL_X/REL_Y/BTN_LEFT are
declared for mouse classification, but unselected controls cannot receive
movement or presses. No physical grab or desktop suppression is performed.
Consequently this is still unwired transport infrastructure: actual RetroArch
per-port mouse enumeration and desktop-pointer interaction policy must be
resolved before enabling launch publication. No live virtual device was created
during implementation; only formatting and source inspection were performed.

Step 148 adds owned endpoint resolution to the publisher and session. It walks
only the inputN identity returned by UI_GET_SYSNAME, requires a single evdev
child, and verifies that the opened character-device descriptor still resolves
to that exact input identity. A not-yet-published node returns not-ready, not an
invented index. This is a point-in-time identity receipt, not a guarantee that a
frontend subsequently opens the same node.

The inspected RetroArch checkout is
`69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`:
`input/drivers/udev_input.c` 678..695 selects
`settings->uints.input_mouse_index[port]` through `udev->pointers`;
3340..3420 rebuilds that indirection on hotplug; 4020..4064 enumerates udev
properties and admits readable devices, and initialization opens keyboard,
mouse, touchpad, then optional touchscreen categories. `configuration.c`
4008..4009 loads the per-player mouse setting as an integer. Consequently a
sysfs event number, display-name match or launcher's independently sorted device
list does not prove the frontend's mouse index. This rules out directly writing
an inferred index as a completed route. Stable frontend routing and desktop
pointer interaction are still unresolved; no runtime checks were performed.
# Relative wheel publication boundary

Pinned RetroArch `69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`,
`input/drivers/udev_input.c::udev_handle_mouse`, recognizes conventional
REL_WHEEL/REL_HWHEEL only at exactly +1/-1. Larger totals are ignored. The
relative-device bridge now serializes wheel totals as bounded unit detents with
individual SYN_REPORT boundaries, validating the entire report before writing.
Oversized reports fail and shut down the owned output rather than truncate.

This is not full wheel fidelity: opposite wheel events within a captured report
are still net-accumulated, and frontend boolean flags may coalesce repeated
detents between polls. Source event ordering and frontend poll coordination remain
required before wheel targets can be marked supported. No device or emulator was
opened to verify this implementation.
# Internal keyboard remap implementation, 2026-09-08

Physical-planning follow-up: explicit key-to-channel bindings now translate to
internal frontend requirements and reuse the existing physical calibration review.
Native descriptors/queries remain untouched. Duplicate native assignments,
wrong-port inputs and conflicting keyboard/ordinary owners of the same channel
are rejected. Analog channels retain bipolar calibration/pairing requirements;
they are not treated as arbitrary digital buttons. The review returns a remap
preview and explicitly false launch readiness. Settings/UI and isolated runtime
remap loading are not connected yet; no support percentage is increased.

Follow-up: pinned `input_driver.c` lines 1557–1574 confirms that polled keyboard
state includes mapper keys, not only keyboard callbacks. The remap renderer now
requires an explicit selected keyboard port, preserves other supplied native
device selections, disables unused ports, writes identity button/axis/port remaps,
disables analog-D-pad conversion and turbo, and rejects multiple keyboard owners.
The private path helper follows the core-library/core-library.rmp fallback in
`configuration.c` (controller-name sorting must be disabled). These are launch
integration prerequisites, not enabled launch support. No remap was executed.

`controller_fbneo/keyboard.rs` now validates exact inspected keyboard targets
against independently selected frontend channels and renders remap-file key
assignments. It clears other key maps within FBNeo's six-port range, rejects
unknown key IDs, duplicate channels and incomplete target coverage, and selects
the keyboard device explicitly. The 24-channel limit is a real frontend mapper
capacity, not complete keyboard coverage. No source measurement, UI or launch
loading is connected to this renderer yet.

Pinned RetroArch `69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`:
`configuration.c::input_remapping_save_file` writes `input_playerN_key_SUFFIX`;
`input/input_driver.c` lines 5823–5875 applies the keyboard mapper using shared
key state. Therefore this is a remap-file contract, not ordinary append-config,
and independent simultaneous keyboard ports cannot be assumed. Isolated remap
loading and keyboard-hotkey behavior still require integration. No uinput
keyboard or desktop key injection was added. Source/format/whitespace only;
no tests, builds, remap generation execution or runtime probes.
