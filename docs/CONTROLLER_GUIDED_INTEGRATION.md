# Guided controller integration — source checkpoint

Updated 2026-09-09. Steps 1–9 were written source-only, without builds, tests,
native probes, device capture or app launches. Step 10 (runtime setup reuse)
ships with unit tests covering reuse, per-game precedence, conflict refusal,
launch-flag content resolution and Dolphin disc identity recomputation.
Formatting/source inspection is not runtime verification.

## Coverage denominator

The read-only local database has 249 standalone candidates and 94 non-BizHawk
RetroArch core names after the eight explicit Beetle/Mednafen aliases.

- RetroArch source contracts: **93/94 (98.9%)**.
- Standalone partial source dispatch: **24/249 (9.6%)**, including ares, bsnes, Stella, VICE, Hatari, DeSmuME, openMSX, Mesen2, BlastEm and xemu.
- Combined source entry presence: **117/343 (34.1%)**.
- Existing native adapters consuming guided choices: **24/24 (100.0%)**.
- Overall finished/verified coverage: **not established**.

The native integration count includes runtime-setup and host/backend restrictions,
not fifteen turnkey integrations. Catalog candidates with libretro alternatives
remain in the backlog; no entries were removed to inflate coverage. Catalog
presence does not imply a standalone binary or compatibility with every OS.
Steem SSE's compatible-runtime exception remains unresolved.

## Step 1 — persist target selection

`controller_target.rs` gives guided choices an exact runtime-kind/core/system
scope. Standalone and libretro choices cannot overwrite each other. The UI
restores the choice and saves it on selection/review using a narrow transaction
that preserves calibration and unrelated drafts.

RetroArch and ares launches consume the same choice. Explicit guided arcade
choices supersede older advanced MAME/FBNeo per-game setups for that launch only;
the advanced records remain intact. Without a guided choice, previous precedence
is retained. Invalid/removed targets fail rather than silently selecting another
layout. Native profiles now carry explicit system/player limits. Default target
lookup and coverage reporting both normalize Beetle/Mednafen aliases.

## Step 2 — connect existing native setup

`controller_guided_native.rs` creates an in-memory launch-only copy of matching
runtime setups for DuckStation, Mednafen, PPSSPP, mGBA, Snes9x GTK, FCEUX Qt and
SameBoy SDL. It applies guided player order, selected target mode and the same
per-controller choices shown in review. Players must be connected, distinct,
calibrated and within the target's capacity. ares already consumes these directly.

Runtime paths, hashes, content identities and saved advanced setups are retained.
Snes9x/FCEUX use their existing multiplayer attachment writers. Mednafen selects
its existing multitap/FourScore configuration according to player count.
DuckStation sets active pad types and disables unused pads in the private
effective input layer; legacy advanced setups retain their prior behavior.

These bridges currently require Linux physical calibration, not SDL3 logical
codes reinterpreted as evdev numbers. Most still need a matching saved runtime/
content setup. Missing setup fails explicitly. Additional bridges and runtime
reuse are described below.

## Step 3 — mGBA SDL runtime discovery

Native Linux mGBA SDL now discovers the executable, SDL2 dependency, bundled
helper, Bubblewrap and portable/XDG config.ini when no saved setup matches.
It creates a temporary setup for the selected ROM/P1, then uses the existing
version/frontend, calibration, private-config and child-handoff checks.

Discovery happens only at launch, not while browsing settings. A captured hash
is file identity, not compatibility. An existing config.ini is required. The
retained native contract is SDL 0.10.5; first-use creation, Qt, Flatpak namespaces,
other OSes and same-GUID ambiguity remain unfinished.

## Step 4 — ordinary arcade mapping

ares offers six/eight-button panels plus Start/Coin, with six as the default.
Buttons 7/8 use ares's virtual L/R triggers, independently of MAME's transport
channels. Layout policy 7 prefers physical triggers for these buttons on modern
pads, with stick clicks as a fallback. Launch still requires actual calibration
for the selected input backend. Manual choices remain authoritative.
No wheel/gun/cabinet-specific work was added.

## Step 5 — Dolphin guided device selection

Dolphin's existing GameCube writer now consumes guided P1–P4 order. A launch-local
flag requires empty native qualifiers and resolves them from the selected physical
paths against Dolphin's current evdev registration order. The resolved copy then
passes the existing qualifier/configuration checks. Legacy advanced setups still
require their explicit qualifier to match. No persistent runtime setup is changed.
System-data directory, native content identity and the existing runtime declaration
are still required. This is not Wii or cross-platform support.

## Step 6 — PCSX2, RPCS3 and melonDS guided targets

