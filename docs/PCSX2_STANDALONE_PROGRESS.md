# Standalone PCSX2 controller mapping

## Shared native dispatch connected

Saved PCSX2 emulator/content setups now enter the shared native launcher,
retaining private config/data and a session log. Startup checks compare native
SDL player/type/name/instance reports and disc serial/CRC, loaded SDL library
and opened physical devices. Cancellation/failure kills and reaps the child;
topology health checks remain active. These checks are implemented, not executed.
Internal routing, additional actions, portable installs and broader SDL/runtime
compatibility remain unverified. No tests/builds/probes/launches. Coverage is now
93/94 RetroArch profiles (98.9%), 12/249 partial standalone integrations (4.8%).

## Explicit native content identity

Saved setups now optionally retain native data_root/serial/disc CRC; launch
preparation requires these instead of external loose arguments. Added native
disc serial/CRC log parsing, distinct from the current ELF CRC, with conflicting
startup identities rejected. Older setups remain reviewable until native fields
are supplied. Actual log capture/confirmation still needs session integration.
No tests/probes/launches. Coverage unchanged: 93/94 RetroArch profiles (98.9%),
11/249 partial standalone integrations (4.4%).

## Fresh routing and backend agreement

Added inventory-to-binding capture comparison and fresh prehandoff SDL capture
comparison covering player assignments, device order, bindings, kernel maps,
hints and runtime identities. Diagnostic event counts/warnings are excluded.
Prepared native launch now selects the same Linux classic SDL backend as the
helper. Actual child runtime/hints still need confirmation. No tests/probes/
builds/launches. Coverage stays 93/94 RetroArch profiles (98.9%) and 11/249
partial standalone integrations (4.4%).

## Physical discovery session connected internally

Added launch-time SDL3 discovery and selected binding capture, resolving saved
physical identities through the existing topology guard. Saved runtime_libraries
provide explicit dependencies (including required libudev). Captures validate
the PCSX2 player contract, library/probe hashes and physical/player agreement,
then expose requests for native profile generation. Actual child startup and
fresh routing confirmation still need wiring; no dispatch count is enabled.
No tests/probes/builds/launches executed. Coverage stays 93/94 RetroArch profiles
(98.9%) and 11/249 partial standalone integrations (4.4%).

## Explicit SDL3 player probe contract

Added --pcsx2-player-probe to the existing SDL3 helper. Pinned PCSX2 source uses
the same added-event opening and post-open index fallback sequence, so that
mechanism is shared while reports carry a distinct PCSX2 contract identifier.
Native 8-bit player bounds are enforced. Existing helper restrictions remain:
Linux classic backend, explicit libudev and SDL 3.2.20; broader runtime support
is not inferred. No helper was executed. Runtime session/dispatch remain pending.
Formatting only; coverage remains 93/94 RetroArch profiles (98.9%), 11/249
partial standalone integrations (4.4%).

## Native SDL player routing

Added native opening-order player allocation (reported index unless invalid or
occupied, then lowest free ID), with native 8-bit field bounds. Added parsing
of actual PCSX2 SDL opened-device reports for instance/player/type/name checks.
Input opening order and post-open indices remain capture obligations; pre-open
hints are not treated as proof. Source: pinned SDLInputSource.cpp. Runtime
session and dispatch remain pending. No tests/probes/launches. Coverage stays
93/94 RetroArch profiles (98.9%), 11/249 partial standalone integrations (4.4%).

## Native launch-plan selection

Added retained native Qt launch preparation selecting the private tree with
`-datapath`, preserving the exact content argument after `--`, and checking
executable/content hashes. Portable markers are rejected because native source
gives them priority over -datapath. Pinned layered settings order is command
line, game, input profile, secrets, base; private controller settings account
for the copied layers. Native content identity and SDL player routing still
need a runtime session before shared dispatch. No tests/probes/builds/launches.
Coverage: 93/94 RetroArch profiles (98.9%), 11/249 partial standalone integrations
(4.4%).

## Additional settings layers preserved

