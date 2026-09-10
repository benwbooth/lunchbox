# Standalone Flycast controller mapping

## Setup UI status synchronized

The Flycast setup dialog title, help, staged confirmation and initial status now
reflect partial native Linux dispatch instead of incorrectly saying it is
pending. Untested runtime/internal routing limitations remain visible. Standard
arcade-panel work is parked here while ordinary standalone gamepad coverage
continues. No tests/launches. Coverage remains 93/94 RetroArch profiles (98.9%)
and 11/249 partial standalone integrations (4.4%).

## Shared native dispatch connected

Flycast saved content setups now enter shared calibrated launch preparation,
retain their native session and owned config/mappings, and use startup console,
loaded SDL library and open physical device checks. Provisional helper IDs must
agree with actual opened-joystick reports; failed startup kills/reaps the child.
Health checks retain topology verification. This is partial integration, not
proof of internal input routing, native game ID, companion-content selection or
runtime compatibility. No tests/builds/probes/launches were run. Coverage is now
93/94 RetroArch profiles (98.9%), 11/249 partial standalone integrations (4.4%).

## Bounded child console capture

Added stdout/stderr readers for native startup identity reports, bounded to 4 MiB
per stream and continuously drained to avoid blocking the emulator. Complete
UTF-8 lines are parsed; read failures, overflow and conflicting reports reject
handoff. Private config enables console INPUT notices without changing user
logging settings. Native session attachment/dispatch remain pending. Source
`core/log/LogManager.cpp` and `Log.h` confirm console and NOTICE controls. No
tests/builds/probes/launches. Coverage remains 93/94 RetroArch profiles (98.9%)
and 10/249 partial standalone integrations (4.0%).

## Native startup identity records

Added bounded parsing of native SDL opened-joystick startup records, including
instance/unique-ID agreement, controller name, initial port and duplicate-open
rejection. Incomplete lines are deferred while logs are being written. Source
inspection confirmed that the printed port precedes saved-port registration,
so it is explicitly not treated as final player-routing evidence. Log capture
and child handoff remain pending; no support-count change. Formatting passed;
no tests/probes/launches. Coverage: 93/94 RetroArch profiles (98.9%), 10/249
partial standalone integrations (4.0%).

## Launch-time physical discovery session

Implemented explicit SDL2 library/probe capture, stable physical controller
resolution, per-device control capture, library/probe hashes, routing freshness
checks and topology health checks. Settings review does not invoke this session.
Identical model GUIDs are allowed when physical identities remain distinct;
missing/ambiguous paths fail preparation. The separate helper's instance IDs
remain distinct from emulator-child identity evidence. Native child handoff and
shared dispatch are still pending. Formatting/whitespace checks passed; no
tests, builds, probes or launches executed. Coverage: 93/94 RetroArch profiles
(98.9%), 10/249 partial standalone integrations (4.0%).

## Native launch-plan composition

Added a retained prepared launch joining the selected native executable, exact
content argument, trusted executable hash, content-byte guard, calibrated panel
and private config environment. Custom argument/environment launches are not
silently rewritten. The selected content file is guarded, not its archive
parents or companion files; native game ID and child SDL identity still need
runtime handoff before shared dispatch can be enabled. No tests/builds/launches.
Coverage unchanged: 93/94 RetroArch profiles (98.9%), 10/249 partial standalone
integrations (4.0%).

## Calibrated panel preparation composed

Added one preparation path joining saved player choices, calibrated physical
SDL controls, per-instance mappings, player-port assignments and private config.
It writes per-game and global instance files for arcade/console startup phases,
without model-wide collisions or importing user mapping bindings. Physical
paths and saved identities must be unique. The native child instance IDs and
game ID still require launch-session verification; this is not yet dispatched.
No tests, builds or launches. Coverage remains 93/94 RetroArch profiles (98.9%)
and 10/249 partial standalone integrations (4.0%).

## Native Linux config selection