`controller_native_targets.rs` registers native profiles directly from the existing
writers' route tables, with explicit system/player limits and transport validation.
PCSX2 and RPCS3 expose their 24 standard pad controls; PCSX2 supports up to eight
players through its existing multitap writer, RPCS3 up to seven. Guided player order
and the algorithm's per-controller/manual assignments populate the saved writers'
source-control fields in a temporary copy. No SDL/native numbering is guessed.

melonDS exposes one primary-instance button target, using DS geometry with only
the twelve mapped buttons/directions. The touchscreen remains a mouse input;
microphone, lid and multiple instances are not claimed. Advanced review uses the
same buttons-only layout. Pressure-sensitive face buttons and additional PS2/PS3
actions are not part of these standard pad contracts.

## Step 7 — standalone Flycast arcade panels

Flycast offers only the ordinary six/eight-button native arcade profiles in the
guided screen (Arcade, NAOMI/NAOMI 2, Atomiswave), up to four players. The launch
bridge consumes the reviewed assignments and explicitly translates the generic
artwork's `select` destination ID to the writer's `coin` key. Native game ID,
runtime paths and executable identity remain unchanged. Dreamcast controllers are
not implied by these arcade profiles.

## Step 8 — standalone MAME guided native translation

MAME offers six/eight-button Arcade targets, up to eight players. Its XML writer
and profile validation now share the same native route table. Guided choices
contain physical source IDs only. After the existing launch-time capture matches
each physical path, a temporary setup obtains its full native GUID and input items
from actual SDL evidence. The normal preparation still checks every translated
item, threshold/rest state and unique GUID before emitting configuration.

Unresolved declarations cannot render XML or claim native review readiness.
Legacy advanced declarations retain their strict comparisons. Same-model devices
with duplicate native GUIDs remain explicitly unsupported by this MAME contract;
no invented serial or unstable ordinal is substituted. Runtime discovery, other
providers/hosts and native routing verification remain unfinished.

## Step 9 — accurate review labels and aliases

Mapping plans now distinguish a guided native bridge requiring runtime setup from
an absent adapter. Calibration and coverage labels no longer say DuckStation/native
mapping is wholly unavailable or label ares as RetroArch. Native metadata remains
separate from `automatic_launch_ready`; preview does not establish runtime readiness.
The QML target filter canonicalizes both requested and profile core names using
the same explicit Beetle/Mednafen aliases as Rust.

Steps 5–9 were formatted and inspected as source only. No builds, tests, native
probes, device capture or app launches were run.

## Step 10 — reuse of an existing runtime setup per game

`controller_guided_native/runtime.rs` clones a saved native runtime declaration
when the same emulator is launched with a different game and no exact per-game
setup exists. The clone is launch-local; saved setups are never rewritten.
Identity comparison removes only the fields the guided bridge overwrites
(content, players, pad types, multitap flags). Any remaining disagreement —
different runtime paths, hashes or future fields — fails with an explicit
choice request instead of silently selecting one setup. Custom launch arguments
that prevent resolving exactly one game file also require an explicit setup.

Reuse covers mGBA, FCEUX, Snes9x, SameBoy, Mednafen, RPCS3, melonDS and Dolphin.
DuckStation, PPSSPP, PCSX2 and Flycast are excluded: their saved setups carry
per-game identity (disc serial, disc-set membership, PCSX2 disc CRC, PSP or
native game IDs) that Lunchbox cannot recompute exactly without running each
emulator's own probe. Copying another game's identity would break the exact
identity rule, so those cores keep the explicit per-game setup requirement.
Dolphin recomputes its game ID and revision from the new disc's raw header;
that classification is re-verified by the existing launch checks.

## Step 11 — bsnes standalone SNES gamepad

`controller_bsnes` adds a native adapter for bsnes v115 (SDL joypad driver):
assignments `0x{id}/{group}/{input}[/{Lo|Hi}]` written into a private
`settings.bml` handed over with `--settings=`; SDL2 numbering is probed at
launch with the trusted library; the user's own settings file is never touched.
Guided target `bsnes:standalone-snes` (two players) plus per-game setup reuse.
See [BSNES_CONTROLLER_CONTRACT.md](BSNES_CONTROLLER_CONTRACT.md). Contract
encoding and rendering are unit-tested; runtime behavior is unverified.

## Step 12 — Stella standalone Atari 2600 panel

