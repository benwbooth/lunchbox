# Controller coverage progress

## Current implementation checkpoint

Scope correction, 2026-09-08: user requires RetroArch completion before standalone
work. Auditing controller_coverage.rs found the historical 95-core denominator
included BizHawk-only Nymashock even though its row correctly identified it as
non-RetroArch. RetroArch denominators now exclude BizHawk-only entries, dynamic
MAME/FBNeo adapters are listed separately, and the misleading hardcoded standalone
adapter zero is replaced with an explicit unaudited state. This closes no new
mapping contract and must not be reported as increased implementation coverage.
The remaining RetroArch queue is MAME, FBNeo and Steem SSE; the latter needs a
compatible runtime because the inspected core is Windows-only. Nymashock belongs
to the later native/BizHawk phase. See CONTROLLER_COMPLETION_ORDER.md. No tests,
builds, database report execution or device access; formatting/whitespace only.

Step 604: added explicit absolute capture controls to the advanced calibration
dialog: exact device/axis selection, 50 ms polling, named edge confirmations,
finish-to-review draft and separate add-to-JSON action. Existing device records
are not silently overwritten. Close, destruction and application focus loss
cancel capture; unreadable responses attempt native cancellation. The UI labels
runtime behavior unverified and does not infer viewport or perspective geometry.
Stationary initialization, device discovery and emulator routing remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds, UI runs
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 603: exposed owned absolute capture through a bounded, strict settings-model
command API: explicit start, poll, edge confirmation, finish-to-draft and cancel.
Responses report active state, sample age and recorded edges; failed transport
sessions are removed, and unsupported platforms return an explicit error. No
command persists settings. UI polling/cancellation wiring remains required before
exposing capture controls; stationary initialization and routing remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 602: connected timed absolute capture to four-edge acquisition in one owned
session. Polling preserves sample age, explicit edge confirmation enforces freshness,
and transport failure/cancellation clears all measurements and closes the source.
A two-minute deadline is checked on polling; callers must poll regularly or drop
the session. Finish consumes capture into a validated review draft without saving.
Capture-controller/UI wiring, stationary initialization and emulator routing remain.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 601: absolute capture now selects the per-client monotonic evdev timestamp
clock and offers timed reports preserving kernel sample age. A fixed conservative
clock anchor maps reports to Instant; malformed, future and backwards output
timestamps fail the session. Untimed polling delegates to the same validated path.
Linux v6.12 input.h confirms EVIOCSCLOCKID's UAPI definition. Capture-controller
integration, UI, stationary initialization and emulator routing remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 600: added explicit four-edge absolute calibration acquisition with immutable
axis/swap selection, physical-bound checks, required newer reports, a 500 ms
sample-age limit and invalidation that clears all measurements. Completion requires
all four edges and nonzero extents; it returns calibration for review, not saved
settings. Freshness depends on actual monotonic sample timestamps supplied by the
future capture controller, not event dequeue times. Timestamp plumbing, capture
UI, stationary initialization, perspective and emulator routing remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 599: separated first-time absolute capture from saved calibration. The raw
reader can now open explicitly selected physical axes, obtain and retain their
actual kernel bounds, and expose those bounds for calibration acquisition without
inventing screen edges. Saved-calibration opening uses the same identity-bound
path and still rejects any mismatch against recorded bounds. No device discovery,
capture UI or initial stationary-state acquisition is supplied by this step.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 598: added a Linux64 scalar absolute reader using the existing opened-device
identity checks, selected-axis capability checks and live min/max revalidation
before and after each bounded poll. It starts at a report boundary, rejects lost
synchronization and closes on error without returning a partial batch. Separately
queried current axis values are not treated as an atomic position; both axes must
appear in the stream, so initial stationary-state acquisition remains unfinished.
No capture UI, button/contact protocol or emulator routing is enabled yet.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 597: added bounded scalar absolute report assembly. X/Y retain their last
reported positions and publish only at SYN_REPORT after both axes have been
observed; there is no guessed center or relative-delta integration. Synchronization
loss, oversized reports and out-of-bounds samples permanently invalidate pending
state. The future reader must discard failed poll batches; initial device-state
acquisition, hardware opening, buttons/contact protocols and routing remain.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds
or hardware access. Uncommitted/unreviewed; wheels paused.

Step 596: wired an advanced absolute-device records dialog into controller
settings. It stages validated JSON lists through the existing settings API and
provides a manual signed-32-bit sample preview with a calibrated-coordinate
rectangle, textual values and clamping status. Editing inputs clears stale
previews. Capture, acquisition and emulator routing remain unfinished; the UI
explicitly separates calibrated-area status from hardware offscreen semantics.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/format/whitespace inspection only; no tests, builds,
UI runs or preview execution. Uncommitted/unreviewed; wheels paused.

Step 595: exposed absolute-device records through the settings model with bounded,
validated whole-list staging and a read-only sample projection API. Invalid lists
leave staged settings untouched; empty lists support explicit removal. Preview
returns calibrated unsigned coordinates or an error and explicitly distinguishes
outside-calibrated-area status from hardware offscreen/reload semantics. No device
is opened and no runtime adapter is enabled by these APIs. UI wiring remains.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or preview execution. Uncommitted/unreviewed; wheels paused.

Step 594: added optional absolute-device storage with exact event/sysfs paths,
distinct scalar axes, recorded min/max bounds and rectangular calibration.
Validation rejects duplicate identities, zero extents, multitouch-slot axes and
calibrated edges outside their corresponding bounds, accounting for swaps.
Projection rejects out-of-hardware-range samples rather than guessing sentinel
offscreen semantics. Existing settings default to no absolute devices. Capture,
calibration acquisition, buttons/offscreen protocols and routing remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 593: added a separate rectangular absolute-position calibration transform
with explicit edge readings, axis swapping, reversed ranges, zero-span rejection,
bounded unsigned output and retained outside-calibrated-area status. Integer
arithmetic covers full i32 raw ranges without stick dead zones or motion integration.
Pinned frontend source confirms absolute inputs use a separate bounds/viewport
path, so relative transport is not treated as gun capture. This transform is a
foundation only: no capture, settings acquisition, perspective correction,
offscreen/reload contract or runtime adapter is enabled by it.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 592: FBNeo prepared relative targets now open a source-to-destination
schematic showing their actual saved physical axis/button, swap/gain/inversion
and native callback identity. The shared relative view accepts explicit generic
display metadata without inventing MAME fields. Destination-only diagrams remain
for unselected/unsupported inputs. Selected relative rows now retain the diagram
control ID needed to open this view. Catalog stays 91/95 cores (95.8%),
289 profiles, 157 layouts; overall/MAME completion unknown. Formatting/source/
whitespace inspection only; no tests, builds or UI runs. Uncommitted/unreviewed;
wheels paused. No new device silhouettes or runtime coverage are claimed.

Step 591: FBNeo prepared relative rows now expose their exact restricted source
settings and no longer claim an unimplemented external adapter. The assignment
view shows source path/identity and required output, keeps platform failures in
Needs attention, and labels runtime behavior unverified. Unselected/unsupported
targets retain external-adapter diagnostics. The global launch switch now names
saved controller mappings, with accessibility text covering relative-only setups.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or UI runs. Uncommitted/unreviewed; wheels paused.

Step 590: fixed an outer launch bypass that returned before saved MAME/FBNeo
setup handling whenever the gamepad calibration registry was empty. Relative-only
setups now reach their full saved/native/source validation; later pad-only paths
retain the old empty-registry behavior. Ordinary frontend spawn also rejects a
MAME/FBNeo session that requires relative input but has no pending startup, rather
than falling through when bootstrap is missing or consumed. Calibrated-launch
opt-in remains required. Catalog stays 91/95 cores (95.8%), 289 profiles,
157 layouts; overall/MAME completion unknown. Formatting/source/whitespace
inspection only; no tests, builds or runtime checks. Uncommitted/unreviewed;
wheels paused.

Step 589: the temporary FBNeo relative launch/attachment gates are removed now
that saved selection, fresh contract binding, mixed/relative-only composition,
private mouse isolation, endpoint preparation, owned spawn/routing, readiness and
health/teardown paths are connected in source. Review/UI report actual platform
and preparation requirements instead of a blanket unfinished flag. The opt-in
frontend extension, exact source identities, route confirmation and focus checks
remain mandatory. This is unbuilt, unverified integration, not proven gameplay.
Unsupported relative conversions, keyboard/gun capture and wider per-game coverage
remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion unknown. Formatting/source/whitespace inspection only;
no tests, builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 588: pinned RetroArch configuration source confirms that explicit mouse
button binding values clear the inherited binding before parsing. FBNeo's private
config now clears hotkey _mbtn fields alongside gamepad buttons/axes, clears mouse
fallbacks for rendered pad controls and empty ports, and disables mouse/pointer
menu navigation when relative sources are selected. Keyboard shortcuts remain
unchanged; raw game mouse callback routes are not replaced by pad bindings.
Relative gates remain during the remaining isolation audit. Catalog stays 91/95
cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown.
Formatting/source/whitespace inspection only; no tests, builds or runtime checks.
Uncommitted/unreviewed; wheels paused.

Step 587: FBNeo pending relative groups now have owned frontend spawn/stdio
handoff, exact prepared-plan checks, child executable/hash verification and the
shared command bridge. Handoff failure kills/waits for the child and drops owned
devices. Readiness waits for routing confirmation; runtime health observes bridge
failure, and bridge teardown precedes remaining session inputs. Consumed startup
cannot fall through to an ordinary spawn. Both relative launch gates remain until
input-isolation review is finished. Catalog stays 91/95 cores (95.8%), 289 profiles,
157 layouts; overall/MAME completion unknown. Formatting/source/whitespace
inspection only; no tests, builds, device capture or runtime checks.
Uncommitted/unreviewed; wheels paused.

Step 586: FBNeo session preparation now retains its exact final launch plan and
can own prepared relative endpoints plus a topology guard. The source path checks
private inputs, waits within a bounded/cancellable endpoint-readiness loop, stages
the shared disabled-mouse bootstrap with owned-command settings, hashes that file
and retains the exact bootstrapped plan. Sources remain unpublished pending a
routing handshake. Saved launch dispatch is wired after session attachment, but
both existing relative gates remain until spawn/handoff is implemented. No source
was opened during this turn. Catalog stays 91/95 cores (95.8%), 289 profiles,
157 layouts; overall/MAME completion unknown. Formatting/source/whitespace
inspection only; no tests, builds or runtime checks. Uncommitted/unreviewed;
wheels paused.

Step 585: FBNeo launch fragment analysis now excludes only fresh-bound relative
mouse targets from gamepad requirements. Preparation skips gamepads for validated
relative-only ports. Session coverage checks the union of prepared gamepad and
relative ports against inspected ports, requiring every remaining target to have
a prepared gamepad. Empty gamepad fragments preserve the inspected native device
selection instead of disabling a relative-only port. Both outer launch and
attachment gates remain until owned relative transport/routing is connected.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 584: FBNeo fresh launch inspection now has private retained relative-source
settings and a binding step that validates the saved setup against the fresh
core/options/files/descriptors/queries. It rebuilds required mouse addresses from
the fresh target set, restricts devices again, checks duplicate identities and
requires exact agreement with saved preparation. Saved launch invokes this binding
step, while its outer relative-launch gate remains. Session attachment also
explicitly rejects retained relative sources until owned routing is implemented.
No source is opened by binding. Catalog stays 91/95 cores (95.8%), 289 profiles,
157 layouts; overall/MAME completion unknown. Formatting/source/whitespace
inspection only; no tests, builds or runtime checks. Uncommitted/unreviewed;
wheels paused. Mixed fragment generation and transport attachment remain work.

Step 583: FBNeo mixed-port review/staging now separates fully validated selected
mouse targets from gamepad requirements. Complete inspected records remain intact;
unselected mouse, unsupported conversions and other external targets are not
hidden. Invalid source preparation keeps the full target set in gamepad review
and reports its error. Selected mouse rows show pending relative ownership even
on mixed ports, while remaining gamepad parts retain calibration validation.
This enables source-level mixed setup composition, not runtime relative gameplay;
the saved-launch gate remains. Catalog stays 91/95 cores (95.8%), 289 profiles,
157 layouts; overall/MAME completion unknown. Formatting/source/whitespace
inspection only; no tests, builds or runtime checks. Uncommitted/unreviewed;
wheels paused. Fresh launch composition and owned routing remain unfinished.

Step 582: the FBNeo relative-source editor now offers explicit gamepad omission
for eligible relative-only ports. It requires complete backend-reported relative
coverage and an empty gamepad-assignment list, rechecks the candidate, and changes
only controller_id. Native device/port selections, sources and other mappings
remain intact. Existing Replace controller restores a gamepad selection; removing
the only relative source is clearly described as leaving an incomplete draft.
Review now describes relative-only ports without meaningless gamepad counts.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused. FBNeo relative launch remains blocked.

Step 581: FBNeo storage and staging now allow a port without a gamepad only when
its complete nonempty inspected surface is covered by its prepared relative source
and it retains no gamepad assignments. Mixed, unknown and uncovered inputs cannot
use this exemption. Review reports these ports without inventing calibration or
gamepad coverage, and keeps their runtime adapter requirement explicit. Relative
launch remains blocked; UI gamepad-removal controls and runtime composition remain
unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion unknown. Formatting/source/whitespace inspection only;
no tests, builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 580: the FBNeo setup UI now offers a relative-source draft editor with
per-port prepared candidates, full saved device settings and explicit use/remove
actions. It preserves other mappings, rejects stale text/controller revisions,
validates additions through backend review and retains orphaned selections for
removal. Assignment review reports selected sources and preparation errors rather
than always claiming none are selected. No settings or devices were changed by
this implementation; relative launch and relative-only player setup remain work.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 579: FBNeo setup storage now accepts optional explicit relative-source
selections by zero-based inspected port. Validation prepares each selected port's
complete mouse requirements, rejects duplicate ports and device paths/identities,
and translates to one-based transport player keys without inferring mouse indices.
Review exposes selections, restricted preparations and preparation errors. Imported
setups start with no selection; existing JSON remains compatible. Launch explicitly
rejects nonempty selections until owned transport integration is connected.
Relative-only player setup and selection UI remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 578: FBNeo relative-port review now prepares each candidate saved device
against a bounded, single-port inspected address set. Preparation preserves
calibration, axis swap and explicit button remaps while retaining only required
physical axes/button outputs. Unsupported conversions and missing capabilities
reject the candidate. Reviews expose these restricted settings alongside the
existing candidate paths; no device is selected or opened and FBNeo relative
launch remains unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles,
157 layouts; overall/MAME completion unknown. Formatting/source/whitespace
inspection only; no tests, builds or runtime checks. Uncommitted/unreviewed;
wheels paused. Saved selection, fresh launch composition and owned routing are
still needed for this FBNeo input path.

Step 577: generated MAME fixed/twin, combined-analog and axis-switch panels now
use a full grid outline that contains their lower control rows. Their SVG caption
identifies logical frontend channels rather than claiming rear-control placement
or cabinet geometry. Control IDs, coordinates and assignments are unchanged; this
corrects presentation of 11 existing layouts, not additional mapping contracts.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds, SVG exports or UI runs. Uncommitted/unreviewed; wheels paused. Exact
visual fit and runtime mapping remain unverified.

Step 576: MAME's explicit digital channel picker now identifies other resolved
switch routes on each candidate channel before preview. It excludes only the
exact field/sequence being edited and lists shared action labels, native field
identities and default/explicit provenance. Unknown route evidence stays unknown;
no-other-switch-routes is explicitly not an analog-ownership or independent-input
capacity guarantee. Existing composition and physical validation remain required.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused. Excess native actions still require
explicit usable routes; this improves conflict-aware assignment, not channel count.

Step 575: the shared controller comparison now stacks source above destination
below 700 px and retains side-by-side diagrams at wider widths. Each diagram
owns its wrapping title; both image and hotspot geometry preserve the catalog
aspect ratio. Connection endpoints include the title offset and stacked curves
run vertically. Deferred repaints follow resizing. This improves the existing
arcade and other layout views, not controller compatibility counts.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused. Responsive visual behavior remains
unverified until the deferred UI verification phase.

Step 574: the shared standard-controller mapping view now reports drawable
assignment coverage separately from source-assignment counts. Missing or ambiguous
geometry is explicit in the route selector and selected-route warning, with a
next-undrawn action; it never silently removes a mapping. Hardware-repeat source
controls now resolve to their owning base position for both connection lines and
diagram highlighting, matching the existing shared-input selection semantics.
This applies to MAME arcade review and other users of the shared view, not new
controller contracts. Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion unknown. Source/whitespace inspection only; no tests,
builds or UI runs. Uncommitted/unreviewed; wheels paused. Geometry gaps still need
actual per-layout implementation where reported; runtime verification is deferred.

Step 573: MAME review now resolves each mouse-button assignment back to its exact
saved physical evdev button and exposes those routes alongside axis mappings.
The shared source/destination view highlights the selected physical button and
native output 1–5, retains device/field identities and opens the selected button
in the combined editor. Schematics explicitly distinguish logical identities
from physical placement and verified capabilities; axis-only discard wording is
updated for combined mappings. No device silhouettes or new catalog layouts are
claimed. Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion unknown. Formatting/source/whitespace inspection only;
no tests, builds or UI runs. Uncommitted/unreviewed; wheels paused. Full
controller-layout visual coverage and end-to-end verification remain unfinished.

Step 572: source inspection confirmed startup button snapshots, exact remapping,
failure/teardown releases and owned frontend routing handoff. The blanket saved
mouse-button staging/launch gate is replaced with the actual 64-bit Linux and
inspected opt-in core-mode requirements. Preview/review pending flags and UI text
now distinguish missing requirements from unverified runtime behavior. Fresh
inspection, private UI-binding validation and live routing guards remain in place.
This connects the opt-in launch path in source; no patched runtime was built or
used and mouse-button gameplay is not verified.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused. Remaining batch
work includes button destination visuals and deferred end-to-end verification.

Step 571: saved MAME staging now composes button/axis mappings and uses the
mouse-pruned gamepad profile for calibration requirements. Saved launch platform
checks and relative transport dispatch now include button-only sources, not just
axis-bearing ports. The outer button staging/launch gate is retained while the
remaining lifecycle paths are inspected; this is source wiring, not enabled or
verified physical mouse-button gameplay.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 570: physical MAME preparation now accounts for retained button-source
ports and derives gamepad profiles without mouse-owned fields. Relative source
preparation accepts combined axis/button permissions and rechecks final inputs
and UI configuration trees before device opening. The saved-settings outer
staging/launch gate remains unchanged, so normal button launch is still disabled.
No device was opened; this is unverified source integration.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 569: the bounded mouse UI-binding tree validator is now shared by fresh
inspection and final prepared-input verification. Button-bearing final trees are
checked after staging and on pre-launch input verification, including newly
added configuration files as well as hash-checked originals. The native mode
declaration is rechecked against retained inspection. This is not an atomic
filesystem lock; physical button preparation and launch remain gated.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 568: button-bearing fresh inspection now checks the complete private cfg
and ctrlr trees, including global/default configurations, for saved mouse UI
bindings. It requires the declared core mode, rejects symlinks/special entries,
honors cancellation and bounds entries/per-file/total bytes. Errors identify the
configuration file. This covers the inspected tree, not later launch-tree changes;
physical preparation and final launch integration remain gated.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 567: fresh MAME launch inspection now accepts and retains explicit
mouse-button assignments, requires mouse inspection and the native mode
declaration, and composes buttons/axes against fresh fields and original
controller/game configurations. Automatic gamepad allocation cannot acquire
button sources. Physical preparation explicitly rejects retained button mappings
until that path is connected; outer staging/launch guards remain intact.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 566: button-bearing staging validation now requires the inspected core's
game-mouse mode declaration before the remaining integration gate. Combined
previews expose missing-mode diagnostics, and review distinguishes a saved mode
declaration from absent/stock evidence without claiming runtime verification.
Axis-only validation does not require the opt-in mode. Staging/launch remain
blocked for buttons; saved UI files and fresh startup integration remain work.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 565: the opt-in core patch now declares its game-mouse mode in native mouse
device names while preserving RetroMouse routing IDs. Inspection captures an
optional bounded ID/name map, and a guard requires all eight exact V1 markers
plus valid mouse routes. Missing/stock names fail the mode guard. This is a
trusted core's declaration, not behavioral or cryptographic attestation; it must
remain tied to the inspected core identity. The guard is not yet wired to launch.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds, patch application or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 564: the combined editor can create/replace mouse-button drafts from exact
inspected controller/auxiliary switch fields, explicit source ports/devices and
outputs 1–5. Shared axis ports require the same device configuration; button-only
ports retain explicit per-port sources. Native composition still rejects field
conflicts and unsupported source outputs before applying. The editor is reachable
after mouse-enabled inspection even without an existing relative assignment.
Staging/launch remain blocked for button-bearing drafts; no clicks enabled.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 563: the combined draft editor can change an existing mouse mapping's
output button (1–5) or explicitly remove it, retaining exact field/player identity
and invalidating prior previews. Native-only axis previews reject pending button
edits instead of silently previewing the old list. Full source validation remains
required; removing the last button releases its source only if no axes use it.
Creating new button mappings and runtime integration remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 562: axis editing now preserves stored mouse-button assignments and keeps
exact button-only sources when axis rows are removed. Shared axis/button ports
use one explicitly selected source, with the UI explaining that replacement
affects both. Preview submits the complete combined source set and buttons.
The player editor also recognizes button-only relative ports. Pending button
warnings and staging/launch guards remain; no click forwarding enabled.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 561: combined axis/button previews can now return a structurally validated
reviewable draft. Omitted/null button lists preserve stored assignments; an
explicit list replaces them and an empty list clears them. Source validation
requires the full combined set. Native-only previews remain non-applicable, and
button-bearing drafts retain explicit staging/launch blocks. No settings saved
or click capture enabled; combined editor source handling remains to finish.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 560: read-only MAME review now accepts stored mouse-button assignments,
uses the combined native planner for field counts and excludes mouse-owned
fields from gamepad profiles/routes. Pending button routes are visible with an
explicit staging-blocked warning; physical readiness remains false. The existing
axis-only replacement API rejects button-bearing setups rather than silently
dropping their assignments. Staging/launch and UI save remain gated.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 559: MAME stored settings now serialize an optional exact
`relative_button_assignments` list, defaulting empty for older setups. Stored
validation composes native button/axis ownership, validates combined source
capabilities and excludes mouse-owned fields from gamepad requirements. Current
axis-only review, staging and launch explicitly reject pending button setups
rather than omit their mappings. UI save/review integration remains unfinished;
no user settings were changed and no click forwarding was enabled.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 558: mouse-owned field removal is now shared by native composition and
gamepad profile previews. The derived snapshot validates exact field identity,
preserves original inspection evidence and removes matching auxiliary records.
Relative previews report each selected port's remaining gamepad requirement,
selected-gamepad status and profile errors instead of treating mouse-owned
actions as gamepad channels. Button persistence/launch remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 557: mouse-button planning now rejects supplied saved controller/game XML
with mouse sequences on UI ports, reporting the action without modifying user
bindings. The bounded parser inspects escaped text and CDATA across all system
blocks conservatively, reusing the common XML structural validator. This is a
file-level guard, not complete runtime isolation: launch still must supply every
effective persistent file and establish the patched core's actual behavior.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 556: source inspection found that explicit digital, relative-axis and
mouse-button generators omitted the exact XML document prefix required by the
shared controller configuration merger. All three now emit the canonical
declaration/root envelope and closing layout, matching the analog generator.
Field routes and ownership are unchanged. This repairs a source-level rejection
path affecting ordinary as well as relative mapping composition; behavior has
not been built or tested. Saved mouse UI-binding isolation remains unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 555: added the opt-in Linux `mame-game-mouse-only` package, pinned to the
native contract revision and a directly hashed source archive. Its recipe enables
the game-only mouse macro and installs only the core library, avoiding the
inherited stock-frontend wrapper. Default runtime selection is unchanged. Package
evaluation/build, patch application, saved UI isolation and loaded-core evidence
remain outstanding; no click forwarding enabled.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/hash/whitespace inspection only; no tests, builds or
runtime checks. Uncommitted/unreviewed; wheels paused.

Step 554: added an opt-in pinned MAME source patch that conditionally suppresses
mouse-generated UI pointer events and first-mouse UI defaults while retaining
game mouse axis/button state. The compile-time mode defaults off. This patch is
not applied, built or packaged; saved UI sequence isolation and actual-core mode
evidence remain required before click forwarding. No mouse launch support added.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds, patch
application or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 553: the relative preview API now accepts an optional explicit mouse-button
list with axes and saved sources. It invokes the combined native planner and
restricted source preparation, including button-only source ports. Axis-only
requests retain their existing path. Button-bearing results explicitly report
pending integration and return no applicable draft configuration, keeping them
out of the unfinished persistence/launch path. No physical forwarding enabled.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 552: explicit mouse-button source preparation now validates pinned
left/right/middle/side/extra output codes and exact source-port coverage across
axis and button assignments. It retains only required physical axes and saved
button remaps, rejects absent capabilities/duplicate devices, and allows a
button-only source without placeholder axes. Existing axis-only callers pass an
empty button set and still discard every click. New button preparation remains
disconnected from launch pending settings integration and native UI isolation.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 551: added a native mouse-button assignment representation and composer for
ordinary buttons 1–5. It checks full inspected switch equality, selected source
ports, duplicate/conflicting field ownership and observed native mouse indices;
explicit mouse-owned fields are removed from default gamepad inference and saved
field sequences. Empty button sets retain the existing axis planner. This layer
is not yet wired to settings, review or launch, and does not enable capture or
solve native UI isolation. No additional game coverage is claimed.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 550: pinned MAME source inspection established that mouse-button routing
also interacts with native UI default assignments and independently emitted
left-click pointer events. The remaining-work ledger now distinguishes implemented
axis launch integration from missing button ownership/UI isolation. The relative
mapping view explicitly states that buttons and scroll are discarded. No mouse
button support was enabled; this contract evidence determines the next work.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 549: the MAME relative editor can replace the selected source player's
device across all queued axes, preserving game fields and output channels while
explicitly adopting the chosen tuning/capture settings. Device labels now show
gain, inversion, swap and exclusive/shared capture to distinguish variants.
Replacement invalidates the previous preview; complete backend capability and
device-uniqueness validation is still required before draft application.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 548: per-game relative source tuning now edits output X/Y sensitivity,
inversion and pre-scaling physical axis swap. Applying updates every queued axis
for that source player while preserving identities and unrelated players; a
conflicting device set or changed batch rejects the operation. Global saved
devices remain untouched. The replacement preview must be regenerated before
draft application, including capability validation after an axis swap.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 547: the relative mapping view can open the exact selected assignment in
the batch editor. Existing batch rows can also populate the field, source port,
output axis and saved-device selectors without modifying the batch or draft.
Selection checks field identity, player and output; stale review blocks entry.
The complete saved batch remains loaded and explicit replacement preview/apply
is still required. UI behavior remains unverified.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 546: saved MAME relative routes now have a reusable source/destination view
with a route selector, highlighted generic physical X/Y axis, saved device
identity, sensitivity/inversion, output axis and exact destination field. The
panels stack at narrow widths; textual routes remain in the technical report.
This is an axis schematic, not a verified device silhouette or new catalog
layout. Runtime and visual rendering remain unverified.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 545: MAME assignment review now exposes each saved relative route outside
the technical report: exact source device and identity, physical axis before
swap, output-axis sensitivity/inversion, source player and exact destination
field. These rows also appear in the technical report and clear on review
invalidation. They are saved routing descriptions, not live verification or
controller diagrams; relative-device diagram coverage remains unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or UI runs. Uncommitted/unreviewed; wheels paused.

Step 544: the MAME player-controller editor can retain zero gamepads when the
draft has assigned relative sources. Removing a gamepad preserves relative axes
and sources; explicit gamepad button/axis ownership still blocks removal. The
editor can reopen a relative-only draft and add gamepads back, reports retained
relative ports, and distinguishes removing a gamepad from removing the whole
player port. Fresh review and staging remain required; no playability claim.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI
runs. Uncommitted/unreviewed; wheels paused.

Step 543: calibrated MAME launch preparation now checks fresh inspected profiles
for every relative-only port before opening gamepad transports or clearing port
bindings. Remaining gamepad-channel requirements reject launch even for callers
outside saved-settings validation. Relative draft preview guidance now describes
the implemented port requirements instead of claiming relative-only selection is
unfinished. This closes a launch-validation gap, not additional game coverage.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 542: MAME native selection now includes both gamepad and relative-source
ports. Relative-only settings no longer require a placeholder controller, but
staging rejects any remaining gamepad-channel requirements on those ports. Review
reports such gaps explicitly. Launch preparation accepts the complete union and
keeps relative-only frontend ports enabled with cleared gamepad bindings. The
relative editor can select ports 1-8 independently; layout percentages explicitly
count gamepad players. This is source implementation, not verified mouse-only play.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 541: the relative-axis editor loads the complete saved draft set, supports
explicit replacement/removal and previews a backend-validated candidate. Applying
changes only relative assignments/sources in the draft; native-only previews
cannot apply, stale previews clear, and empty replacement explicitly removes all.
Other mappings remain intact. Hybrid source structure no longer creates a blanket
pending flag on 64-bit Linux; physical/profile gaps and runtime verification remain
separate. Relative-only players remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or UI/device runs. Uncommitted/unreviewed; wheels paused.

Step 540: saved MAME setups with relative assignments now prepare only their
validated sources, wait boundedly for virtual endpoints with cancellation, retain
a topology guard and stage disabled bootstrap routing. The main launch worker
selects owned-pipe spawn/bridge attachment when such a group is pending; ordinary
sessions keep the existing spawn path. This connects hybrid gamepad-plus-relative
dispatch on 64-bit Linux, but it is unbuilt/unverified and still requires usable
gamepad profiles for selected players. Relative-only players remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds, devices or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 539: fresh MAME launch inspection now accepts and retains exact relative
assignments, composing them with analog/digital mappings against the fresh snapshot
and original persistent configuration. Relative planning requires explicit mouse
inspection; automatic gamepad allocation cannot acquire relative sources. Saved
setup call sites pass the retained assignments, but the unfinished-dispatch guard
still prevents selecting this path for physical relative launch.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 538: MAME settings now serialize exact relative assignments and identified
sources, with empty defaults for old setups. Validation reuses the native relative
planner and complete source-capability checks. Review and staging count composed
relative fields; review still marks their physical routing as pending. The
gamepad-only launch path explicitly rejects these selections rather than silently
ignoring them until the remaining runtime dispatch is connected.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 537: added private disabled-mouse bootstrap staging tied to the exact final
MAME launch plan, plus an opt-in spawn path that pipes child stdio and attaches
the owned bridge. Attachment errors kill and reap the child. Ordinary spawn
stdio remains unchanged. This completes those adapter operations in source;
saved relative assignments and selection by the main launch worker are still
missing, so physical relative launch is not advertised as enabled.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 536: added all-disabled mouse bootstrap configuration and an opt-in native
route-setting command. The setter validates all eight indices and rejects active
sharing or replacement of a different active table; only disabled initialization
or an identical repeat is accepted. The bridge derives owned routes from its
child's log, sends the assignment, then waits for the effective-state reply before
forwarding. Source packaging includes both runtime patches. Outer launch still
needs assignment persistence, bootstrap staging and the post-spawn call.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
patch application, builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 535: the relative command channel now owns and drains the same child's
stderr, retaining a bounded startup log until the first routing reply. Bridge
route resolution consumes that log rather than a caller-supplied string. Later
stderr is drained without unbounded retention; new mouse-enumeration records are
rejected. All three command/log pipes now come from the executable-checked child.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Relative assignment persistence and pre-spawn routing remain
unfinished. Formatting/source/whitespace inspection only; no tests, builds or
runtime checks. Uncommitted/unreviewed; wheels paused.

Step 534: the launch worker now consults controller-handshake readiness during
startup. Ordinary sessions retain the 700 ms survival delay; an attached pending
handshake may wait up to three seconds. Handshake failure or timeout kills and
waits for the child before reporting failure, and play-session recording remains
after readiness. This connects the readiness gate, not relative assignment
persistence, private routing preparation or the post-spawn attachment itself.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 533: retained MAME sessions can now own the relative bridge through an
explicit post-spawn handoff that checks the child executable, inspected mouse
contract and retained inputs, then takes that child's command pipes. Existing
launch health checks include bridge failure, and field drop order joins the
bridge before releasing private session files. Added separate route-handshake
readiness reporting. Attachment is not yet called by the launch worker; relative
assignment persistence and startup-log/config preparation remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 532: the opt-in frontend query now reports native window focus separately
from routing settings. The Rust observation parser requires this field. Relative
bridge startup waits up to two seconds for focused routing; reported focus loss
after confirmation stops the session and releases owned resources without auto
resume. Detection remains sampled, not instantaneous; outer child lifetime and
launch integration remain unfinished. Catalog stays 91/95 cores (95.8%), 289
profiles, 157 layouts; overall/MAME completion unknown. Source reads, formatting
and whitespace inspection only; no tests, builds or runtime checks.
Uncommitted/unreviewed; wheels paused.

Step 531: relative bridge startup now owns the command channel and waits for a
matching runtime reply before forwarding. It requires unstarted readers so their
initial drain discards handshake-era movement. Ongoing queries check all routes,
driver and endpoint identities; failures shut down the group. Initial route
confirmation is exposed separately from health. Monitoring is sampled, not an
atomic route lock, and outer launch/focus/process integration remains unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds, threads or runtime checks started. Uncommitted/unreviewed; wheels paused.

Step 530: added a nonblocking owned-child stdin/stdout channel for routing
queries, with single-query ownership, bounded drains, deadlines, framing and
unsolicited/duplicate reply rejection. The opt-in frontend patch now terminates
both supported and unsupported replies. The channel never spawns a frontend or
opens devices. Private routing configuration enables stdin commands and disables
network commands; process identity, stderr evidence and failure cleanup remain the
launch owner's responsibility. It is not connected to live launch yet.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source reads, formatting and whitespace inspection only;
no tests, builds, pipe sessions or runtime checks. Uncommitted/unreviewed;
wheels paused.

Step 529: added an opt-in Linux flake package for the pinned RetroArch routing
extension, with the exact archive SHA-256 and explicit command/udev build flags.
It reuses the pinned Nixpkgs dependencies, omits the stock-runtime wrapper and
does not replace the default app/runtime or enable a network command listener.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source/recipe reads, archive hashing and whitespace inspection
only; no package evaluation, patch application, tests, builds or activation.
Command-channel and launch integration remain incomplete. Uncommitted/unreviewed;
wheels paused.

Step 528: added an opt-in patch for the pinned RetroArch command handler to
report its active udev driver and all eight mouse indices in one reply. Added
the corresponding bounded Rust parser, rejecting unsupported, truncated and
malformed replies. This is a source-level runtime extension, not an installed
frontend: packaging, command-channel ownership and launch integration remain.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source reads, formatting and whitespace inspection only;
no patch application, tests, builds or runtime checks. Uncommitted/unreviewed;
wheels paused.

Step 527: pinned frontend source inspection identified a concrete relative-launch
integration gap: GET_CONFIG_PARAM cannot report the mouse driver or player mouse
indices. Updated the backlog and native preview warning so the remaining work is
an effective-state acquisition contract, not a presumed generic query client.
No custom runtime dependency was introduced and no new input support is claimed.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Source reads, formatting and whitespace inspection only;
no tests, builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 526: relative bridge startup now requires the complete effective eight-port
mouse state, not only a selected-player index map. It checks the udev driver,
every owned route and every unused-port sentinel before starting the worker.
Configuration generation shares the same expected-state construction. The launch
owner still must obtain effective state from the actual frontend process and
establish startup provenance; deserialization is not proof of application.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Physical launch integration remains incomplete. Formatting
and source/whitespace inspection only; no tests, builds or runtime checks.
Uncommitted/unreviewed; wheels paused.