Private configuration now uses the native `XDG_CONFIG_HOME/flycast/emu.cfg`
directory structure and can select that root in a LaunchPlan environment.
Existing XDG config selection is replaced without modifying HOME, data paths
or working directory. Source: pinned `core/linux-dist/main.cpp` startup and
`find_user_config_dir`. This is config selection only: fallback mapping
isolation and native child identity/dispatch remain pending. Formatting and
whitespace checks passed; no tests, builds or launches. Counts remain 93/94
RetroArch profiles (98.9%) and 10/249 partial standalone integrations (4.0%).

## Explicit player-port config

Private config preparation now requires explicit native SDL instance/player
assignments and writes `[input] maple_sdl_joystick_ID` values, converting players
1-4 to native ports 0-3. Unselected observed joysticks are assigned -1 so native
enumeration defaults cannot duplicate a player's input. Missing or duplicate
devices/ports are rejected. Source: pinned `core/sdl/sdl.cpp` and
`core/input/gamepad_device.cpp` registration. The caller must establish child
instance IDs; helper IDs are not assumed equivalent. Child identity handoff,
config-directory selection and dispatch remain pending. No tests or launches.
Coverage unchanged: 93/94 RetroArch profiles (98.9%) and 10/249 partial standalone
integrations (4.0%).

## Private config preparation

Added a session-owned `emu.cfg` copy selecting the generated mappings directory,
preserving original settings and guarding source identity/content. The writer
accounts for native INI quote removal before the directory-list parser, using
both quoting layers. Native repeated-section/last-assignment behavior was read
from pinned `core/cfg/ini.cpp`. User config is never overwritten; private config
writes after startup are session-local. Launch directory selection, fallback
mapping isolation and controller port routing still need integration. No tests,
builds or launches. Coverage: 93/94 RetroArch profiles (98.9%), 10/249 partial
standalone integrations (4.0%).

## Native mapping search-path values

Implemented the pinned native semicolon/quoted directory-list grammar for
`[config] Dreamcast.MappingsPath`, including doubled quotes and safe rejection
of unterminated fields. Private mapping directories now expose an encoded
single-directory value for config preparation. This setting is global, not a
per-game option; it does not disable the native read-only fallback. Config
isolation and dispatch remain pending. Source: pinned `core/cfg/option.h`
and `option.cpp`. Formatting and whitespace checks only; no tests or launches.
Coverage remains 93/94 RetroArch profiles (98.9%) and 10/249 partial standalone
integrations (4.0%).

## Private mapping directory

Added session-owned generated mapping files with source guards, path/content
revalidation and filename-collision checks. Identical profiles may share a native
filename; different profiles cannot silently replace each other. User mappings
are untouched. Native mapping-directory selection, higher-priority override
isolation and launch dispatch remain pending. No tests, builds, probes or
launches run. Counts stay 93/94 RetroArch profiles (98.9%) and 10/249 partial
standalone integrations (4.0%).

## Mapping-file discovery

Added read-only filename-first/directory-second selection, with source-byte and
canonical-path guards plus higher-priority absence rechecks. Custom and fallback
directories must be explicitly resolved by the caller; no paths are guessed.
Unreadable files fail preparation instead of silently skipping them. Native
mapping caches require a fresh child. Private config selection and dispatch
remain pending. No tests, builds, probes or launches run. Coverage remains
93/94 RetroArch profiles (98.9%) and 10/249 partial standalone integrations (4.0%).

## Setup UI connected

Added settings-model review/staging actions and a standalone Flycast button in
the controller setup UI. It reuses source/destination diagrams and explains
native game IDs, runtime paths and six/eight-button source links. Status remains
explicitly launch-pending. No tests, builds, probes or launches run. Coverage
stays 93/94 RetroArch profiles (98.9%) and 10/249 partial standalone integrations
(4.0%); the Flycast UI alone does not increase dispatch coverage.

## Saved arcade setups and visual review data

Added persisted emulator/content setups with explicit native game ID, runtime
paths, executable hash, one to four panel players and complete physical source
links. Six-button panels default automatically; eight is selectable. Read-only
review returns source/destination layouts and mapping rows using existing
calibration. UI actions and launch dispatch remain pending. No tests, builds,
probes or launches run. Counts stay 93/94 RetroArch profiles (98.9%) and 10/249
partial standalone integrations (4.0%).