`controller_stella_native` adds a native adapter for Stella 7.x: mappings are
written as the `joymap` JSON into a private persistent `stella.sqlite3`
selected with `-basedir`, with `event_ver` pinned; the launch and probe run
with `SDL_JOYSTICK_LINUX_CLASSIC=1` (the DuckStation-verified classic
backend numbering). Two players; console switches on player one. See
[STELLA_NATIVE_CONTROLLER_CONTRACT.md](STELLA_NATIVE_CONTROLLER_CONTRACT.md).
Contract encoding and rendering are unit-tested; runtime behavior is
unverified.

## Step 13 — VICE standalone Commodore joystick

`controller_vice_native` adds a native adapter for VICE: a private
`-config`/`-joymap` pair binds the selected host devices to control ports one
and two and maps the digital joystick pins, probed over the trusted SDL2
runtime. Two players across the Commodore 8-bit platforms. See
[VICE_NATIVE_CONTROLLER_CONTRACT.md](VICE_NATIVE_CONTROLLER_CONTRACT.md).
Contract encoding and rendering are unit-tested; runtime behavior is
unverified.

## Step 14 — Hatari standalone Atari ST joystick

`controller_hatari_native` adds a native adapter for Hatari: a private HOME
and `-c` configuration bind the selected SDL devices to the two ST ports with
fire-slot button indices and the declared TOS image; directions are pinned to
Hatari's hardcoded SDL axes 0/1 with hat 0 override. See
[HATARI_NATIVE_CONTROLLER_CONTRACT.md](HATARI_NATIVE_CONTROLLER_CONTRACT.md).
Contract rendering and constraints are unit-tested; runtime behavior is
unverified.

## Step 15 — DeSmuME standalone DS buttons

`controller_desmume_native` adds a native adapter for DeSmuME's posix
frontends: the private `[JOYKEYS]` keyfile under an isolated XDG_CONFIG_HOME
maps the twelve standard DS controls with the 4-hex-digit joypad codes.
Single player, reusing the DS buttons-only layout. See
[DESMUME_NATIVE_CONTROLLER_CONTRACT.md](DESMUME_NATIVE_CONTROLLER_CONTRACT.md).
Code encoding and rendering are unit-tested; runtime behavior is unverified.

## Step 16 — openMSX standalone MSX joysticks

`controller_openmsx_native` adds a native adapter for openMSX: a private
`OPENMSX_HOME` plus `-setting` file carries the `msxjoystickN_config` TCL
dicts built from the calibrated controls, with `-command` plugging the second
port when two players are selected. See
[OPENMSX_NATIVE_CONTROLLER_CONTRACT.md](OPENMSX_NATIVE_CONTROLLER_CONTRACT.md).
Spec, dict and settings rendering are unit-tested; runtime behavior is
unverified.

## Step 17 — Mesen2 standalone NES controller

`controller_mesen2_native` adds a native adapter for Mesen2's Linux
frontend: the private settings.json maps the NES Port1 controls with the
evdev-keyed KeyMapping UInt16s under an isolated XDG_DATA_HOME, with the
sole-qualifying-gamepad constraint pinning the pad slot. See
[MESEN2_NATIVE_CONTROLLER_CONTRACT.md](MESEN2_NATIVE_CONTROLLER_CONTRACT.md).
Encoding and rendering are unit-tested; runtime behavior is unverified.

## Step 18 — BlastEm standalone Genesis pads

`controller_blastem_native` adds a native adapter for BlastEm: a private
HOME carries the tern-config pad bindings keyed by SDL device index with
`gamepads.<port>.<button>` targets for the six-button Genesis pad. See
[BLASTEM_NATIVE_CONTROLLER_CONTRACT.md](BLASTEM_NATIVE_CONTROLLER_CONTRACT.md).
Rendering is unit-tested; runtime behavior is unverified.

## Step 19 — xemu standalone Xbox pads

`controller_xemu_native` adds a native adapter for xemu: a private
`-config_path` file mounts the declared boot ROM, flash image and game while
binding each SDL GUID to its port with standard-index controller mappings
composed from the calibrated controls matched against SDL's own resolved
gamepad bindings, all over the classic SDL joystick backend pinned by
environment in both the probe and the child. The probe's classic capture
gate is extended to the SDL 3.2+ line with the DuckStation player
projection unchanged. See
[XEMU_NATIVE_CONTROLLER_CONTRACT.md](XEMU_NATIVE_CONTROLLER_CONTRACT.md).
Rendering is unit-tested; runtime behavior is unverified.

## Remaining work

Finish native guided runtime discovery and SDL3 translation, integrate additional
BizHawk controller modes, implement missing standalone contracts and applicable modes,
and finish Windows/macOS paths, isolation and input backends. Do not substitute
libretro configuration for standalone configuration. Once allowed, run all
repository gates and verify actual native input behavior. The goal stays active.