Step 525: shared relative frontend routing now enforces the pinned RetroArch
driver's 16-device bound instead of accepting unusable indices up to 255. Owned
groups can generate private udev configuration with explicit selected-player
indices and the source-defined no-mouse sentinel on every unused port. Duplicate
routes are rejected. Generation does not apply configuration, start forwarding
or prove routing in the eventual game startup; launch integration remains open.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Pinned-source reads, formatting and whitespace inspection
only; no tests, builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 524: saved MAME setups now preserve an enabled reviewed native mouse class
during launch reinspection, explicitly pinning the wrapper polling option. Native
session command generation retains the same requested class instead of reverting
to the digital-only command. Automatic gamepad inspection remains unchanged.
Existing snapshot comparison, unresolved-field and physical calibration gates
remain mandatory; this does not add relative assignments or physical forwarding.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 523: game review and preset comparison now share explicit field-coverage
reporting. The full inspected user-input percentage is retained alongside a
separate assignment-scope percentage. Disabled inputs and native-service bindings
are reported separately, never counted as mapped; unresolved inputs remain in
both denominators. Empty denominators have no percentage. These figures describe
only the inspected game/mode, not calibration, playability or all MAME coverage.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed; wheels paused.

Step 522: opt-in mouse inspection now requires explicit mame_mouse_enable=enabled
in addition to native -mouse. The pinned wrapper gates relative polling separately
from native device-class enumeration. Generated opt-in requests pin that option;
manually supplied requests reject missing, disabled or duplicate values before
starting a process. Ordinary gamepad requests retain their existing contract.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Physical relative launch remains incomplete; wheels paused.
Pinned-source reads, formatting and whitespace inspection only; no tests, builds,
device capture or runtime checks. Uncommitted/unreviewed.

Step 521: MAME default-field filtering and resolved switch routing now index
explicit assignments by field identity instead of scanning every override for
each inspected field. Candidate matches still require full field equality;
multiple sequences retain their input order. This also applies to retained
labels, analog state and keyboard ownership when deriving default snapshots.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Wheels paused. Formatting/source/whitespace inspection only;
no tests, builds, benchmarks or runtime checks. Uncommitted/unreviewed.

Step 520: ordinary numbered-button swaps can now begin from the reviewed action
list without a drawable physical mapping. Eligibility and preservation use all
reviewed switch owners, retaining shared-channel rejection and unrelated numbered
assignments. Physical rows enrich the preview only; unavailable connections show
the explicit channel instead. Calibration and staging gates remain unchanged.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Wheels paused. Source/whitespace inspection only; no tests,
builds, device capture or runtime checks. Uncommitted/unreviewed.