## Mapping filenames and selection precedence

Added native SDL mapping filename sanitization and ordered candidates: per-game
instance/model, global instance/model, then Dreamcast fallback for arcade. Source
shows instance names use process-local SDL instance IDs, not persistent hardware
IDs; helper IDs must not be assumed to equal the child's. Actual mapping-path
lookup, private file ownership and dispatch remain pending. No tests, builds,
probes or launches run. Counts remain 93/94 RetroArch profiles (98.9%) and 10/249
partial standalone integrations (4.0%).

## SDL-derived trigger classification

Added effective trigger classification from captured SDL mappings, matching
Flycast's left/right trigger lookup, half-axis exclusion and inversion-marker
precedence. Nonempty saved trigger lists retain precedence; missing mappings for
recognized gamepads are rejected. Calibrated panel composition can now derive
classification instead of requiring per-axis declarations. Native device/config
selection and dispatch remain pending. No tests, builds, probes or launches run.
Counts remain 93/94 RetroArch profiles (98.9%) and 10/249 partial standalone
integrations (4.0%).

## Calibrated arcade panel composition

Connected physical button/hat/axis resolution to six/eight-button routes and
native mapping output. Every panel source requires saved native calibration;
every SDL axis requires explicit effective trigger classification, which is
retained in the generated mapping. Duplicate source controls and conflicting
native ownership are rejected. Automatic classification, device identity and
launch dispatch remain pending. No tests, builds, probes or launches run.
Coverage stays 93/94 RetroArch profiles (98.9%) and 10/249 partial standalone
integrations (4.0%).

## Digital axis and trigger semantics

Added native digital axis qualification (16384 activation) and normal/reversed
trigger qualification (100-unit travel from the appropriate endpoint). Classified
triggers always use the positive mapping slot. Mapping output can retain explicit
trigger metadata including reverse markers. Source inspection found that Flycast
auto-populates empty trigger lists from SDL; effective classification therefore
remains a required caller input, not something inferred from a gesture alone.
Full physical resolver integration is pending. No tests, builds, probes or
launches run. Counts stay 93/94 RetroArch profiles (98.9%) and 10/249 partial
standalone integrations (4.0%).

## SDL button and hat translation

Added physical calibration translation through the existing SDL backend map,
then Flycast's native event codes: buttons retain their SDL index; hats use
((hat + 1) << 8) plus up/down/left/right offsets 0/1/2/3. Released-state checks
require captured controls to be inactive. Axis trigger classification and full
native identity/config dispatch remain pending. No tests, builds, probes or
launches run. Counts remain 93/94 RetroArch profiles (98.9%) and 10/249 partial
standalone integrations (4.0%).

## Standard Naomi/JVS panels

Added six-button default and selectable eight-button routing using pinned
maple_jvs.cpp: A/B/C/X/Y/Z feed buttons 1–6, D feeds Coin, and second-D-pad
left/right feed buttons 7–8. Directions and Start retain native meanings.
Existing arcade diagrams are referenced; no cabinet inspection is needed.
Native device translation and dispatch remain pending. No tests, builds,
probes or launches run. Coverage remains 93/94 RetroArch profiles (98.9%) and
10/249 partial standalone integrations (4.0%).

Added the native version-3 mapping writer supported by source pin
fb286f777ce690ef8acf3359a75ab84b61566ad9. It emits contiguous digital/analog bind
lists, native 0–3 port suffixes, dead zone and saturation settings, with distinct
physical ownership per port. Input codes remain provider-native, not libretro.

Arcade panel routing, physical device translation, native mapping filenames,
saved UI and dispatch remain pending. Arcade scope is still ordinary six/eight
buttons, directions, Start and Coin; no peripheral/per-cabinet expansion.

No tests, builds, probes or launches run. Coverage remains 93/94 RetroArch
profiles (98.9%) and 10/249 partial standalone integrations (4.0%).
