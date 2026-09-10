# Standalone MAME controller mapping

## Shared native dispatch connected

Standalone MAME now selects saved emulator setups in the shared launcher and
retains its native session through spawn, verification and health checks. The
coverage report marks this integration partial. Supported preparation is Linux,
raw SDL, unique GUIDs, explicit runtime/cfg paths and plain machine shortnames;
native internal routing and runtime compatibility are unverified. No tests,
builds, probes or launches run. Coverage is now 93/94 RetroArch profiles (98.9%)
and 10/249 partial standalone integrations (4.0%).

## Native session and startup handoff

Added an owned native session joining SDL capture and launch preparation, with
plan equality and prelaunch verification. Child handoff checks native executable,
loaded SDL path and opened selected device nodes; cancellation/failure kills and
reaps the child. Config owners remain alive and runtime cfg writes are not
mistaken for prelaunch tampering. This code was not executed and does not prove
MAME's internal item routing or gameplay. Shared dispatcher wiring remains
pending. Coverage stays 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%). No tests, builds or launches run.

## SDL capture session

Added native Linux capture using the existing SDL2 helper and kernel topology
guard. It resolves selected stable controller IDs to runtime paths, captures
per-device controls, checks GUID uniqueness and revalidates routing/helper hash.
Saved runtime paths are explicit and optional until launch preparation. Capture
was not executed. Actual MAME child SDL-library identity and startup handoff
remain pending, so native dispatch is not counted yet. No tests, builds or
launches run. Counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%).

## Prepared native launch composition

Added an owned launch plan combining physical mapping checks, trusted executable
identity, private controller profile, private default/game cfg copies and matching
native flags. It requires an explicit original cfg_directory and plain machine
shortname invocation; custom command/environment resolution remains pending.
Construction requires caller-supplied SDL evidence and does not perform live
capture or child handoff. Native dispatch is therefore still not counted.
No tests, builds or launches run. Coverage remains 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%).

## Private default/game configuration copies

Connected selected input filtering to owned temporary default.cfg and exact
machine-basename cfg copies, matching pinned config.cpp load order. Source bytes
and identities are rechecked before launch, including absent-file changes.
Unrelated configuration is copied; originals are never written. Native runtime
cfg changes remain session-local and are not merged back. Caller must resolve the
effective cfg directory and basename and retain the owner; dispatch remains
pending. No tests, builds or launches run. Counts remain 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%).

## Selected input override filtering

Added a bounded XML transform for private cfg copies: removes only port elements
whose input type is explicitly selected for the panel. Other input assignments,
UI controls and non-input configuration are retained. It rejects unsupported
versions, DTDs and malformed/nested-over-limit documents. No user files are
modified. Copy ownership and launch integration remain pending. No tests, builds
or launches run. Counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%).

## Private controller profile ownership

Added a session-owned temporary controller file with content/path revalidation
and matching ctrlrpath/ctrlr, raw SDL, joystick enablement, Sixaxis disablement
and exact decimal threshold arguments. User ctrlr/cfg files are never replaced.
The owner must survive the child; actual launch wiring, existing option conflicts
and game-input override handling remain pending. No tests, builds or launches
run. Coverage remains 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%).

## Composed SDL preparation

Connected saved source links, native GUID matching, physical SDL item translation,
released-state checks and XML generation in a preparation function. Every target
must match its declared native item; threshold_basis_points defaults to 3000
(native 0.3). The function requires caller-owned same-runtime inventory and
physical paths; it does not capture devices or launch MAME. Native topology,
runtime options (including Sixaxis off), freshness and dispatch remain pending.
No tests, builds or launches run. Counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## Ordinary axis-switch translation

Raw SDL translation now handles standard absolute axes using measured native
rest/press values and MAME's float threshold conversion. It mirrors SDL value
doubling and the native >= comparison, emits POS/NEG SWITCH tokens, and rejects
thresholds that cannot distinguish rest from press. POS/NEG avoids the separate
X/Y joystick-map path; this does not add peripheral mapping. Launch must apply
the same threshold and provider. No tests, builds or launches run. Coverage
remains 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone integrations
(3.6%); native capture and dispatch remain pending.