Step 519: the relative bridge's public start path now requires applied player
routes to equal freshly resolved owned-endpoint routes before forwarding. The
unchecked worker-start helper is private. Mismatches release owned resources;
startup provenance and obtaining genuinely effective routes remain launch-owner
requirements, not inferred from desired configuration. Catalog stays 91/95 cores
(95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown. Wheels paused.
Formatting/source/whitespace checks only; no tests, builds or processes started.
Uncommitted/unreviewed; MAME launch integration remains incomplete.

Step 518: relative batch planning indexes inspected identities, conflicting owners
and assigned fields instead of rescanning whole lists for every assignment and
retained field. Full field equality remains required after identity lookup;
ordering stays deterministic. Launch call-site inspection confirms relative
transport handoff remains a prerequisite to persistent launch settings. Catalog
stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown.
Wheels paused. Formatting/source/whitespace checks only; no tests, builds or
benchmarks. Uncommitted/unreviewed.

Step 517: the relative preview UI now builds a temporary multi-axis/multiplayer
batch with exact-field replacement and explicit removal. Combined preview merges
one device selection per source player and rejects conflicting selections instead
of overwriting them. Closing discards the batch; no persistence or capture was
added. Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME
completion unknown. Wheels paused. Source/whitespace checks only; no tests,
builds or endpoint calls. Uncommitted/unreviewed; launch integration incomplete.

Step 516: MAME review exposes a read-only relative-axis preview dialog with explicit
native field, source player, output axis and saved device choices. Native-only
preview is an explicit alternative. Selection or draft/controller changes clear
results; no Apply, capture or Run action exists. The existing bounded backend
validates and renders the proposed native configuration. Catalog stays 91/95 cores
(95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown. Wheels paused.
Whitespace/source inspection only; no tests, builds or endpoint calls.
Uncommitted/unreviewed; persistence and launch integration remain incomplete.

Step 515: the owned relative mouse group can resolve player frontend indices
against its actual endpoints, checking topology before/after and revalidating the
complete endpoint set. Failure shuts down the group; resolution does not start
forwarding. The caller still must establish startup-log provenance and verify
route application before bridge handoff. Catalog stays 91/95 cores (95.8%), 289
profiles, 157 layouts; overall/MAME completion unknown. Wheels paused. Formatting
and whitespace checks only; no devices, tests, builds or processes started.
Uncommitted/unreviewed; MAME launch integration remains incomplete.

Step 514: the advanced inspection form exposes a default-off native mouse-evidence
option for generated requests. Toggling clears the request and trust confirmation;
execution remains an explicit Run action. Opted-in inspection now rejects missing,
disabled or invalid native mouse routes before accepting its outcome. Physical
capture and launch routing remain separate. Catalog stays 91/95 cores (95.8%),
289 profiles, 157 layouts; overall/MAME completion unknown. Wheels paused.
Formatting/whitespace checks only; no tests, builds or inspection ran.
Uncommitted/unreviewed.

Step 513: explicit native inspection requests accept default-off inspect_mouse,
which adds MAME's mouse-class enable option only for that inspection command.
Automatic request builders and ordinary session commands retain their prior
behavior. Source inspection confirmed axis-only virtual endpoints already keep
unselected capabilities neutral. No device was opened and no inspection ran.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion
unknown. Wheels paused. Formatting/whitespace checks only; no tests or builds.
Uncommitted/unreviewed; relative launch integration remains incomplete.

Step 512: validated relative source selections now produce session-specific
settings containing only physical axes required by assigned post-transform X/Y
outputs. Unused axes, scroll axes and all mouse buttons are excluded; identity,
calibration and existing exclusive-source choice are preserved in cloned settings.
Preview returns this preparation without opening hardware or changing saved data.
Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion
unknown. Wheels paused. Formatting/whitespace checks only; no tests, builds or
capture. Uncommitted/unreviewed; launch integration remains incomplete.

Step 511: relative previews accept explicit source selections alongside native
assignments. Validation requires exactly the assigned source players, complete
post-transform X/Y capability and distinct saved physical paths/identities.
Duplicate, missing and unused source selections fail; native-only previews remain
available. Saved-source validation is reported separately from physical routing,
which remains disabled. Catalog stays 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion unknown. Wheels paused. Formatting/whitespace checks only;
no tests, builds, endpoint calls or capture. Uncommitted/unreviewed.

Step 510: added native relative-assignment composition and a bounded read-only
preview endpoint. Exact inspected fields, selected sources, X/Y outputs and mouse
routes are validated; duplicate/conflicting analog or digital ownership is rejected.
The planner composes existing layers, clears competing axis directions and removes
saved sequences only for assigned fields. Preview changes no settings and explicitly
leaves launch/physical routing disabled. Catalog stays 91/95 cores (95.8%), 289
profiles, 157 layouts; overall/MAME completion unknown. Wheels paused. Formatting
and whitespace checks only; no tests, builds or endpoint calls. Uncommitted/unreviewed.

Step 509: relative-device candidates now must cover the complete unresolved X/Y
axis set for the suggested native player, rather than matching just one field.
Saved axis swaps are respected through the shared capability checker. Review
retains each exact field and reports the required axis set; it does not split
one player across devices or assign any candidate automatically. Catalog stays
91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown.
Wheels paused. Formatting/whitespace inspection only; no tests, builds or capture.
Uncommitted/unreviewed; relative launch integration remains incomplete.

Step 508: recognized relative X/Y requirements can generate pinned MAME mouse-axis
sequence tokens from observed native indices. Review resolves the suggested native
player only when selected and reports a sequence or exact native-evidence error.
Mouse-route validation is reused once per review, not repeated per field. These
remain candidate sequences, not saved assignments or launch authority. Catalog
stays 91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown.
Wheels paused. Pinned-source/formatting/whitespace inspection only; no tests,
builds or capture. Uncommitted/unreviewed.

Step 507: unresolved MAME review now identifies exact P1..P8 DIAL, TRACKBALL and
MOUSE X/Y relative-axis requirements and lists structurally compatible saved
relative-device candidates using the shared swap-aware capability check. Unknown,
absolute, keyboard and malformed identities are not inferred as relative axes.
Requirements remain unmapped and launch-disabled; candidate indices are review
data, not persistent device identity. Catalog stays 91/95 cores (95.8%), 289
profiles, 157 layouts; overall/MAME completion unknown. Wheels paused.
Formatting/whitespace inspection only; no tests, builds or capture. Uncommitted/unreviewed.

Step 506: pinned input_retro.cpp confirms RetroMouse0..7 correspond to frontend
ports 0..7. A native resolver now requires enabled current evidence and exact
unique IDs, returning observed one-based native sequence indices. MAME review
exposes resolution or its error as informational evidence only; physical routing
remains false. No mouse mode is launch-enabled. Catalog stays 91/95 cores (95.8%),
289 profiles, 157 layouts; overall/MAME completion unknown. Wheels paused.
Pinned-source/formatting/whitespace inspection only; no tests, builds or capture.
Uncommitted/unreviewed.

Step 505: MAME native inspection can now retain optional mouse-class enable state
and native device IDs/indices through the existing Lua device API. Validation
rejects invalid/duplicate identities and indices; snapshot comparison normalizes
enumeration order. Missing older evidence stays absent, not enabled or routed.
No physical/frontend association or playable mouse mode is claimed. Catalog stays
91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown.
Wheels paused. Pinned-source inspection and formatting/whitespace checks only;
no tests, builds, native inspection or capture. Uncommitted/unreviewed.

Step 504: shared relative frontend support now resolves one-to-eight selected
players against one parsed startup mouse table, requiring exact distinct relative
endpoints and unique indices. This is a pure resolver, not launch integration or
persistent route evidence. Source inspection also confirmed MAME snapshots lack
native mouse identity evidence; that contract remains outstanding. Catalog stays
91/95 cores (95.8%), 289 profiles, 157 layouts; overall/MAME completion unknown.
Wheels paused. Formatting/whitespace checks only; no tests, builds, capture or
runtime checks. Uncommitted/unreviewed.

Step 503: source inspection found the existing identified relative reader,
virtual mouse and owned session bridge. Saved output-capability matching is now
shared in RelativeDeviceSettings and consumed by FBNeo, handling X/Y swaps and
button remaps without opening devices. The backlog now targets MAME native
relative routing/frontend ownership rather than duplicating capture. No new MAME
mode is claimed. Catalog remains 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion unknown. Wheels paused. Formatting/whitespace checks only;
no tests, builds, capture or runtime checks. Uncommitted/unreviewed.

Step 502: source audit records substantive remaining adapter/contract boundaries
in CONTROLLER_MAPPING_REMAINING.md and separates per-game/native adapters from
static profile counts. The next investigation targets reusable physical relative
input transport, not more preset UI polish or wheel support. This checkpoint adds
no mapping coverage: catalog remains 91/95 cores (95.8%), 289 profiles, 157 layouts;
overall/MAME completion remains unknown. Source/document inspection only; no
tests, builds or runtime checks. Uncommitted/unreviewed.

Step 501: MAME's player picker labels destination-layout errors, unavailable
calibration, absent profiles and physical-mapping gaps. A next-gap action cycles
through affected players and retains navigation preference without editing any
mapping. It requires a current review and does not equate absence of these gaps
with complete native-input setup or runtime verification. Catalog stays 91/95
cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Source/whitespace inspection only; no tests, builds or
runtime checks. Uncommitted/unreviewed.

Step 500: the MAME channel picker shows current saved physical button labels and
calibration-gap markers beside stable channel identities. Labels require a unique
reviewed player/output row and fall back explicitly when unavailable or unmapped.
Only current-draft review data is used, never proposed mappings. A fixed channel
model keeps label refreshes separate from selection; all channels remain editable.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Source/whitespace inspection only;
no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 499: MAME review returns selected-player destination-profile accounting:
generated, failed and no-active-profile counts, with a percentage over selected
players. Main review and preset comparison display this separately from native
field coverage and physical calibration. Accounting requires one result per
selected player; no overall game-support or playability percentage is inferred.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Formatting/source/whitespace inspection
only; no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 498: preset comparison distinguishes partial review from successful profile
generation, includes exact per-player layout errors, and marks incompatible
candidates. Such candidates remain inspectable but Use in draft is blocked when
the error affects the player override or shared-preset inheritors being changed.
Unrelated player errors remain visible without preventing incremental repair;
full review/staging validation remains required. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Source/whitespace inspection only; no tests, builds or runtime
checks. Uncommitted/unreviewed.

Step 497: read-only MAME review validates native inputs separately from destination
profiles, then reports profile errors per player without hiding other players.
Affected players retain exact editable switch actions and an explicit layout
error, with no fabricated diagram or physical rows. Physical-pending blocks
staging eligibility; stored validation, staging and launch retain full profile
validation. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion remains unknown. Wheels paused. Formatting/source and
whitespace inspection only; no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 496: Automatic MAME geometry now counts trigger-threshold action channels
toward the ordinary six/eight-button capacity instead of forcing legacy gamepad
geometry merely because a trigger channel is present. Existing sparse placement
packs these actions into free button positions. More than eight independent
actions and directional-switch channels retain fixed geometry; native wires,
threshold behavior and analog-pressure ownership are unchanged. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Formatting/source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed.

Step 495: MAME edit/removal previews now toggle between current-draft and proposed
source/destination mappings. Before-state matching requires an exact unique player
port and controller identity. Diagrams, routes, calibration gaps and warnings all
follow the displayed version; both snapshots clear together when preview inputs
change. The toggle never changes the draft. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion remains unknown. Wheels
paused. Source/whitespace inspection only; no tests, builds or runtime checks.
Uncommitted/unreviewed.

Step 494: MAME channel-edit and override-removal previews now expose proposed
source/destination diagrams for every returned player, with native routes,
physical gaps and per-player warnings. Candidate data clears on selection, draft,
controller revision changes or closing the editor. Missing destination layouts
remain explicit; previews neither change the draft nor authorize staging. Catalog
stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion
remains unknown. Wheels paused. Source/whitespace inspection only; no tests,
builds, review endpoint calls or runtime checks. Uncommitted/unreviewed.

Step 493: single numbered-button edits offer default-on preservation of other
resolved numbered-button channels across selected players. Preview and Apply use
the same exact-route preservation helper, adding overrides only for defaults and
rejecting ambiguous or inconsistent identities before draft mutation. Directions,
non-button defaults and removal behavior are unchanged. Sharing channels and
layout geometry still require review. Catalog stays 91/95 cores (95.8%), 289
profiles and 157 layouts; overall/MAME completion remains unknown. Wheels paused.
Source/whitespace inspection only; no tests, builds, review endpoint calls or
runtime checks. Uncommitted/unreviewed.

Step 492: MAME ordinary swaps can retain the first selected action while the user
chooses the second from either controller diagram. Exact unique hotspot/route
matching selects a candidate only; shared, missing, same-action and ineligible
connections clear the candidate instead of guessing. The existing picker remains
available and Swap buttons is still the explicit draft mutation. Cancel, closing
review or changing players clears diagram selection. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Source/whitespace inspection only; no tests, builds or runtime
checks. Uncommitted/unreviewed.

Step 491: ordinary MAME button swaps now show physical control labels in the
candidate picker and a two-action before/after summary before changing the draft.
Labels require an exact unique reviewed target/output row; unavailable or unmapped
connections and calibration gaps remain explicit. Stale reviews hide the summary;
the existing explicit swap action and mandatory re-review are unchanged. Catalog
stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion
remains unknown. Wheels paused. Source/whitespace inspection only; no tests,
builds or runtime checks. Uncommitted/unreviewed.

Step 490: retained repeat-alias recordings can explicitly move to their uniquely
declared independent base when it has no recording. The existing read-only capture
validator checks the binding under the base ID before any draft mutation; failure
preserves all records. Success preserves the measurement and other bindings, pauses
recording, and changes no saved calibration until Apply with full validation.
Removed or ambiguous IDs remain removal-only. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion remains unknown. Wheels
paused. Source/whitespace inspection only; no tests, builds, validator calls,
calibration or runtime checks ran. Uncommitted/unreviewed.

Step 489: calibration exposes retained records outside the current independent
recordable-control list, including removed IDs and hardware-repeat aliases. An
explicit selection can remove one from the draft without resetting other records;
recording pauses afterward, pending capture blocks removal, and saved calibration
is unchanged until Apply. This enables repair of stale layouts without Start over.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Source/whitespace inspection only;
no records were removed, and no tests, builds, calibration or runtime checks ran.
Uncommitted/unreviewed.

Step 488: the calibration wizard evaluates retained records individually through
the read-only validator while visible. Invalid records remain intact but appear in
required-control repair navigation and the selector, with selected-control error
details. Progress now explicitly counts individually valid records rather than
presence and excludes pending capture; cross-control conflicts, complete native
measurements and live behavior remain separate. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion remains unknown. Wheels
paused. Source/whitespace inspection only; no validator calls, calibration, tests,
builds, UI/device or runtime checks were run. Uncommitted/unreviewed.

Step 487: Record this control again now establishes an exact single-control target
through the same selection path as targeted repair, including after a failed
handoff. It cannot silently resume sequential recording and is disabled without a
current control/layout. Back navigation retains accurate paused-state messaging.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Source/whitespace inspection only;
no calibration, tests, builds, UI/device or runtime checks. Uncommitted/unreviewed.

Step 486: completed calibration gestures now pass a bounded read-only backend
validation call before the wizard retains them. It reuses Calibration validation
for the selected control's type, native direction, measured-axis consistency and
pressure rules. Validation is isolated to that control to permit incremental repair;
duplicate checks and full save-time validation remain. Invalid results restore the
previous binding. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion remains unknown. Wheels paused. Formatting/whitespace
checks only; no endpoint calls, calibration, tests, builds or runtime checks.
Uncommitted/unreviewed.

Step 485: calibration checks completed logical and native input identities against
other saved bindings after release-time axis direction refinement. Conflicts use
the existing error recovery path, restoring the previous binding or removing the
unfinished replacement rather than retaining an unusable duplicate. Controller
identity/release matching and backend save validation remain intact. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Source/whitespace inspection only; no calibration, tests,
builds, UI/device or runtime checks. Uncommitted/unreviewed.

Step 484: targeted calibration now pauses recording when the requested layout,
control or repeat-owner identity does not match uniquely. Any pending capture is
cancelled/restored and saved bindings are preserved; the user must explicitly
choose or resume a control. MAME handoff messaging now reflects the pause instead
of leaving the first wizard control capture-ready behind an error. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Source/whitespace inspection only; no calibration, tests,
builds, UI/device or runtime checks. Uncommitted/unreviewed.

Step 483: MAME candidate previews now offer explicit whole-controller and focused
source-control calibration handoffs. They require a completed non-stale comparison
and exact draft player/controller identity. Handoff clears retained comparison
results and review authority, preserves the draft, and applies no candidate preset;
even cancellation requires a new comparison/review. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Source/whitespace inspection only; no calibration, tests, builds,
UI/device or runtime checks were run. Uncommitted/unreviewed.

Step 482: selecting an unmapped source hotspot now clears the previous connection
and retains a separate source-control highlight. MAME's explicit targeted-calibration
button accepts that identity with a known source layout; hardware repeats resolve
to their base input. Source/row changes clear the independent highlight. Clicking
does not start capture, edit bindings or infer a source/destination connection.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Source/whitespace inspection only;
no tests, builds, UI/device or runtime checks. Uncommitted/unreviewed.

Step 481: MAME's physical-planning failure review retains the source layout only
after saved calibration validates. Main review and preset comparison can therefore
show the known source outline beside the destination during repair, with explicit
warnings and unmapped connections. Missing/invalid calibration still has no source
layout, and no native measurements or readiness are inferred. Catalog stays 91/95
cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Formatting/whitespace checks only; no tests, builds, UI or runtime
checks. Uncommitted/unreviewed.

Step 480: MAME review routes physical-profile planning errors through the existing
per-player repair path instead of aborting the entire review. Exact game-side
requirements, native routes and profile conditions remain visible with unmapped
physical rows and the retained error; other players continue to review. Successful
physical plans are retained/reused. Staging and launch validation remain unchanged.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Formatting/whitespace checks only;
no tests, builds, UI or runtime checks. Uncommitted/unreviewed.

Step 479: MAME review preserves generated profile conditions when calibration is
unavailable, alongside the calibration error. Preset comparison now displays the
selected player's warnings beside its diagrams, retaining sparse-placement and
shared-threshold caveats even without a physical source mapping. No binding or
readiness behavior changed. Catalog stays 91/95 cores (95.8%), 289 profiles and
157 layouts; overall/MAME completion remains unknown. Wheels paused. Formatting
and whitespace checks only; no tests, builds, UI or runtime checks.
Uncommitted/unreviewed.

Step 478: generated MAME preset comparison results now retain per-player visual
data. Selecting a successful candidate exposes source/destination diagrams, exact
native action labels and calibration gaps through the shared mapping view, with
player selection defaulting to the scoped player. Stale results hide the visual
preview; missing destination profiles are explicitly identified. No extra review
or inspection is invoked by preview selection. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion remains unknown. Wheels
paused. Source/whitespace inspection only; no tests, builds, UI or runtime checks.
Uncommitted/unreviewed.

Step 477: player-scoped MAME comparison includes Inherit shared preset alongside
the five explicit choices. Preview and Apply remove only the selected override,
omitting an empty override object while preserving other players and assignments.
Shared comparison still offers five choices; player comparison reports six and
uses the same generation, staleness and fresh-review guards. Catalog stays 91/95
cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Source/whitespace inspection only; no tests, builds, UI or runtime
checks. Uncommitted/unreviewed.

Step 476: MAME visual player review can launch a player-scoped preset comparison.
Candidate generation and Apply share one draft transformation that changes only
the selected player's preset override, preserving the shared preset, other players
and exact assignments. The whole setup remains validated; reports disclose that
unrelated player errors may block generation. Shared comparison remains available.
Stale/completion guards and fresh review requirements remain intact. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Source/whitespace inspection only; no tests, builds, UI or
runtime checks. Uncommitted/unreviewed.

Step 475: completed MAME preset comparisons now offer an explicit Use in draft
action for successfully generated choices. Applying requires unchanged draft and
controller revision, preserves player overrides/exact assignments, and invalidates
review; it never stages or persists settings. Selection is reset when results
change and remains disabled while comparison is incomplete or stale. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Source/whitespace inspection only; no comparison, tests,
builds or runtime checks were run. Uncommitted/unreviewed.

Step 474: MAME preset comparison now schedules one choice per timer callback,
shows completed-choice progress, and cancels queued work on closure or stale
draft/controller state. A generation guard rejects results from superseded work.
Each individual review call remains synchronous; this is not a background-worker
implementation or a measured responsiveness claim. Drafts remain unchanged.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Source/whitespace inspection only;
comparison not invoked, no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 473: coverage UI now includes per-game MAME/FBNeo adapters in Some launch
support while retaining them under Incomplete/preview. Search includes feature,
gap and detail text. MAME's unverified implementation list reflects sparse presets,
shared-pressure composition, visual navigation/comparison and completeness checks;
remaining work explicitly includes the game/mode denominator and deferred runtime
evidence. No counters or adapter acceptance status changed. Catalog stays 91/95
cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Source, formatting and whitespace inspection only; no tests, builds,
UI runs or runtime checks. Uncommitted/unreviewed.

Step 472: MAME setup now offers a user-triggered shared-preset comparison using
the existing read-only assignment review API. All five choices retain per-player
overrides and report generated destination layouts, native field counts, physical
gaps or generation errors without editing/staging the draft. It identifies which
players inherit the shared preset and flags stale text/controller revisions.
Profile generation is explicitly not launch readiness. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion remains unknown.
Wheels paused. Source/whitespace inspection only; comparison was not invoked and
no tests, builds, UI/emulator runs or probes occurred. Uncommitted/unreviewed.

Step 471: automatic MAME profile generation and current-schema saved setup
validation now use the same final completeness-checked composer as per-game review
and launch. Automatic mode supplies its inspected selected-player set and empty
overrides; saved validation supplies all retained assignments. Empty editable
drafts remain optional and automatic unresolved-field rejection remains intact.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Formatting/whitespace checks only;
no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 470: the common MAME explicit-profile entry now checks final output-set
completeness for both default-only and overridden profiles. Resolved native switch
outputs plus assigned analog outputs must equal the generated binding outputs;
each output has one destination and each destination exists uniquely in its layout.
Absent profiles cannot discard required outputs. Diagnostics list missing and
unexpected outputs. This is a composition invariant, not runtime verification.
Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Formatting/whitespace checks only;
no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 469: Automatic geometry in explicit MAME switch composition now excludes
validated shared pressure outputs from its digital geometry inputs, matching the
non-override analog composer. The full required-output set remains intact for
binding and route coverage. Unshared triggers and direction switches retain fixed
geometry; pure analog requirements remain with the analog composer. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels paused. Formatting/whitespace checks only; no tests, builds or
runtime checks. Uncommitted/unreviewed.

Step 468: saved MAME setup validation now uses the combined digital/analog profile
builder when no explicit switch overrides exist. This closes the digital-only
validation path that could reject shared-pressure presets before review, and also
validates pure-analog profile geometry. Empty editable profiles remain optional;
staging/launch readiness and exact analog-field checks are unchanged. Catalog
remains 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion
remains unknown. Wheels paused. Formatting/whitespace checks only; no tests,
builds or runtime checks. Uncommitted/unreviewed.

Step 467: mixed MAME profiles now validate analog assignments and remove duplicate
shared pressure representations before digital preset capacity/geometry selection.
This avoids rejecting a full button row merely because a native switch also uses
a trigger already owned by analog pressure. Only duplicate physical requirements
are removed; native fields/sequences remain intact and the analog composer adds
the measured pressure control. Pure digital and unrelated outputs stay unchanged.
Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remains unknown. Wheels paused. Formatting/whitespace checks only;
no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 466: explicit six/eight-button and Neo Geo presets can now place sparse
L2/R2 switch outputs in free button positions. They count against actual preset
capacity and retain their native axis-switch semantics and digital full-travel
fallback. Explicit composition excludes outputs already owned by measured analog
controls from button placement, preserving shared proportional pressure. Automatic
trigger selection still uses fixed-channel geometry; no physical peripheral
support is added. Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion remains unknown. Wheels paused. Formatting/whitespace
checks only; no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 465: Automatic MAME button geometry now uses distinct required button-channel
count rather than the highest occupied channel: up to six ordinary channels use
six-button geometry, seven/eight use eight-button geometry. Default and explicit
composition share the selector and sparse allocator. Extended trigger/direction
channels retain fixed geometry; twin-stick handling stays separate. Review and
launch source paths consume resolved profile bindings, not assumed button numbers.
No runtime proof is claimed. Catalog remains 91/95 cores (95.8%), 289 profiles and
157 layouts; overall/MAME completion remains unknown. Wheels paused. Formatting
and whitespace checks only; no tests, builds or probes. Uncommitted/unreviewed.

Step 464: six/eight-button and Neo Geo destination presets now allocate sparse
digital button channels by occupied capacity, not just their highest channel
number. Conventional occupied positions stay fixed; higher channels use remaining
positions in deterministic order. Default and explicit mappings share the allocator,
with explicit composition recomputing from the full channel set to prevent slot
collisions. Native outputs are unchanged; trigger/axis contracts and over-capacity
rejection remain intact. This expands sparse button-preset combinations, not the
core count. Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion remains unknown. Wheels paused. Rust formatting and
whitespace checks only; no tests, builds or runtime checks. Uncommitted/unreviewed.

Step 463: MAME setup now defaults to a readable draft summary instead of the raw
JSON editor. It shows machine/content, player identities and effective presets,
unstaged-change and review state, with guidance toward visual review and staging.
The raw editor stays instantiated behind an explicit advanced toggle, preserving
all text, assignments and existing validation paths. Malformed drafts show repair
guidance without being rewritten. Staging remains distinct from Save settings and
runtime verification. Catalog stays 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion remains unknown. Wheels remain paused. Source
and whitespace inspection only; no tests, builds, UI runs or probes. Uncommitted.

Step 462: MAME's mapped controller-action picker now offers Show connection.
It matches the exact native field, source player, channel and sequence to one
native route and one displayed destination row, then highlights that connection
and retains the selected action on shared channels. Missing or ambiguous drawable
routes and stale reviews disable the shortcut; independent action editing remains
available. No assignments change. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion remains unknown. Wheels remain paused.
Source and whitespace inspection only; no tests, builds, UI runs or probes.
Changes remain uncommitted and runtime behavior unverified.

Step 461: MAME visual review now has a next-control-needing-setup action.
The shared mapper selects displayed rows with no source assignment or a reported
physical calibration gap, advances in row order and wraps at the end. The button
shows the affected displayed-row count and is disabled for a stale review. This
only changes the highlighted connection; it neither changes assignments nor
claims that other native fields or physical devices are verified. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion remains
unknown. Wheels stay paused. Source and whitespace inspection only; no tests,
builds, UI runs or emulator/device probes. Changes remain uncommitted.

Step 460: added one explicit prepared-session capture-and-finish operation. It
uses the generated config and checked application args, rejects unsafe startup
settings/disc handoff before capture, invokes the owned capture pipeline, and
rechecks topology/source/cancellation before handing off. Default launch remains
config-only; supported runtime/build selection is still required before opt-in.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Wheels stay paused. Source, formatting and whitespace checks
only; no tests, builds or runtime probes/captures were executed.

Step 459: native digital-session preparation now returns one owned intermediate
containing config, bindings' checked topology and arguments. Existing launch uses
the same config-only finish path. A capture finish path validates the supplied
observation against the prepared config/plan and retains the captured handoff in
CalibratedLaunch; plan arguments change only after successful checks. No default
capture invocation is enabled. Runtime/build policy and capture orchestration
remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Wheels stay paused. Source, formatting and
whitespace checks only; no tests, builds or runtime probes.

Step 458: CalibratedLaunch can retain either the existing native config owner or
the captured cartridge handoff owner. Shared launch-input checks dispatch to the
matching verification path, and the capture constructor checks the session before
returning it. Existing launch paths retain their behavior; no adapter selects the
capture constructor yet. Final-plan/capture invocation and supported build policy
remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Wheels stay paused. Source, formatting and
whitespace checks only; no tests, builds or runtime probes.

Step 457: cartridge handoff now consumes and retains the actual PreparedConfig
owner, requiring its path to match the checked CLI selection. Temporary config
lifetime no longer depends on a separate caller handle. Repeat verification
checks both the source configuration and the generated file fingerprint. No
normal launch call site is enabled; loaded-core/build policy remains unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Wheels stay paused. Source, formatting and whitespace checks
only; no tests, builds or runtime probes.

Step 456: added owned cartridge handoff preparation combining invocation, CLI,
deck/content, startup and projected-configuration checks. The actual selected
config is fingerprinted, its read bytes compared to that fingerprint, and retained
with the capture result and checked arguments. Final-plan comparison and repeat
verification are available without refreshing baselines. The caller must still
own temporary-config lifetime; loaded-core/build policy and ordinary launch wiring
remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Wheels stay paused. Source, formatting and
whitespace checks only; no tests, builds or runtime probes.

Step 455: captured-result handoff validates bounded UTF-8 application arguments,
requires one explicit absolute config and the original absolute content path
last, and rejects unsupported startup options/extra content through the shared
capture option policy. Mono/assembly prefixes stay separate. Returned config
paths still require owned-file checks and content validation; normal launch is
not yet connected. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Wheels stay paused. Source, formatting and
whitespace checks only; no tests, builds or runtime probes.

Step 454: capture results retain the requested executable/assembly paths, working
directory identity and explicit environment. Invocation comparison rejects a
different setup and rechecks selected files; directory retargeting is checked
after observation and before reuse. Inherited environment, application arguments,
transitive libraries and normal launch integration remain unresolved. Catalog
stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion
unknown. Wheels stay paused. Source, formatting and whitespace checks only;
no tests, builds or runtime probes.

Step 453: added read-only captured-handoff startup policy for state/ROM/movie/Lua/
watch/cheat autoloads and single-instance forwarding. Common tool autoloads are
checked; unresolved custom tool settings fail explicitly. A combined cartridge
handoff configuration gate now joins startup policy, retained identity checks
and capture projection. Arguments/environment, loaded-core provenance and normal
launch integration remain unfinished. Catalog stays 91/95 cores (95.8%), 289
profiles and 157 layouts; overall/MAME completion unknown. Wheels stay paused.
Source, formatting and whitespace checks only; no tests, builds or runtime probes.

Step 452: added capture-projection comparison for candidate launch configuration.
It reuses the exact capture encoder, compares the resulting configuration to the
retained fingerprint, and checks deck/content/config contracts. Binding/core and
other non-projected changes cannot be hidden by different JSON formatting.
Suppressed startup actions still require separate launch policy; a projection
match is not launch approval. Ordinary integration remains unfinished. Catalog
stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion
unknown. Wheels stay paused. Source, formatting and whitespace checks only;
no tests, builds or runtime probes.

Step 451: capture results retain exact digital-deck topology, media mode and
capture-configuration SHA-256. Reuse checks compare deck/media identity before
repeating full definition/content validation and retained artifact checks. A
separate byte-exact config check detects a different capture setup; it is not
misapplied to ordinary launch configs with intentionally different policies.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Launch integration and runtime provenance remain unfinished.
Wheels stay paused. Source, formatting and whitespace checks only; no tests,
builds or runtime probes.

Step 450: captured deck results retain executable, optional managed assembly and
selected Waterbox fingerprints. The same baselines are checked by child preparation,
after capture and on definition access; a separate runtime-file recheck is exposed
for handoff. Baselines are not refreshed after capture. Transitive dependencies,
loaded-core/build provenance and ordinary launch integration remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Wheels stay paused. Source, formatting and whitespace checks
only; no tests, builds or runtime probes.

Step 449: prepared deck capture now returns an owned result retaining its
cartridge/database identity snapshot beyond child completion. Read-only definition
access rechecks retained cartridge inputs; an explicit cartridge-evidence check
fails for disc observations instead of treating absent evidence as success.
This provides a recheckable handoff object, not ordinary launch integration or
complete runtime/core provenance. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Wheels stay paused. Source,
formatting and whitespace checks only; no tests, builds or runtime probes.

Step 448: cartridge capture identity now uses the owned configuration's exact
PreferredPlatformsForExtensions selection when the database/loader leaves the
system empty. Known systems retain precedence. Added .rom normalization beside
.bin; missing, empty or malformed preferences fail instead of using the requested
deck as evidence. Capture validates and shares one owned configuration read with
the identity resolver. Catalog stays 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Ordinary launch integration remains
unfinished. Wheels stay paused. Source, formatting and whitespace checks only;
no tests, builds or runtime probes.

Step 447: added pinned 1 MiB SNES/Satellaview database-miss detection to cartridge
identity preparation: LoROM/HiROM scoring, title-character predicate and BSX/SNES
identity distinction. Shift-JIS decoding uses pinned encoding_rs 0.8.35;
malformed titles fail explicitly pending .NET replacement parity. The upstream
unconditional checksum helper is not misrepresented as ROM checksum validation.
No Satellaview deck is enabled. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Wheels stay paused. Dependency
lock resolution, source, formatting and whitespace checks only; no tests,
builds or runtime probes.

Step 446: prepared native digital-deck capture now independently checks cartridge
system/hash before spawning, using retained file fingerprints, normalized bytes
and the ordered effective database snapshot. It checks input/database stability
again after capture. Database misses use traced fixed extension rules; ambiguous
extensions and 1 MiB SNES/Satellaview detection remain explicit gaps. Disc identity,
build policy/core provenance, post-identity patches and ordinary launch handoff
remain unfinished. No capture was run. Catalog stays 91/95 cores (95.8%), 289
profiles and 157 layouts; overall/MAME completion unknown. Wheels stay paused.
Source, formatting and whitespace checks only; no tests, builds or runtime probes.

Step 445: connected database snapshot root derivation to the explicit Linux
direct-Mono launch environment. It requires unique existing absolute
BIZHAWK_HOME/BIZHAWK_DATA_HOME values, checks the installation identity, and
uses their gamedb subdirectories as pinned PathUtils/GameDBHelper specify.
Inherited, relative, invalid and wrapper-selected environments are not guessed.
The entry point is implemented but capture/launch does not call it yet; database
policy, patching and content expectation handoff remain unfinished. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Wheels stay paused. Source, formatting and whitespace checks only; no tests,
builds or runtime probes.

Step 444: implemented ordered BizHawk bundled/user database include loading,
user-root fallback, optional-missing tracking and last-record-wins indexing.
Snapshots retain root identities and input fingerprints and recheck them before
lookup. Include cycles, depth/count/byte overruns, unsupported paths/encodings
and malformed records fail preparation rather than yielding partial provenance.
Runtime root selection, patching and capture/launch integration remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Wheels stay paused. Source, formatting and whitespace checks
only; no tests, builds or runtime probes.

Step 443: implemented BizHawk database record parsing and ordered digest-index
lookup on normalized cartridge bytes. Later records replace matching keys;
lookup prefers SHA-1, then MD5, then CRC32. Parsed records preserve system,
name, patch metadata, region and forced-core fields at their source-defined
positions. Bounded primitives reject directives and malformed digest keys;
include expansion, effective-database ownership and launch integration remain
unfinished. An index miss is not treated as proof of a complete database miss.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Wheels stay paused. Source, formatting and whitespace checks
only; no tests, builds or runtime probes.

Step 442: implemented BizHawk pre-database cartridge normalization for the six
digital-deck adapters' file-format families. The owned result retains normalized
bytes, original and normalized SHA-1, and header/SMD transformation details.
It preserves the pinned header exceptions, SMD 4 MiB cap and zero-filled tail.
Non-cartridge/unknown extensions are rejected rather than applying these rules
to discs or unrelated loaders. Database resolution, post-lookup patching and
launch integration remain unfinished; the helper is not yet called by launch.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Wheels stay paused. Source, formatting and whitespace checks
only; no tests, builds or runtime probes.

Step 441: traced BizHawk's independent cartridge-identity prerequisite for native
definition capture. The pinned loader normalizes content before database lookup;
its reported identity can be SHA-1, MD5 or CRC32 and precedes later patching.
Recorded the exact source rules and remaining provenance/handoff implementation
in BIZHAWK_DEFINITION_CAPTURE.md. No new mapping or launch coverage is claimed:
the automatic capture gate is still unfinished. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; four catalog core entries remain without
enabled profiles, while overall/MAME completion is unknown. Wheels stay paused.
Source audit and whitespace inspection only; no tests, builds or runtime probes.

Step 440: source diagram hardware-repeat hotspots now select/cycle the declared
base control's mappings, inherit its calibration-gap indication, and explain
that they are not independent inputs. Owner lookup requires unique catalog
identities and a non-repeated base; destination action identities remain exact.
Connection geometry and mapping counts are unchanged. This covers the N30 turbo
source layout in the shared visual mapper, including MAME review. Wheels remain
paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Source/whitespace inspection only; no tests,
builds, UI runs or device probes.

Step 439: calibration has a direct required/optional control picker, including
recording-presence labels. It uses exact layout control identities and excludes
repeated diagram aliases. Selecting a control preserves other bindings and uses
the existing single-control recording/pause path; selection is disabled during
pending capture. This lets users reach optional controls or another repair after
targeted calibration without starting over. Presence labels do not establish
measurement validity. Wheels stay paused. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion unknown. Source/whitespace
inspection only; no tests, builds, UI runs or device probes.

Step 438: calibration shows required-control recording presence in the wizard
draft and can target the next missing required control without resetting other
bindings. Progress no longer follows the sequential step index, which counted
skips as advancement; provisional captures are excluded. Optional/repeated layout
positions are not added to the required denominator. End-of-layout/paused labels
no longer imply every control was recorded. Counts are explicitly not validated
measurements or game coverage. Wheels stay paused. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Source/whitespace inspection only; no tests, builds, UI runs or device probes.

Step 437: calibration no longer attaches saved bindings to the first catalog
layout when their saved layout is missing. New/unknown layouts require selection;
cross-OS measurements are not loaded. Diagram/preview/save controls tolerate the
unselected state, and stored calibration remains unchanged until an explicit save.
Duplicate-input messages also tolerate obsolete control IDs. Wheels stay paused.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds, UI runs
or device probes.

Step 436: the calibration wizard retains its loaded serialized calibration and
uses a guarded model save. The model compares that controller's current state
with the baseline before calling the existing validated save path. A changed
calibration reports a conflict without overwriting saved data or discarding the
wizard draft; unrelated controller changes do not invalidate it. Existing save
validation and connected-device requirements remain. Wheels stay paused. Catalog
stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion
unknown. Source, Rust formatting and whitespace checks only; no tests, builds,
UI runs or device probes.

Step 435: input diagnostics now report whether the published binding started a
new backend calibration capture or arrived while one was already pending. The
wizard accepts only new starts, asking for release/settling before retrying later
presses. The flag is published with the binding before its input revision. This
prevents a rejected initial gesture from lending its completion to a subsequent
attempt; normal input diagnostics still report those events. Wheels remain paused.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Source, Rust formatting and whitespace checks only; no tests,
builds, UI runs or device probes.

Step 434: calibration now requires a matching completion for digital controls as
well as measured axes. Backend code, normalized kind/direction and physical-code
presence/identity must agree; raw native axis polarity may still be refined by
the release measurement. Missing/mismatched completion data triggers retry and
restores the previous binding instead of accepting a provisional recording.
Source confirms the backend completion owns its initial captured binding.
Wheels stay paused. Catalog remains 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Source/whitespace inspection only;
no tests, builds, UI runs or device probes.

Step 433: calibration replacement capture now retains the previous binding until
the recording completes. Measurement errors, interrupted control selection and
Back restore the prior binding (or remove only a new provisional one). Successful
completion clears the retained state; Skip deliberately removes the selected
binding after cancelling any pending attempt. Fresh-open/start-over clear pending
state. This protects targeted and sequential calibration without changing save
authority. Wheels remain paused. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Source/whitespace inspection
only; no tests, builds, UI runs or device probes.

Step 432: targeted physical-control calibration now pauses input recording after
that control is completed or explicitly skipped, instead of advancing into other
saved bindings. A retry action records the same control again. Back-navigation is
disabled in targeted mode; opening normal calibration or explicitly starting over
resets it. Only Use calibration applies the wizard draft. The ordinary sequential
workflow is unchanged. Wheels remain paused. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion unknown. Source/whitespace
inspection only; no tests, builds, UI runs or device probes.

Step 431: MAME review identifies each player's selected preset as inherited or
overridden, both beside the diagram and in the technical report. Shared-preset
edits explicitly list preserved per-player overrides and invalidate review.
Selection labels remain distinct from the resolved destination geometry and
physical/runtime evidence. No mapping semantics changed. Wheels stay paused.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI runs.

Step 430: MAME player/preset edits may retain an existing controller identity
whose calibration is unavailable, allowing layout work before calibration repair.
The identity must exactly match that port in the unchanged base draft, remain
nonempty/bounded and satisfy controller uniqueness. New/replacement identities
still require valid saved calibration choices. Apply reports retained gaps and
invalidates review; staging calibration checks are unchanged. Wheels remain
paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Source/whitespace inspection only; no tests,
builds, UI runs or device probes.

Step 429: the MAME player editor exposes per-player destination presets with an
explicit inherit-shared option. Controller replacement preserves that player's
preset; new players inherit; removing players removes their layout overrides on
Apply. Apply validates preset IDs, preserves explicit input assignments and
invalidates review. Cancel leaves the draft unchanged. Shared preset labels are
reused across selectors. Wheels remain paused. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion unknown. Source/whitespace
inspection only; no tests, builds, UI runs or device probes.

Step 428: saved MAME setups support optional player_digital_layouts overrides,
with omitted ports inheriting digital_layout. Settings validation, assignment
review, staging and calibrated launch resolve the effective per-player preset.
Unselected-port overrides are rejected both in settings and at preparation;
automatic setups pass an empty override map. Session descriptions identify each
player's effective preset. Native field/channel routing is unchanged. This step
adds backend/JSON support; the dedicated per-player UI editor is still pending.
Wheels stay paused. Catalog remains 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Source, Rust formatting and whitespace
checks only; no tests, builds, UI runs or device probes.

Step 427: corrected a remaining pre-review blocker missed by Step 425: stored
MAME settings validation still demanded a nonempty digital profile for ports
without explicit mappings. It now validates optional digital profiles consistently,
allowing zero-mapping drafts to reach the unresolved-input review. Present-profile
geometry/type errors still fail. Staging's explicit per-player profile requirement
and launch preparation checks remain intact. Per-player layout overrides were
investigated but not implemented. Wheels stay paused. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown. Source,
Rust formatting and whitespace checks only; no tests, builds or UI runs.

Step 426: MAME assignment/removal previews now name newly unresolved and no-longer
unresolved native fields alongside route changes. Comparison uses exact field
identity, rejects duplicates/missing comparison data and carries setup guidance
for new gaps. Leaving the unresolved list is explicitly not called physical
mapping coverage or playability. Drafts remain unchanged by preview. Wheels stay
paused. Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Source/whitespace inspection only; no tests,
builds, UI runs or device probes.

Step 425: MAME assignment review now consistently uses the explicit-field
composition planner, including when both explicit-assignment lists are empty.
Zero-mapping drafts can expose their unresolved inputs instead of failing the
digital launch planner's minimum-mapping check. This also enables baseline/removal
previews when no ordinary defaults resolve. Staging and launch retain their own
planner/completeness checks; no launch readiness is inferred from review success.
Wheels remain paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Source, Rust formatting and whitespace
checks only; no tests, builds, UI runs or device probes.

Step 424: MAME's unresolved-switch shortcut now optionally includes keyboard and
auxiliary switch fields already accepted by the explicit planner. Ordinary
controller fields remain the default view; the opt-in resets on opening review.
Exact inspected-field/class checks and fresh-review checks guard editor entry.
Analog, unknown, internal and machine-setting fields remain excluded. Keyboard
owner/enable-state validation remains mandatory in the backend. Wheels remain
paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Source/whitespace inspection only; no tests,
builds, UI runs or device probes.

Step 423: unresolved MAME fields now carry planner-result guidance distinguishing
ordinary switches, analog modes, keyboard evidence and unknown contracts. Review
shows it in the unresolved summary, selected-action guidance and technical report.
Messages explain limited default channels and intentional sharing without claiming
the exact cause is proven, or equating unresolved with disabled. Mapping/staging
semantics are unchanged. Wheels stay paused. Catalog remains 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion unknown. Source, Rust
formatting and whitespace checks only; no tests, builds, UI runs or device probes.

Step 422: MAME review can open calibration at the highlighted saved physical
control. The shortcut requires a source layout and physical ID; the wizard checks
the exact current layout/control and resolves declared repeated-position owners
before selecting a calibration step. Missing/mismatched controls leave the normal
wizard start with an explanation, without resetting bindings. General player
calibration remains available for unmapped rows. No destination-to-source guess
is made. Wheels remain paused. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Source/whitespace inspection
only; no tests, builds, UI runs or device probes; navigation remains unverified.

Step 421: mixed ordinary-plus-twin digital direction clusters now produce a
partial MAME review instead of an early topology error. When twin-stick fields
are present, ordinary-cluster defaults are omitted so the third cluster cannot
silently alias the left stick. Its exact fields become unresolved/editable;
known twin-stick mappings remain visible. Unselected-port disabling is preserved,
and unresolved fields still block staging/launch. This does not implement an
automatic third independent stick. Wheels remain paused. Catalog stays 91/95
cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Source, Rust formatting and whitespace checks only; no tests, builds or UI runs.

Step 420: fresh MAME reviews retain the selected source-player port within the
same exact emulator/core/content/hash/machine/library context. This navigation
preference survives calibration-driven review invalidation, but removed ports
or changed contexts fall back to the first available player (none for an empty
list). New results still replace all review evidence and staging guards; no old
mapping approval is reused. Wheels remain paused. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Source/whitespace inspection only; no tests, builds, UI runs or device probes.

Step 419: MAME review now links directly to the selected player's existing
calibration wizard by exact saved controller identity. The parent requires one
current inventory match; missing/ambiguous identities produce an error rather
than selecting another device. The per-game draft stays intact, and the previous
review is invalidated before opening calibration, even if it is later cancelled.
No layout reset or calibration write is performed by the navigation handler.
Wheels remain paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Source/whitespace inspection only;
no tests, builds, UI runs or device probes; popup/focus behavior is unverified.

Step 418: MAME's switch editor now bounds its height and scrolls the form while
keeping apply/remove controls outside that scroll area. Ordinary button guidance
is concise; keyboard and incremental-axis guidance appears only for those fields.
Native channel tokens and draft-sharing counts are optional details, while axis
sharing/threshold warnings stay visible. The details toggle does not replace the
channel model or intentionally alter selection. Mapping semantics are unchanged.
Wheels stay paused. Catalog remains 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Source/whitespace inspection only;
no tests, builds or UI runs, so visual sizing and focus behavior remain unverified.

Step 417: MAME switch overrides now have a removal preview using the same exact
before/after route comparison as assignment preview, plus unresolved-field counts.
It operates on the saved field/sequence independently of unsaved source-channel
selection. Removal requires one matching saved override; nonexistent/ambiguous
removals no longer report success. Preview leaves the draft unchanged and warns
about analog-direction semantics without claiming live behavior. Wheels remain
paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Source/whitespace inspection only; no tests,
builds, UI runs or device probes.

Step 416: MAME default channel exhaustion now produces a partial, actionable
review instead of aborting it. Ordinary layouts retain up to ten action channels;
twin-stick layouts retain up to six. Excess actions have no generated sequence
and enter the existing unresolved-field list and exact-field assignment picker.
They are neither silently shared nor marked NONE/disabled. Unselected ports keep
their intentional disabled mappings, and staging plus both native launch paths
still reject unresolved fields. This does not add independent transport channels.
Wheels stay paused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion unknown. Source, Rust formatting and whitespace checks
only; no tests, builds, UI runs or device probes.

Step 415: sparse native MAME button numbers above 10 no longer force fixed-pad
geometry when their allocated frontend channels fit an arcade preset. Automatic
chooses six/eight/fixed geometry from actual channel requirements; explicit
six/eight/Neo Geo presets accept fitting sparse assignments. Both default and
override-composed profile paths follow this rule. Exact action identities remain
in review, with a warning that diagram positions are not native button numbers.
Native routing, independent-channel limits and twin-stick rules are unchanged.
Wheels stay paused. Catalog remains 91/95 cores (95.8%), 289 profiles and 157
layouts; overall/MAME completion unknown. Source, Rust formatting and whitespace
checks only; no tests, builds, UI runs or device probes.

Step 414: MAME channel-effects preview now compares current and proposed resolved
switch routes across every selected source player. Exact native field identity
plus sequence keys distinguish shared labels and cross-player moves. The report
names added, removed, moved and default/override route changes, rejecting duplicate
keys or unavailable baseline data. Missing switch routes are not described as
disabled inputs. No drafts/settings are mutated by preview. Wheels remain paused.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Source/whitespace inspection only; no tests, builds or UI runs.

Step 413: MAME's switch editor can preview a proposed channel assignment through
the existing review resolver without modifying the draft. The bounded scrolling
report names same-channel actions including inferred defaults, opposing-axis
switches and explicit analog fields sharing the axis. Selection, draft and
controller-revision changes invalidate the preview. Resolver failures are shown
as failures rather than an empty/clear conflict report. Complete post-edit review
remains required. Wheels stay paused. Catalog remains 91/95 cores (95.8%), 289
profiles and 157 layouts; overall/MAME completion unknown. Source/whitespace
inspection only; no tests, builds, UI runs or device probes.

Step 412: MAME review derives the game profile before checking physical
calibration. A missing/invalid calibration now retains the destination diagram,
required-control rows and exact native action labels. All such rows have null
physical IDs and inputs, with explicit calibration gaps; no source connections or
native-measurement percentage are fabricated. The calibration-error branch also
explicitly marks physical preparation pending. Existing calibrated planning is
unchanged. Wheels remain paused. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Source, Rust formatting and
whitespace checks only; no tests, builds, UI runs or device probes.

Step 411: MAME review now retains per-player semantic switch actions independently
of physical calibration and diagram rows. A complete ordinary-controller action
picker opens the existing exact-field editor even when calibration is unavailable.
Editing from either picker or diagram requires a unique current-player route match;
draft freshness and calibration/staging gates remain unchanged. This adds an
editing path, not measured input evidence. Wheels remain paused. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Source, Rust formatting and whitespace checks only; no tests, builds or UI runs.

Step 410: MAME's visual numbered-action swap picker now includes the existing
LeftTrigger/RightTrigger switch channels, allowing ordinary actions on channels
9/10 to participate alongside the first eight. Each candidate must uniquely match
an exact current reviewed route, and every frozen numbered field must uniquely
match the inspected draft before mutation. Shared-channel actions remain excluded;
calibration, native threshold handling and fresh-review requirements are unchanged.
Wheel work remains paused. Catalog stays 91/95 cores (95.8%), 289 profiles and
157 layouts; overall/MAME completion unknown. Source and whitespace inspection
only; no tests, builds, UI runs or device probes.

Step 409: generated Lua now catches definition-observation failures and publishes
a bounded schema-v2 error envelope atomically. Host parsing verifies version,
nonce, message length and strict fields, then reports escaped child diagnostics
instead of accepting a definition or waiting for timeout. File-publication errors
still rely on stderr/timeout. Catalog stays 91/95 cores (95.8%), 289 profiles and
157 layouts; overall/MAME completion unknown. Provenance and launch integration
remain unfinished. Formatting/whitespace only; no tests or capture scripts run.

Step 408: capture now collects nonblocking stderr diagnostics with bounded
per-poll reads, a 1 MiB total-output limit and an 8 KiB retained tail. Failure
messages include escaped, explicitly labelled child output; cleanup errors are
preserved. No pipe drain runs on the UI thread, and no capture was executed.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Provenance/launch integration remain unfinished. Formatting
and whitespace checks only; no tests, captures or emulator runs.

Step 407: capture schema v2 now records pause and frame-count evidence before
and after controller-definition observation. Generated Lua and host parsing
require paused endpoints and the same nonnegative frame count; v1/missing fields
are rejected. This checks the reported observation interval, not zero frames
since startup or authenticated provenance. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion unknown. Formatting and
whitespace checks only; no tests, captures or emulator runs.

Step 406: private capture configs explicitly disable SingleInstanceMode and
background controller/hotkey input. Pinned MainForm checks single-instance
forwarding before normal startup and uses the background flags for input routing.
This prevents inheriting those saved policies; it does not block focused-window
interaction or establish complete process isolation. Only capture copies change.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion unknown. Formatting/whitespace only; no tests or emulator runs.

Step 405: deck-aware capture now fingerprints the selected Snes9x/TurboNyma/GPGX
Waterbox payload and retains it through spawn, response reading and cleanup.
Normal session preparation and capture share the same BIZHAWK_HOME resolution
guard, rejecting a different selected installation. Capture retains its prepared
base directories. This is top-level payload integrity, not a transitive runtime
manifest or loaded-core provenance. Catalog stays 91/95 cores (95.8%), 289
profiles and 157 layouts; overall/MAME completion unknown. Formatting/whitespace
only; no tests, captures or emulator runs.

Step 404: completed prepared-config preflight dispatch for all six native
digital decks. SNES ports, NES ports/Famicom expansion, SMS standard ports and
keyboard disable, PCE fixed ports and TurboNyma port devices/multitap are checked
against encoder fields alongside explicit core preference and no-fallback policy.
Checks run at deck-aware preparation and before spawn; GPGX retains its existing
preflight. This is config agreement, not runtime/core/content provenance or an
enabled normal-launch gate. Catalog stays 91/95 cores (95.8%), 289 profiles and
157 layouts; overall/MAME completion unknown. Formatting/whitespace checks only;
no tests, captures or emulator runs.

Step 403: GPGX deck-aware capture now checks the integrity-guarded generated
config at preparation and before spawn. PreferredCores.GEN must be Genplus-gx
with fallback disabled; UseSixButton and both native port-type values must
match the requested topology, including adapters/Activator. Missing or differently
typed selection fields fail. This proves config agreement only, not loaded-core
or game-override provenance. Other core config preflights and normal-launch
integration remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Formatting/whitespace only;
no tests, captures or emulator runs.

Step 402: deck-aware preparation now returns a private-field owner retaining
the exact deck, system, expected hash and media metadata with its capture files.
Its consuming worker method checks cancellation/timeout before spawning, then
compares against that retained request after owned-child cleanup. Callers cannot
replace the request through this API between preparation and comparison. This
does not prove source-config topology or runtime/core/content provenance and is
not wired into normal launch. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Formatting/whitespace only;
no tests, capture preparation or emulators run.

Step 401: shared native definition requests now validate topology, system/media
agreement, expected hash and FDS side bounds before capture preparation or waiting.
The new deck-aware preparation entry point checks requests before creating
private files. Exact-definition comparison repeats these checks. Invalid supplied
requests no longer need a successful response to be rejected. This does not
derive provenance or enable launch gating. Catalog stays 91/95 cores (95.8%),
289 profiles and 157 layouts; overall/MAME completion unknown. Formatting and
whitespace checks only; no tests, capture preparation or emulators run.

Step 400: private BizHawk capture configs now request StartPaused and disable
periodic SaveRAM autosaving. Pinned MainForm sets startup pause before Shown's
Lua loading and resumes non-frame-waiting scripts outside frame advancement;
the generated capture script never requests a frame. This reduces capture-side
gameplay/persistence effects, not a complete sandbox or proof of zero frames.
Core/content provenance and launch integration remain unfinished. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Formatting/whitespace checks only; no tests, captures or emulator runs.

Step 399: connected all six native digital-deck validators through a shared
DigitalDeck definition/capture entry point. Independently supplied system, hash
and typed cartridge/disc/NES console expectations select the matching validator;
incompatible medium/deck combinations fail. PCEHawk runtime CD-system identity
is enforced. This is shared comparison plumbing, not an enabled launch gate.
Provenance and automatic expectation derivation remain unfinished. Catalog stays
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Formatting/whitespace only; no tests, captures or emulator runs.

Step 398: added TurboNyma exact captured-definition comparison and owned-capture
wrapper. Fixed connected ports require all 14 native pad/mode inputs; Power/Reset
are always expected, while explicit CD mode requires Open/Close Tray and exactly
the Disk Index axis. Invalid multitap topology, system/hash mismatch, duplicates
and missing/extra controls fail. Axis ranges, core/content provenance and launch
integration remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall/MAME completion unknown. Formatting/whitespace checks
only; no tests, captures or emulator runs.

Step 397: added SMSHawk exact-definition validation and owned-capture comparison
for SMS, Game Gear and SG-1000. SMS/SG expect both declared player ports plus
Reset/Pause even when only one port is mapped; GG expects P1 with Start and Reset.
Wrong system/hash, axes, duplicates and missing/extra controls fail, including
unexpected keyboard/peripheral declarations. Provenance and launch integration
remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall/MAME completion remain unknown. Formatting/whitespace only; no tests
or capture routines run.

Step 396: added PCEHawk exact captured-definition validation and owned-capture
comparison. Connected ports keep fixed P1–P5 identities; each needs eight native
buttons, with no invented console buttons. Exact expected runtime system/hash,
no axes, and no duplicate/missing/extra buttons are required. Empty decks fail.
Pinned disc construction reports PCECD, so callers supply runtime-system identity
explicitly. Provenance and launch integration remain unfinished. Catalog remains
91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Formatting/whitespace checks only; no tests, captures or emulator runs.

Step 395: added NesHawk exact-definition comparison and owned-capture wrapper
for mixed NES pads, Four Score slots, SNES adapters and Power Pads. Expectations
include Power/Reset plus independently supplied FDS side count and VS controls.
Wrong system/hash, axes, duplicates and missing/extra buttons fail. FDS capture
supports 1–256 explicit sides. Provenance and launch integration remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall/MAME
completion remain unknown. Formatting/whitespace only; no tests or captures run.

Step 394: added exact native Snes9x digital-definition validation and an owned
capture-comparison wrapper. Expected controls follow compacted connected joypad/
multitap slots (12 buttons each), Reset and Power; wrong system/hash, axes,
duplicates and missing/extra controls fail. Empty decks are rejected. This does
not establish mouse/gun support, core provenance or launch integration. Catalog
remains 91/95 cores (95.8%), 289 profiles and 157 layouts; overall and MAME
completion remain unknown. Formatting/whitespace only; no tests or captures run.

Step 393: connected owned capture completion to GPGX's exact definition
comparison, including requested topology, GEN system, explicit expected ROM
hash, disc buttons and rejection of unexpected axes/missing/extra controls.
The wrapper consumes the process owner and retains cancellation handling.
It does not derive ROM identity or authenticate the loaded core; pinned source
shows GameInfo.Hash may follow SHA1/MD5/CRC database lookup after ROM transforms.
Core provenance and launch integration remain unfinished. Catalog stays 91/95
cores (95.8%), 289 profiles and 157 layouts; overall/MAME completion unknown.
Formatting/whitespace checks only; no tests, captures or emulator runs.

Step 392: capture preparation now requires one explicit absolute content file,
appends it as the sole final positional argument and fingerprints it alongside
the config/runtime artifacts. The capture-only option parser accepts config and
display options, rejecting unknown options and extra positional content; archive
member selectors are rejected. This binds the selected top-level file, not disc
dependencies, extracted members or BizHawk's reported ROM hash. Those identities,
core binding and launch integration remain unfinished. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall/MAME completion remain unknown.
Formatting/whitespace checks only; no tests, preparation routines or emulators run.

Step 391: added consuming, worker-thread capture waiting with a 1 ns–60 second
deadline measured from child ownership, cancellation, 20 ms publication polling
and early-exit detection without reaping. Published invalid responses fail
immediately; every result passes through owned-group cleanup, and success
rechecks file integrity, cancellation and deadline. Cleanup failures are surfaced.
No routine was executed. Core/content binding and launch integration remain
unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall and MAME completion remain unknown. Formatting/whitespace checks only.

Step 390: added Linux capture-child ownership with a separate process group,
private child handle, retained config/script owners, and selected executable/
optional Mono assembly fingerprints. Spawn rechecks integrity; exit observation
uses WNOWAIT and cleanup stops the owned group before reaping. ECHILD suppresses
signals after ownership is lost. No process was started. Timeout/cancellation,
response polling, runtime/core/content binding and launch integration remain
unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall and MAME completion remain unknown. Formatting/whitespace checks only.

Step 389: added a Linux PreparedCapture owner combining private capture script,
private sanitized config and rewritten application arguments. It reuses existing
config-path resolution and duplicate-option rejection, requires absolute base
directories and UTF-8 arguments, and retains both temporary-file owners together.
Preparation and response reading check config/script integrity; input arguments
remain untouched on failure. Process ownership, core/content binding and launch
integration remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall and MAME completion remain unknown. Formatting and
whitespace checks only; no tests, builds, preparation routines or emulators run.

Step 388: BizHawk definition capture can prepare an owned private config copy
with recent Lua/session/ROM/movie/watch/cheat autoload and last-state loading
disabled, and empty capture-only common/custom tool settings. The original
config stays untouched; core/controller settings and unrelated fields are
preserved. This closes saved-startup preparation gaps identified in pinned
LuaConsole, MainForm and ToolManager source. Child ownership, config selection
in launch arguments and runtime binding remain unfinished. Catalog remains
91/95 cores (95.8%), 289 profiles and 157 layouts; overall and MAME completion
remain unknown. Formatting/whitespace checks only; no tests, builds or runs.

Step 387: BizHawk definition-capture ownership now prepares application
arguments with an integrity-checked private --lua script path. Preparation
rejects competing Lua sessions, external tools, movie/state loading, dump/exit
options, response files, terminators and oversized/control-character arguments.
It returns a new argument vector without mutating the caller's launch plan;
direct-Mono prefixes must stay separate. No process launch or capture integration
is claimed. Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts;
overall and MAME completion remain unknown. Formatting/whitespace checks only;
no tests, builds, scripts, emulator runs or probes.

Step 386: MAME review now reports per-player saved native measurement coverage
separately from semantic source-assignment coverage. Counts use required mapping
rows and the same native-measurement predicate as staging, with missing counts
and an explicit no-denominator state. Missing calibration/profile reports no
percentage, not 100%; saved measurements are not runtime verification. Catalog
remains 91/95 cores (95.8%), 289 profiles and 157 layouts. Overall and MAME
completion percentages remain unknown. Formatting/whitespace checks only;
no tests, builds, UI runs or device probes.

Step 385: MAME visual review passes existing native-measurement gap evidence
into the shared mapping diagram. Affected source/destination hotspots and drawn
connections are marked red; tooltips, accessible descriptions, assignment-list
text and selected-row guidance explicitly identify calibration gaps. Missing
sources remain unmapped rather than acquiring fabricated connections. Other
callers default to no supplied gap evidence, not verified readiness. Catalog
remains 91/95 cores (95.8%), 289 profiles and 157 layouts; overall and MAME
completion remain unknown. Source/whitespace inspection only; no tests or runs.

Step 384: combined MAME analog profiles now honor explicit six-button,
eight-button and Neo Geo geometry when the default digital subset is empty.
Ordinary explicit Automatic switch mappings resolve their six/eight-button
geometry before analog composition; pure analog Automatic retains fixed channels.
Existing derived analog layouts, native routes and calibration/conflict gates
are reused. Catalog stays 91/95 cores (95.8%), 289 profiles and 157 stored layouts.
Overall and MAME completion remain unknown. Formatting and whitespace checks
only; no tests, builds, UI runs or device probes.

Step 383: corrected MAME's explicit-only Automatic profile fallback. When no
default bindings survive and ordinary switch outputs are mapped explicitly,
the destination now retains six-button arcade geometry instead of reverting
to a fixed-channel gamepad. Existing extended-channel and eight-button selection
still precede this fallback; analog profile handling and native routes are
unchanged. Catalog remains 91/95 cores (95.8%), 289 profiles and 157 layouts.
Overall and MAME completion remain unknown. Rust formatting and whitespace
checks only; no tests, builds, UI runs or device probes.

Step 382: MAME visual review can restore a selected player's own numbered
buttons to the preset in one draft edit, including overrides frozen by swaps.
The action displays its override count and checks review freshness, selected
player and unique inspected field identities before mutation. Cross-player
routes, other players, directions, service and analog assignments are preserved;
fresh review is required afterward. Catalog remains 91/95 cores (95.8%), 289
profiles and 157 layouts. Overall and MAME completion percentages remain unknown.
Source/whitespace inspection only; no tests, builds or runtime checks.

Step 381: MAME visual review now offers all unresolved ordinary controller
buttons in an exact-field picker with a direct assignment-editor shortcut.
The shortcut checks review freshness, unresolved membership and unique inspected
field identity before opening the existing draft-only editor. Analog, keyboard,
miscellaneous and internal inputs are not promoted into ordinary button setup.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; this closes a
UI workflow gap, not a new core contract. Overall and MAME completion remain
unknown. Source and whitespace inspection only; no tests, builds or runtime
checks. Wheel work remains paused.

Step 380: added private capture-file ownership with random nonces, protected
script creation, script-integrity checks and bounded no-follow/nonblocking reads.
Lua publishes the finished response by atomic rename. Reader rejects unexpected
file types, hard links, owner mismatches and concurrent file replacement/change.
No process is launched by this owner; process/core/content binding and launch
enforcement remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall and MAME completion remain unknown. Formatting and
whitespace checks only; no tests, builds, scripts, UI runs or probes executed.

Step 379: added exact GPGX captured-definition comparison against requested
per-player pad/Activator controls, console/disc buttons, GEN system ID and an
explicit expected content hash. Missing/extra controls, duplicates and unexpected
axes are rejected. This does not establish capture provenance, core identity or
native slot indices; process ownership and launch enforcement remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 157 layouts; overall and MAME
completion remain unknown. Formatting/whitespace only; no tests, builds, capture
scripts, UI runs or device probes executed.

Step 378: added Rust-generated one-shot BizHawk Lua control-definition capture
and a bounded response parser. Uses inspected getimmediate/system-ID/ROM-hash
APIs, captures button/axis names without advancing frames or injecting input,
and checks nonce, schema, sizes and duplicate names. This is not yet connected
to process ownership, core identity or launch enforcement; parsed data alone is
not trusted runtime evidence. Catalog stays 91/95 cores (95.8%), 289 profiles
and 157 layouts; overall and MAME completion remain unknown. Formatting and
whitespace checks only; no tests, builds, scripts, UI runs or probes executed.

Step 377: added GPGX per-connector Activator editor controls, conflict gates,
socket labels and per-player visual target selection for mixed Activator/pad
setups. Saved physical/SDL previews surface sensor-independence failures; invalid
topologies do not silently preview as ordinary pads. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall and MAME completion remain unknown.
Trusted runtime capture, content overrides and other peripherals remain unfinished.
Formatting/whitespace only; no tests, builds, UI runs or device probes.

Step 376: connected GPGX Activator to per-connector saved topology, native config
encoding, translation/session normalization and saved-player validation. Mixed
Activator/normal-pad setups select each player's own control contract; overlapping
multitap/Activator choices are rejected. Loaded-input validation recognizes the
Activator's system/device codes. Editor and preview integration remain unfinished,
as do trusted runtime capture and content overrides. Catalog stays 91/95 cores
(95.8%), 289 profiles and 157 layouts; overall and MAME completion remain unknown.
Formatting/whitespace only; no tests, builds, UI runs or device probes.

Step 375: added a sixteen-channel GPGX Activator schematic and calibrated
raw/logical translation. Independence checks reject shared physical button/axis
identities and shared SDL outputs between sensors. The diagram is a labeled
channel grid, not a claimed physical ring orientation. Connector/config/session
and editor integration remain unfinished. Catalog stays 91/95 cores (95.8%),
289 profiles; stored layouts increase to 157. Overall and MAME completion remain
unknown. Formatting/whitespace only; no tests, builds, UI runs or device probes.

Step 374: shared native sessions now retain integrity checks for their selected
Waterbox payload: GPGX gpgx.wbx, TurboNyma turbo.wbx and Snes9x snes9x.wbx.
Conflicting effective BIZHAWK_HOME paths are rejected before hashing the configured
installation's dll payload. This closes a preparation-time core-file change gap,
not loaded-input verification. Catalog stays 91/95 cores (95.8%), 289 profiles
and 156 layouts; overall and MAME completion remain unknown. Formatting and
whitespace checks only; no tests, builds, UI runs or device probes.

Step 373: GPGX per-player assignment now selects labeled connector/adapter
sockets rather than an unlabeled numeric port. Already-assigned sockets are
identified and out-of-topology saved players stay visible without silently
clamping their stored slot. Existing complete/unique assignment gates remain
authoritative. Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts;
overall and MAME completion remain unknown. J-Cart/content overrides and trusted
runtime capture remain unfinished. Whitespace checks only; no tests, builds,
UI runs or device probes.

Step 372: added GPGX Team Player/4-Way Play editor controls, expanded socket
labels/player capacity and topology conflict gates. Pad/connector edits now
preserve adapter fields; existing players remain for explicit correction.
Supports staging up to eight requested pad assignments through existing native
paths. J-Cart/content overrides and trusted runtime capture remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts; overall and MAME
completion remain unknown. Whitespace checks only; no tests, builds, UI runs or probes.

Step 371: extended GPGX topology/config encoding with per-connector Team Player
and paired 4-Way Play, using the pinned Genesis Plus GX input initializer.
Supports up to eight requested pads, exact device-slot placement and separate
serialized ControlType versus runtime INPUT_SYSTEM values. Loaded-device
validation now checks exact slot positions, not just counts. Editor support,
J-Cart/content overrides and trusted runtime capture remain unfinished. Catalog
stays 91/95 cores (95.8%), 289 profiles and 156 layouts; overall and MAME completion
remain unknown. Formatting/whitespace only; no tests, builds, UI runs or probes.

Step 370: added GPGX post-load input-contract validation for the two native
system fields and eight device slots. Rejects changed connector systems,
unexpected devices/pad modes and incorrect pad counts; returns actual slot
indices in compacted player order. This validator is not yet connected to a
trusted runtime capture or launch gate, so content-override coverage remains
unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts;
overall and MAME completion remain unknown. Formatting and whitespace checks
only; no tests, builds, UI runs or device probes.

Step 369: added GPGX editor selection, left/right normal-pad toggles, shared
three/six-button mode, saved topology round-trip and source/destination previews.
The UI displays compacted player-to-connector routing and blocks recording until
all connected pad slots are assigned. Preview uses the gpgx logical-calibration
scope. Multitaps and game-driven overrides remain unfinished and runtime behavior
is unverified. Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts;
overall and MAME completion remain unknown. Formatting and whitespace checks
only; no tests, builds, UI runs or device probes.

Step 368: added optional GPGX saved topology, dedicated logical-calibration
scope, Genesis/Mega Drive/Sega CD platform routing and selected three/six-button
saved-input validation. The shared digital-core exclusivity and compacted-slot
checks apply; absent fields preserve legacy behavior. Editor integration,
multitaps and loaded-game overrides remain unfinished. Catalog stays 91/95 cores
(95.8%), 289 profiles and 156 layouts; overall and MAME completion remain unknown.
Formatting and whitespace checks only; no tests, builds, UI runs or device probes.

Step 367: connected GPGX ordinary-pad preparation to shared native-session
dispatch, including compacted slot validation, three/six-button normalization
targets, owned configuration lifetime and device-routing guards. Requested versus
loaded-device warnings remain explicit. Saved settings/editor selection, content
override handling and multitaps remain unfinished. Catalog stays 91/95 cores
(95.8%), 289 profiles and 156 layouts; overall and MAME completion remain unknown.
Formatting and whitespace checks only; no tests, builds, UI runs or device probes.

Step 366: connected GPGX calibrated input translation to snapshot-based config
composition and transactional private-config argument preparation. Requires
complete topology assignments and distinct SDL devices, retains per-player
translation warnings and identifies each requested physical connector. Loaded
game overrides remain explicitly unverified; session/settings/editor integration
and multitaps remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles
and 156 layouts; overall and MAME completion remain unknown. Formatting and
whitespace checks only; no tests, builds, UI runs or device probes.

Step 365: added GPGX native Genesis/CD configuration encoding for ordinary
three/six-button pads. Writes shared pad mode and left/right None/Normal settings,
requires exact per-player controls, rejects duplicate input strings and preserves
console/disc buttons while clearing stale player analog/feedback/autofire routes.
Content overrides, multitaps and session/settings/editor integration remain
unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts;
overall and MAME completion remain unknown. Formatting and whitespace checks
only; no tests, builds, UI runs or device probes.

Step 364: added GPGX ordinary/empty connector topology, shared three/six-button
mode, compacted native player-to-connector resolution and complete unique-player
validation. Right-only normal pads resolve to P1, not P2. Multitap and content
override handling, configuration/session and editor integration remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts; overall and MAME
completion remain unknown. Formatting and whitespace checks only; no tests,
builds, UI runs or device probes.

Step 363: added source-grounded GPGX three/six-button pad translation and saved
calibration checks using existing Genesis schematics. Native control names are
explicit: eight controls for three-button, twelve including Mode for six-button.
Topology, configuration/session and editor integration remain unfinished; special
peripherals are separate contracts. Catalog stays 91/95 cores (95.8%), 289
profiles and 156 layouts; overall and MAME completion remain unknown. Formatting
and whitespace checks only; no tests, builds, UI runs or device probes.

Step 362: corrected TurboNyma's separate mode selectors to auxiliary emulator
actions in the catalog. The existing resolver can now assign available spare
buttons instead of rejecting every ordinary-pad candidate for the unknown menu
IDs. One-to-one assignment and native duplicate-input rejection remain intact;
insufficient physical controls remain unmapped rather than sharing a toggle.
Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts; overall and MAME
completion remain unknown. Static source/JSON inspection and whitespace checks
only; no tests, builds, UI runs or device probes.

Step 361: connected TurboNyma to native editor selection, saved topology
round-trip, fixed-port controls and source/destination previews. Added explicit
multitap control and recording gates for hidden P2-P5 or incomplete assignments;
existing assignments are retained for correction. Preview uses the fourteen-control
native layout and separate scoped SDL lookup. Catalog stays 91/95 cores (95.8%),
289 profiles and 156 layouts; overall and MAME completion remain unknown.
Formatting and whitespace checks only; no tests, builds, UI runs or device probes.

Step 360: added optional TurboNyma saved topology, digital-session/platform
routing, separate scoped SDL calibration and fourteen-control saved-input
validation. PCEHawk/TurboNyma conflicts on the same emulator ID are rejected
at settings validation because their platform routes overlap. Existing settings
retain their behavior when the new field is absent. Editor/preview integration
remains unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles and 156
layouts; overall and MAME completion remain unknown. Formatting and whitespace
checks only; no tests, builds, UI runs or device probes.

Step 359: added TurboNyma shared-session dispatch with fixed port identities,
multitap-aware layout selection, pre-probe topology validation and the existing
owned configuration/device-routing lifecycle. Inspected the DesiredInput override
hook and found no writer in the pinned PCE source; installed-runtime override
behavior is still unverified. Saved settings and editor routing remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts; overall and MAME
completion remain unknown. Formatting and whitespace checks only; no tests,
builds, UI runs or device probes.

Step 358: connected TurboNyma calibrated translation to snapshot-based config
composition and transactional private-config argument preparation. Topology is
validated before translation and repeated SDL device identities are rejected.
Per-player warnings are retained, including explicit initial-mode/content-override
limitations. Session, settings/UI and content-override handling remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles and 156 layouts; overall and MAME
completion remain unknown. Formatting and whitespace checks only; no tests,
builds, UI runs or device probes.

Step 357: added TurboNyma native configuration encoding for all fourteen pad
controls, fixed gamepad/none ports and explicit multitap selection. Ports 2–5
require the multitap; each connected port must have exactly one complete mapping.
Existing console/disc bindings are retained while stale player analog, feedback
and autofire bindings are cleared. Initial mode is not forced. Snapshot/session
and settings/UI integration remain unfinished. Catalog stays 91/95 cores (95.8%),
289 profiles and 156 layouts; overall and MAME completion remain unknown.
Formatting and whitespace checks only; no tests, builds, UI runs or probes.

Step 356: added source-grounded TurboNyma fourteen-control schematic and
calibrated raw/logical translation, including separate native two/six-button
mode selectors. Preserved the existing libretro toggle-style layout. Native
configuration/session/settings/editor integration remains unfinished. Catalog
core checkpoint stays 91/95 (95.8%), with 289 profiles; stored layouts increase
to 156. Overall and MAME completion remain unknown. Formatting and whitespace
checks only; no tests, builds, UI runs or device probes.

Step 355: native Add player now chooses from unassigned enabled ports instead
of assuming every numeric slot is connected. PCEHawk skips disconnected ports,
lists connected ports needing assignments and warns about retained players on
disconnected ports. Other native decks retain their existing slot ranges. This
fixes the editor path for sparse PCEHawk topology without renumbering players.
Catalog stays 91/95 cores (95.8%), 289 profiles and 155 stored layouts; overall
and MAME completion remain unknown. Whitespace checks only; no tests, builds,
UI runs or device probes.

Step 354: connected PCEHawk to the native editor and source/destination preview.
Added a two-button core choice, five fixed-port toggles, saved topology round-trip,
and a recording gate requiring exactly one player per connected port. Port edits
retain assignments for explicit correction. Preview uses pce-2 geometry and the
pcehawk scoped SDL lookup; PlayStation peripheral controls remain hidden. These
paths are implemented but unverified. Catalog stays 91/95 cores (95.8%), 289
profiles and 155 stored layouts; overall and MAME completion remain unknown.
Formatting and whitespace checks only; no tests, builds, UI runs or probes.

Step 353: added optional PCEHawk saved port settings, exclusive-core validation,
platform routing, a dedicated logical-calibration scope and saved two-button
calibration completeness checks. Stored configurations can now select the shared
native session adapter, retaining fixed P1-P5 ports. Dedicated editor and preview
selection remain unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles
and 155 stored layouts; overall and MAME completion remain unknown. Formatting
and whitespace checks only; no tests, builds, UI runs or device probes.

Step 352: connected PCEHawk to shared native digital session preparation,
including configuration ownership, device-routing guards and normalized-device
logical calibration. Fixed P1-P5 identities are retained; disconnected/duplicate
slots and incomplete connected-port assignments are rejected before probing.
Saved settings and editor routing remain unfinished, so this is not yet a
user-selectable launch path. Catalog stays 91/95 cores (95.8%), 289 profiles
and 155 stored layouts; overall and MAME completion remain unknown. Formatting
and whitespace checks only; no tests, builds, UI runs or device probes.

Step 351: PCEHawk now translates calibrated raw/logical SDL inputs into its
eight native controls, including explicit I-to-B1, II-to-B2 and Start-to-Run
conversion. Added fixed-port snapshot composition and transactional private
configuration argument preparation; duplicate SDL devices are rejected.
Session dispatch, settings and UI integration remain unfinished. Catalog stays
91/95 cores (95.8%), 289 profiles and 155 stored layouts; overall and MAME
completion remain unknown. No tests, builds, UI runs or device probes.

Step 350: source/destination diagram hotspots now use focusable Qt Quick
AbstractButtons, with side-specific accessible names, assignment descriptions,
accessible press actions and visible keyboard focus. Keyboard activation uses
the same selection/cycling and controlActivated path as pointer activation,
including unmapped destinations. This applies to MAME and other consumers of
the shared mapping view; it does not add new core contracts. Catalog checkpoint
remains 91/95 cores (95.8%), 289 profiles and 155 stored layouts. Overall and
MAME completion remain unknown. No tests, builds, UI runs or device probes.

Step 349: MAME's visual review now displays the existing per-game native-field
mapping count and percentage without opening the technical report, alongside
disabled and unresolved counts. A separate selected-player source-assignment
count and percentage follows the player picker, with explicit zero-denominator
and unverified-runtime wording. These measures do not establish playability.
Catalog checkpoint remains 91/95 cores (95.8%), 289 profiles and 155 stored
layouts; overall and MAME completion remain unknown. Static whitespace checks
only; no tests, builds, UI runs or device probes. Wheel work remains paused.

Step 348: added source-grounded PCEHawk two-button encoding for five independently
connected fixed-numbered ports. The inspected core has no six-button controller
mode; that remains a separate adapter boundary. Translation, session, settings
and UI integration are unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles,
155 stored layouts; overall and MAME completion unknown. No tests, builds,
UI runs or device probes.

Step 347: native setup recording now compares the loaded baseline against the
current exact emulator/system scope. Stale edits, removed setups and new drafts
that would overwrite an existing scope are rejected before mutation. New distinct
scopes still preserve existing setups. Catalog stays 91/95 cores (95.8%), 289
profiles, 155 stored layouts; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 346: added explicit removal of one saved native emulator/system setup with
confirmation and expected-value checking; changed or missing setups are rejected.
Other setups and all calibration records remain intact. The UI distinguishes this
from disabling every native setup and explains that the current draft is retained.
Catalog stays 91/95 cores (95.8%), 289 profiles, 155 stored layouts; overall and
MAME completion unknown. No tests, builds, UI runs or device probes.

Step 345: native mapping preview now shares launch's exact emulator/system SDL
lookup, preferring scoped calibration and labeling legacy fallback candidates.
NES peripheral preview modes resolve to the owning NES runtime scope rather than
inventing separate calibration namespaces. A missing emulator identity disables
the saved-SDL view, not the physical diagram. Catalog stays 91/95 cores (95.8%),
289 profiles, 155 stored layouts; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 344: captured SDL gestures now save under their exact runtime scope; another
installation's or legacy bindings are not overwritten. Capture-panel listing and
clear actions operate on that scope, and clearly disclose legacy fallback after
clearing. Capture state retains the selected runtime identity. General native
mapping preview still needs scoped lookup. Catalog stays 91/95 cores (95.8%),
289 profiles, 155 stored layouts; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 343: added bounded runtime-scoped SDL calibration storage keyed by exact
emulator/system/controller identity. Native launch prefers its scope and falls
back to legacy controller-only bindings; live context checks remain unchanged.
Physical recalibration invalidates changed bindings across every scope. Capture
and editor writes still need connection to the new storage. Catalog stays 91/95
cores (95.8%), 289 profiles, 155 stored layouts; overall and MAME completion
unknown. No tests, builds, UI runs or device probes.

Step 342: native SDL calibration now explicitly selects a saved emulator/system
runtime instead of implicitly using the most recently recorded setup. The backend
resolves the exact scope and rejects stale/ambiguous selection. Changing runtime
or saved choices cancels capture so released/pressed samples cannot mix runtimes.
Catalog stays 91/95 cores (95.8%), 289 profiles, 155 stored layouts; overall and
MAME completion unknown. No tests, builds, UI runs or device probes.

Step 341: the native editor now lists every saved emulator/system scope, loads
one only through an explicit replace-draft action and starts new drafts without
deleting saved setups. Loading re-reads the exact selected scope; settings changes
invalidate the picker selection. Updated the obsolete PlayStation-only editor
notice. Catalog stays 91/95 cores (95.8%), 289 profiles, 155 stored layouts;
overall and MAME completion unknown. No tests, builds, UI runs or device probes.

Step 340: native BizHawk configurations can now coexist by exact emulator ID and
system scope. Recording another scope preserves the previous setup; launch selects
the matching scope and rejects ambiguity. Legacy single-entry settings remain
readable. Disabling the native adapter clears all staged scopes. Editor selection
of older scopes remains to be connected. Catalog stays 91/95 cores (95.8%), 289
profiles, 155 stored layouts; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 339: source inspection confirms SMSHawk's SG-1000 standard digital deck.
Added explicit SG selection, SG core preference, platform routing, six-control
translation/normalization and editor preview with a generic two-button schematic.
Game Gear SMS compatibility retains its GG controller deck in the inspected core;
content-dependent runtime selection remains unverified. Catalog stays 91/95 cores
(95.8%), 289 profiles; stored layouts increase to 155. Overall and MAME completion
unknown. No tests, builds, UI runs or device probes.

Step 338: the native editor now selects Master System or Game Gear, applies their
player capacities, hides PlayStation-only options and records SMSHawk settings.
Source/destination preview resolves the corresponding Sega layout and explains
console Pause/Reset ownership. Catalog stays 91/95 cores (95.8%), 289 profiles,
154 stored layouts; overall and MAME completion unknown. No tests, builds,
UI runs or device probes.

Step 337: saved native settings now select SMSHawk's Master System or Game Gear
contract exclusively from the other native cores. Saved-input validation and main
launch routing use the selected system, including its normalization target and
player rules. Existing editor recording is blocked for loaded SMSHawk setups until
its dedicated controls are connected. Catalog stays 91/95 cores (95.8%), 289
profiles, 154 stored layouts; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 336: SMSHawk joins shared native session preparation, including generated
logical bindings for normalized devices, raw-device translation, runtime identity
checks and private-config ownership. Deck validation runs before probing and
distinguishes optional SMS host assignments from fully populated Nintendo decks;
Game Gear still permits only P1. Saved setup and UI selection remain unfinished.
Catalog stays 91/95 cores (95.8%), 289 profiles, 154 stored layouts; overall and
MAME completion unknown. No tests, builds, UI runs or device probes.

Step 335: SMSHawk now composes calibrated multi-player translation with config
encoding and owned temporary-file argument preparation. Shared player validation
rejects duplicate slots and a Game Gear second player; SDL device ownership must
also be distinct. Session selection, settings and UI remain unfinished. Catalog
stays 91/95 cores (95.8%), 289 profiles, 154 stored layouts; overall and MAME
completion unknown. No tests, builds, UI runs or device probes.

Step 334: added Master System's six-control schematic and SMSHawk physical/logical
SDL translation for SMS and Game Gear. Explicit catalog b/a to native B1/B2
conversion preserves Sega button identities; SMS does not acquire Start/Select.
Saved-input completeness uses the shared resolver. Launch/settings/UI integration
remains unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles; stored layouts
increase to 154. Overall and MAME completion unknown. No tests, builds or probes.

Step 333: added a source-grounded SMSHawk encoder with separate Master System
six-control and Game Gear seven-control contracts, native port settings and
console-button preservation. Game Gear rejects a second player. Translation,
settings, launch and UI integration remain unfinished. Catalog stays 91/95 cores
(95.8%), 289 profiles, 153 stored layouts; overall and MAME completion unknown.
No tests, builds, UI runs or device probes.

Step 332: the native Nintendo editor now identifies each logical player's owning
left/right port, device and core-local multitap/Four Score slot. Preview selection
uses the same slot model. Missing slots are listed explicitly and block recording,
including duplicate-player arrangements with a superficially correct total count.
Catalog stays 91/95 cores (95.8%), 289 profiles, 153 stored layouts; overall and
MAME completion unknown. No tests, builds, UI runs or device probes.

Step 331: Power Pad preview now reports incomplete/non-independent physical input
assignments and rejects saved SDL views that reuse one logical output for multiple
pad switches. Launch translation applies the same logical independence check,
including opposite gestures on one SDL axis. Catalog stays 91/95 cores (95.8%),
289 profiles, 153 stored layouts; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 330: connected native NesHawk Power Pad selection to its twelve native PP
bindings, per-player translation, saved-input validation, normalization target and
visual preview. Independent physical identities are required; opposite directions
of one axis cannot supply independent pad switches. Catalog remains 91/95 cores
(95.8%), 289 profiles, 153 stored layouts; overall and MAME completion unknown.
No tests, builds, UI runs or device probes.

Step 329: added the missing NES Power Pad diagram with twelve independent PP1–PP12
switch identities. Its numbered 4-by-3 geometry follows the pinned BizHawk virtual
pad schema, explicitly not a photographic mat-side depiction. The native adapter
is not connected yet. Catalog remains 91/95 cores (95.8%), 289 profiles; stored
layouts increase to 153. Overall and MAME completion unknown. No tests, builds,
UI runs or device probes.

Step 328: NesHawk now supports its source-defined ControllerSNES adapter alongside
ordinary NES pads and Four Score halves. Per-player port resolution selects eight
or twelve buttons consistently for encoding, translation, saved-input validation,
normalization and visual preview. The native editor exposes the adapter explicitly.
Catalog remains 91/95 cores (95.8%), 289 profiles; overall and MAME completion
unknown. No tests, builds, UI runs or device probes.

Step 327: the native editor now selects NesHawk, edits left/right joypad or Four
Score halves, records complete deck assignments and previews the NES source and
destination mapping. Preview requests identify the core explicitly; unknown cores
and incompatible Nintendo modes are rejected. Existing player assignments remain
explicit when changing topology. Catalog remains 91/95 cores (95.8%), 289 profiles;
overall and MAME completion unknown. No tests, builds, UI runs or device probes.

Step 326: saved native settings now select NesHawk ports exclusively from SNES;
validation requires complete NES inputs and exact deck capacity. Native NES launch
routing uses the shared session and NES normalization target. Legacy Nymashock
defaults remain when neither Nintendo mode is selected. The current editor blocks
recording loaded NES setups until its NES controls are connected. Catalog stays
91/95 cores (95.8%), 289 profiles; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 325: NES and SNES now share native digital session preparation, including
runtime artifact capture, exact device-path resolution, cancellation, before/after
SDL routing checks and private-config lifetime. NES gains a session entry point;
normalized recognized/raw device handling follows the existing SNES path. Main
NES launch selection and settings/UI wiring remain unfinished. Catalog remains
91/95 cores (95.8%), 289 profiles; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 324: native NesHawk now composes multi-player SDL translation with its
encoder, rejecting missing/duplicate logical slots and duplicate SDL devices
before translation. It also uses the existing owned temporary-config transaction
and native argument parser, preserving arguments on failure. Live session,
settings and UI routing remain unfinished. Catalog stays 91/95 cores (95.8%),
289 profiles; overall and MAME completion unknown. No tests, builds or probes.

Step 323: native NesHawk now translates its eight joypad controls through a shared
digital-pad resolver and SDL translator extracted from SNES. Both retain logical
mapping-context checks, calibrated raw-axis translation and duplicate native-source
rejection. NES saved-input completeness uses the same resolver. NES launch/settings
integration remains unfinished. Catalog stays 91/95 cores (95.8%), 289 profiles;
overall and MAME completion unknown. No tests, builds, UI runs or device probes.

Step 322: added a source-grounded native NesHawk joypad configuration encoder
with explicit disconnected/joypad/Four Score port halves, eight bindings per
logical player, exact core/settings names and preservation of system controls.
Translation, saved settings, launch and UI integration remain unfinished; no new
coverage is counted. Catalog stays 91/95 cores (95.8%), 289 profiles; overall and
MAME completion unknown. No tests, builds, UI runs or device probes.

Step 321: normalized SNES controllers now support SDL raw-joystick exposure as
well as recognized GameController exposure. This corrects Step 317's unnecessary
recognition requirement: raw devices use calibrated native input translation;
recognized devices derive fresh logical bindings. Neither route reuses saved
physical SDL bindings for virtual devices, and ownership/routing checks remain.
Catalog remains 91/95 cores (95.8%), 289 profiles; overall and MAME completion
unknown. No tests, builds, UI runs or device probes.

Step 320: saving native SNES settings now requires all twelve gameplay controls
to resolve to calibrated Linux inputs with native identities. Save and launch use
the same SNES resolution helper; failures name the player and missing control.
This is input completeness, not SDL routing or runtime acceptance. Catalog stays
91/95 cores (95.8%), 289 profiles; overall and MAME completion unknown. No tests,
builds, UI runs or device probes.

Step 319: native player preview now takes explicit Snes9x mode and resolves the
SNES destination diagram and its twelve controls. The runtime editor enables
source/destination previews for SNES; incompatible PlayStation modes are rejected.
Normalized players no longer offer misleading saved-physical-SDL preview bindings:
their logical bindings are generated for the launch-owned device. All previews
remain candidates, not runtime readiness evidence. Catalog coverage remains 91/95
cores (95.8%), 289 profiles; MAME and overall completion unknown. No tests, builds,
UI runs or device probes.

Step 318: the native runtime editor now selects Nymashock or Snes9x and exposes
SNES left/right None, Joypad and Multitap topology with ordered player capacity.
Core changes require explicit removal of existing players; topology changes retain
assignments for correction. SNES recording uses its own topology and hides
PlayStation-only modes. SNES visual preview remains explicitly unavailable pending
core-aware preview integration. Catalog coverage remains 91/95 cores (95.8%), 289
profiles; overall and MAME completion unknown. No tests, builds or UI runs.

Step 317: native Snes9x normalized input now selects the SNES target when creating
the session-owned virtual controller, instead of requiring PlayStation controls.
Session preparation derives logical bindings from the opened virtual device's SDL
mapping and rejects an unrecognized normalized device. Saved native settings allow
normalized joypads while retaining peripheral/rumble restrictions. Full Snes9x
editor support and runtime verification remain unfinished. Catalog coverage stays
91/95 cores (95.8%), 289 profiles; MAME and overall completion unknown. No tests,
builds, emulator runs or device probes were performed.

Step 316: saved native settings can explicitly select Snes9x port topology;
absent selection retains Nymashock. Validation rejects incompatible PlayStation
modes and incomplete deck assignments. Native launch dispatch now selects the
matching platform/session encoder. Normalized Snes9x input and full editor support
remain unfinished; the legacy editor cannot overwrite a loaded Snes9x setup.
Catalog coverage remains 91/95 cores (95.8%), 289 profiles; overall completion
unknown. No tests/builds or runtime/device checks.

Step 315: native Snes9x now has a launch-session preparation transaction using
runtime artifact capture, physical topology ownership, exact SDL path resolution,
before/after routing checks and owned temporary configuration. Argument mutation
occurs only after preparation checks succeed. Discovery-versus-opened routing
comparison is shared with Nymashock. Settings/UI dispatch and normalized-device
integration remain unfinished, so no new coverage is counted. Catalog stays 91/95
cores (95.8%), 289 profiles; overall unknown. No tests/builds or probes were run.

Step 314: native Snes9x now prepares owned temporary configuration and rewrites
EmuHawk arguments through the same transaction used by Nymashock. Common helpers
retain exact --config parsing, source-change checks, private-config lifetime and
unchanged arguments on failure. Nymashock delegates to its existing encoder;
Snes9x delegates to its own translation/encoder. Full session and settings/UI
routing remain unfinished. Catalog coverage stays 91/95 cores (95.8%), 289 profiles;
overall completion unknown. No tests/builds or runtime/device checks.

Step 313: native Snes9x translation and encoding are now composed by a multi-player
preparation function. It requires every configured merged-deck slot exactly once,
rejects duplicate SDL device ownership, and returns player-scoped translation
warnings with configuration text. It performs no probing or writes; native session
ownership, settings/UI and launch integration remain unfinished. Catalog coverage
stays 91/95 cores (95.8%), 289 profiles; overall completion unknown. No tests/builds
or runtime/device checks.

Step 312: native Snes9x now has calibrated twelve-button translation for probed
raw SDL joysticks and recognized SDL GameControllers. It reuses measured native
translation, verifies saved logical context/layout where required, resolves SNES
semantics through the shared solver and rejects missing or duplicate native
sources. Fallback rules are returned as warnings. Session/settings/UI integration
remains unfinished; no new core coverage is counted. Catalog coverage stays 91/95
cores (95.8%), 289 profiles; overall completion unknown. No tests/builds or probes.

Step 311: implemented a separate native BizHawk Snes9x configuration encoder with
explicit none/joypad/multitap ports, one-based deck-order player completeness and
twelve-button validation. It selects Snes9x, preserves Reset/Power and unrelated
config, and replaces the owned SNES input/autofire/analog/feedback dictionaries.
Numeric enum serialization and core naming were checked against pinned source.
Host translation, saved-settings/UI and launch integration remain unfinished; no
new adapter is counted. Catalog coverage remains 91/95 cores (95.8%), 289 profiles;
MAME and overall completion unknown. No tests/builds or runtime/device checks.

Step 310: moved from MAME UI refinements to a source-grounded native-runtime
extension. The pinned BizHawk Snes9x deck, twelve joypad inputs, merged player
numbering and separate port topology are recorded in
`BIZHAWK_SNES9X_CONTROLLER_CONTRACT.md`. This is contract discovery, not an
implemented adapter: native configuration serialization and launch wiring remain.
Catalog-wide coverage stays 91/95 cores (95.8%), 289 profiles; MAME and overall
all-mode completion remain unknown. No tests/builds or runtime/device checks.

Step 309: generated MAME inspection requests retain their source draft identity.
Changing that draft clears trust and blocks running the stale request until it is
regenerated; result application continues using backend receipt validation.
MAME all-game completion remains unknown and untested. Separately, catalog-wide
profile coverage remains 91/95 cores (95.8%), 289 profiles; this is not a MAME
completion percentage. No tests/builds or runtime/device checks. Wheel work paused.

Step 308: MAME physical-readiness failures now identify each affected player's
target control and distinguish absent source assignments from missing native
measurements. Structured gap records accompany visible per-player warnings;
inactive-player guidance points to port removal or inspected-control setup.
Coverage remains 91/95 catalog cores (95.8%), 289 profiles; overall completion
remains unknown. No tests/builds or runtime/device checks. Wheel work remains paused.

Step 307: existing MAME setups now allow adding an explicitly selected unused
player port through the player editor. No controller is auto-assigned: a valid,
distinct saved calibration must be chosen before Apply. Ports remain sorted and
bounded to 1–8; game support is determined by the unchanged mapping review and
staging gates. Coverage remains 91/95 catalog cores (95.8%), 289 profiles; overall
completion remains unknown. No tests/builds or runtime/device checks.

Step 306: MAME player editing now allows removing an unused port while retaining
at least one player. Removal is local until Apply, Cancel preserves the draft,
and ports owning explicit button/axis assignments cannot be removed without first
handling those assignments. Apply rechecks references and invalidates review;
native default handling for unselected ports remains unchanged. Coverage remains
91/95 catalog cores (95.8%), 289 profiles; overall completion remains unknown.
No tests/builds or runtime/device checks. Wheel work remains paused.

Step 305: MAME review now reports physical readiness using the same native-input
presence condition checked during staging, rather than relying only on displayed
source IDs. Missing/unmeasured physical rows and selected players with no active
profile disable the Stage button and appear in the summary. Backend validation
remains unchanged. Coverage remains 91/95 catalog cores (95.8%), 289 profiles;
overall completion remains unknown. No tests/builds or runtime/device checks.

Step 304: MAME review now reports missing/invalid calibration per player and
continues reviewing valid players. Unavailable players receive explicit warnings
and no invented diagram rows; a separate calibration-pending flag blocks UI
staging and appears in the overall review summary. Backend staging remains strict.
Coverage remains 91/95 catalog cores (95.8%), 289 profiles; overall completion
remains unknown. No tests/builds or runtime/device checks. Wheel work remains paused.

Step 303: missing or invalid FBNeo calibration now blocks only that player's
review rows, rather than aborting the full review. Its native targets remain
visible with explicit repair guidance and no fabricated source choices; valid
players remain editable, and stale-assignment inventory remains available.
Staging/launch validation is unchanged. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall completion remains unknown. No tests/builds or
runtime/device checks. Wheel work remains paused.

Step 302: FBNeo review now inventories assignments with absent native addresses,
unsupported binding parts or mismatched player ownership, and exposes individual
draft-only removal. Removal rechecks the current contract, exact assignment and
index before deleting one entry; other assignments remain unchanged. No automatic
cleanup or relaxed staging is introduced. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall completion remains unknown. No tests/builds or
runtime/device checks. Wheel work remains paused.

Step 301: FBNeo assignment review now supports action/native-ID search and a
Needs attention filter for missing/invalid controls, duplicate or aliased conflicts
and external-adapter targets. Player-level calibration failures keep that player's
entire input set in the attention view so paired-axis/alias errors are not hidden.
Filters reset when review opens and never mutate mappings. Coverage remains 91/95
catalog cores (95.8%), 289 profiles; overall completion remains unknown. No tests,
builds or runtime/device checks. Wheel work remains paused.

Step 300: FBNeo review now identifies the exact shared-channel assignment owner
and part, counts duplicates on that owner, and offers direct visual navigation
to repair it. Pressure-to-digital fallback retains its actual pressure part;
clearing a dependent alias still does not remove the owner's binding. Duplicate
and alias staging rules remain unchanged. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall completion remains unknown. No tests/builds or
runtime/device checks. Wheel work remains paused.

Step 299: source inspection confirmed that different-layout FBNeo replacements
retain repairable missing-control rows. Duplicate exact address/part assignments
now also remain reviewable instead of aborting the entire editor: each affected
part shows its conflict count and explains that only its first source is displayed.
Existing source/clear actions remove all duplicates for that exact part; staging
validation is unchanged. Coverage remains 91/95 catalog cores (95.8%), 289 profiles;
overall completion remains unknown. No tests/builds or runtime/device checks.

Step 298: existing FBNeo setups now support replacing a player's controller from
saved Linux calibrations without editing JSON. Exact native port/device and
assignments are retained; duplicate controllers and changed drafts are rejected.
Application opens current assignment review so incompatible controls can be
repaired rather than silently remapped or dropped. Coverage remains 91/95 catalog
cores (95.8%), 289 profiles; overall completion remains unknown. No tests/builds
or runtime/device checks. Wheel work remains paused.

Step 297: FBNeo source edits and paired-axis edits now resolve eligibility from
the current draft and saved calibration immediately before mutation, rather than
trusting cached dropdown/diagram rows. Exact native address/part membership and
source eligibility are required; clearing a present part remains possible.
Backend staging/launch checks remain unchanged. Inventory inspection confirmed
the remaining adapter/runtime gaps still preclude claiming additional core
coverage: 91/95 catalog cores (95.8%), 289 profiles; overall completion unknown.
No tests/builds or runtime/device checks. Wheel work remains paused.

Step 296: native player selection now includes valid saved calibrations absent
from the current inventory, using stored display names and exact IDs without
claiming live connectivity. Existing unavailable player IDs remain visible for
repair. The per-player toggle-control list also refreshes on calibration revision
changes instead of retaining stale controls. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall completion remains unknown. No tests/builds or
runtime/device checks. Wheel work remains paused.

Step 295: native preview now names the exact reserved DualShock toggle and each
desktop-cursor destination excluded from controller mapping, with validation
limits stated separately. The preview is scrollable within the runtime dialog so
long missing-input/exclusion explanations remain accessible. Coverage remains
91/95 catalog cores (95.8%), 289 profiles; overall completion remains unknown.
No tests/builds, visual or runtime/device checks. Wheel work remains paused.

Step 294: native pointer, neGcon and rhythm mappings now share calibrated-source
selection with gamepads and preview. All five native resolver paths and preview
also share target-control selection, including desktop-cursor axis exclusion.
Existing translation and runtime checks remain in their callers; no peripheral
support is newly claimed by this consolidation. Coverage remains 91/95 catalog
cores (95.8%), 289 profiles; overall completion remains unknown. No tests/builds
or runtime/device checks. Wheel work remains paused.

Step 293: native physical/SDL candidate selection now shares one helper between
preview and native logical/raw gamepad mapping. Physical calibration membership,
optional logical-binding intersection and reserved toggle exclusion therefore use
the same implementation. Callers retain calibration/context/translation checks;
this is not evidence of runtime equivalence. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall completion remains unknown. No tests/builds or
runtime/device checks. Wheel work remains paused.

Step 292: native PlayStation preview can compare physical-only candidates with
the intersection of saved physical and logical SDL calibrations. Invalid/missing
SDL captures and layout mismatches disable that view with an explicit reason;
toggle reservation and desktop-cursor exclusions remain consistent. No live SDL
context match, native translation or normalized-device binding is inferred.
Coverage remains 91/95 catalog cores (95.8%), 289 profiles; overall completion is
unknown. No tests/builds or runtime/device checks. Wheel work remains paused.

Step 291: native PlayStation preview now displays physical-calibration candidate
relationships using the shared layout resolver. It reserves the calibrated mode
toggle, excludes desktop-cursor axes and rejects inconsistent toggle/cursor modes.
Missing controls are shown rather than synthesized. This is explicitly not final
native mapping: logical SDL availability and native gesture/axis translation may
change eligibility. Coverage remains 91/95 catalog cores (95.8%), 289 profiles;
overall completion remains unknown. No tests/builds or runtime/device checks.

Step 290: native PlayStation player editing now exposes side-by-side source and
emulated-controller schematics through the existing validated mode-layout query.
This connects the previously unused preview backend. It deliberately shows no
inferred binding lines or launch-ready claim. Player/calibration changes close
the preview to prevent stale mode geometry. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall all-game/all-mode completion remains unknown. No
tests, builds, visual/runtime or device checks. Wheel work remains paused.

Step 289: FBNeo now clears cached assignment reviews and closes stale mapping
previews when controller settings/inventory revision changes. Its open inspection
import picker refreshes saved controller choices and resets the selected index,
avoiding accidental reassignment when list ordering changes. Existing context and
setup drafts remain unchanged; reopening assignment review resolves current data.
Coverage remains 91/95 catalog cores (95.8%), 289 profiles; overall all-game/all-mode
completion remains unknown. No tests/builds or runtime/device checks. Wheel work
remains paused.

Step 288: MAME mapping review now invalidates when the shared controller settings
revision changes, not only when draft JSON changes. It clears cached mapping
rows/summary, closes the stale preview and disables staging until review runs
again. Draft assignments are preserved; backend validation remains authoritative.
This also conservatively invalidates on inventory or unrelated controller-setting
changes. Coverage remains 91/95 catalog cores (95.8%), 289 profiles; overall
all-game/all-mode completion remains unknown. No tests, builds or runtime/device
checks. Wheel work remains paused.

Step 287: MAME visual review now offers Restore selected button to preset for
explicit standard digital controller overrides. It removes only the exact native
field's standard override, retaining other overrides and analog assignments.
The draft must match the review; restoration invalidates review and warns that
automatic default allocation may change other channels. This does not assert
the restored preset can satisfy the game. Coverage remains 91/95 catalog cores
(95.8%), 289 profiles; overall all-game/all-mode completion is unknown. No tests,
builds or runtime/device checks. Wheel work remains paused.

Step 286: existing MAME drafts now have a player-controller replacement editor
using valid saved calibrations. Existing exact ports (including sparse ports),
native assignments and inspection data are preserved. Missing calibrations,
duplicate devices and concurrent draft changes are rejected; applying a change
invalidates mapping review and does not stage settings. Coverage remains 91/95
catalog cores (95.8%), 289 profiles; overall all-game/all-mode completion remains
unknown. No tests/builds or runtime/device checks. Wheel work remains paused.

Step 285: the MAME inspection dialog can now apply its completed result directly
to the draft through the same receipt-validation path as the main editor. Errors
are displayed inside the modal dialog; successful application invalidates prior
mapping review and returns to the draft. Changing runtime/discovery paths clears
the old request and trust selection, preventing those fields from appearing to
control a stale generated request. Coverage remains 91/95 catalog cores (95.8%),
289 profiles; overall all-game/all-mode completion is unknown. No tests, builds,
inspection, emulator or device checks were run. Wheel work remains paused.

Step 284: MAME new-setup drafts now support one to eight players selected from
structurally valid saved calibrations, including disconnected devices. Exact
saved IDs are retained; missing selections and duplicate controller assignments
are rejected. Reducing player count omits extra ports from the generated draft.
This reads saved settings only and does not assert connectivity or game support.
Coverage remains 91/95 catalog cores (95.8%), 289 enabled profiles; overall
all-game/all-mode completion remains unknown. No tests/builds or runtime/device
checks. Wheel work remains paused.

Step 283: added a one-player MAME new-setup form for exact shortname ZIP/7z
archives, explicit emulator/core/library/calibration identities and dedicated
storage paths. It generates an Automatic-layout draft without fabricated hashes
or a snapshot; the existing native inspection, receipt application, assignment
review and staging gates remain required. Additional players, merged sets and
custom manifests still require advanced work. No filesystem writes or inspection
are triggered by creating the draft. Coverage remains 91/95 catalog cores (95.8%),
289 profiles; overall all-game/all-mode completion is unknown. No tests/builds or
runtime/device checks. Wheel work remains paused.

Step 282: simplified the MAME per-game editor's ordinary button flow. Keyboard,
service and incremental-axis field choices and the analog editor now require an
explicit Advanced controls toggle. Preset guidance points to the existing visual
source/destination review and button swaps. This is presentation filtering only:
saved assignments and full unresolved-input/staging checks remain unchanged.
Catalog counts remain 91/95 cores (95.8%), 289 enabled profiles and 152 stored
layouts plus 12 derived schematics (164/164 available, not visually verified).
Overall all-game/all-mode completion remains unknown. No tests, builds, runtime
or device checks were run. Wheel work remains paused.

Step 280: direct-Mono native preparation now validates installation, SDL,
native/managed library and working-directory roots before controller probing.
Missing paths and file-instead-of-directory mistakes report the specific role
and path. Saved-structure validation remains portable; this does not pin
transitive libraries or establish runtime fidelity. Coverage stays 91/95 cores
(95.8%), 289 profiles and 164/164 schematics (100% by construction). Overall
completion unknown. No tests/builds or runtime checks; wheel work remains paused.

Step 279: native BizHawk direct-Mono configuration now supports explicit managed
assembly search directories separately from native libraries. The runtime editor
loads/saves the optional paths; validation preserves the combined directory bound
and direct-Mono requirement. Existing settings retain installation/dll behavior.
This closes a split-runtime configuration gap, not runtime fidelity. Coverage
stays 91/95 cores (95.8%), 289 profiles and 164/164 schematics (100% by
construction). Overall completion unknown. No tests/builds or runtime checks;
wheel work remains paused.

Step 278: automatic MAME inspection now selects active digital ports up to the
available-controller count before constructing its mapping. Later profile
preparation uses that same selected set. This connects the unused-port handling
to the actual automatic launch path, while retaining the full snapshot and
unknown/peripheral gates. Explicit saved player sets are unchanged. Coverage
stays 91/95 cores (95.8%), 289 profiles and 164/164 schematics (100% by
construction). Overall completion unknown. No tests/builds or runtime checks;
wheel work remains paused.

Step 277: coverage navigation now distinguishes MAME/FBNeo per-game adapter code
from a missing implementation, with direct links to their setup editors and
explicit remaining requirements. Steem SSE reports its runtime/port prerequisite.
Per-game rows remain incomplete and do not inflate catalog core/platform counts.
Coverage stays 91/95 cores (95.8%), 289 profiles and 164/164 schematics (100% by
construction). Overall completion unknown. No tests/builds or runtime checks;
wheel work remains paused.

Step 276: MAME now applies twin-stick collision and action-capacity checks only
to selected frontend players. Unselected ports explicitly disable known native
Buttons 1-16 alongside existing direction/start/coin disabling, so an unused
player's larger layout no longer blocks selected players. Unknown/peripheral
fields retain their existing gates; disabled fields are not counted as mapped.
Catalog coverage stays 91/95 cores (95.8%), 289 profiles and 164/164 schematics
(100% by construction). Overall completion unknown. No tests/builds or runtime
checks; wheel work remains paused.

Step 275: the default MAME layer now preserves native bindings for exactly
SERVICE/SERVICE1-SERVICE4 when reported as non-analog miscellaneous inputs.
These maintenance controls no longer demand gameplay-button calibration;
explicit overrides still take precedence and other unknown controls still block.
Review accounts for native service inputs separately without counting them as
mapped or excluding them from the user-input denominator. Catalog coverage stays
91/95 cores (95.8%), 289 profiles and 164/164 schematics (100% by construction).
Overall completion unknown. No tests/builds or runtime checks; wheel work paused.

Step 274: MAME visual review now swaps two independent ordinary button actions
on the same source player. Shared, analog and directional-axis channels are not
offered by this shortcut. Other numbered actions on the port are preserved as
explicit overrides so sparse/twin allocation cannot silently shift them;
direction fields remain intact for layout inference. Changes
are draft-only and require re-review/staging. Catalog coverage stays 91/95 cores
(95.8%), 289 profiles and 164/164 schematics (100% by construction). Overall
completion unknown. No tests/builds or UI/runtime checks; wheel work stays paused.

Step 273: the MAME visual review can now open the existing override editor for
the exact highlighted game action, preselecting its current source port/channel.
Shared connections require an explicit action choice; stale review text or
mismatched field identity rejects the shortcut. Edits still invalidate review
and require normal staging validation. Catalog coverage stays 91/95 cores
(95.8%), 289 profiles and 164/164 schematics (100% by construction). Overall
completion unknown. No tests/builds or UI/runtime checks; wheel work stays paused.

Step 272: MAME review now leads with source-assignment and unresolved-native-input
counts, plus captured names for controls needing extra setup. Analog-calibration
attention remains visible. The full technical report is optional and retains all
details; validation and staging gates are unchanged. Catalog coverage stays
91/95 cores (95.8%), 289 profiles and 164/164 schematics (100% by construction).
Overall completion unknown. No tests/builds or UI/runtime checks performed;
wheel work remains paused.

Step 271: normal MAME defaults now offer a direct layout preview, showing both
six/eight-button arrangements for Automatic and the selected preset otherwise.
The advanced preset list leads with Automatic while preserving saved IDs and
legacy missing-field behavior. Preview does not start inspection or change a
mapping. Catalog coverage remains 91/95 cores (95.8%), 289 profiles and 164/164
schematics (100% by construction). Overall completion unknown. No tests/builds
or UI/runtime checks performed; wheel work remains paused.

Step 270: returned priority to the everyday arcade mapping view following user
feedback. Connections and tooltips now show captured native action names beside
their diagram positions, falling back to existing labels when names are absent.
Technical native-field details are optional; mappings and validation remain
unchanged. Wheel capture work remains paused. Catalog coverage stays 91/95 cores
(95.8%), 289 profiles and 164/164 schematics (100% by construction). Overall
completion unknown. No tests/builds or UI/runtime checks performed.

Step 269: relative-mouse output now splits wheel totals into bounded unit-detent
events with report boundaries, matching the pinned frontend's +1/-1-only wheel
handling. Oversized reports fail before publication rather than being truncated.
Opposite-event preservation and frontend polling/coalescing remain unresolved,
so wheel targets are still external-adapter requirements. Catalog coverage stays
91/95 cores (95.8%), 289 profiles and 164/164 schematics (100% by construction).
Overall completion unknown. No tests/builds or device/UI/runtime checks performed.

Step 268: FBNeo mouse targets now have a native-event destination schematic
covering X/Y deltas, five buttons and four wheel directions. Review can highlight
the exact callback identity, including wheel targets whose conversion remains
unsupported. The panel does not invent a physical source or claim launch routing.
Catalog coverage remains 91/95 cores (95.8%), 289 profiles; schematic availability
is 164/164 (100% by construction). Overall completion unknown. No tests/builds
or UI/runtime checks performed.

Step 268 source review also corrected the earlier FBNeo lightgun panel's
non-catalog family/group identifiers to supported pointer/auxiliary semantics;
the new mouse panel likewise uses the existing pointer vocabulary.

Step 267: FBNeo relative-device review now groups the full mouse target set per
frontend port. A candidate must satisfy every axis/button requirement; any
unsupported conversion prevents a complete candidate. The UI distinguishes
whole-port capability matches from live routing, source selection and cross-port
exclusivity. Corrected settings access in the earlier per-target candidate path.
Catalog coverage stays 91/95 cores (95.8%), 289 profiles and 163/163 schematics
(100% by construction). Overall completion unknown. No tests/builds or UI/runtime
checks performed.

Step 266: FBNeo review now resolves mouse X/Y and five mouse buttons to exact
Linux relative-device output requirements. Saved candidate matching respects
axis swapping and button-output remaps, and rejects invalid saved devices.
Candidates remain capability matches only; external-adapter and launch blockers
are preserved. Wheel conversion and live route ownership remain unfinished.
Catalog coverage stays 91/95 cores (95.8%), 289 profiles and 163/163 schematics
(100% by construction). Overall completion unknown. No tests/builds or UI/runtime
checks performed.

Step 265: MAME's switch editor now identifies shared analog axes and opposite
switch outputs directly from the native-channel model. Before editing it shows
same-channel, opposing-channel and analog-field counts for the selected source
port's saved draft, explicitly excluding inferred defaults. Cancellation and
threshold behavior are visible at selection time, not just in final profile
conditions. Catalog coverage remains 91/95 cores (95.8%), 289 profiles and
163/163 schematics (100% by construction). Overall completion unknown. No tests,
builds or UI/runtime checks performed.

Step 264: explicit MAME mappings now expose eight stick-direction threshold
switches, bringing selectable switch endpoints to 24 per source port. These
share four bipolar axes (opposing buttons cancel), not eight independent action
channels. Automatic action allocation is unchanged. Added four derived fixed/twin
switch panels, including mixed analog variants; shared proportional channels
retain their measured bindings. Catalog coverage stays 91/95 cores (95.8%),
289 profiles; schematic availability is now 163/163 (100% by construction).
Overall all-game completion unknown. Pinned-source inspection and formatting
only; no tests/builds or UI/runtime checks performed.

Step 263: inspected non-twin MAME games now allocate sparse Buttons 11-16 to
unused action channels, preserving conventional routes for observed Buttons 1-10.
The ten-independent-action capacity is enforced before allocation. Automatic
uses fixed-channel geometry and explicit numbered arcade presets reject these
sparse remaps; native field routes retain the actual action identities/labels.
Unselected ports receive NONE. Catalog coverage stays 91/95 cores (95.8%),
289 profiles and 159/159 schematics (100% by construction). Overall completion
unknown. No tests/builds or UI/runtime checks performed.

Step 262: twin-stick routing now considers the full native BUTTON1-BUTTON16
identity range while retaining its six-action channel limit. Sparse high-number
actions no longer fail solely due to their number. Fixed logical twin panels and
their analog variants expose all sixteen action identities; explicit allocation
uses the same range. Unassigned action defaults are cleared through BUTTON16.
This adds identities, not independent channels or cabinet-geometry claims.
Catalog coverage stays 91/95 cores (95.8%), 289 profiles and 159/159 schematics
(100% by construction). Overall completion unknown. No tests/builds or UI/runtime
checks performed.

Step 261: MAME twin-stick automatic routing now permits six observed actions
among native buttons 1-8, up from four. Eight direction inputs keep their routes;
the fifth/sixth actions use L2/R2 threshold switches. Native action numbers remain
the diagram identities. Mixed proportional fields share one measured trigger
binding regardless of the action's diagram position. Larger action sets remain
unresolved, not silently dropped. Catalog coverage stays 91/95 cores (95.8%),
289 profiles and 159/159 schematics (100% by construction). Overall all-game
completion remains unknown. No tests/builds or UI/runtime checks performed.

Step 260: automatic per-game MAME routing now covers inspected non-twin action
buttons 9/10 through the two trigger switch channels. Native buttons 1-8 and
generic defaults retain their existing routes. Automatic chooses extended fixed
geometry; undersized explicit arcade presets reject the extra controls. Review
resolves these default routes, and mixed analog profiles share one measured
pressure binding when the same trigger also owns an analog field. Twin-stick
limits remain separate. Catalog coverage stays 91/95 cores (95.8%), 289 profiles,
159/159 schematics (100% by construction); overall all-game completion unknown.
Formatting/whitespace checks only, no tests/builds or UI/runtime verification.

Step 259: MAME explicit switch assignments now include L2/R2 through native
RZAXIS_NEG_SWITCH/ZAXIS_NEG_SWITCH tokens, expanding selectable switch channels
from 14 to 16 per selected frontend port without changing numbered-button
defaults. Fixed/twin schematics expose dedicated optional digital controls;
Automatic chooses fixed-channel geometry when these outputs are requested.
Shared proportional assignments retain measured pressure requirements; otherwise
the wrapper supports digital full-travel fallback. Native switch thresholds still
apply. Catalog coverage remains 91/95 cores (95.8%), 289 profiles and 159/159
schematics (100% by construction). Overall completion unknown. Source inspection
and formatting only; no tests/builds or UI/runtime checks, no review acceptance.

Step 258: the shared source/destination diagram now cycles through all assignments
when repeatedly clicking a shared control. Control tooltips list its connections,
and the view counts displayed assignments with and without a source, explicitly
excluding whole-game coverage and runtime-readiness claims. Catalog coverage
remains 91/95 cores (95.8%), 289 profiles and 159/159 schematics (100% by
construction). Overall completion unknown. No tests/builds or UI/runtime checks.

Step 257: per-game reviews now expose native mapping denominators. MAME reports
mapped/disabled/unresolved user fields with a partition check against the complete
snapshot; machine settings/internal signals are excluded. FBNeo reports supported
native parts mapped (including legitimate aliases), while separately listing
external-adapter targets. Zero denominators yield no percentage, not artificial
100%. These are assignment counts, not runtime or whole-library coverage. Catalog
coverage stays 91/95 cores (95.8%), 289 profiles and 159/159 schematics (100% by
construction). Overall completion unknown. No tests/builds or runtime checks.

Step 256: FBNeo review now reuses saved-measurement eligibility per controller,
input part and source control, and full-axis candidates per controller. The
caches live only for one immutable review request; edits/calibration changes
cannot reuse stale results. Native target, completeness and alias validation
remain separate and unchanged. This avoids repeating physical normalization for
every descriptor/query alias in large per-game maps. Coverage stays 91/95 cores
(95.8%), 289 profiles and 159/159 schematics (100% by construction). Overall
completion unknown. No tests/builds, benchmarks or UI/runtime checks performed.

Step 255: FBNeo source selectors now apply the real saved-measurement normalizer
to each candidate control for the actual native target/part. Unsupported codes,
missing axis halves, unsuitable pressure travel and other invalid calibration
choices are excluded from clickable/dropdown choices. Existing invalid explicit
or inherited sources remain visible with a specific reason; combined-map checks
still catch cross-assignment conflicts. No input devices are opened by these
candidate checks. Coverage stays 91/95 cores (95.8%), 289 profiles and 159/159
schematics (100% by construction). Overall completion unknown. No tests/builds
or UI/runtime checks performed.

Step 254: FBNeo visual mapping now offers full-axis assignment from verified
saved-measurement pairs, including reversed polarity. Candidates require two
opposite gestures on the same native axis, valid bipolar measurements and the
transport's supported axis codes; no live device is opened. Selecting a pair
updates both target halves together and preserves unrelated native aliases for
explicit review. Coverage stays 91/95 cores (95.8%), 289 profiles and 159/159
schematics (100% by construction). Overall completion unknown. No tests/builds
or UI/runtime checks performed.

Step 253: FBNeo staging now applies the shared saved-calibration and semantic
binding checks to every selected player before mutating settings. Missing input
parts, external-adapter targets, channel collisions and invalid measurements
reject staging; failed edits remain in the text draft and the prior setup stays
intact. Acceptance uses typed analysis, not review JSON or fabricated runtime
numbering. Live identity, device numbering and core checks remain launch duties.
Coverage stays 91/95 cores (95.8%), 289 profiles and 159/159 schematics (100% by
construction). Overall completion unknown. No tests/builds or runtime checks.

Step 252: added the FBNeo logical lightgun-button destination panel and routed
supported trigger, auxiliary, direction, select, Start/Pause and offscreen-shot
bindings to it. Aliased Start/Pause share one diagram control through the existing
native translation. The panel explicitly excludes coordinate capture/offscreen
status claims; these remain separate adapter requirements. Coverage is 91/95
cores (95.8%), 289 profiles and 159/159 schematics (100% by construction).
Overall completion unknown. No tests/builds or UI/runtime checks performed.

Step 251: added a virtual FBNeo RetroPad destination-channel schematic and
connected supported gamepad/pressure/axis parts to the two-panel visual mapper.
Destination control IDs come from the same native-to-frontend translation used
by launch. Selecting a calibrated source on the diagram edits the draft; native
addresses/action descriptions remain explicit. Other peripherals retain the
source-plus-native-target view and are not presented as gamepad geometry. This
adds one derived layout: 158/158 schematics (100% by construction), with core
coverage unchanged at 91/95 (95.8%) and 289 profiles. Overall completion unknown.
No tests/builds or UI/runtime checks performed.

Step 250: extracted FBNeo semantic binding analysis from configuration rendering
and shared it with saved-mapping review. Missing target parts, external-adapter
targets, alias collisions and mismatched bipolar halves are now checked before
device opening using the same rules as launch. Review does not invent joydev
numbering or generate a launch-ready configuration; live numbering remains in
the renderer. The editor reports per-player unresolved counts and errors while
allowing corrections. Coverage stays 91/95 cores (95.8%), 289 profiles and
157/157 schematics (100% by construction). Overall completion unknown. No tests,
builds or UI/runtime checks performed.

Step 249: extracted FBNeo saved-measurement normalization from live device
preparation and reused it in per-game review. Invalid axis endpoints, inconsistent
shared measurements, pressure/bipolar role conflicts and missing native captures
can now be reported while editing without opening devices. Launch uses the same
normalizer and retains live numbering, identity/bounds, native-binding and bridge
health checks. Review errors do not prevent correcting draft assignments and do
not imply that otherwise-valid mappings are runtime verified. Coverage remains
91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by construction).
Overall completion unknown. No tests/builds or UI/runtime checks performed.

Step 248: FBNeo review now resolves observed Arcade Gun axis and lightgun
Pause/Start aliases, plus one-way pressure-to-digital fallback, for visual source
display. It distinguishes explicit bindings from inherited ones and identifies
the owning native address; clearing an explicit binding does not silently clear
its alias. The editor warns that shared bindings must satisfy launch validation
and that pressure fallback is not a second independent binding. Native identities
and launch collision checks remain unchanged. Coverage stays 91/95 cores (95.8%),
289 profiles and 157/157 schematics (100% by construction). Overall completion
unknown. No tests/builds or UI/runtime checks performed.

Step 247: FBNeo's per-target editor now includes a clickable calibrated source
controller schematic beside the exact native destination/part. Both diagram
and dropdown use the existing eligible physical choices and draft update path;
the selected source follows refreshed review data. The view explicitly labels
the destination as a native address, not reconstructed cabinet geometry. Targets
without supported binding parts still require their external adapters. Coverage
remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by
construction). Overall completion unknown. No tests/builds or UI/runtime runs.