Private data preparation now retains optional secrets.ini settings without
printing their contents, guards both source presence and bytes, and isolates
controller/folder entries in that layer too. Per-game folder values are resolved
against the original data root before private game/input directory replacement.
All generated files remain inside the session-owned temporary directory.
Native layered precedence/runtime behavior still needs verification when testing
is allowed; launch dispatch remains pending. No tests/probes/launches. Coverage:
93/94 RetroArch profiles (98.9%), 11/249 partial standalone integrations (4.4%).

## Private data-tree composition

Joined bounded main-config capture, folder resolution, native game-settings
selection and controller overlays into an owned inis/PCSX2.ini + gamesettings
tree. Source config/game files remain guarded and unchanged. Folder parsing
rejects duplicate case-insensitive keys; generated files are checked before
launch. Runtime content identity, secrets.ini handling, any per-game folder
overrides and native launch selection still need integration. No tests/probes/
builds/launches. Coverage stays 93/94 RetroArch profiles (98.9%) and 11/249
partial standalone integrations (4.4%).

## Data-root folder preservation

Added native folder resolution for all 16 LoadConfig paths, including debugger
folders relative to inis rather than the data root. Private folder values move
only GameSettings/InputProfiles while keeping BIOS, memory cards, saves and
other resources at their original absolute locations. Caller must supply the
selected effective folder settings and original root; config parsing and private
tree creation remain pending. Source: pinned Pcsx2Config.cpp. No tests/probes/
launches. Coverage unchanged: 93/94 RetroArch profiles (98.9%), 11/249 partial
standalone integrations (4.4%).

## Native game-settings selection

Added serial/CRC-first and legacy CRC-only file selection, bounded reads,
source-content/path and higher-priority absence guards. Private overlays use
the preferred native filename even when sourced from legacy settings. CRC zero
and serials needing unimplemented native sanitization are rejected. Native
content identity capture and data-root dispatch still need integration. No
tests/builds/probes/launches. Coverage: 93/94 RetroArch profiles (98.9%), 11/249
partial standalone integrations (4.4%).

## Per-game controller override isolation

Added a private game-settings overlay that replaces controller sections and
removes every EmuCore/InputProfileName selector, preserving unrelated per-game
emulator settings. This prevents a copied game config from selecting an old
external controller profile. Native serial/CRC filename selection and source
absence guards remain caller responsibilities; source VMManager.cpp checks the
serial/CRC file before legacy CRC.ini. Data-directory/session dispatch remains
unfinished. No tests/probes/launches. Coverage unchanged: 93/94 RetroArch profiles
(98.9%), 11/249 partial standalone integrations (4.4%).

## Private main-config controller overlay

Added whole-section replacement of InputSources/Pad/Pad1-Pad8 in a private
PCSX2.ini copy, preserving other settings and avoiding inherited pad macros or
duplicate controller-binding keys. Multiline INI values are rejected rather
than misparsed. Source confirms Qt startup selects all app data using -datapath,
not a settings-only switch; relocation and per-game profile precedence still
need handling before dispatch. No tests/builds/launches. Counts unchanged:
93/94 RetroArch profiles (98.9%), 11/249 partial standalone integrations (4.4%).

## Private profile retention and proportional triggers

Added session-owned input-profile files with prelaunch identity/content checks.
Pinned VMManager source selects profiles through game settings
`EmuCore/InputProfileName`, resolved beneath EmuFolders::InputProfiles; no
unverified command-line profile switch is assumed. Private config/game-settings
selection remains to be connected. Axis-backed L2/R2 now use proportional range
translation rather than accepting a digital mapped output; physical buttons
remain usable as binary triggers. Formatting only; no tests/probes/launches.
Coverage remains 93/94 RetroArch profiles (98.9%) and 11/249 partial standalone
integrations (4.4%).

## Calibrated profile composition

Joined saved source links with verified classic-backend SDL3 mappings and the
native profile writer. Stick directions use continuous axis translation and
require four distinct axes with opposite calibrated halves; buttons/hats cannot
masquerade as sticks. Physical paths and controller identities must be distinct.
Native SDL player IDs remain an explicit caller input. Runtime capture, private
config selection and dispatch remain pending. No tests/probes/builds/launches.
Coverage remains 93/94 RetroArch profiles (98.9%) and 11/249 partial standalone
integrations (4.4%).

## Setup UI connected