## Raw SDL identity and switch translation

Pinned input_sdl.cpp actually registers raw SDL joysticks with GUID-only IDs;
its locally constructed serial suffix is not passed to device creation. Added
selected-path/GUID uniqueness checks and physical-to-SDL-to-MAME button/hat
translation using existing SDL backend maps. Duplicate model GUIDs are rejected
instead of claiming stable ownership. Same-runtime inventory capture, axis
thresholds and launch dispatch remain pending. No tests, builds or launches run.
Counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone
integrations (3.6%).

## Physical source diagrams

Added optional source_controls declarations mapping panel controls to saved
physical-layout controls. Review validates existing calibration and unique source
controls, then shows both diagrams and declared links. Missing links remain
unresolved. These links are not native device/item discovery and do not imply
launch readiness. No tests, builds or launches run. Coverage remains 93/94
RetroArch profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## Graphical destination review

The setup editor now renders the existing six/eight-button destination diagram
and its declared native token routes. Source connections remain explicitly
unresolved instead of assigning physical geometry from MAME item numbers.
Editing declarations clears stale review results. Full source-to-destination
association, native inventory and launch dispatch remain pending. No tests,
builds or launches run. Coverage remains 93/94 RetroArch profiles (98.9%) and
9/249 partial standalone integrations (3.6%).

## Setup editor

Added a standalone MAME JSON editor with review and staging actions, backed by
settings-model invokables. Review reports declared panel layouts and native
tokens without treating them as observed devices. The dialog explicitly marks
launch mapping unavailable; saving still uses the main settings workflow.
Graphical source/destination review and launch dispatch remain pending. No tests,
builds or launches run. Counts remain 93/94 RetroArch profiles (98.9%) and 9/249
partial standalone integrations (3.6%).

## Saved panel settings

Added persisted standalone MAME setups with executable hash, explicit joystick
provider, physical/native identities, six/eight-button panel choices and typed
native controls. Validation rejects duplicate players/devices and incomplete
panels. XML rendering requires a separately supplied native inventory; saved IDs
are not treated as observed devices. UI and launch dispatch remain pending.
No tests, builds or launches run. Coverage remains 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%).

## Native device-slot XML

Added mapdevice emission from full MAME-native joystick IDs, with XML escaping,
distinct slots, required inventory presence and substring-ambiguity rejection.
Every joystick token must refer to an explicitly mapped slot. Source is the
pinned docs/source/advanced/devicemap.rst: provider IDs can be nonunique and
missing controllers can change numbering. A caller must still capture and
revalidate the actual native inventory; this writer does not prove identity or
enable launch dispatch. No tests, builds or launches run. Coverage remains
93/94 RetroArch profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## Standard native joystick items

Added typed button, hat, axis-half, Start and Select token generation from
MAME's native input token table and code_to_token implementation. Axis halves
explicitly request SWITCH class for ordinary digital panel controls. This avoids
passing arbitrary item-name strings from physical calibration. Native device
ownership, provider item discovery and threshold handling remain pending; the
helper does not equate MAME device numbers with SDL or joydev numbers.
No tests, builds or launches run. Counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

Added a native controller XML writer for standard six/eight-button panels,
directions, Start and Coin, up to eight players. This is distinct from the
existing libretro MAME adapter. It accepts already-resolved native provider
tokens; it does not infer device numbering from SDL, joydev or player index.

Source: MAME 0.280 commit ec9abd86c6c9029f67e9cf4908ef5426b78d3eab,
docs/source/advanced/ctrlr_config.rst. Default type assignments do not supersede
game-specific overrides or driver PORT_CODE definitions. Provider resolution,
saved setup and launch integration remain pending; this writer is not yet
counted as standalone dispatch. No per-cabinet or peripheral expansion.

No tests, builds or launches run. Coverage remains 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%).