Step 246: inspection captures exact analog field behavior (native keydelta,
optional centerdelta, sensitivity, reverse, reset and wrapping) without changing
it. Identity-validated optional metadata is carried into visual route review,
including button-driven axes; missing older evidence is labelled as missing.
Review comparison normalizes ordering and the explicit-default subset prunes
metadata consistently. Coverage remains 91/95 cores (95.8%), 289 profiles and
157/157 schematics (100% by construction). Overall completion is unknown. No
tests/builds, native inspections or UI/runtime runs performed.

Step 245: the button editor now supports incremental analog fields with separate
increment/decrement choices. Editing or removing one direction preserves the
other, and old switch assignments retain standard semantics. Both editors flag
conflicting button-versus-axis ownership before changing the draft. Labels explain
native keydelta/centering behavior, disabled unassigned directions, shared-button
interactions and the distinction from physical analog/relative input. Coverage
remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by
construction). Overall completion is unknown. No tests/builds or UI/runtime runs;
whitespace checks only.

Step 244: added button-driven MAME analog increment/decrement assignments to the
saved digital-assignment contract (old entries default to standard sequences).
Native serialization groups directions by exact field, clears inherited standard
and unused direction sequences, rejects mixed axis/button ownership and duplicate
field/sequence pairs, and counts each field once. Resolved routes/profile outputs
include both directions. Native keydelta, centering, wrapping and sensitivity
remain driver/configuration behavior, not measured analog input. Advanced JSON
can express these assignments; dedicated editor direction controls remain next.
Coverage stays 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by
construction). Overall completion unknown. No tests/builds or runtime runs.