Added PCSX2 settings-model JSON read/review/stage actions and a controller-setup
button using the existing source/destination diagram viewer. Help describes
runtime paths, source-control IDs and multitap ordering. Staging validates all
setups and only updates in-memory settings until the main Save action. The UI
explicitly says launch integration is pending. Formatting/whitespace checks
passed; no UI launch, tests or probes. Coverage remains 93/94 RetroArch profiles
(98.9%) and 11/249 partial standalone integrations (4.4%).

## Saved setups and visual review data

Persisted PCSX2 per-content setups with explicit runtime paths/hash, multitaps,
players and physical source links. Review validates calibrated source controls
and emits DualShock source/destination diagram rows with native Pad sections.
Review performs no device I/O. Settings-model/QML access and native dispatch
remain pending; optional analog/pressure/rumble choices are not yet exposed by
this saved schema. No tests/builds/launches. Coverage stays 93/94 RetroArch
profiles (98.9%) and 11/249 partial standalone integrations (4.4%).

## Measured SDL3 input translation

Added digital and proportional-axis translation using existing measured SDL3
range/alias checks, then converting to current PCSX2 positional and Joy tokens.
Pinned source marks suppressed raw axes by input index. Raw hats on recognized
gamepads are rejected because PCSX2 only allocates hat state for non-gamepads.
Physical evdev-to-SDL resolution and launch routing remain caller obligations;
the adapter is not dispatched yet. Formatting only; no tests/probes/launches.
Coverage stays 93/94 RetroArch profiles (98.9%), 11/249 partial standalone
integrations (4.4%).

## Complete controller-profile writer

Added deterministic fresh controller-profile INI generation for one through
eight players, with explicit multitaps and native interleaved Pad-slot mapping.
Every selected pad requires all 24 standard controls; optional analog/pressure
and motor outputs use their native keys. Duplicate player/device/control
ownership and motor/input mismatches are rejected. Unused pads are disconnected;
no macros or hotkeys are inherited by this fresh profile. Physical-axis fidelity,
profile selection and launch dispatch still need implementation. Sources:
pinned SIO/Sio.cpp, Pad.cpp and PadNotConnected.cpp. Formatting only; no tests or
launches. Coverage remains 93/94 (98.9%) RetroArch profiles and 11/249 (4.4%)
partial standalone integrations.

## SDL3 native binding encoder

Added typed current positional gamepad buttons/axes, raw JoyButton/JoyAxis,
full/half/inverted raw axes, cardinal hats and motor/haptic output tokens.
PCSX2's 8-bit SDL player ID is explicitly distinct from SDL instance IDs and
target pad slots. Named gamepad indices are range-checked; deprecated legacy
label aliases are not emitted. Source: pinned SDLInputSource.cpp and
InputManager.h. Physical translation/config/dispatch remain pending. Formatting
only; no tests/builds/launches. Coverage unchanged: 93/94 RetroArch profiles
(98.9%), 11/249 partial standalone integrations (4.4%).

## DualShock 2 native destination contract

Added the 28 native DualShock2 binding keys with button/half-axis/motor types,
24 existing DualShock visual-control routes, and native Pad1-Pad8 section names.
Analog toggle and pressure modifier remain explicit optional actions rather
than repurposed face buttons. Native source confirms SDL3; legacy SDL2 A/B/X/Y
tokens migrate by label, so the upcoming encoder must use current positional
or explicit raw-joystick syntax. Physical translation, settings and dispatch
are not connected yet. No tests/builds/launches. Counts remain 93/94 (98.9%)
RetroArch profiles, 11/249 (4.4%) partial standalone integrations.

Next ordinary standalone gamepad adapter. Source pinned to PCSX2/pcsx2 commit
`98697735f1bb1a1452d975251269abd1019876d1` (master ref read during this pass).
Authoritative tree locates pad implementation under `pcsx2/SIO/Pad/`, including
`Pad.cpp`, `PadDualshock2.cpp`, `PadTypes.h`; native input handling is under
`pcsx2/Input/InputManager.cpp`. No PCSX2 adapter or support count is enabled yet.
No tests, builds, device probes or emulator launches.

Current coverage: 93/94 RetroArch profiles (98.9%); 11/249 standalone candidates
with partial native integrations (4.4%). These are presence counts, not tested
compatibility or all-mode completion percentages.