Step 243: fixed twin-stick explicit action outputs being rejected solely because
their conventional diagram button number was already occupied by a differently
numbered native action. Profile derivation preserves existing bindings, prefers
an available explicitly named native button position, then selects a free action
position deterministically. The frontend output is unchanged; profile conditions
record the position and resolved routes retain exact native identity. Direction
clusters are not repurposed by this fallback. Coverage remains 91/95 cores
(95.8%), 289 profiles and 157/157 schematics (100% by construction). Overall
completion is unknown. No tests/builds or UI/runtime runs performed.

Step 242: fixed explicit MAME routing being rejected by pre-override twin-stick
limits. Native defaults, resolved-route review and physical profiles now share
the same derived subset of fields without explicit switch assignments. Fully
assigned extra action buttons/third direction clusters no longer go through
inapplicable fixed-channel checks; unresolved remainder still does. Original
inspection evidence is retained and every override validates against it; labels
and keyboard ownership are pruned only in the derived inference view. Explicit
fields are counted once. Coverage remains 91/95 cores (95.8%), 289 profiles and
157/157 schematics (100% by construction). Overall completion is unknown. No
tests/builds or UI/runtime runs; formatting/whitespace checks only.

Step 241: inspection now captures bounded native action/key labels keyed by
exact field identity. Digital and analog editors show those labels alongside
tag/type/mask/default, and visual route review includes them as plain text.
Labels never identify assignments or replace native ownership checks; duplicate
names remain distinct. Older snapshots fall back to identity-only display and
can be reinspected for labels. Coverage remains 91/95 cores (95.8%), 289 profiles
and 157/157 schematics (100% by construction). Overall completion is unknown.
No tests/builds or UI/runtime runs performed; formatting/whitespace checks only.

Step 240: added explicit controller-button mapping for inspected MAME KEYBOARD/
KEYPAD fields. Inspection captures exact port-device ownership, matching the
pinned native keyboard grouping; validation requires that owner to exist and be
enabled. KEYBOARD sequences additionally require natural keyboard mode off;
KEYPAD retains its native mode distinction. Missing older ownership evidence
requires reinspection. The existing switch editor/profile/review/launch path
handles these assignments; unassigned matrix fields remain unresolved. This is
individual matrix-key mapping, not text entry or full keyboard support. Coverage
remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by
construction). Overall completion is unknown. No tests/builds or runtime runs.

Step 239: native inspection now records natural-keyboard mode and exact keyboard/
keypad device enable states without mutating them. The pinned ioport.cpp digital
path accepts switch sequences but respects lockout; natkeyboard.cpp sets lockout
for disabled devices and natural keyboard mode. Optional typed snapshot data is
bounded and identity-validated, with device ordering normalized during review.
Old snapshots retain missing evidence, not inferred readiness. Keyboard matrix
assignment support remains unfinished; this captures a required precondition.
Coverage remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100%
by construction). Overall completion is unknown. No tests/builds, inspections or
UI/runtime runs performed; formatting/whitespace checks only.

Step 238: unified resolved MAME switch routes for explicit-profile requirements
and review. The diagram now includes both native default routes and overrides,
with exact field identity and origin; the text review lists channels shared by
multiple fields. Disabled/unhandled fields are not reported as mapped. This
makes default-plus-override action sharing visible rather than showing only
explicit assignments. Coverage remains 91/95 cores (95.8%), 289 profiles and
157/157 schematics (100% by construction). Overall completion remains unknown.
No tests/builds or UI/runtime runs performed; formatting/whitespace checks only.

Step 237: added the MAME digital switch editor with exact inspected field,
selected source-player and frontend output choices. Its channel list is supplied
by the same Rust table used by native serialization. New fields have no automatic
source choice; add/replace/remove preserves unrelated draft data, rejects stale
drafts and invalidates prior review. Shared explicit outputs and possible default
action sharing are disclosed; removal restores default handling, not disabling.
Coverage remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100%
by construction). Overall completion is unknown. No tests/builds or UI/runtime
runs performed; formatting/whitespace checks only.

Step 236: wired explicit digital assignments through saved MAME setup validation,
physical review/staging and launch preparation. Old setups default to an empty
list. Native preparation retains the exact list and session preparation rejects
changes to it, while both staging and launch derive the combined calibrated
profile. Automatic arcade setup continues to pass no explicit overrides. The
review text and selected diagram control now expose exact digital native routes.
Advanced JSON can specify these assignments; a dedicated switch editor remains
unfinished. This is source implementation, not verified runtime compatibility.
Coverage remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100%
by construction). Overall completion is unknown. No tests/builds or UI/runtime
runs performed; formatting and whitespace checks only.

Step 235: added physical-profile derivation for explicit MAME digital overrides.
It retains the original inspected twin-stick channel numbering, removes obsolete
digital requirements only when no other field uses the channel, adds all selected
override outputs, and preserves analog requirements. Automatic button geometry
can expand to eight buttons; incompatible explicit presets fail instead of
silently changing layout. Shared channels require one physical binding. This
backend is not wired into saved setup/review/launch yet, so coverage remains
91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by construction).
Overall completion remains unknown. No tests/builds or UI/runtime runs performed.

Step 234: added an exact-field digital override planner for MAME. It accepts
only inspected user-mappable switches and known frontend switch outputs on
selected source ports, rejects duplicate field identities, composes after the
digital/analog layer, and removes saved sequences only for the assigned fields.
Unrelated native settings/device maps remain intact; previously disabled or
unhandled fields are accounted for separately from already mapped fields.
This is backend planning only: saved settings, physical-profile requirements,
launch wiring and UI selection still need integration before these overrides
are usable. Coverage remains 91/95 cores (95.8%), 289 profiles and 157/157
schematics (100% by construction). Overall completion is unknown. No tests,
builds or UI/runtime runs performed.

Step 233: the mapping diagram can show all connections together and highlights
the selected connection above them. MAME analog review now links each frontend
control to all explicitly assigned native fields, showing exact tag/type/mask/
default identity, range and controller-aim/stick-velocity/absolute mode. Routes
come from the same channel control table used to generate the profile; this is
not an inference from game titles or a claim of physical gun support. Coverage
remains 91/95 cores (95.8%), 289 profiles and 157/157 schematics (100% by
construction, not visual QA). Overall completion remains unknown. No tests,
builds or UI/runtime runs performed.

Step 232: added explicit controller-aim selection for MAME LIGHTGUN_X/Y fields,
using the pinned core's absolute input branch. Saved mode defaults false and
must match the field kind; only full/reversed bipolar stick channels are allowed.
The form/review distinguishes controller aim from physical gun capture and notes
that trigger, auxiliary and game-specific off-screen/reload behavior still need
their own mappings. Physical lightgun support is not claimed. Coverage remains
91/95 cores (95.8%), 289 catalog profiles and 157/157 schematics (100%). Overall
completion is unknown. No builds/tests or UI/runtime runs performed.

Step 231: added explicit stick-velocity mode for native DIAL/TRACKBALL/MOUSE
fields, through the existing inspected/calibrated path. Saved relative_velocity
defaults false and the form requires opt-in. Only full/reversed centered stick
axes are accepted; pressure/half ranges would move at rest. This uses pinned
ioport.cpp's absolute-input velocity branch (rawvalue/8 with native reset and
sensitivity), not physical relative delta capture. That separate device path
remains unfinished. Coverage remains 91/95 cores (95.8%), 289 catalog profiles
and 157/157 schematics (100%). Overall completion is unknown. No tests/builds or
UI/runtime runs performed.

Step 230: extended the explicit MAME absolute-channel path to POSITIONAL and
POSITIONAL_V fields, including the assignment form. Pinned ioport.cpp:3613-3623
and 3855-3900 defines absolute-to-position conversion and upper-endpoint clamp;
the adapter leaves native position count/wrapping logic intact. The UI identifies
this as absolute position selection, not physical relative-encoder support.
Coverage remains 91/95 cores (95.8%), 289 catalog profiles and 157/157 schematics
(100%). Overall completion is unknown. No builds/tests or UI/runtime runs.

Step 229: MAME per-game review now includes selectable-player source/destination
schematics using its actual combined mapping rows and exact source/target layout
IDs. Native field/range choices, calibration errors and preserved-field details
remain in the textual review below; diagrams are not presented as runtime proof.
Coverage remains 91/95 cores (95.8%), 289 catalog profiles and 157/157 schematics
(100%). Overall completion is unknown. No builds/tests or UI/runtime runs.

Step 228: added saved/UI selection of native full, reversed, positive-half and
negative-half ranges for MAME stick channels. Tokens use native REVERSE/POS/NEG
after measured normalization. Existing assignments default to full; pressure
channels retain their fixed NEG conversion and reject added stick modifiers.
Review shows the range and the form explains half-axis pedal control. Coverage
remains 91/95 cores (95.8%), 289 catalog profiles and 157/157 schematics (100%).
Overall completion is unknown. No builds/tests or UI/runtime runs performed.

Step 227: corrected native MAME pressure sequences to RZAXIS_NEG/ZAXIS_NEG.
Pinned input_retro.cpp negates analog trigger values and uses the negative-half
modifier for pedal assignments; inputdev.cpp expands that half to the full
absolute range. Bare axes would leave released pressure at midrange. Stick
channels retain full-axis tokens. Coverage remains 91/95 cores (95.8%), 289
catalog profiles and 157/157 schematics (100%). Overall completion remains
unknown. Source inspection and formatting only; no builds/tests or runtime runs.

Step 226: connected public MAME setup staging to combined native planning and
normalized calibration checks. Analog setups can stage only with no unhandled
active fields, all requested physical bindings, and valid normalized inputs.
Review distinguishes saved-measurement acceptance from live/runtime validation;
staging remains atomic and opens no devices. This completes source wiring for
explicit absolute stick/paddle/pedal setups, not runtime acceptance or general
MAME coverage. Relative/lightgun and other special fields remain unsupported.
Coverage remains 91/95 cores (95.8%), 289 catalog profiles and 157/157 schematics
(100%). Overall completion is unknown. No builds/tests or UI/runtime runs.

Step 225: fresh MAME launch inspection now chooses the mixed native planner
when a saved setup contains explicit analog assignments, using the actual
controller/game configuration layers. The inspection retains those assignments;
calibrated-session preparation rejects a different set. Automatic arcade setup
continues to pass no analog choices. Unsupported active fields still block
staging. Public setup staging remains to be connected to combined calibration
validation. Coverage remains 91/95 cores (95.8%), 289 catalog profiles and
157/157 schematics (100%). Overall completion is unknown. No builds/tests or
hardware/runtime runs performed.

Step 224: MAME calibrated-session preparation now accepts explicit analog
assignments and uses combined profiles. Analog players start an owned normalized
gamepad bridge, recheck physical identity, wait within a bounded/cancellable
joydev-readiness window and emit frontend bindings from the translated virtual
calibration. Digital-only players keep their prior transport. Bridges remain
owned by the existing launch health/cleanup lifecycle. Native inspection-plan
selection and setup staging still prevent end-to-end analog launch activation.
Coverage remains 91/95 cores (95.8%), 289 catalog profiles and 157/157 schematics
(100%). Overall completion is unknown. No builds/tests or hardware/runtime runs.

Step 223: combined MAME review now prepares a pure normalized-input contract
using the existing measured gamepad normalizer. It retains opposite observations
for selected axes, maps stick center/travel and pressure into virtual units, and
checks that normalization preserves every requested physical assignment. Only
selected gameplay bindings remain in the returned calibration. No device is
opened by this preparation. Live bridge/configuration launch wiring remains
unfinished. Coverage remains 91/95 cores (95.8%), 289 catalog profiles and
157/157 schematics (100%). Overall completion is unknown. No builds/tests or
UI/runtime runs performed.

Step 222: analog measurement validation now checks the actual combined MAME
mapping rows shown in review, rather than independently resolving an analog-only
profile. It also checks unique expected output rows and exact correspondence to
the selected saved physical bindings. Per-player validation failures remain
visible in review. Coverage remains 91/95 cores (95.8%), 289 catalog profiles
and 157/157 schematics (100%). Analog launch wiring and overall completion remain
unfinished. No builds/tests or UI/runtime runs performed.

Step 221: added per-player combined MAME profiles and connected them to setup
review. Digital bindings retain their chosen preset, while only explicitly used
analog channels are added to the corresponding combined layout. Analog-only
players receive an analog target without invented buttons. Conflicting target
IDs/frontend outputs fail rather than overwrite another assignment. Profiles
remain per-inspection drafts, not general core coverage. Coverage remains 91/95
cores (95.8%), 289 catalog profiles and 157/157 schematics (100%). Combined
measurement checks/launch wiring and overall completion remain unfinished.
No builds/tests or UI/runtime runs performed.

Step 220: added five deterministically derived MAME digital-plus-analog target
layouts (fixed, twin-stick, six-button, eight-button and Neo Geo). They retain
digital ordering in an upper band and expose ten analog direction/pressure
controls below; existing SVG/explorer generation covers the derived layouts.
Also corrected shape validation to accept the grid shape already used by two
bundled rhythm layouts. Catalog availability is now 152 stored + 5 derived =
157/157 schematic layouts (100% availability by construction, not visual QA).
Core coverage remains 91/95 (95.8%), 289 profiles. Combined profile/launch wiring
and overall all-game completion remain unfinished. No builds/tests or UI runs.

Step 219: added a MAME analog assignment form listing supported inspected
absolute fields, selected source players and six native channels. It loads
existing choices, adds/replaces/removes exact field assignments in the draft,
rejects stale editor state and clears prior review approval after edits. No
source channel is guessed for a new field. Review/calibration and analog launch
integration remain separate; no devices are opened. Coverage remains 91/95
cores (95.8%), 289 profiles and 152/152 schematics (100%). Overall completion
is unknown. No builds/tests or UI/runtime runs performed.

Step 218: corrected transport selection for partial controller profiles. A
pressure bridge is now required only when the mapping requests a pressure
control, not merely because the target layout contains unused triggers. This
removes an erroneous trigger requirement for stick-only analog profiles while
retaining checks for requested but unmapped pressure controls. MAME combined
profile/configuration launch integration remains unfinished. Coverage remains
91/95 cores (95.8%), 289 profiles and 152/152 schematics (100%). Overall all-game
completion remains unknown. No builds/tests or runtime probes performed.

Step 217: MAME analog review now resolves each used native channel through the
catalog mapper and checks saved Linux physical measurements. Bipolar channels
require compatible opposite measurements of one axis; pressure channels require
continuous travel. Distinct output channels cannot share a physical axis. Review
shows mapped rows or a per-player calibration error without pretending native
channel selection proves physical input. Analog launch integration remains
unfinished. Coverage remains 91/95 cores (95.8%), 289 profiles and 152/152
schematics (100%). All-game completion is unknown. No builds/tests or runtime runs.

Step 216: the MAME setup review now consumes mixed/analog-only plans and shows
each explicit analog field identity with its source player/channel. Digital
calibration rows remain visible when present; analog-only ports no longer need
a fake digital profile to review. The UI explicitly marks analog calibration
and launch integration as pending and keeps launch staging disabled for these
drafts. Coverage remains 91/95 cores (95.8%), 289 profiles and 152/152 schematics
(100%). All-game completion remains unknown. No builds/tests or UI/runtime runs.

Step 215: saved MAME setups now retain explicit analog field/source/channel
assignments with stable snake-case channel names and strict unknown-field
rejection. Older setups default to no analog assignments. Validation reuses the
native sequence generator's active-field/routing/duplicate checks, requires
selected source players and current inspection, and permits an analog-only port
without a fake digital profile. Physical calibration proof, editor integration
and analog launch activation remain unfinished. Coverage remains 91/95 cores
(95.8%), 289 profiles and 152/152 schematics (100%). All-game completion is
unknown. No builds/tests or runtime probes performed.

Step 214: analog-aware MAME planning now accepts analog-only games without
requiring a synthetic digital assignment. The shared digital layer can report
zero mapped fields, while the digital-only public entry retains its original
nonzero requirement. Explicit analog assignments must use selected frontend
ports and still pass active-field validation; unsupported fields remain visible.
Calibration/settings/launch integration remain unfinished. Coverage remains
91/95 cores (95.8%), 289 profiles and 152/152 schematics (100%). Overall all-game
completion remains unknown. No builds/tests or runtime probes performed.

Step 213: added mixed digital/absolute-analog MAME plan composition. Original
controller device maps remain first; explicit analog defaults follow digital
defaults. Saved standard/increment/decrement overrides are removed only for
exact selected analog field identities, using native masked-defvalue matching;
unrelated fields and sensitivity/reverse attributes remain intact. Assigned
analog fields leave the plan's unhandled list. This still requires at least one
mapped digital field; analog-only planning, calibration and launch integration
remain unfinished. Coverage remains 91/95 cores (95.8%), 289 profiles and 152/152
schematics (100%). All-game completion remains unknown. No builds/tests or
runtime probes performed.

Step 212: added exact-field native MAME XML generation for explicit absolute
stick, paddle and pedal assignments, using all six source-defined libretro
analog channels and observed joystick routing. It rejects stale/duplicate fields
and relative/lightgun types, and clears digital increment/decrement aliases on
the owned fields. L2 maps to RZAXIS and R2 to ZAXIS. Physical capability proof,
mixed digital/analog config merging and launch integration remain required;
no generic analog-game coverage is claimed. Coverage remains 91/95 cores
(95.8%), 289 profiles and 152/152 schematics (100%). All-game completion remains
unknown. No builds/tests or runtime probes performed.

Step 211: native BizHawk/Nymashock preparation now retains SHA-256 and resolved
path identity for the selected launch program, EmuHawk assembly, SDL library and
probe before controller probing. The prepared config rechecks those artifacts
at preparation completion and through the existing pre-launch input check.
Reads are bounded and reject observed file growth or changes. This does not
cover transitive dependencies or prove the pinned runtime contract, and cannot
remove the final check-to-exec race. Core coverage remains 91/95 (95.8%), with
289 profiles and 152/152 schematics (100%). All-game completion remains unknown.
No builds/tests or runtime probes performed.

Step 210: added a kernel uevent topology guard and required it in the relative
background worker. Subscribe after output creation but before frontend startup;
input events permanently invalidate numbering, including transient hotplug.
Queue loss, malformed messages and excessive backlog also fail closed. The
worker checks before each pump and shuts down the group on failure. This is not
atomic with frontend reindexing and cannot prove race-free numeric routing;
exact frontend routing and launch/focus integration remain unfinished. Coverage
remains 91/95 cores (95.8%), 289 profiles and 152/152 schematics (100%). Overall
all-game completion is unknown. No builds/tests or hardware probes performed.

Step 209: confirmed the pinned RetroArch udev startup log reports mouse indices,
coordinate modes and exact event nodes, and added a bounded strict parser plus
owned-REL-endpoint lookup. Duplicate indices/paths, malformed rows, gaps and
missing endpoints are rejected. Startup observations are explicitly not durable
routes: hotplug invalidation and log/runtime provenance remain launch-owner
requirements. No launch adapter was enabled. Coverage remains 91/95 cores
(95.8%), 289 profiles and 152/152 schematics (100% representation availability).
All-game completion remains unknown. No builds/tests or hardware execution.

Step 208: added an owned relative-device background bridge using the existing
gamepad worker's interruptible 2ms polling pattern. It requires complete endpoint
readiness before starting, forwards group errors through health/shutdown results,
and joins during shutdown/drop so physical capture and virtual outputs are not
left running by an abandoned owner. It does not itself establish frontend routes
or observe focus. Wiring to emulator exit/focus policy remains required before
enabling launch. Coverage remains 91/95 cores (95.8%), 289 profiles, 152/152
schematic layouts (100% representation availability); all-game completion is
unknown. No builds, tests, UI execution or hardware probes performed.

Step 207: added explicit, default-off exclusive physical-source capture to saved
relative-device records and their form. Saved-session opening requests EVIOCGRAB
only after output creation succeeds; a denied grab fails opening rather than
silently falling back. Neutralization now closes the source descriptor, releasing
capture even if a stopped session remains allocated. Group startup/failure/drop
inherits that cleanup. The UI warns that the entire physical event node is
captured and a separate stop input is needed. Virtual-output desktop exclusion
and exact frontend routing remain unfinished; no launch coverage is promoted.
Coverage remains 91/95 cores (95.8%), 289 profiles and 152/152 schematic layouts
(100% representation availability). All-game completion remains unknown.
No builds, tests, or hardware probes performed.

Step 206: added grouped ownership of selected player-relative transports. The
complete selection is validated before opening; partial-open failure drops all
created sessions. Endpoint readiness returns only a complete set; endpoint or
pump errors shut down every member. Player keys remain distinct from frontend
mouse indices. Pinned RetroArch udev source reindexes pointers on hotplug, so
event-node numbering cannot establish a stable frontend route. Exact routing
and launch integration remain required. Coverage remains 91/95 cores (95.8%),
289 profiles, and 152/152 layout schematics (100% representation availability).
Overall all-game completion is unknown. No builds/tests or device execution.

Step 205: added a structured relative-device draft form for exact event and
physical identity paths, all four supported axes, all eight one-to-one mouse
button outputs, X/Y swap, inversion and 1–1000% output sensitivity. Existing
JSON drafts are preserved on parse, duplicate identity or device-count errors;
the Rust validator remains authoritative when staging the complete list. Adding
an entry does not open hardware or enable frontend routing. Core coverage is
unchanged at 91/95 (95.8%), with 289 profiles and 152/152 schematic layouts
(100% representation availability). All-game completion remains unknown.
No builds, tests, hardware probes or UI execution performed.

Step 204: added a source/destination visual layout explorer covering all 152
catalog layouts through generated schematics. Users select either layout and
click controls or select an assignment to highlight both endpoints and a
connecting curve; unmapped controls and mapping reasons remain visible. The
same component is integrated into actual calibration/profile previews. The
explorer uses the existing Rust assignment solver and clearly labels generic
suggestions separately from physical calibration. Grid layouts now use a panel
outline, and nondirectional analog controls retain their proper labels instead
of a right-arrow fallback. Formatting/static checks passed; no build, UI run or
tests occurred. Schematic availability is 152/152 layouts (100%); visual fidelity
and runtime behavior remain unverified. Core coverage remains 91/95 (95.8%),
289 profiles. Overall all-game completion is unknown.

Step 203: added portable saved relative-device settings and an advanced JSON
editor. Records retain exact event/physical-identity paths, selected axes,
one-to-one button maps and motion calibration. Settings validation bounds device
counts and paths and rejects duplicate identities/controls. Staging is atomic;
the main Save action persists the list. A native session entry point consumes
the saved record and reuses exact-device opening checks. No capture or emulator
launch is triggered by editing/staging. Formatting/static checks passed; no
builds/tests or device probes ran. Coverage remains 91/95 cores (95.8%), 289
profiles and 152 layouts. Frontend mouse routing and guided device discovery
remain unfinished; all-game completion is unknown.

Step 202: relative sessions now accept explicit physical-to-output mouse-button
maps. Sources and outputs must be distinct mouse-button identities; identity
mapping remains the default. The publisher advertises mapped outputs, complete
button state is transformed before publication, and shutdown releases output
identities even after capture failure. Formatting/static checks passed; no
builds/tests or device probes ran. Coverage remains 91/95 cores (95.8%), 289
profiles and 152 layouts. Relative calibration UI and frontend routing remain
unfinished; all-game completion is unknown.

Step 201: relative-motion calibration now supports explicit X/Y swapping before
output-axis sensitivity and inversion. Virtual-device selection follows the
same permutation, including single-axis sources; wheels and buttons remain
unchanged. Missing swap_xy settings default false. Formatting/static checks
passed; no builds/tests or device probes ran. Coverage remains 91/95 cores
(95.8%), 289 profiles and 152 layouts. User-facing calibration and frontend
mouse routing remain unfinished; all-game completion is unknown.

Step 200: relative-mouse sessions now accept explicit per-axis sensitivity
(1–1000 percent) and inversion. Default sessions retain 100-percent movement.
Signed fractional counts carry between complete reports, wheels/buttons pass
through unchanged, and shutdown discards residual movement. Arithmetic overflow
fails the session instead of clipping movement. This is connected to the session
pump; user-facing calibration and FBNeo frontend routing remain unfinished.
Formatting/static checks passed; no builds/tests or device probes ran. Coverage
remains 91/95 cores (95.8%), 289 profiles and 152 layouts. All-game completion
remains unknown.

Step 199: relative mouse transport now retains conventional horizontal and
vertical wheel detents and advertises selected wheel axes on its owned virtual
device. High-resolution wheel events remain separate and ignored unless a
future explicit conversion is implemented. Native RetroArch inspection shows
its udev adapter accepts only unit wheel values and retains boolean flags;
frontend wheel conversion/polling and exact mouse routing remain unfinished.
This does not enable FBNeo mouse launch. Formatting/static checks passed; no
builds/tests or device probes ran. Coverage remains 91/95 cores (95.8%), 289
profiles and 152 layouts. All-game completion remains unknown.

Step 198: automatic arcade setup now offers opt-in dependency discovery in the
selected archive's directory. The setting defaults off for existing/self-contained
set workflows. Enabled launches use the same core-derived split/nonmerged
discovery as advanced inspection, retain the expanded manifest for session
staging and source checks, and allow 120 seconds for combined discovery and
inspection. Exact saved setups retain their explicit manifests and priority.
The default-layout selector now mentions twin-stick selection. Formatting/static
checks passed; no builds, tests or native runs occurred. Coverage remains 91/95
cores (95.8%), 289 profiles and 152 layouts; all-game completion remains unknown.

Step 197: twin-stick assignment planning now requires physical calibration for
every observed control, including actions marked optional in the reusable layout.
It rejects shared buttons/axes across sticks and actions. Only an opposite
direction pair of the same stick may share a physical axis, with opposite raw
directions. These checks run through assignment review and launch preparation.
Formatting/static checks passed; no builds/tests or hardware probes ran.
Coverage remains 91/95 cores (95.8%), 289 profiles and 152 layouts. All-game
completion remains unknown.

Step 196: twin-stick games now support up to four observed numbered action
buttons. A shared snapshot-derived sequence map reserves switches 1-4 for right
directions and allocates actions in native-number order to switches 5-8. Profiles
retain button1..button8 identities; only observed actions are required. Native
configuration and frontend calibration use the same allocation. More than four
actions or an additional normal joystick cluster remain blocked. Formatting and
static whitespace checks passed; no builds/tests ran. Coverage remains 91/95
cores (95.8%), 289 profiles and 152 layouts. All-game completion is unknown.

Step 195: added a dedicated MAME twin-digital-stick layout and native field
routing. Left directions use the native hat; right directions use native buttons
4/1/3/2 for up/down/left/right. Automatic selection uses observed native
JOYSTICKLEFT/JOYSTICKRIGHT fields. Explicit button-row presets reject this layout.
Mixed games needing normal joystick fields or buttons 1-4 alongside twin sticks
remain blocked because those outputs conflict; no independent controls are
silently combined. Only observed controls enter the calibration profile, and
unselected players retain NONE sequences. Formatting/static checks passed; no
builds, tests or native inspections ran. Coverage remains 91/95 cores (95.8%)
and 289 profiles, now with 152 layouts. All-game completion remains unknown.

Step 194: required dependency lookup now honors an explicitly declared exact
staging destination before searching folders. BIOS archives and disks can remain
at selected absolute regular-file locations outside discovery roots. Multiple
declared alternatives and duplicate destinations fail; an invalid explicit path
does not fall through to another file. Staging fingerprints and receipt matching
remain in force. Formatting/static checks passed; no builds or tests ran.
Coverage remains 91/95 cores (95.8%), 289 profiles and 151 layouts; all-game
completion is unknown, with merged archives and special controls unfinished.

Step 193: native dependency metadata now retains validated SHA-1 identities
for good CHD dumps. Discovery can search differently named ancestor disks only
when those identities match, using the same bounded parent-folder search and
ambiguity rejection. Missing hashes and bad/undumped metadata do not authorize
filename substitution. This follows romload.cpp's parent hash comparison; actual
CHD validation remains the native loader's responsibility. Repeated conflicting
disk identities fail. Formatting/static checks passed; no builds or tests ran.
Coverage remains 91/95 cores (95.8%), 289 profiles and 151 layouts. Merged ROM
archives and broader special-control coverage remain incomplete; all-game
completion is unknown.

Step 192: CHD discovery now searches the exact ROM-parent folder chain for the
same disk filename and stages the selected file under the requesting set's
directory. Missing parent metadata, cycles and ambiguous candidates fail.
Source evidence is pinned MAME driver.cpp's parent search-path construction,
infoxml.cpp's romof emission and romload.cpp's do_open_disk lookup. Different
parent filenames still require hash-bearing metadata or an explicit manifest.
Scoped formatting and static whitespace checks passed; no builds, tests or
runtime inspections ran. Coverage remains 91/95 cores (95.8%), 289 profiles and
151 layouts. All-game completion remains unknown.

Step 191: discovery now unions mandatory dependencies with the explicitly
declared manifest. Optional ROM/disk files are preserved even when native
metadata does not require them. Identical source/destination pairs coalesce;
duplicate declared destinations and conflicting discovered sources still fail.
All retained files pass through the same staging, fingerprint and receipt checks.
Scoped formatting and static whitespace checks passed; no builds, tests or
runtime inspections were run. Coverage remains 91/95 cores (95.8%), 289 enabled
profiles and 151 layouts. This expands the advanced setup workflow without
promoting another general core. All-game completion remains unknown.

Step 190: added opt-in dependency discovery to advanced MAME field inspection.
The selected runtime/core emits bounded native XML for the exact machine and
reachable parents/devices; the existing resolver locates split/nonmerged archives
and CHDs in explicitly selected directories. Discovery and field inspection share
the runtime/core fingerprint checks, cancellation and deadline. The completed
receipt retains both the requested and expanded manifests; applying it requires
the original draft to match and preserves declared configuration sources. The
dialog accepts one search directory per line and requires renewed trust after
request edits. Native output uses a nonblocking Unix socket in the supervisor so
an inherited writer cannot make a reader join ignore cancellation. No native code
was run during implementation. Formatting and static whitespace checks are the
only validation performed; compilation, tests, runtime checks and independent
acceptance review remain pending. Coverage remains 91/95 cores (95.8%), 289
enabled profiles and 151 layouts. Merged archives, complete new-setup guidance,
analog/peripheral contracts and all-game coverage remain unfinished; the overall
all-game percentage is unknown.

Step 189: MAME frontend configuration now uses the bounded source-aware reader
shared with core options. Preparation retains the main file's lexical location,
canonical target and contents and rechecks them before/after inspection and before
launch. A valid configuration symlink is supported; a changed target with identical
bytes, a broken link, disappearance, unreadable/non-regular source or oversized
file stops preparation. A missing main file no longer means empty defaults:
pinned RetroArch configuration.c:3642-3676 can select a legacy home configuration
or a build-specific system skeleton, so MAME requires an explicit initialized
primary file rather than guessing those sources. Other adapters' shared base
reader is unchanged. Scoped formatting, tracked whitespace and direct untracked
trailing-whitespace checks passed. Coverage remains 91/95 cores (95.8%), 289
enabled profiles and 151 layouts; this hardening adds no general core/game
coverage. No builds, tests, hardware inspections or emulator runs were performed.

Step 188: separated stored MAME snapshot validation from current inspection,
review and launch validation. Settings load/save now retain structurally valid
schema-3 setups without rewriting their version or legacy button geometry.
They remain visible with a needs-reinspection status, and loading a setup clears
its assignment approval. New inspection parsing, profile generation, staging and
exact saved-setup launch still require schema 4; stale saved overrides do not fall
through to automatic defaults. The existing inspection-from-draft path does not
consume the stale snapshot and can replace it with a matching completed receipt.
Unsupported schemas and malformed stored identities remain errors. Scoped Rust
formatting and tracked whitespace checks passed; no builds, tests, hardware
inspections or emulator runs were performed. Fresh catalog counts remain 91/95
cores (95.8%), 289 enabled profiles and 151 layouts. All-game coverage remains
unknown; required independent review and pre-commit verification are outstanding.

Step 187: added conservative native internal-signal classification to snapshot
schema 4. The pinned Lua fields table inserts every non-internal field's name;
collisions can replace a value but cannot remove its key. When the class getter
fails, only absence of the exact successfully read field name proves native
exclusion. Collisions and other errors remain unknown and block digital staging.
The mask-based field traversal is unchanged. Proven internal signals are retained
and displayed separately from controller bindings and DIP/configuration settings.
Earlier snapshots require reinspection. The pending step-186 scoped formatting
and whitespace checks have now completed. Coverage remains 91/95 cores (95.8%),
289 enabled profiles and 151 layouts; all-game coverage is not established.
No builds, tests, hardware inspections or emulator runs were performed.

Step 186: formatted the scoped Rust implementation and reconciled current
layout/MAME documentation with the source. The catalog recount confirms 151
layouts and 289 enabled profiles across 91 distinct cores: 91/95 (95.8%) against
the documented canonical-core baseline, with fbneo, mame, nymashock and steemsse
still outside general catalog coverage. The 22,801 possible layout pairs are
not an executed audit. Pinned RetroArch source confirms the existing save/state
default handling; unlike the config-directory setting, explicit empty save/state
values do not select the content directory. All-game/mode coverage and remaining
work cannot be inferred from core counts. No builds, tests or emulator runs were
performed; required independent review and pre-commit checks remain outstanding.

Step 185: MAME core-option selection now distinguishes missing candidates from
filesystem errors, preserves game/folder/core/global precedence and retains the
selected filename, canonical target and bytes. Broken option-file symlinks,
unreadable or non-regular files and files over 8 MiB fail explicitly. Valid
symlinked options remain usable; retargeting or new higher-priority selection
requires renewed preparation even when bytes match. Draft generation shares
this resolver and verifies its snapshot after collecting state inputs.
Coverage remains 91/95 cores (95.8%), 289 enabled profiles. No builds, tests or
emulator runs were performed.

Step 184: automatic MAME player discovery now uses a typed optional profile:
an empty standard digital binding subset is distinct from an invalid snapshot
or incompatible preset. Errors retain player context and propagate instead of
being swallowed by an is_ok check, and each active profile is constructed once.
The complete field plan still rejects unsupported native fields before player
discovery; saved setups still require every selected player to have controls.
Coverage remains 91/95 cores (95.8%), 289 enabled profiles. No builds, tests or
emulator runs were performed.

Step 183: MAME persistence and native inspection now share one retained frontend
and core-options snapshot. Before/after inspection and before launch, preparation
rechecks the base configuration, effective option selection and applicable
override absence. Automatic setups also retain lexical-to-canonical save roots
and both presence and absence of native default/game cfg files. Changes require
renewed preparation instead of mixing configuration generations. Source review
resolved omitted versus explicitly empty/default config directories; override,
core-option and draft lookups now use that same MAME-specific resolver.
Coverage remains 91/95 cores (95.8%), 289 enabled profiles. No builds, tests or
emulator runs were performed.

Step 182: automatic MAME preparation now rejects applicable RetroArch core,
content-folder and game .cfg overrides before creating persistent directories or
starting native inspection. Explicitly disabled override loading is respected;
unrelated games' files are not rejected. Symlinks and unreadable candidates are
not treated as proof of absence. This initial guard checked empty/default roots
at both possible locations; step 183 replaced that with the exact source-derived
config-directory resolver.
This closes the base-only routing gap identified after step 181; effective
override merging remains unsupported. Coverage remains 91/95 cores (95.8%),
289 enabled profiles. No builds, tests or emulator runs were performed.

Step 181: wired an automatic MAME digital-arcade fallback after exact saved
setups, with a persistent default selector in settings. It accepts exact
self-contained ZIP/7z set names on the Arcade platform, inspects native fields,
requires only active controls, disconnects unused ports and reuses owned native
configuration/physical transport. Native cfg layers are retained; its initial
base-only frontend save routing required the override guard added in step 182.
Unsupported inputs and unresolved dependencies remain
explicit errors; this does not claim every arcade game or all MAME systems.
No general catalog profile was enabled: coverage remains 91/95 cores (95.8%),
289 enabled profiles. No builds, tests or native inspection were run.

Step 180: connected MAME presets to saved per-game setup validation, physical
assignment review and native launch transport. The setup dialog now offers
Automatic, six-button, eight-button and Neo Geo choices without hand-editing
the layout field. Changing a preset invalidates review. Older saved setups
retain fixed-channel geometry and still take precedence over defaults.
Coverage remains 91/95 cores (95.8%), 289 enabled profiles. No tests run.

Step 179: added reusable six/eight-button physical arcade layouts and family
rules (policy 6), plus active-field-derived MAME six/eight/Neo Geo presets.
The eight-button arrangement preserves the first six positions and routes its
extra buttons through fixed frontend L3/R3. Only observed controls are required;
too-small presets reject the game instead of silently omitting buttons.
Launch/UI integration follows. Coverage remains 91/95 cores (95.8%), 289 enabled
profiles; these layouts do not by themselves enable another core. No tests run.

Step 178: added an exact metadata-driven MAME dependency resolver for declared
parent sets, device ROMs and disks. It traverses a bounded dependency closure
and rejects missing or ambiguous local files instead of guessing similar names.
It currently requires separate archives for ROM-bearing dependencies; merged
archive member resolution, metadata import/provenance and UI wiring remain
unfinished. This does not establish complete dependency coverage. Coverage stays
91/95 cores (95.8%), 289 enabled profiles. No builds or tests run.

Step 177: added inspection-request generation from a setup draft and explicit
native RetroArch path. It resolves effective options, applies the digital
contract and discovers persistent state without running the emulator. Snapshot
transfer accepts discovered state only under the draft's selected roots while
requiring its ROM/configuration manifest to match. ROM/BIOS/CHD inputs still
require explicit declaration. Coverage stays 91/95 cores (95.8%), 289 enabled
profiles. Formatting only; no builds, tests or native inspection run.

Step 176: connected completed MAME inspection receipts to the setup editor.
Applying a result checks exact machine/core/input-manifest identity, transfers
the inspected core/content hashes and snapshot, and invalidates the assignment
review. It neither stages nor saves the setup automatically. Missing metadata
and mismatched drafts remain explicit errors. Automatic request/dependency
generation remains pending. Coverage stays 91/95 cores (95.8%), 289 enabled
profiles. No builds, tests or native inspection were run.

Step 175: added complete file discovery under the selected MAME NVRAM and disk
difference roots. Saved-setup launch preparation merges this current manifest
with declared ROM/configuration inputs, rejecting conflicting state entries.
Pre-spawn verification checks the file list as well as existing source hashes,
so added/removed state files require reinspection. Discovery is bounded and
does not follow nested symlinks. ROM/BIOS/CHD dependency discovery and guided
inspection setup remain pending. Coverage stays 91/95 cores (95.8%), 289 enabled
profiles. Formatting/source inspection only; no builds or tests.

Step 174: connected an advanced MAME inspection dialog to a background settings
action using the existing native supervisor. It accepts explicit request JSON,
requires a trusted-runtime confirmation, exposes cancellation/status/result and
does not save or silently accept a snapshot. Closing cancels active inspection.
Automatic request/dependency discovery and guided snapshot-to-setup integration
remain pending. No inspection was executed during this implementation.
Coverage stays 91/95 cores (95.8%), 289 enabled profiles. No builds or tests run.

Step 173: moved unresolved MAME native-field rejection into setup staging as
well as launch. Review displays each unresolved field's class and disables
staging until all active input contracts are handled. Source inspection confirms
the class getter throws for internal fields and the alternative named-field
table may lose colliding names; neither is used to silently discard unknown
fields. Internal-signal classification remains unresolved, not counted as support.
Coverage stays 91/95 cores (95.8%), 289 enabled profiles. No builds or tests run.

Step 172: added native MAME field classes to snapshot version 3. DIP-switch and
configuration fields are retained as machine settings rather than counted as
missing controller mappings. Keyboard, unknown and unsupported miscellaneous
inputs remain visible and are not silently ignored. The review reports preserved
settings separately. Source: pinned luaengine_input.cpp 499-514 and ioport.cpp
870-896; getter failures are classified as unknown, not assumed internal.
Older snapshots require reinspection. Coverage remains 91/95 cores (95.8%),
289 enabled profiles. Formatting/source inspection only; no builds or tests.

Step 171: added readable MAME physical-assignment review from saved calibration
data. The UI shows per-player target/source/output rows, mapping warnings and
unhandled native fields; editing the setup requires refreshing the review before
staging. The preview explicitly does not verify connected hardware or runtime
behavior. Guided native inspection and dependency discovery remain pending.
Coverage remains 91/95 cores (95.8%), 289 enabled profiles. No builds or tests.

Step 170: added advanced MAME setup management in the settings UI and registered
the QML component. Users can load/paste/stage setups and confirm removals; settings
must still be saved to persist changes. Staging validates saved calibration
coverage without opening devices or running a core. The UI explicitly separates
saved data from runtime verification and preserves un-staged editor text on close.
Guided inspection/review and complete dependency discovery remain pending.
Coverage stays 91/95 cores (95.8%), 289 enabled profiles. No builds or tests run.

Step 169: added serialized per-content MAME controller setups and connected
them to normal calibrated launch preparation. The branch matches exact
emulator/core/content identities, checks saved core/content hashes, reinspects
native fields, compares the reviewed snapshot, resolves selected connected
calibrations, stages native inputs and invokes the calibrated session builder.
Settings validate bounded dependency manifests and unique player/setup identities.
GUI creation/review and complete dependency/state discovery remain pending;
this saved-data path has not been executed. Coverage stays 91/95 cores (95.8%),
289 enabled profiles. Formatting/source inspection only; no builds or tests.

Step 168: added the MAME fixed-digital target layout, active-field-derived
per-player calibration profiles and a calibrated-session constructor. It uses
the existing physical numbering/transport/rendering code, requires the exact
inspected player set, rejects duplicate physical assignments, clears unused
ports and attaches native configuration with retained session ownership.
Only active digital controls are requested per player. Dependency/state
resolution and the saved-setup/GUI caller remain pending; no general MAME core
profile was enabled. Coverage stays 91/95 cores (95.8%), 289 enabled profiles.
Formatting/source inspection only; no builds, tests or native execution.

Step 167: corrected MAME inspection options to enforce the native writer's
fixed button-order contract before the fresh core starts. Required button-profile,
four-way, auto-save, config-write and external-path settings are overlaid on the
effective options while retaining unrelated settings. The low-level runner also
rejects missing, duplicate or mismatched required options, preventing callers
from bypassing this prerequisite. Physical calibration integration is still
pending. Coverage stays 91/95 cores (95.8%), 289 enabled profiles; full controller
mode coverage remains incomplete. Formatting/source inspection only, no tests.

Step 166: added the MAME frontend attachment adapter. It writes a private
configuration overlay, uses explicitly resolved persistent frontend save/state
directories, replaces the original content argument and transfers staged file
ownership to the calibrated session. Generated commands preserve the original
content basename for frontend save/state naming. Automatic overrides/remaps and
option re-selection are disabled for the snapshot. Physical mapping creation,
complete dependency/state resolution and the GUI workflow caller remain pending.
Coverage stays 91/95 cores (95.8%), 289 enabled profiles. No builds or tests run.

Step 165: MAME session commands now require explicit persistent NVRAM, disk
difference, state, snapshot and recording directories. Inspection commands keep
private routing. Session staging checks resolved directory identity and binds
declared NVRAM/difference baselines to those same persistent sources; inspection
staging now accepts declared difference files. No silent temporary-save fallback
is used for game commands. Complete state-manifest resolution, frontend path
routing and workflow invocation remain pending. Coverage stays 91/95 cores
(95.8%), 289 enabled profiles. Formatting only; no builds or tests.

Step 164: added MAME input ownership to the shared calibrated launch session.
The ownership-transfer method checks final runtime/core/command identity and
rejects conflicting native session owners. Existing pre-spawn input checks now
revalidate retained MAME sources and generated files while the same session
keeps physical transports alive. All other constructors explicitly leave MAME
absent. The final configuration/persistence adapter and workflow invocation
remain pending; no MAME support claim was enabled. Coverage remains 91/95 cores
(95.8%), 289 enabled profiles. Formatting/source inspection only, no tests.

Step 163: separated MAME game-session staging from the inspection tree. Only
declared source inputs and generated mapping/options/command files enter the
new owned session directory; inspection-created cfg, NVRAM and differencing
files are not promoted. Destination containment and uniqueness are rechecked
before copying. Source/copy hashes continue to bind staging to inspection.
Launch invocation, calibrated frontend ownership and persistent-state routing
remain pending. Coverage stays 91/95 cores (95.8%), 289 enabled profiles; full
controller-mode completion remains unproven. No builds or tests were run.

Step 162: retained MAME's owned inspection tree for session staging and added a
game-session command without the inspection autoboot/exit script. Staging
restores declared baseline inputs, hashes the copied dependencies, installs the
mapped controller/game configuration and checks source, frontend and staged-file
identity. Unhandled active fields prevent a complete digital staging claim.
The calibrated frontend/session owner and launch-workflow caller are still not
wired; persistence routing and inspection-created extra state need resolution.
Coverage remains 91/95 cores (95.8%), 289 enabled profiles. Full-mode completion
is unproven. Formatting/source inspection only; no builds or tests.

Step 161: added launch-bound MAME inspection preparation, checking the selected
native frontend/core/content identity, resolving effective core options and
passing runtime fields into digital mapping. Inspection now returns the exact
copied pre-run controller/game configuration bytes plus source hashes, so the
mapper does not reopen potentially changed user configuration. The result retains
the launch identity and baseline frontend configuration for final staging.
Dependency discovery, launch-workflow invocation and retained session staging
remain pending. Coverage stays 91/95 cores (95.8%), 289 enabled profiles; this
is not a full-mode completion percentage. Formatting/source inspection only;
no builds, tests or native execution.

Step 160: added a Rust native MAME process supervisor with owned temporary
staging, explicit dependency destinations, original cfg/controller inputs,
pre/post source SHA-256 checks, cancellation, deadlines, child cleanup and a
bounded field-result read. This is private staging, not an OS sandbox. The
caller still must resolve the full dependency graph, supply the exact effective
core options and wire the result into launch preparation. Native exit behavior
and the runtime configuration have not been executed or verified. Coverage
remains 91/95 cores (95.8%), 289 enabled profiles; all-mode completion is still
unproven. Formatting/source inspection only; no builds or tests.

Step 159: added MAME-specific inspection command construction with private
configuration/state paths and the generated native field snapshot adapter.
Source inspection ruled out reusing FBNeo's one-frame descriptor-refresh helper
for this runtime path. Process supervision, input staging and launch wiring
remain pending. Coverage stays 91/95 cores (95.8%), 289 enabled profiles. Only
source inspection and formatting were performed; no builds or tests.

Step 158: added preservation of the original MAME controller-profile layer.
Generated overrides are appended after existing sections in native load order,
retaining device maps, remap tables, unrelated bindings and comments. The field
planner accepts both original controller and game configurations; isolated
runtime/configuration provenance and launch staging are still pending. Coverage
remains 91/95 cores (95.8%), 289 enabled profiles. Formatting/source inspection
only; no builds, tests or native execution.

Step 157: made MAME field planning use exact runtime-observed native joystick
slots, accounting for controller-profile device-map swaps. Snapshot version 2
requires all eight exact RetroPad identities and unique native indices; disabled
joystick input cannot produce a digital plan. Preserved controller-layer staging
and the isolated runner remain pending. Coverage stays 91/95 cores (95.8%),
289 enabled profiles. Source inspection/formatting only; no builds or tests.

Step 156: connected discovered MAME field identities to machine-specific native
controller overrides and saved-config merging. The plan separates mapped,
disabled and unhandled fields, preserves exact tag/mask/default identities and
does not count unknown or analog peripherals as digital support. Isolated
inspection and launch staging remain pending. Coverage stays 91/95 cores
(95.8%), 289 enabled profiles. Source inspection/formatting only; no builds,
tests or native execution.

Step 155: added a Rust-owned MAME active-field snapshot adapter and bounded
response parser using the core's native Lua API. Mask-based lookup avoids
display-name collisions and retains exact tag/type/mask/default identities.
Conditional/overlapping field limitations are explicit; the isolated runtime
runner and launch integration remain pending. Coverage stays 91/95 cores
(95.8%), 289 enabled profiles. Formatting/source inspection only; no generated
adapter, emulator, build or test was executed.

Step 154: implemented MAME per-game digital-sequence merging with exact system
selection and byte-preserving edits outside controlled standard sequences.
Field identities, toggle settings, nonstandard sequences and unrelated settings
are retained; ambiguous/malformed configuration fails. Driver fields absent from
saved config and owned launch routing remain pending. Coverage stays 91/95 cores
(95.8%), 289 enabled profiles. Formatting/source inspection only; no tests,
builds or native execution.

Step 153: added a MAME-specific standard digital native sequence writer for up
to eight explicitly selected frontend ports, including numbered buttons, hats,
Start/Select and arcade start/coin types. Pinned source confirms MAME uses a
different button order from SAME CD-i; the writer records the exact source
channels and required fixed-profile options. Per-game overrides and launch
routing are not yet connected, so coverage stays 91/95 cores (95.8%), 289 enabled
profiles. Source inspection and formatting only; no builds or tests.

Step 152: fixed calibration of real evdev pressure axes reported by GilRs as
button events. The wizard now normalizes their initial binding to match the
measured release result, admits them for pressure/analog targets, and rejects
digital-only substitutes. Pressure prompts distinguish gradual full press from
stick motion; sharing one physical axis between independent pressure controls
is rejected during calibration. Coverage remains 91/95 cores (95.8%), 289 enabled
profiles. Source inspection only; no QML execution, builds or tests.

Step 151: added a source-specific LRPS2 catalog guard for its exact native
control/channel mapping, twelve pressure roles, four digital controls, bipolar
sticks, required per-port options, fresh-start policy and two-port limit. It
rejects USB/multitap overrides or incomplete/optional required controls instead
of letting catalog edits silently change the contract. Coverage stays 91/95
cores (95.8%), 289 enabled profiles; all-mode support remains incomplete.
Source inspection/formatting only, no builds or tests.

Step 150: added the explicit LRPS2/pcsx2 two-port DualShock 2 profile and its
twelve-pressure-button target layout. All pressure roles require independent
measured axes; Select/Start/L3/R3 remain digital and both sticks remain bipolar.
Per-port options set unit scale, no extra core deadzone/inversion, initial analog
mode and rumble disabled. Pressure-channel keyboard/mouse fallbacks are cleared.
Catalog coverage is now 91/95 cores (95.8%), 289 enabled profiles, 148 layouts.
Remaining cores without enabled catalog contracts: fbneo, mame, nymashock,
steemsse. This is source-grounded catalog implementation, not overall all-mode or
runtime completion. JSON metadata inspection and formatting only; no builds,
tests or live-device execution.

Step 149: extended pressure signal metadata beyond shoulders to measured analog
face and D-pad controls, with corresponding calibration, assignment and launch
normalization changes. This prepares LRPS2's twelve pressure-sensitive DualShock
2 controls without substituting full digital presses. Source audit is pinned in
LRPS2_CONTROLLER_CONTRACT.md; the core's catalog profile is not enabled yet.
Catalog coverage remains 90/95 cores (94.7%), 288 profiles; all-mode completion
is incomplete. Formatting/source inspection only; no builds or tests.

Step 148: added exact owned virtual-mouse endpoint resolution (inputN to evdev
child, verified again through the opened descriptor). Missing publication is
reported as not ready; ambiguous or substituted nodes fail without fallback.
Pinned RetroArch inspection confirms mouse selection is a numeric, hotplug-
rebuilt pointer list, not eventN or a stable physical identity. Routing remains
unwired until this frontend boundary is addressed. Catalog coverage remains
90/95 cores (94.7%), 288 profiles; all-mode completion remains unproven. Only
source inspection and formatting were performed, with no builds or tests.

Step 147 (relative transport infrastructure): added an owned uinput mouse and a
capture-to-output session pump. Selected deltas remain relative kernel counts;
packets are validated before writes, output identity comes from UI_GET_SYSNAME,
and failures/teardown release buttons and destroy the owned device. Mouse
classification capabilities remain neutral unless selected. No launch caller
is connected: per-port RetroArch selection and desktop pointer interaction
remain unresolved. Catalog coverage stays 90/95 cores (94.7%), 288 profiles;
this is not a new enabled mode. Formatting/source inspection only, no tests,
builds or runtime execution.

Step 146 (capture infrastructure, not a new enabled mapping): added a separate
Linux relative-mouse reader with exact opened-device identity checks, selected
REL_X/REL_Y and mouse-button capability validation, bounded nonblocking drains,
per-SYN_REPORT deltas, and terminal failure/neutralization on SYN_DROPPED or read
errors. Initial queued history is discarded before button-state snapshot. Shared
evdev open/bitmap helpers retain the gamepad reader's previous identity checks.
No launch caller or virtual mouse transport is connected yet. Catalog coverage
remains 90/95 cores (94.7%), 288 profiles; overall all-mode coverage is incomplete.
Formatting and source inspection only; no builds, tests or runtime execution.

Step 145 (source analysis, not a new mapping): confirmed Mouse Ball/Full Mouse
poll native relative mouse addresses and cannot use the Arcade Gun analog-alias
route. Recorded the separate delta-transport and physical mouse-selection
requirements. Catalog coverage remains 90/95 cores (94.7%), 288 profiles;
relative mouse support remains unimplemented. No builds, tests or execution.

Step 144: added FBNeo Arcade Gun absolute-axis bindings through its native
analog-channel route. Advertised ANALOG and queried subclass-1029 addresses
share assignments; conflicting duplicates are rejected, and axis-pair validation
also works when halves are assigned across aliases. The editor explains absolute
stick positioning versus physical lightgun/mouse input. Catalog coverage remains
90/95 cores (94.7%), 288 profiles; this is an added per-game native mode, not a
new catalog core. Other pointer/keyboard/touch adapters remain unfinished.
Formatting/source review only; no builds, tests or native execution.

Step 143: connected launch cancellation through FBNeo manifest staging. Copies
check cancellation between 64-KiB chunks and around hashing; cancellation before
plan replacement drops the private session and leaves the original plan intact.
Hash operations themselves remain synchronous. Catalog coverage stays 90/95
cores (94.7%), 288 profiles. External adapters remain unfinished. Formatting and
source review only; no builds, tests or runtime execution.

Step 142: added per-port device selection to the FBNeo request editor. Choices
come from the Rust source-defined topology contract, with exact device IDs;
context edits invalidate the chooser until refreshed. Selection changes only
the request and preserves other fields. Unapplied dependency edits prevent
device changes. The controller-port editor now respects FBNeo's six-port bound.
Catalog coverage stays 90/95 cores (94.7%), 288 profiles. This is not live
topology discovery or proof of external-adapter support. No builds/tests/UI
execution; formatting and source review only.

Step 141: added exact saved-controller selection to the FBNeo import context.
The chooser lists valid saved Linux calibration identities without opening
hardware, writes only the selected zero-based port in the context draft, and
rejects assigning the same controller to another port. Runtime identity checks
remain authoritative. Catalog coverage remains 90/95 cores (94.7%), 288
profiles. Topology discovery and external adapters remain unfinished. Formatting
and source review only; no builds, tests or UI execution.

Step 140: resolved missing/empty/default FBNeo save and state directories for
the native desktop Linux runtime contract to the already-resolved RetroArch
config root's saves/states directories. Original-content/core sorting still
applies. Invalid explicit directories produce a targeted error because RetroArch
would ignore them; final destinations must exist. Catalog coverage remains
90/95 cores (94.7%), 288 profiles. Non-desktop runtime variants and external
input adapters remain incomplete. Formatting/source review only; no tests/builds.

Step 139: FBNeo save/state routing now handles omitted sorting flags using the
pinned RetroArch defaults: per-core sorting on, per-content sorting and
content-directory saves off. Explicit values still take precedence. This removes
the unnecessary six-flag requirement from step 138; platform-specific directory
defaults remain unresolved rather than guessed. Catalog coverage remains 90/95
cores (94.7%), 288 profiles. Formatting/source review only; no builds or tests.

Step 138: connected the reviewed content manifest to FBNeo launch. Primary
content and explicit dependencies are copied into the same private directory,
checked against inspection hashes, and substituted only at the exact content
argument. Save/state paths are resolved against the original content with
content/core sorting preserved, then pinned and rechecked; unresolved defaults
or missing directories produce an error instead of temporary saves. The obsolete
original-directory restrictions on dependencies/system files were removed.
Catalog coverage remains 90/95 cores (94.7%), 288 profiles. External adapters
and broader default-path resolution remain incomplete. Formatting/source review
only; no builds, tests or runtime execution.

Step 137: FBNeo launches now stage the explicitly reviewed system files into a
private system directory, verify copied size/hash against inspection, and track
the copies in prelaunch integrity checks. RetroArch's content-directory system
fallback is disabled for this session, including when the reviewed system list
is empty. User source files/base config are unchanged. Content-side dependency
routing and external input adapters remain unfinished. Catalog coverage remains
90/95 cores (94.7%), 288 profiles. Formatting/source review only; no builds,
tests or runtime execution.

Step 136: isolated FBNeo gamepad mappings from inherited RetroArch frontend
hotkeys in the private launch configuration. Gamepad hotkey buttons/axes,
menu/quit combinations and turbo are disabled; keyboard shortcut assignments
and the user's base config are unchanged. Keyboard-adapter source inspection
also established a 24-channel/global-state limitation in RetroArch's keymapper,
so it is not being treated as full FBNeo keyboard coverage. Catalog coverage
stays 90/95 cores (94.7%), 288 profiles. Formatting/source review only; no builds,
tests or execution. Full controller-mode completion remains unproven.

Step 135: enabled FBNeo's legacy lightgun Pause button (native ID 5) through
RetroArch's gun_start binding. Pause and Start (ID 6) remain distinct report
addresses but share one physical gesture: either assignment satisfies both
observed targets, and conflicting explicit assignments are rejected, including
button-versus-axis suffix collisions. The assignment UI explains this alias.
Catalog coverage stays 90/95 cores (94.7%), 288 profiles; this adds a native
binding case, not another catalog core. Formatting/source review only; no builds,
tests or runtime execution. Coordinate and other external adapters remain open.

Step 134: added a dedicated FBNeo dependency-file editor for explicit content
dependencies and system files. Applying lists preserves other request fields;
stale request snapshots and duplicate/non-absolute paths are rejected. Unapplied
list edits block inspection, report import and draft acceptance until applied or
reset. This is manual manifest editing, not automatic dependency discovery.
Catalog coverage stays 90/95 cores (94.7%), 288 profiles; all-mode completion
remains unproven. Source inspection only, with no builds, tests or execution.

Step 133: connected explicit trusted-core inspection to the FBNeo editor, with
native-code confirmation, a supervised background worker, cancellation and
draft-only output. Invalid topology/controller selections are rejected before
starting the worker; cancelled results cannot be accepted. Dependency/topology
discovery and external input adapters remain unfinished. Catalog coverage stays
90/95 cores (94.7%), 288 profiles. No builds, tests or execution were performed.

Step 132: connected the existing-report import dialog with bounded request,
report and context inputs, background status, and explicit draft acceptance.
Changed import inputs or existing text edits prevent accidental replacement.
Duplicate import port keys are rejected. Direct guided inspection/dependency
resolution and remaining input adapters are unfinished. Catalog coverage stays
90/95 cores (94.7%), 288 profiles. No builds, tests, rendering or execution.

Step 131: added background import of existing inspection reports into unassigned
FBNeo setup drafts. The importer revalidates request/file provenance and explicit
topology, copies reviewed facts from the report, and leaves physical bindings
empty for user selection. It does not execute a core or stage settings. The
import dialog and remaining adapters are unfinished. Catalog coverage stays
90/95 cores (94.7%), 288 profiles. No builds, tests or execution were run.

Step 130: connected calibrated source selectors to the FBNeo editor. Targets
retain their exact native addresses and labels, full-axis halves are edited
separately, stale saved sources are identified, and unsupported adapters remain
visible. Changes update only the draft until validation/staging. Initial native
inspection/setup creation and external adapters remain unfinished. Catalog
coverage stays 90/95 cores (94.7%), 288 profiles. No tests or QML execution.

Step 129: added assignment-review data for the FBNeo editor, sharing target
construction with live inspection. Each target exposes required parts, existing
assignments, calibrated source choices and external-adapter requirements.
Incomplete drafts remain editable but are never labeled launch-ready. The guided
editor controls and remaining adapters are unfinished. Catalog coverage stays
90/95 cores (94.7%), 288 profiles. Formatting only; no tests or execution.

Step 128: connected an advanced FBNeo setup editor to the controller UI. It
lists exact per-game setups, edits reviewed JSON, stages validated changes,
confirms removal and retains un-staged text drafts across closing. It never
starts inspection. A guided inspection/assignment wizard and external input
adapters remain unfinished. Catalog coverage stays 90/95 cores (94.7%), 288
profiles. No build, QML execution, rendering or tests were run.

Step 127: added settings-model APIs to list, retrieve, stage and remove reviewed
FBNeo setups using exact compound identity keys. Edits validate a cloned mapping
before replacing staged state and use the existing Save settings workflow; no
inspection starts while editing. The QML editor and external input adapters
remain unfinished. Catalog coverage stays 90/95 cores (94.7%), 288 profiles.
Formatting only; no builds, tests or execution.

Step 126: connected explicitly saved FBNeo per-game setups to the main calibrated
launch flow. Settings bind emulator/core/content identity, reviewed input files,
options/descriptors/queries, physical controller IDs and exact assignments.
Launch re-inspects, rejects changed contracts, prepares retained players and
attaches the private session. The UI editor and external input adapters remain
unfinished; catalog-enabled coverage stays 90/95 cores (94.7%), 288 profiles.
Formatting only; no builds, tests or native/device execution.

Step 125: resolved the pressure-session remap gate from RetroArch's fresh-start
initialization path. Sessions now require a direct ELF runtime, retain its hash,
isolate remap storage, prevent automatic remap/override loading, preserve measured
analog range and reject conflicting CLI device overrides. Pressure no longer
has an unconditional attachment blocker. UI/settings and external input adapters
remain unfinished. Coverage stays 90/95 cores (94.7%), 288 enabled profiles.
Formatting only; no builds, tests or native execution.

Step 124: added FBNeo session ownership to `CalibratedLaunch`, including
controller health checks, input/config revalidation, private frozen options and
atomic launch-plan attachment. Complete fragments are required; pressure is
still gated on identity-remap initialization and external input adapters remain
unfinished. The UI/settings entry point is not connected. Coverage remains
90/95 cores (94.7%), 288 enabled profiles. Formatting only; no tests or execution.

Step 123: added owned Linux FBNeo player transport from explicit physical
calibration IDs. Measured bipolar/pressure axes feed the config writer through
a retained virtual gamepad, with device revalidation, cancellation and health
checks. Axis inversion is preserved; pressure's shared digital fallback is
reported as an alias. Final launch/UI/session integration and other input
adapters remain unfinished. Coverage stays 90/95 cores (94.7%), 288 enabled
profiles. Formatting only; no build, test or input-device/core execution.

Step 122: implemented explicit FBNeo-to-RetroArch binding translation and a
per-port config-fragment writer for pad buttons, bipolar axes, pressure and gun
buttons. It validates physical numbering/normalized axes, rejects channel
collisions, clears inherited bindings and reports missing/external targets.
Pressure also exposes its identity-remap requirement. Fragments are not yet
accepted launch sessions; coordinate/keyboard/touch adapters and the final UI/
launch caller remain unfinished. Coverage stays 90/95 cores (94.7%), 288 enabled
profiles. Formatting only; no builds, tests or native execution.

Step 121: added launch-plan-bound FBNeo inspection preparation. It verifies the
prepared native RetroArch invocation, selects the effective game/folder/core/
global option file, preserves hyphenated FBNeo keys, and passes exact core,
content, dependency and device identities to inspection. It returns the report,
mapping targets and option baseline together. Dependency/topology resolution
and the final launch/UI caller remain pending. Coverage stays 90/95 cores
(94.7%) and 288 enabled profiles. No builds, tests or core execution were run.

Step 120: added bounded input-query capture during the inspection frame and a
typed FBNeo mapping-target union of descriptors and observed addresses. This
preserves query-only inputs, shared labels, analog-pressure descriptor aliases,
relative/absolute coordinates and unknown encodings without flattening them
into buttons. One idle frame is not exhaustive behavior coverage. Launch/UI
integration remains pending; coverage stays 90/95 cores (94.7%) and 288 enabled
profiles. Only formatting was run; testing and native execution remain deferred.

Step 119: added the application-side FBNeo inspection runner. It passes the
selected runtime environment to a direct native worker, owns cancellation and
the deadline, reaps that worker before temporary-file cleanup, and validates the
bounded result against request provenance and driver topology. It is not yet
called by the launch flow; request construction and mapping-plan integration
remain unfinished. Coverage stays 90/95 cores (94.7%), 288 enabled profiles.
Only formatting was run; builds, tests and core execution remain deferred.

Step 118: added typed inspection-report validation against the request and
current core/content/dependency hashes, requested options/devices, bounded
labels, and before/after descriptor-refresh counters. The CLI validates before
emitting a report; the FBNeo application parser additionally checks the complete
driver-derived advertised topology. Invocation and mapping-plan integration
remain pending. Coverage stays 90/95 cores (94.7%), 288 enabled profiles; only
formatting was run, with no builds, tests or native-core execution.

Step 117: added `lunchbox-libretro-content`, a supervised process entry point
for real-content inspection. Its versioned request validates core identity,
absolute paths, unique option overrides and port selections. The supervisor
enforces a deadline, reaps the worker before cleaning private files, and keeps
native stdout separate from the bounded JSON result. Application-side report
validation and launch wiring remain pending. Coverage is unchanged at 90/95
cores (94.7%) and 288 enabled profiles; no tests, builds or cores were run.

Step 116: wired inspection-only option state into the callback lifecycle and
added a real-content descriptor-inspection library entry point. It stages
private content/dependency/system copies and saves, selects advertised devices,
requires a descriptor refresh during one idle frame, and reports input-file
hashes, options and descriptors. CLI process supervision and application launch
integration are still pending; this path has not been executed. Coverage stays
90/95 cores (94.7%), with 288 enabled profiles, not full all-mode coverage.

Step 115: added the isolated legacy core-option callback adapter needed for
FBNeo inspection, with bounded native-string capture, stable lookup pointers,
version-zero negotiation and persistent callback-error reporting. Inspection
and launch wiring remain pending. Coverage remains 90/95 cores (94.7%) with at
least one enabled contract, 288 enabled profiles; this is not all-mode coverage.
Five cores remain without accepted launch contracts. Testing remains deferred.

SimCoupe source tracing found that its libretro joystick receiver discards both
fire-button bits, its reset wrapper is empty, and its right-mouse wrapper is
empty. No profile is enabled for those nonfunctional routes. Keyboard-backed
input remains incomplete. The opt-in maintained core now has source patches for
fire forwarding, mode-release handling and requested disk insertion, with three
guarded joystick profiles; see
[SIMCOUPE_CONTROLLER_CONTRACT.md](SIMCOUPE_CONTROLLER_CONTRACT.md).

Nymashock now has native launch code for digital pads, DualShock, Dual Analog,
Analog Joystick, neGcon, Dance Pad, Pop'n Music and controller-driven mouse,
GunCon and Justifier. These routes include calibrated SDL, raw classic/evdev and
owned normalized sessions. Optional desktop-cursor gun aim keeps buttons on the
selected controller and is limited to one player. Relative mouse motion has a
separate gain setting. These are implementation claims, not accepted coverage:
physical mouse deltas, device-specific lightgun capture, HID/ambiguous mappings,
normalized rumble and runtime verification remain incomplete.

A fresh read-only inventory including Hatari's four ST profiles, SimCoupe's
disabled virtual-keyboard preview and patched joystick modes, Steem SSE's layout,
ScummVM's guarded RetroPad profile, Dolphin's mixed-trigger layout and SAME
CD-i's two explicit controller-driven pointer modes gives
**147 layouts**, **293 output profiles**,
**288 launch-enabled profiles** across **90 canonical cores**. The database still
contains the same **95 canonical core entries**, leaving **5** uncovered
(**94.7% some-mode coverage**, not full controller-mode completion). Hatari's
keyboard, mouse, other machines and media modes remain incomplete. No tests, application
builds or runtime probes have been run.

Dolphin's analog trigger controls now use the existing pressure-shoulder
classification, activating measured-pressure calibration requirements and the
normalized-axis transport path. Digital fallback controls remain separate.
Dolphin now has an enabled, explicit 24-control raw-GameCube profile with paired
pressure/fallback outputs, soft-press stick clicks, up to four ports and retained
content/native-configuration snapshots. Wii, Triforce, compressed media,
microphone and rumble remain incomplete; runtime validation is deferred.

SAME CD-i now has a source-derived relative-pointer layout. Its third button
encodes the buttons-1+2 chord; the stock mouse receiver does not read the middle
button. Native sequence writers and two enabled, explicit controller-driven
profiles now cover stick velocity and D-pad increments, including the third
button action. Owned configuration/BIOS routing, retained disc snapshots and
original-disc save/state paths are connected to launch. Physical mouse capture,
additional media/config variants and runtime validation remain incomplete.

Hatari now has separate jump-shortcut and Space-shortcut joystick layouts.
Source tracing confirms that the descriptor's third fire button is actually
held autofire, and that frontend players map to swapped ST ports. Four explicit
fresh-start profiles cover one/two players and jump/Space shortcuts. Launch
requires read-only firmware/config snapshots and disabled extra ports; see
[the Hatari contract](HATARI_CONTROLLER_CONTRACT.md).

Native players now have an explicit session-normalized input option. Preparation
validates the physical device, creates and retains owned virtual gamepads, uses
their rewritten calibration and actual SDL2 mappings, and checks bridge health
for the session. Failure paths drop the owned bridges. Force-feedback forwarding
is not implemented, so normalized players cannot request rumble. Ambiguous SDL2
outputs remain explicit failures. No tests, builds or runtime probes run;
some-mode coverage remains **85/95 (89.5%)**, with **10** missing cores.

Native BizHawk launch preparation now consumes saved SDL2 logical gestures for
digital pads and directly representable DualShock sticks, stick clicks, explicit
analog toggle and rumble configuration. The UI captures released/pressed pairs
asynchronously, records selected controls with runtime/mapping provenance, and
supports clearing stale records. Physical recalibration invalidates affected
logical records. Full-stick mappings requiring recentering, asymmetric half-axis
scaling or composite SDL mappings still require normalized transport; HID
identity and broader Nymashock integration remain unfinished. Coverage stays
**85/95 (89.5%)**, with **10** missing cores. No tests or builds run.

Native BizHawk preparation now resolves selected Linux controller nodes against
the target SDL2 inventory by kernel input identity, then probes and rechecks the
resolved paths consistently. Classic joydev and evdev translations retain their
own control ordering and axis conversion. Ambiguous matches and unsupported HID
identity paths are rejected. This adapter is still in implementation: it does not
yet promote Nymashock to covered status. Some-mode coverage remains **85/95
(89.5%)**, with **10** missing cores; full-mode coverage is incomplete. No tests
or builds run.

Nymashock's database relationship is owned by BizHawk, not RetroArch. The
coverage UI now labels it accordingly and omits inapplicable RetroArch mode
choices. It remains uncovered and in the original denominator pending a BizHawk
adapter. Coverage remains **85/95 (89.5%)**, with **10** missing cores.
No tests or builds run; full-mode coverage remains incomplete.

Panda3DS adds standard buttons and native Circle Pad mapping. Touch remains a
separate frontend-pointer requirement; the unimplemented C-stick and
configuration-dependent Circle Pad Pro controls are explicitly excluded. Current
some-mode coverage is **85/95 cores (89.5%)**, with **10** missing cores. Full
controller-only coverage is not claimed. No tests or builds run.

Mu adds m500 and m515 buttons-plus-stick-stylus profiles. The left stick moves
the touchscreen cursor, R supplies contact, and the full Graffiti area remains
visible. Model-specific up/down, application and power buttons are mapped;
conditional T3 five-way controls are not assumed. Current some-mode coverage is
**84/95 cores (88.4%)**, with **11** missing cores. No tests or builds run.

Mini vMac adds digital and left-stick mouse profiles with GUI controls. The
contract identifies the required Select startup toggle, separate guest/GUI
mouse-source state and A-only guest click delivery. README-only shortcuts are
not advertised. Current some-mode coverage is **83/95 cores (87.4%)**, with
**12** missing cores. No tests or builds run; full-mode coverage is incomplete.

b2 adds QAOP and AZOP keyboard presets for all nine models (18 profiles).
Each fixes all key options, exposes 15 distinct keys and leaves A unbound to
avoid the core's parallel joystick-fire action. b2 now has 27 profiles.
Core coverage remains **82/95 (86.3%)**, with **13** missing cores. Additional
and mixed-input modes remain incomplete; no tests or builds run.

b2 analog joystick coverage now spans all nine actively selectable machine
models, including B+, B+128 and Master 128 MOS 3.20. All use the same two-port
ADC contract with keyboard assignments cleared. Commented-out model definitions
are excluded. Core coverage remains **82/95 (86.3%)**, with **13** missing cores.
Keyboard and mixed-input profiles remain work; no tests or builds run.

b2 adds a BBC B/Acorn 1770 two-analog-joystick profile. Native left sticks
feed ADC channel pairs and A supplies fire; all 24 keyboard assignments are
cleared privately. The core's change-only axis initialization caveat is shown.
Current some-mode coverage is **82/95 cores (86.3%)**, with **13** missing cores.
Keyboard presets and other b2 models remain work. No tests or builds run.

ep128emu adds guarded ZX TAP direct-file and TVC MOPS CRT cartridge routes.
ZX uses the core's exact TAP-header heuristic; other TAP files are not assumed
to be Spectrum content. Existing controller topology and configuration checks
are retained. Coverage remains **81/95 cores (85.3%)**, with **14** missing
cores. No tests or builds run; full-mode coverage remains incomplete.

ep128emu adds TVC64 TVCWAV tape coverage, including the GameCard external
3/4 input paths found in the VM source. Five usable default mappings are
exposed; the unused sixth slot is excluded, and shared internal/external-1
directions are documented. ep128emu now has four launch profiles. Core coverage
remains **81/95 (85.3%)**, with **14** missing cores. No tests or builds run.

ep128emu adds ZX128 TZX default-adapter coverage: Kempston, Sinclair 1,
Sinclair 2 and Protek, with per-port labels for their keyboard aliases. The
contract explicitly warns that these are not four independent player inputs
and that overlapping keys can interfere. Core coverage remains **81/95
(85.3%)**, with **14** missing cores; ep128emu now has three launch profiles.
No tests or builds run; full-mode coverage remains incomplete.

Corrected ep128emu Enterprise's tape entrypoint to advertised native TAP content:
EPT has a detection branch but is absent from `valid_extensions`. The guard now
requires the native eight-byte tape signature. Extra joystick fire-line aliases
are documented from the DAVE hardware implementation. Coverage remains
**81/95 cores (85.3%)**, with **14** missing cores. No tests or builds run.

ep128emu adds an Enterprise 128 EPT tape profile with the default internal plus
five external joystick mappings. Player-one shortcuts are separate from the
five external-port layouts. Machine-specific system configuration and shared
sidecar/marker checks remain mandatory. Core coverage stays **81/95 (85.3%)**,
with **14** missing cores; ep128emu now has two launch profiles. No tests or
builds run, and full-mode coverage remains incomplete.

ep128emu's CPC second-port layout now displays its keyboard-matrix aliases:
6/5/R/T for directions and G/F for fire. Guest conversion confirms the
player-one shortcut labels and combines distinct active source keys sharing a
matrix contact. Coverage remains **81/95 cores (85.3%)**, with **14** missing
cores; no tests or builds run.

ep128emu CPC coverage now includes CDT cassette media as well as DSK disks.
The guard requires the TZX v1 signature together with the CDT extension, matching
the core's machine-selection branch. Controls and configuration checks are shared
with the disk route. Some-mode coverage remains **81/95 cores (85.3%)**, with
**14** missing cores. Full-mode coverage is incomplete; no tests or builds run.

ep128emu adds a native CPC default two-joystick disk contract. Header and
configuration-absence checks are repeated before launch; the resolved system
directory and shortcut/autofire options are pinned privately. Player two omits
player-one keyboard/system shortcuts. Current some-mode coverage is **81/95
cores (85.3%)**, with **14** missing cores. ep128emu custom configuration, other
machines and Flatpak support remain work. No tests or builds run.

ep128emu now has four source-derived target layouts: one-/two-fire joysticks,
each with and without player-one shortcuts. Primary fire is RetroPad X; CPC
secondary fire is A. These layouts are not yet launch-enabled, so coverage remains
**80/95 cores (84.2%)**, with **15** missing cores. No tests or builds run.

ep128emu source tracing found content-directory machine configuration files that
override header detection by existence, in addition to system and per-game
configuration. Device callbacks also rebuild all ports. The prerequisite contract
is recorded in `EP128EMU_CONTROLLER_CONTRACT.md`; no profile is enabled from these
findings alone. Some-mode coverage remains **80/95 (84.2%)**, with **15** missing
cores. Full-mode coverage is incomplete; no tests or builds run.

Neko Project II Kai adds digital, left-stick and right-stick mouse profiles.
Stick click and L2-shifted click are mapped explicitly; the menu shortcut moves
to the other stick button to avoid collisions. Mouse modes retain physical
keyboard polling, unlike the keyboard presets. NP2kai now has eight profiles.
Core coverage remains **80/95 (84.2%)**, with **15** cores lacking contracts.
No tests or builds run; full-mode coverage remains incomplete.

Neko Project II Kai adds five fixed keyboard presets: arrows/keypad in normal
and three-button forms, plus keypad fighting. L3 opens the core menu without
colliding with the twelve mapped keys. These modes replace physical-keyboard
polling, a limitation now shown in each contract; physical mouse input remains
available for menu interaction. Current some-mode coverage is **80/95 cores
(84.2%)**, with **15** cores lacking contracts. No tests or builds run.

PX68k adds analog Cyberstick alone and paired with each of its three supported
second-port adapters. The target uses native left X/Y and reversed right Y for
the three eight-bit axes, eight hardware buttons, and player-one system
shortcuts. D-pad is labeled for menu navigation, not analog motion. PX68k now
has 17 profiles. Core coverage remains **79/95 (83.2%)**, with **16** cores
lacking contracts. No tests or builds run; full-mode coverage is incomplete.

PX68k adds nine mixed topologies: six pairings among standard/CPSF-MD/CPSF-SFC
and three digital-Cyberstick-first pairings. Complete per-port binding replacements
now accompany per-port layouts, so the second port can have more or different
controls without inheriting player-one shortcuts. Assignment and coverage paths
honor these heterogeneous ports. PX68k has 13 profiles; core coverage remains
**79/95 (83.2%)**, with **16** cores lacking contracts. No tests or builds run.

PX68k adds standard two-button, CPSF-MD, CPSF-SFC and digital Cyberstick
profiles. The first three support both ports; Cyberstick is first-port-only in
the core options. Player one owns the F12 menu and registration-key shortcuts.
Private options establish the pad types after legacy configuration loading.
Current some-mode coverage is **79/95 cores (83.2%)**, with **16** cores lacking
contracts. Analog Cyberstick and mixed adapters remain work. No tests or builds run.

QUASI88 adds four BASIC-mode profiles and distinct per-port keyboard layouts:
the first pad sends keypad directions and Z/X, while the second sends R/F/D/G
and Q/Tab. The shared catalog now supports explicit per-port target layouts in
both assignment and coverage paths. A D88 guard rejects paths that the core
would misroute to its playlist loader. Current some-mode coverage is **78/95
cores (82.1%)**, with **17** cores lacking contracts. No tests or builds run.

PUAE adds D-pad and native-left-stick mouse modes for ten computer models and
CDTV: 22 additional profiles, bringing PUAE to 34. Three mouse buttons and speed
modifiers are mapped; keyboard shortcuts belong to player one. The second pad
drives the second mouse port when guest software supports it. Analog mouse mode
fixes a 15% deadzone and 1.0 speed, while digital mode requires no native stick.
Core coverage remains **77/95 (81.1%)**, with **18** cores lacking contracts.
No tests or builds were run; full-mode coverage remains incomplete.

PUAE adds two CD32 profiles (standard and Fast RAM), with all seven hardware
buttons on two pads and player-one-only virtual-keyboard access. Explicit device
517 selects the CD32 serial pad; Play/Pause no longer inherits the computer
profile's Return shortcut. PUAE now has twelve profiles. Core coverage remains
**77/95 (81.1%)**, with **18** cores lacking contracts. No tests or builds run.

PUAE adds ten explicit Amiga computer-model profiles with two two-button
joysticks and player-one keyboard controls. Device 257 avoids automatic
CD32/Arcadia controller substitution; fixed machine options avoid Automatic
filename-tag selection. The native save-directory snapshot guard protects the
model/global/content UAE configuration boundary. These direct-floppy profiles
disable physical mouse input, turbo and mapper substitutions. Other PUAE modes
remain work. Current some-mode coverage is **77/95 cores (81.1%)**, with **18**
cores lacking contracts. No tests or builds were run.

DOSBox Pure adds an explicit two-player topology: device 1281 on frontend port
one drives DOS joystick one, while device 1537 on frontend port two drives DOS
joystick two. Player two receives only directions, two buttons and the native
left stick; shared keyboard/menu controls belong to player one. There are now
**17 DOSBox Pure profiles**. Core coverage remains **76/95 (80.0%)**, with
**19** cores lacking contracts. No tests or builds were run.

DOSBox Pure's eight fixed presets now each have menu-enabled and no-menu
variants: **16 explicit single-controller modes**. The seven new no-menu layouts
derive their fallback keys without reserving L3, so they do not reuse misleading
menu-enabled labels. Current some-mode core coverage remains **76/95 (80.0%)**,
with **19** cores lacking contracts. Multiplayer and mutable per-game mappings
remain work. No tests or builds were run.

DOSBox Pure now has nine explicit modes. Seven additional profiles cover mouse
on either stick, Gravis gamepad, either two-button DOS joystick, Thrustmaster
flight stick and both DOS joysticks on one pad. Each exposes the complete
collision-shifted keyboard fallback rather than assuming generic button labels.
Core coverage remains **76/95 (80.0%)**; multiplayer, Windows and other
entrypoints remain work. No tests or builds were run.

DOSBox Pure adds two explicit generic-keyboard profiles: direct F1/F2 and
menu/keyboard overlay. Both fix the keyboard preset with device 257, bypassing
saved and detected mappings, and account for all eight frontend ports with one
active controller. Their two-stick layouts expose the actual keyboard pairs;
the overlay variant reserves L3 and shifts F1 to R3, dropping direct F2.
At that checkpoint mouse, joystick, multiplayer, Windows and additional entrypoint modes remained.
Current some-mode coverage is **76/95 cores (80.0%)**, with **19** cores lacking
launch contracts. No tests or builds were run; this is not full-mode coverage.

Atari800 adds seven explicit computer-machine profiles (400/800, 800XL, 130XE,
three expanded XL/XE memory configurations and XEGS) and two single-player 5200
input modes. Computer profiles expose four joystick ports on 400/800 or two on
XL/XE, with console/virtual-keyboard controls owned by player one. Their media
guard checks signatures and extension case before admitting direct disk, tape
or executable files. Private options disable legacy configs, controller hacks,
paddles, XEP80 port occupation and autofire. 5200 digital and native-analog modes
are explicit alternatives, with all keypad digits accessible through the overlay.
The actual L3 mapping is keypad 7; shared keypad/second-fire state is not claimed
as independent multiplayer input. Cartridge-container variants and additional
peripherals remain work. At that checkpoint some-mode coverage was **75/95 cores (78.9%)**,
with **20** cores lacking launch contracts. No tests or builds were run.

EightyOne adds explicit cursor-key and QAOP ZX81 layouts with all thirteen
direct key mappings fixed outside the core's auto/game-database path. Select
operates its virtual keyboard, and frontend port two is reserved for a physical
keyboard rather than a second joystick sharing overlay state. BK adds five
explicit machine-model joystick profiles, one four-button joystick layout, and
a full-path BASIC-before-FOCAL model-override guard. Its two frontend pads feed
one ORed joystick register; the second pad is therefore cleared. BK's keyboard
path remains physical-keyboard input, not a controller-operated overlay.
At that checkpoint some-mode implementation coverage was **74/95 cores (77.9%)**, with **21**
cores lacking launch contracts. No tests or builds were run. Other keyboard,
peripheral, content and host modes remain work within already-covered cores.

CrocoDS adds a native-Linux, one-player CPC joystick/virtual-keyboard contract
and a read-only input-state resolver. It composes the initialized global INI and
the full-basename per-content INI, including global shoulder-menu defaults, and
rechecks those file snapshots before launch. Custom conflicting mappings and
unsupported INI syntax are surfaced without rewriting settings. The actual core
polls one frontend pad; its unused two-port table is not claimed as multiplayer.
Flatpak persisted-HOME resolution, first-run initialization and non-DSK modes
remain outstanding. At that checkpoint some-mode coverage was **72/95 cores (75.8%)**, with
**23** cores lacking launch contracts. No tests or builds were run.

Caprice32 adds four explicit CPC modes: the default joystick plus keyboard
shortcuts, QAOP, Incentive cursor-key controls, and two independent joystick
ports. Each has a labeled target layout and fixed private core options disabling
the internal remap database. Select remains a held command modifier: Select+Start
toggles the virtual keyboard, D-pad moves its cursor and A clicks. The two-player
mode limits player two to directions and two fire buttons, leaving keyboard and
combo ownership with player one. Snapshot/archive/playlist/GX4000/lightgun modes
remain outside these direct CPC disk/tape contracts. Implementation coverage is
**71/95 cores (74.7%)** at that checkpoint, with **24** cores still lacking any launch contract.
No tests or builds were run; this is not runtime-verified or full-mode coverage.

Virtual Jaguar now has an explicit two-player standard-pad contract covering
directions, A/B/C, Pause, Option and all twelve keypad keys. Two native analog
sticks carry the extra keypad inputs; the core uses signed thresholds of 20000.
Opposite directions on one axis cannot be held together, so arbitrary simultaneous
keypad chords are not claimed. Private options set every input mapping on both
ports, disable per-title substitutions and keyboard keypad replacement, and force
standard pads. Team Tap, Pro presets and other peripherals remain separate work.
At the Jaguar checkpoint some-mode implementation coverage was **70/95 cores (73.7%)**, with **25**
cores lacking contracts. No tests or builds were run.

The VICE batch adds ten explicit launch profiles across six previously uncovered
cores: C64, C64SC, C128 and Plus/4 each have port-1/port-2 joystick plus virtual
keyboard modes; VIC-20 has its single built-in joystick plus keyboard; PET has a
controller-operated keyboard without claiming a built-in joystick. Two new
layouts describe the joystick/keyboard and keyboard-only workflows. A launch
guard mirrors VICE's full-path, case-insensitive j1-before-j2 filename override
and rejects a conflict with the selected startup port. Private options remove
saved mapper overrides, disable vicerc/userport adapters, and retain native
physical keyboard pass-through. Command-bearing playlists, command files,
archives and snapshots remain outside these direct-content contracts.
At the VICE checkpoint implementation coverage was **69/95 cores (72.6%)**, with **26** lacking
launch contracts. No tests or builds were run. This is some-mode coverage, not
full controller/peripheral completion for those cores.

The latest core-contract batch adds SkyEmu GB/GBC and GBA controls, plus two-pad
and dual-multitap digital Saturn modes for Kronos and YabaSanshiro. All six modes
are explicit fresh-start contracts with source-pinned bindings, content extension
constraints and private core-option snapshots. Kronos C/Z map to RetroPad R/L;
YabaSanshiro C/Z map to L/R. The distinction follows each core's input loop.
Kronos ZIP/ST-V inputs are excluded from Saturn modes. SkyEmu's fixed GB/GBA
hardware options prevent a saved core override from selecting DS instead.
This adds three previously uncovered cores: **63/95 (66.3%)**, with **32** cores
still without any launch contract. DS touch, Saturn analog peripherals, ST-V,
and single-multitap behavior remain mode-level work, not implied coverage.
No tests or builds were run. Stella Flatpak infrastructure is paused while the
remaining RetroArch core contracts take priority.

Analog-pressure transport infrastructure now includes a whole-gamepad frame
assembler: explicitly selected gamepad buttons, raw stick/hat axes with declared
neutral positions, and measured normalized pressure axes share packet boundaries.
Complete snapshots are required before input is accepted; invalid samples clear
pending state, and loss/shutdown produces neutral values for all selected controls.
Keyboard, mouse, digitizer and multitouch controls are excluded. A read-only
whole-gamepad evdev reader now consumes this assembler, checks selected button
capabilities, validates axis bounds, snapshots button/axis state, and drains queued
history before drop recovery. Polling and signal retries are bounded, and failures
invalidate the stream until a new session is created. A session-local uinput
publisher now declares only those gamepad controls, places normalized pressure on
the positive half of a centered virtual axis, validates complete output packets,
and emits one SYN_REPORT per frame. It obtains its own inputN identity from
UI_GET_SYSNAME, neutralizes/destroys its device on failure or shutdown, and reports
permission errors without changing permissions. No live virtual device has been
created. A bridge owner now runs reader/publisher forwarding on an owned worker,
requires an initial synchronized frame before reporting startup success, exposes
worker failure, and wakes/joins on shutdown. Its joydev lookup checks only children
of the owned inputN identity and verifies the character-device major/minor against
sysfs. Both calibrated launch preparation paths now retain per-player transports,
validate physical input before starting a pressure bridge, and select its actual
joydev numbering. Pressure bindings use positive normalized axes; digital-only
profiles keep direct device input. Required mirrored axes need recorded bounds
and neutral positions, and conflicting axis roles/measurements are rejected.
The emulator boundary checks bridge health before/during startup and periodically
during play; a later failure reports a warning without killing the running game.
Snapshot/event concurrency and device lifetime remain part of deferred checks.
The physical reader additionally opens only event-number nodes without following
symlinks, checks the descriptor's character-device identity through /sys/dev/char,
and compares the opened node's inode/device numbers with the current path. This
binds forwarding to the selected joystick's input identity rather than relying
on a previously discovered path string. Hotplug race behavior is still untested.
No tests or builds were run. Core coverage is now 69/95 (72.6%), with 26 core
entries and all 171 standalone automatic adapters still outstanding. The pressure
Planning now promotes a matching shoulder role to pressure only for a requested
analog target and a recorded proportional evdev gesture. Digital-only planning
and the static layout catalog remain unchanged. Capture preserves measured axis
capability even when GilRs emits button events for that trigger. The pressure
batch now includes a source-grounded Flycast standard Dreamcast analog-controller
launch contract: four ports, explicit selection, classified disc extensions,
fresh-start guard and private Dreamcast/analog-trigger options. Other Flycast
device and arcade modes remain outstanding; rumble forwarding is not implemented.
Flatpak visibility, device identity
races, and normalization fidelity have not been exercised during this phase.

Scope clarification: physical keyboards remain native/pass-through. Full keyboard
remapping and arbitrary text entry are not completion requirements for this
controller-mapping goal. Controller buttons may still target emulated keys or
controller-operated virtual keyboards where a game's controller workflow needs it.

This inventory is implementation coverage, not runtime verification. The UI now
has a **Controller coverage — implemented / remaining** button in controller
settings. It reads the active discovery database, splits core lists, applies the
same explicit core-identity aliases as launch discovery, and lists every
core/platform and non-core emulator/platform relationship. Missing support stays
visible. Database errors are reported rather than treated as an empty inventory.

The new UI/report source has been implemented; build, UI rendering and runtime
verification are deliberately deferred until the requested implementation phase
is complete. This is not yet an activated application change.

The report also derives a distinct core/platform-pair count using the same
platform-profile matching as launch selection. The UI displays matching and
missing pairs plus their percentage separately from core-entry coverage. A CD
contract cannot inflate AES/MVS relationship coverage. These live totals have
not been executed or rendered during the no-testing implementation phase.

| Layer | Current | Remaining |
| --- | --- | --- |
| Physical layouts | 115 semantic layouts, saved calibration | Additional control families and hardware-specific capture needs |
| Layout assignment | One shared capability-constrained solver | Capability transforms for additional input types; no hand-written pair matrix |
| Output contracts | 245 profiles, 241 launch-enabled | Remaining core/mode facts and transports |
| RetroArch catalog cores | 80 of 95 canonical cores have some launch contracts (84.2%): 33 with automatic modes, 47 requiring explicit selection | 15 cores have none (15.8%); additional modes and host adapters within the 80 also remain |
| Non-core emulator definitions | 171 in the current database; no automatic adapters | Standalone integration; DuckStation has two mapping-only previews |
| UI | Calibration, preview, new searchable coverage and capability report source | Build/render confirmation, per-game launch-plan readiness visibility |

Database snapshot: 249 linked emulator definitions, 200 linked platforms, 511
relationship rows. The core count is 95 after canonical alias deduplication, not
the historical 98 raw names. Counts are distinct at their named layer; a single
emulator can have both core and non-core relationships. They are not installed
emulator counts, and a core with a contract is not fully covered.

## Cores with some launch contracts (80)

- `np2kai` (five fixed keyboard presets and three mouse profiles with core-menu access)

- `px68k` (17 fixed/mixed two-button, CPSF-MD/SFC and digital/analog Cyberstick profiles)

- `quasi88` (four BASIC modes with distinct primary/secondary keyboard layouts)

- `puae` (34 computer/CD32/CDTV joystick, gamepad and mouse profiles; native save-directory guard)

- `dosbox_pure` (17 explicit keyboard, mouse, gamepad and joystick modes)

- `atari800` (seven computer configurations and explicit digital/analog 5200 modes)

- `81` (explicit cursor-key and QAOP layouts with virtual/physical keyboard paths)
- `bk` (five explicit BK machine models with one shared joystick)

- `crocods` (native Linux initialized joystick/keyboard mode; Flatpak pending)

- `cap32` (four explicit CPC joystick/keyboard modes)

- `virtual_jaguar` (explicit standard pads with full keypad via native sticks)

- `blastem` (explicit mode selection)
- `bluemsx` (explicit MSX/MSX2/MSX2+ two-joystick modes)
- `ardens`
- `bsnes`
- `citra` (explicit Old/New 3DS gamepad-only modes)
- `desmume` (explicit relative/absolute stick-stylus modes)
- `melonds_ds` (explicit DS/DSi joystick-touch modes)
- `bsnes_hd_beta` (explicit standard-pad mode)
- `emuscv` (explicit two-controller / shared console keypad mode)
- `fake08`
- `fceumm`
- `fmsx` (explicit joysticks and joystick/keyboard-shortcut modes)
- `flycast` (explicit Dreamcast analog controllers, arcade sticks, digital/native-analog twin sticks and Saturn twin-stick wiring)
- `freechaf` (explicit eight-action hand controllers and console overlay)
- `freeintv` (explicit analog disc and controller-operated keypad)
- `freej2me` (explicit Standard/Nokia/Siemens/Motorola keypad/navigation and stick-pointer variants)
- `fuse` (explicit Cursor/Kempston/Sinclair/Fuller joystick modes)
- `gambatte`
- `geolith` (explicit AES/MVS and CD/CDZ standard-pad modes)
- `gw` (explicit script-facing digital-pad mode)
- `gearlynx`
- `gearcoleco` (explicit standard controllers with full keypad)
- `gearsystem` (explicit Game Gear 2 ASIC mode only)
- `genesis_plus_gx`
- `jollycv` (explicit COL-content shifted keypad / Super Action mode)
- `kronos` (explicit Saturn digital two-pad / dual-multitap modes)
- `mednafen_lynx`
- `mednafen_ngp`
- `mednafen_pce_fast`
- `mednafen_pcfx`
- `mednafen_psx`
- `mednafen_psx_hw`
- `mednafen_saturn` (explicit digital-pad topology; fresh start required)
- `mednafen_supergrafx` (two-button automatic; six-button explicit topology)
- `mednafen_vb`
- `mednafen_wswan`
- `mesen` (explicit mode selection)
- `mesen-s` (explicit standard-pad mode)
- `mgba`
- `mupen64plus_next`
- `neocd`
- `nestopia` (explicit mode selection)
- `opera`
- `o2em` (explicit Odyssey 2 / G7000 / G7400 / Jopac controller modes)
- `picodrive` (explicit mode selection)
- `pcsx_rearmed` (explicit digital/DualShock two-port and multitap topologies)
- `pokemini`
- `potator`
- `ppsspp`
- `prosystem`
- `retro8`
- `redream` (explicit historical libretro digital-trigger controller mode)
- `sameboy`
- `skyemu` (explicit GB/GBC and GBA modes)
- `snes9x`
- `stella` (explicit runtime-detected digital controllers; native Linux RetroArch only)
- `swanstation`
- `uzem`
- `vbam` (GBA automatic; DMG/GBC explicit hardware modes)
- `vecx`
- `vemulator`
- `vice_x128` (explicit port-1/port-2 joystick and virtual keyboard)
- `vice_x64` (explicit port-1/port-2 joystick and virtual keyboard)
- `vice_x64sc` (explicit port-1/port-2 joystick and virtual keyboard)
- `vice_xpet` (controller-operated virtual keyboard only)
- `vice_xplus4` (explicit port-1/port-2 joystick and virtual keyboard)
- `vice_xvic` (single built-in joystick and virtual keyboard)
- `wasm4`
- `yabasanshiro` (explicit Saturn digital two-pad / dual-multitap modes)

Contracts remain limited to their explicitly declared platform, device, option
and player modes. Standard joypads do not imply lightgun/mouse/keypad/multitap,
link or other peripheral coverage. Four catalog profiles remain preview-only:
Genesis Plus GX three-button, Mupen64Plus-Next default C-button mode, and
DuckStation digital/analog.
They are separate from the 139 enabled profiles.

Flycast's Dreamcast arcade-stick mode now has a separate four-port contract and
six-button target, without a fabricated Genesis Mode control. Device 1025 selects
MDT_AsciiStick. The runtime mapping writes C from R/R2 and Z from L/L2, opposite
the nearby descriptor labels; the contract follows the runtime code and uses R/L.
This mode zeros unused analog registers in the core and uses direct calibrated
input without a pressure bridge. Dreamcast system selection and fresh startup
are explicit. Naomi/Atomiswave modes remain unfinished.

Flycast now also has explicit normal Twin Stick digital-input and Saturn-wiring
contracts (devices 513 and 769). They share a fourteen-control digital target:
two directional clusters, triggers/turbo per hand, Start and Special. The Saturn
branch uses different RetroPad outputs and is mapped separately. The core exposes
no proportional Maple axes for this device. A separate normal-mode native-analog
contract now requires both physical sticks and passes left/right X/Y to Flycast's
strict +/-11000 threshold checks. It does not substitute the frontend's digital
threshold, while preserving existing frontend analog processing. Trigger/turbo
and Start/Special remain digital inputs. Digital and native-analog modes sharing
device 513 both require explicit profile selection. No tests were run.

Flycast now has an explicit proportional-trigger Dreamcast launch contract. Its
L/R targets require measured analog capabilities rather than digital shoulder
buttons. Four standard controllers use a fresh Dreamcast startup with digital
triggers disabled and zero trigger deadzone. Other core options are preserved.
This adds one distinct core with some launch coverage, not all Flycast modes.

The shared axis module now implements measured release-to-full-press conversion
to 0..32767, including reversed axes, non-centered release positions, saturation
outside gesture endpoints and rejection of out-of-device-range samples. This
primitive feeds a whole-gamepad reader, owned uinput publisher and launch-scoped
worker. Measured matching shoulder roles supply the requested analog capability;
the static catalog and digital-only planning remain unchanged. None of this has
been built or tested.

Pressure-layout calibration now requires measured release/full-press data and
rejects tiny discrete-axis ranges through the shared pressure converter. The raw
joydev launch writer rejects proportional shoulder targets unless its per-player
transport owns the normalized axes. Positive virtual half-axes retain full
physical trigger travel. No tests or builds have run.

Pressure conversion now also has a frame accumulator: it requires a complete
initial axis snapshot, batches changed values until a synchronization boundary,
and invalidates/neutralizes on dropped input or session failure. Resynchronization
must use an authoritative device snapshot, not guessed release values. The event
reader, virtual-device publisher and launch-session ownership are now connected.
No tests or builds have run.

A 64-bit Linux read-only pressure reader now consumes evdev packets, revalidates
recorded bounds, commits at SYN_REPORT, neutralizes after SYN_DROPPED and takes a
new axis snapshot after discarding the damaged packet. Reads are nonblocking and
bounded per poll; partial packets and unplug errors invalidate the session. It
does not grab devices or alter kernel calibration. The whole-gamepad reader used
by launch also drains the remaining queued history before recovery snapshots.
No builds, tests or live device reads have been performed for this addition.

Historical open-source Redream now has an explicit four-port controller contract
with a proportional stick and digital trigger endpoints. Its frontend explicitly
converts L2/R2 to zero/full values; intermediate Dreamcast trigger pressure remains
unavailable through that callback. Device disconnection is also not implemented
by the core's no-op setter. This does not cover modern standalone Redream. No
builds or tests have run for this addition.

PCSX ReARMed now has eight explicit contracts: digital or DualShock controllers
with two console ports, a five-player multitap on either port, or eight players
with both multitaps. Device 517 is admitted only for this core's reviewed
DualShock mode. All eight frontend ports are accounted for and inactive ports
are disconnected. Conditions document port order, the shared Analog-toggle latch
and early-return polling behavior, fresh-state requirements and missing rumble.
Analog Joystick, neGcon and pointer peripherals remain separate work. No builds
or tests have run for these additions.

FreeJ2ME now also has six vendor-mode contracts: Nokia, Siemens and Motorola,
each with navigation-only and analog stick-pointer variants. Dedicated navigation
and vendor soft-key semantics have separate layouts from Standard numeric mode.
The core's vendor joypad paths do not provide independent numeric 2/4/6/8 or a
separate vendor-center key; these limitations remain explicit. Existing pointer
click/drag limitations and backend configuration persistence still apply. No builds
or tests have run, and the distinct-core launch percentage is unchanged.

FreeJ2ME now has explicit Standard-phone keypad and analog stick-pointer variants.
The keypad covers all twelve numeric/star/hash keys and both soft keys. The pointer
variant preserves the core's contextual keypad-5/click behavior and documents the
absence of analog-stick drag events. Both require a fresh start and the matching
Java backend. Independent full-keypad vendor input remains separate work. No builds or tests
have run for these additions.

O2EM now has four explicit BIOS-selected controller modes for Odyssey 2, Videopac
G7000, Videopac+ G7400 and French G7400/Jopac. Player one receives virtual-keyboard
controls and numeric shortcuts in addition to its joystick; player two receives
only directions and Fire. Existing gamepad-swap policy is preserved. Conditions
describe virtual-keyboard input suspension and Enter release behavior. Native
keyboard input remains pass-through. No builds or tests have run.

EmuSCV now has an explicit two-controller contract with its shared console keypad.
It includes both fire buttons, keypad navigation/selection and the core's compound
Enter-plus-both-fire action. Pause, Reset and Power are accessed through the
overlay, rather than incorrectly labelling Start as Pause. The native keyboard
attachment is preserved outside controller slots. No builds or tests have run.

Stella now has three runtime-guarded native Linux RetroArch launch profiles sharing
an Atari joystick/console layout. Player one owns the eight console controls;
player two receives only its documented controller subset. The core exposes only Automatic/None devices;
it detects actual controller types from content and changes frontend port routing
when the left jack contains paddles. The runtime guard must establish matching
types and routes before these digital profiles proceed. Flatpak detection, paddles,
driving and pointer devices also remain unfinished. No builds or tests have run.

Stella now has a typed post-detection routing module. Joystick-family and driving
controllers consume one frontend slot; a paddle pair consumes two, so left
paddles shift the right jack to frontend port three. Console actions stay on
frontend port one. The module returns distinct digital actions for each supported
kind and rejects pointer/unknown peripherals. It does not infer detected types
from filenames or profile selections: ROM properties, signature detection and
post-detection port swapping are established by its final report. Driving/paddle analog transport remains
separate from their digital-action lists. No tests or builds have run.

The Stella module now parses its final `Game console created` report rather than
earlier heuristic detection messages. It requires the complete ROM/name/MD5/
controller/display/bankswitch field sequence, an exact expected cartridge MD5,
and explicit left/right post-swap controller descriptions. Missing, malformed,
unknown or conflicting reports fail; identical duplicate reports are accepted
because the core can emit to both stdout and its logging callback. A bounded,
collector now supplies fresh output from the selected native core/content/property
context. The parser alone does not authorize launch or trust saved user logs.

The collector process mechanism is now implemented with separate nonblocking
stdout/stderr pipes, bounded draining, one-megabyte limits per stream, cancellation
and a ten-second deadline. It parses only raw stdout after a successful exit.
Owned process-group cleanup keeps the leader unreaped until cleanup to avoid
signalling reused IDs; loss of child ownership fails closed. Private save/config
locations and exact core/content/property snapshots are prepared by the native
adapter. It has not been executed.

Private Stella cartridge/property snapshots are now implemented. They preserve
the original basename, copy an optional companion `.pro`, enforce the pinned
512-KiB cartridge limit, make only the private copies read-only, and compare
SHA-256 identities of both originals and snapshots. MD5 is computed separately
for matching Stella's console report. Property appearance/disappearance and read
errors are not treated as a default configuration. No snapshot or collector was run.

A native RetroArch detection context now prepares private saves, states, system,
cache and configuration directories, snapshots the selected core options, and
constructs a one-frame null-input/audio/video command. Automatic states, history,
runtime logs, overrides/remaps and configuration saving are disabled. Core bytes
and cartridge/property inputs are checked before and after collection. This is
not an OS sandbox or a Flatpak adapter. Launch supplies the selected native
executable/core and retains the context for a final pre-spawn identity check.
No detection command has been executed.

Stella's three digital profile expectations now have explicit acceptance rules:
both final controllers must be Joystick, both Genesis, or each BoosterGrip/
Joy2BPlus for that shared three-fire-event contract. Acceptance also checks the
two-port topology and player-one-only console ownership. Mixed pointer/paddle or
different digital types cannot silently pass as joysticks. Native detection can
return accepted per-port routes through this gate; the launch adapter compares
them with the selected profile's exact per-port output maps before proceeding.

The launch module now prepares native Stella detection from its exact selected
core/content and resolves game, folder, per-core or global options using the
existing RetroArch precedence rules. Options are preserved as a map for the
game-launch snapshot; malformed/duplicate entries, include directives,
custom environments and unresolved runtime wrappers fail instead of falling
back to defaults. Accepted options are always frozen, including an empty map,
so normal per-game/core options cannot diverge after detection. No detection was executed.

Calibrated launch preparation now receives the UI's cancellation token through
both fixed-profile and mode-aware paths. Per-player preparation, virtual joydev
readiness and final config attachment check cancellation. Bridge synchronization
checks it every twenty milliseconds while retaining its two-second deadline;
cancellation drops the owner and stops its worker/device. The same token is now
used for Stella collection through its launch guard. No tests ran.

Stella detection inputs/runtime directories can now be created under an explicit
private storage parent, preserving the native temporary-directory default. This
is preparation for Flatpak's separate temporary namespace; the Flatpak adapter
must still choose a visible app-cache location and build scoped access/lifetime
arguments. No Flatpak detection command is enabled or executed by this change.

The Flatpak preparation layer now constructs a private detection command for
the selected `org.libretro.RetroArch` launcher. It accepts the existing generated
launch prefix and rejects unresolved custom runtime/installation selections,
retains content/core identity checks, stores snapshots under the application's
cache, and rejects paths in namespaces owned by Flatpak. It requests a sandbox
with launch-scoped read-only input/core access and writable private runtime
storage. A dedicated instance-ID descriptor is owned by the command and made
inheritable only in the child, so unrelated concurrent launches cannot inherit
it. The future collector must close its parent writer after spawning and use the
reported instance, never the application ID, for sandbox cleanup. This command
builder is not wired into launch: owned-instance collection and cleanup remain
unfinished. Coverage remains 60/95 cores (63.2%), with 35 core entries and 171
standalone automatic adapters outstanding. No tests, builds, or Flatpak detection
commands were run.

The Flatpak invocation now creates and owns its instance-report pipe together
with the command. Both ends start close-on-exec, are reserved above the standard
stdio descriptors, and only the read end is nonblocking. Incremental polling is
bounded and accepts identity only at EOF, matching Flatpak's newline-free report.
Empty, oversized, nonnumeric, noncanonical or out-of-range reports permanently
invalidate that channel; a partial number cannot become a cleanup target.
This is still preparation, not enabled Flatpak support: instance lifetime/cleanup
and launch integration remain. Counts are unchanged at 60/95 cores (63.2%),
35 remaining core entries, and 0/171 standalone automatic adapters. No tests,
builds, or runtime detection were performed.

The fresh instance report can now acquire an owned reference lease from the
invocation's resolved XDG runtime directory. It opens the instance directory and
its `.ref` file without following their final symlinks, checks ownership/type and
inode identity, and takes a nonblocking open-file-description read lock. This
participates in Flatpak's reference-lock protocol while avoiding process-wide
POSIX lock release by an unrelated thread. Identity is checked again after locking
and before constructing numeric-instance cleanup; no application-ID cleanup is
generated. The lease must remain held until cleanup finishes. Cancellation/startup
race handling and launch integration still remain, so Flatpak support is not
enabled and coverage is unchanged: 60/95 cores
(63.2%), 35 core entries remaining, 0/171 standalone automatic adapters. No tests,
builds, or cleanup/detection commands were run.

The cleanup runner now retains the lease while invoking the selected launcher
with only the reported numeric instance ID. It isolates the cleanup subprocess,
polls under a one-second deadline, and reports nonzero exits/timeouts as errors.
Before cleanup, bounded no-follow reads validate the instance metadata's app,
numeric identity, sandbox flag and the exact unique private-runtime mount from
this invocation. GLib string-list escapes are decoded before whole-argument
comparison; substring matches and ambiguous duplicate identity fields are not
accepted. This binds the lease to the detection invocation rather than just a
potentially recycled number. Collector lifecycle wiring remains unfinished,
including startup/cancellation races and already-exited instance handling.
No cleanup command has been executed, and the coverage totals are unchanged.

JollyCV now has an explicit two-player ColecoVision contract covering its shifted
keypad workflow, standard side buttons, extra Super Action colors and digital
spinner steps. Source code, rather than contradictory descriptor labels, defines
spinner polarity. COL content is supported by this contract; ROM/BIN can select
CreatiVision by hash and still needs content-aware machine validation. CreatiVision,
My Vision and physical roller/driving fidelity remain separate work. No builds or
runtime tests have run for this addition.

FreeIntv now has an explicit two-controller contract preserving left-stick disc
angles, right-stick keypad shortcuts, three action buttons and controller-driven
access to all twelve keypad keys. The profile pins single-screen keypad overlays
and player-one's initial right-controller assignment. A cold-start guard now
rejects automatic state loading: the core restores machine memory but omits
controller-swap and keypad-cursor state. Its conditions distinguish
actual action-button code from contradictory pause-help labels and document
angle quantization, keypad overlay behavior and combined-input limitations.
Pointer-driven multi-screen overlays remain separate work. No builds or runtime
tests have run for this addition.

FreeChaF now has an explicit two-controller Channel F contract with all eight
digital hand-controller actions. A new semantic layout distinguishes push/pull
and rotation from generic gamepad labels. Both players can toggle the shared
console-button overlay and swap emulated controller sides. Conditions explain
overlay navigation, held-input limitations and restored core state. No builds or
runtime tests have run for this addition.

Gearcoleco now maps two standard ColecoVision controllers using its own button
contract, including the complete keypad. Unlike blueMSX, its left/right side
buttons use B/A, and keypad 0/9 use separate left analog X/Y axes with a 4000
threshold. These keys remain digital target controls. Spinner input is disabled
privately and extra right-axis Blue/Purple buttons remain unbound. No build or
runtime checks have run for this addition.

blueMSX now maps standard ColecoVision and SVI-603 Coleco controllers, including
all twelve keypad keys on both players. Keys 9/0 use the core's opposite right-X
axis encodings; they remain digital controls and may use RetroArch's existing
button-to-analog-direction bindings. Simultaneous opposing 9+0 can cancel and is
not claimed. Right-Y composite events stay unbound. The new keypad target reports
missing physical controls instead of omitting keys. Super Action, roller and
driving controllers remain separate work. No builds or runtime tests have run.

Fuse now offers explicit Cursor, Kempston, Sinclair 1, Sinclair 2 and Fuller
single-joystick modes, plus a two-player Sinclair 1+2 topology. Each maps one
Fire and four directions; per-button core keyboard overrides are cleared
privately. A native Spectrum Keyboard attachment is preserved on frontend port
three, outside controller slots. Fixed-port validation permits these pass-through
keyboard attachments without treating them as controller mappings. Timex, mouse
and virtual-keyboard controller modes remain separate work. No builds or runtime
tests have run.

blueMSX also has explicit SVI-318, SVI-328 and SVI-328 MK2 two-joystick
profiles. Their shared one-button target reflects the actual SVI joystick
callback, which reads one trigger rather than both MSX trigger events. These
fill the database-linked Spectravideo controller gap without increasing the
coarse core-entry count. SVI-603 Coleco expansion remains separate. No builds
or runtime tests have run.

blueMSX now has explicit MSX, MSX2 and MSX2+ two-joystick profiles. Each selects
its machine type and maps only the two triggers and four directions consumed by
the standard joystick device. Physical keyboard input remains native. Coleco,
Spectravideo and Sega-specific controllers are not inferred from these MSX
contracts. No builds or runtime tests have run for these additions.

fMSX now has two-joystick support and an explicit mixed-device mode with keyboard
shortcuts on player one (device 257) and a plain second joystick (device 1).
The ten shortcuts are F1-F5, Space, Graph, Control, Enter and Escape. Both modes
disable the core's automatic Space autofire privately. These are emulated MSX
events, not operating-system keyboard injection; full keyboard/text entry and
media/firmware workflows remain separate work. No builds or runtime tests ran.

The fMSX predefined keyboard mode is now a separate one-controller profile using
device 513. Its directions emit arrow keys, and its remaining controls emit
Space, Enter, N, M, F1-F5, Graph, Control and Escape. Port two is disconnected and
cleared. This mode does not emit joystick-one signals and is not a complete text
keyboard. A custom-keyboard mode or independently mapped second joystick still
needs additional contracts. No build or runtime checks have run.

melonDS DS has explicit DS/DSi profiles with relative joystick touch, press/drag,
noise microphone input while held, and next-layout selection. Its modifier differs
from DeSmuME: L2 changes pointer speed and L2+Y toggles the lid while still sending
Y to the console. That chord is documented rather than labelled as a dedicated
lid output. The Download Play alias covers controls, not wireless session setup;
DSi firmware/NAND, cameras, real microphone and Slot-2 peripherals remain separate
work. No build or runtime checks have run for these additions.

DeSmuME now maps the native DS buttons, right-stick stylus movement and press,
lid toggle, simulated microphone noise and layout-dependent screen switch.
Two explicit modes distinguish relative cursor motion from stick-to-coordinate
absolute positioning. Both disable competing left-stick and native mouse/touch
pointer paths, select unrotated input and pattern microphone noise. The lid can
toggle repeatedly while held after the core's 30-frame debounce; use a brief
press. Physical touchscreen calibration, voice input and Slot-2 peripherals are
not implied. These profiles have not been built or runtime-tested.

Citra now has explicit Old and New 3DS gamepad-only profiles with separate
Circle Pad and C-stick axes where present. They select the corresponding hardware
mode and pin right-stick behavior to C-Stick, avoiding the default simultaneous
touch-pointer movement. Touch, motion, microphone, Circle Pad Pro and the shared
Home/screen-swap action remain unfinished. Native mouse/touch settings are not
changed or counted as configured input adapters. No build or runtime checks have
run for these additions.

An additional explicit Old 3DS profile maps the core's relative touch cursor to
right analog axes and touch press/release to R3, including hold-and-move dragging.
It selects OpenGL, top/bottom screen layout and visible pointer rendering, while
disabling competing native mouse/absolute-touch sources in its private options.
The core's effective cursor deadzone is preserved. This controller-driven cursor
does not establish absolute touchscreen calibration, motion, microphone, or
simultaneous independent C-stick/touch support. No build or runtime checks ran.

Ardens now maps the six native Arduboy gameplay inputs read by its libretro
backend: A, B and directions, on one port. Its hex/arduboy content contract does
not invent Start/Select, a second pad, or physical reset/power inputs. FX data,
serial/link and desktop debugging remain separate workflows. This addition has
not been built or runtime-tested.

Geolith has explicit AES and MVS two-pad cartridge contracts, plus a CD/CDZ
two-pad contract restricted to cue/chd disc content. The cartridge modes pin the
selected hardware and disable the four-player cabinet extension privately. AES
Select is native Select; MVS Select inserts a coin. A content guard reads NEO v1
magic and the little-endian NGH game ID at byte 40 from the complete 4096-byte
header. It rejects the source-identified mahjong, Irritating Maze and V-Liner
IDs before a standard-pad configuration is generated. This is controller-mode
identification, not full ROM integrity or game/BIOS compatibility validation.
The CD profile preserves the selected CD hardware/BIOS mode. All three leave
combination macros and arcade service inputs unbound. Four-player cabinet mode,
Universe BIOS and dedicated mahjong/trackball/V-Liner interfaces remain unfinished.
The database-linked AES/MVS rows now have matching explicit standard-pad modes.
The core-entry count does not change when these platform gaps are filled; it is
not platform/mode coverage or total completion. No build or runtime checks have
run for these additions; only Rust formatting has run.

A separate explicit MVS profile adds the working cabinet Test switch to player
one, while player two retains just pad/Coin/Start requirements. Service is not
fabricated: the inspected core's geo_input_poll_stat_a masks with the R3 button
ID (15) instead of the intended 0x03 mask, leaving its active-low service bit
unchanged. Service remains an upstream implementation gap. The Test profile
retains the cartridge identity guard and does not enable special peripherals.
No build or runtime checks have run for this addition.

Game & Watch now has an explicit two-player script-facing digital-pad contract,
covering all 16 digital inputs delivered by its libretro backend. Its target is
a virtual interface, not a physical handheld diagram. Per-game action labels,
reduced control requirements, and the separate pointer interface remain gaps.
The profile requests the third (pointer) port disconnected and does not map it.
This addition is source-grounded only; no build or runtime testing has run.

PPSSPP now has a native PSP target with independent digital directions and a
proportional analog nub. Enhanced second-stick patches and system-panel keys
are not native PSP controls and remain separate work. Beetle Saturn offers four
explicit digital-pad topologies: two pads, either console port with a six-player
adapter (seven total), or both adapters (twelve total). 3D pads, wheels, Mission
Sticks, Twin-Stick, mouse, keyboard, guns and ST-V arcade inputs remain unfinished.

The inspected Saturn core can restore device types from a save state. Its
profiles therefore require a fresh start: the launch guard rejects automatic
state loading, unresolved included/custom configurations, and custom entry-state
arguments instead of silently overriding the user's resume settings. Manual
state loading later can still change devices and requires rechecking the mode.
The guard and mappings have been formatted, but not built or tested.

The launch writer now supports explicit mixed per-console-port device types.
An attachment such as a multitap remains connected even when its first player
slot is unassigned; empty slots have inherited keyboard, button and axis bindings
cleared. This prevents disconnecting later players behind the same attachment.
The catalog and coverage report carry the per-port device IDs separately from
the shared button mappings. Automatic mode inference does not select these
mixed topologies.

Two profiles use this: bsnes-hd five-player (standard pad plus four-slot multitap)
and Mesen-S four-player (standard pad plus three mapped multitap slots). The
inspected Mesen-S core initializes only four players' key bindings; its fifth
player remains unmapped and is not counted as supported. These additions have
been formatted but not built or tested.

Uzem now maps the complete SNES-style pad read by its one-port libretro backend.
The physical console's additional ports are not exposed by that code. Retro8
maps two PICO-8 gameplay pads; Fake-08 maps one plus its Pause action. Their O/X
RetroPad assignments are opposite, so they deliberately use different contracts
with the same semantic target. VeMUlator maps A/B and directions only: its Mode
input is disabled upstream and Sleep is not exposed. Those inactive controls
are not fabricated or counted as mapped.

bsnes-hd and Mesen-S also offer explicit two-standard-pad profiles. The latter
uses device 257 and includes Satellaview host controls, not broadcast/peripheral
workflows. Multitap is limited to the explicit profiles described above;
mouse/lightgun and Game Boy subsystem coverage is not implied.
All six additions are implementation-only, with builds and tests deferred.

Opera now has a standard 3DO pad layout and preserves the effective active-device
count (one by default, up to eight). Flight stick, lightgun, mouse and Orbatak
trackball modes remain separate work. Beetle PC-FX maps all six buttons and both
independent mode-switch actions on two ports. It follows RetroArch's supported
bitmask path; the inspected core's non-bitmask fallback has an upstream indexing
defect and is not a supported host path.

Beetle SuperGrafx preserves the effective multitap setting for its two-button
contract. Its six-button single-port and five-port contracts require an explicit
choice, pin startup pad mode/topology privately, and include the native Mode
Switch. Private turbo-hotkey disabling prevents III/IV becoming accidental turbo
switches when the user changes into two-button mode. Mixed per-port pad types
and mouse input remain unfinished. These changes have not been built or tested.

The latest batch adds VecX proportional analog sticks and four numbered buttons
on two ports, VBA-M single-player GBA/DMG/GBC modes, and four WASM-4 gamepads.
VecX's digital direction override is left unbound so it cannot override native
analog motion. VBA-M's inspected revision reads ordinary inputs from port zero
even while iterating other slots, so SGB multiplayer is not claimed. GB A/B
assignments follow the actual input buffer and hardware register code rather than
the reversed descriptors. WASM-4 mouse/pointer support remains unfinished.
The database links VBA-M to Nintendo e-Reader: the contract covers its GBA host
pad when launching GBA content, not card scanning/loading or standalone card data.
Build and runtime checks are still deliberately deferred.

The newest six core integrations add NeoCD, Potator, Beetle Virtual Boy,
Beetle WonderSwan, Beetle Lynx and Gearsystem. Virtual Boy and WonderSwan
reuse independent directional-cluster mapping; their core-side duplicate/rotated
keymaps are pinned off in private options. Display settings remain unchanged.
Beetle Lynx uses the actual Option 1/2 hardware bit assignments rather than the
reversed descriptors in the inspected revision. Gearsystem currently requires
explicit selection of native Game Gear 2 ASIC mode and uncompressed GG content;
other machine modes remain incomplete. None of these additions has been built
or tested in this implementation phase.

The preceding Nestopia, Mesen, BlastEm and PicoDrive contracts also require
explicit mode selection. NES two-/four-pad and Sega three-/six-button choices
do not imply automatic peripheral detection or compatibility with every game.

The four new small-digital contracts (Neo Geo Pocket, Atari 7800, Lynx and
Pokemon Mini) are source-grounded implementations with testing deferred. They
add target descriptions, shared left/right primary-pair rules, discrete emulated
auxiliary actions and per-port control subsets. They do not add physical-device
pair tables. ProSystem explicitly separates its player-one console panel from
player two. Gearlynx retains the core's own screen-rotation handling. PokeMini's
shake binding is a core-provided digital event, not host motion capture.

## Cores without enabled catalog launch contracts (4)

- `fbneo`
- `mame`
- `nymashock`
- `steemsse`

## Smart implementation sequence

1. Keep discovery coverage exhaustive and visible, independently of support.
2. Extend reusable target descriptions/capability transforms by input family:
   small digital pads; multiple directional clusters; analog/trigger variants;
   keypad/console controls; relative/absolute pointing and motion; keyboard and
   per-machine arcade controls. Missing physical capabilities remain explicit.
3. Add source-grounded core/mode output contracts that reuse those descriptions.
   Do not duplicate physical-device handling in contracts or enable a contract
   solely because its core name resembles an existing one.
4. Reuse composed mapping plans in standalone configuration/driver adapters.
5. Expose the effective selected controller, layout, core mode, mapping and any
   remaining requirements before the user launches a game.
6. After implementation, verify calibration backends, shared rule families and
   output adapters independently, plus representative end-to-end compositions.
   Do not hand-test a physical-controller × core Cartesian product.

Every progress update must name what changed, current/remaining coverage, and
estimated remaining work for the current batch. New support claims require actual
contracts/implementation; UI visibility alone does not increase supported counts.
