# Mednafen standalone controller mapping

## Analog saved-review validation

Dual Analog setup review now requires measured proportional opposite directions
on four distinct native axes, with consistent kernel metadata, before device
access. Hats, missing measurements and mismatched axis pairs receive errors in
review rather than only at launch. Corrected-neutral checks remain launch-time.
The visual planner now describes Mednafen's partial saved-setup integration
instead of incorrectly calling it an unimplemented RetroArch adapter. No tests,
builds or launches run. Counts remain 93/94 RetroArch profiles (98.9%) and 9/249
partial standalone integrations (3.6%).

## Dual Analog integration

Connected play-station-dual-analog saved setups, proportional calibration,
the existing dual-stick visual layout and CCD/CUE launch preparation. Per-player
digital/Dual Analog overrides can be mixed, including multitaps; aggregation
preserves each selected port while disconnecting unused ones. This is native
SCPH-1180 forced analog mode, not DualShock rumble or mode switching. No tests,
builds or launches run. Coverage remains 93/94 RetroArch profiles (98.9%) and
9/249 partial standalone integrations (3.6%), not tested compatibility.

## Proportional stick calibration resolver

Added a separate joydev stick resolver using recorded evdev bounds and captured
native axis corrections. It rejects hats and discrete gestures, requires native
corrected neutral and bipolar travel, and preserves proportional half-axis input
without applying the digital button threshold. Current released values may vary
inside the native correction dead zone. Profile wiring remains pending; this
helper alone does not enable an analog controller mode. No tests, builds or
launches run. Coverage remains 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%).

## PlayStation Dual Analog native generator

Added SCPH-1180 forced-analog assignments: sixteen buttons, eight proportional
stick directions, explicit unity axis scaling and existing multitap routing.
Each opposing pair must use the same physical axis with opposite polarities;
four axes must be distinct and cannot overlap button ownership. Native source
uses separate direction settings with proportional magnitude, not button
threshold output. Calibration qualification and profile/launch selection are
pending, so this generator is not enabled as a saved controller mode yet.
No tests, builds or launches run. Coverage remains 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%).

## PlayStation digital dispatch integration

Connected play-station-digital saved setups, existing PlayStation visual layout,
native calibration vocabulary, module config discovery and CCD/CUE launch
preparation. Optional psx_multitaps selects each physical port's four-way adapter;
defaults are off, with 2/5/8 sequential virtual slots. Native firmware and internal
joystick IDs remain unverified; analog controllers and other content formats
remain incomplete. No tests, builds or launches run. Counts remain 93/94
RetroArch profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## PlayStation digital binding generator

Added the native SCPH-1080 fourteen-control assignment generator from pinned
psx/input/gamepad.cpp, frontio.cpp and psx.cpp. It supports independent physical
port multitaps (2/5/8 sequential virtual slots), disconnects unused pad ports,
clears native rapid-fire face-button bindings and preserves memory-card settings.
Saved setup, visual profile and launch integration are still pending; this is
not yet an enabled PlayStation adapter. No tests, builds or launches run.
Counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone
integrations (3.6%).

## PC Engine CD dispatch

Both pce and pce_fast saved pad setups now accept CCD/CUE content alongside raw
cartridges, retaining mixed two/six-button pads and the existing port routing.
Disc dependencies use the same snapshot guards as Saturn. Native LoadCD entry
points were inspected at the existing source pin; CD BIOS configuration remains
native and firmware identity is not verified. No tests, builds or launches run.
Counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone
integrations (3.6%); this expands an existing adapter, not the emulator count.

## CUE disc dependencies

Saturn saved digital-pad dispatch now accepts CUE as well as CCD. Preparation
captures every FILE dependency, including native-supported audio track formats,
and rechecks paths, sizes and hashes before launch. Filename tokenization follows
the pinned CDAccess_Image.cpp UnQuotify routine. Descriptors currently require
UTF-8; native legacy-encoding conversion, TOC and M3U remain uncovered. Track
layout and audio decoding remain native responsibilities, not verified here.
No tests, builds or launches were run. Coverage remains 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%).

Current status: native Linux GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine saved-setup dispatch is connected with
calibration, private config layers, child handoff and health checks. Coverage is
93/94 RetroArch profiles (98.9%) and 9/249 standalone candidates with partial
dispatch (3.6%). Child internal ID confirmation, other native drivers/systems,
compressed content and runtime verification remain incomplete. No tests or launches
run. Older checkpoints below describe the implementation sequence.

## Saturn CloneCD saved launch

Connected digital-pad and tap assignments to Saturn `.ccd` launch with retained
CCD/IMG/SUB snapshots, private config layers and existing child handoff checks.
Source `mednafen.cpp` LoadCDGame sets the outside descriptor basename before
the same module/per-game config load sequence. Native BIOS is still required;
firmware identity, other disc formats and runtime compatibility remain open.
No tests, builds or launches; counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## NES IPS-aware inspection

NES preparation now snapshots adjacent IPS content and computes controller
overrides from patched bytes in memory. Pinned `IPSPatcher.cpp` semantics are
preserved: literal/RLE writes, zero RLE count means 65536, zero-filled gaps and
ignored data after EOF. Patch identity or absence is retained through startup.
User ROM and patch files are not modified. No tests, builds or launches;
counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone
integrations (3.6%).

## NES IPS precedence correction checkpoint

Source tracing found native `LoadIPS` runs before NES module loading and input
database selection. NES setup now rejects an adjacent `ROM.ext.ips` until
patched-byte inspection is implemented, and retains an absence guard through
preparation/startup so a newly appearing patch cannot invalidate the checksum.
This avoids approving controller choices from the wrong content. No tests,
builds or launches; counts remain 93/94 RetroArch profiles (98.9%) and 9/249
partial standalone integrations (3.6%).

## Disc dependency capture in progress

Added CloneCD dependency snapshots for CCD/IMG/SUB, including native extension
case propagation, sector/subchannel size checks, canonical paths and hashes for
all companions. Descriptor capture alone is not treated as disc identity.
CUE/TOC/playlists and Saturn launch integration remain unfinished. No tests,
builds or launches; counts remain 93/94 RetroArch profiles (98.9%) and 9/249
partial standalone integrations (3.6%).

## Saturn saved visual setup

Added `saturn-digital` saved setup and per-player visual review using the
existing Saturn digital layout. Optional `saturn_multitaps: [bool, bool]`
controls permitted 2/7/12 slot counts. Native disc preparation is still pending;
review reports pending and launch explicitly rejects it rather than running
without the selected bindings. No tests, builds or launches. Counts remain
93/94 RetroArch profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## Saturn digital mapping layer

Added thirteen-control native Saturn pad settings with explicit six-way taps
on either physical port, supporting 2/7/12 sequential virtual slots. Unused
ports select none. Pinned `ss/input/gamepad.cpp` declares no rapid-fire controls,
so none are fabricated. Builtin reset and analog devices remain separate.
Saved setup and disc launch integration remain unfinished. No tests, builds or
launches; counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%).

## Genesis saved-setup dispatch

Connected `md-three`/`md-six` pads, per-player mixed choices and explicit
`md_tap` selection (none/port-one/port-two/dual/four-way) through visual review,
native calibration and `.bin`/`.md`/`.smd` cartridge dispatch. Ports use native
sequential virtual ordering; native input auto-selection is disabled so the
saved tap remains authoritative. CD and game compatibility remain incomplete.
No tests, builds or launches. Counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## SNES Faust saved-setup dispatch

Connected `gamepad: "snes-faust"` to native module selection, the SNES visual
layout, calibration and the Faust-specific eight-slot configuration generator.
Both SNES modules expose the same saved physical-slot convention; Faust virtual
indices are translated from actual multitap placement. Cross-module player
overrides remain rejected. No tests, builds or launches. Counts remain 93/94
RetroArch profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## SNES Faust routing layer

Added a distinct Faust configuration generator. Its `MapDevices` consumes
virtual slots sequentially, unlike the older SNES module's fixed tap ordering.
Saved physical slots are translated explicitly; all Faust slots support none,
and its settings use `sport1/2.multitap`. Shared twelve-control validation is
reused only after checking Faust's own GamepadIDII. Setup and launch integration
remain unfinished. No tests, builds or launches; counts remain 93/94 RetroArch
profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## SNES saved-setup dispatch

Saved `gamepad: "snes"` now connects visual review, twelve-control calibration,
both multitaps and native `.sfc`/`.smc`/`.swc`/`.fig` dispatch. All selected pads
are assembled together, so unused-slot clearing cannot overwrite another player.
This is the native `snes` module, not `snes_faust`; special cartridge modes and
runtime verification remain incomplete. No tests, builds or launches. Coverage
remains 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone integrations
(3.6%).

## SNES mapping layer checkpoint

Added native `snes` twelve-control gamepad assignment generation for eight
logical ports. Native tap routing is preserved: 3–5 belong to physical port 2,
6–8 to port 1. Multitaps are enabled from selected slots, unused slots have
all gameplay/rapid bindings cleared, and no nonexistent selector is emitted
for fixed gamepad-only subports. Source: pinned `src/snes/interface.cpp`.
Saved setup and launch integration remain unfinished. No tests, builds or
launches; counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%).

## FDS standard pad dispatch

NES setups now accept headered and raw `.fds` images, using native magic and
complete-side counting from pinned `src/nes/fds.cpp`. FDSLoad leaves device
overrides empty, so saved native pad settings apply. This adds pad dispatch,
not disk-management controller actions. Native `disksys.rom` firmware must
already be available; firmware identity and disk-change controls remain open.
No tests, builds or launches; counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## Unused ROM-forced NES pads

ROM-forced gamepads on unused ports are now retained with all eight gameplay
and both rapid-fire bindings cleared in every private layer. Selected-player
device conflicts and expansion-device conflicts still reject preparation. The
reconciliation is transactional: failed conflict checks leave assignments
unchanged. This handles UNIF's forced ports 3/4 without sharing player inputs.
No tests, builds or launches; counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## UNIF controller metadata

Native NES dispatch now accepts `.unf`/`.unif` and reads conventional UNIF chunks
to apply CTRL metadata before checking controller-device conflicts. Fixed-size
metadata with inconsistent declared lengths is rejected because the native
loader consumes handler-specific lengths. Repeated CTRL chunks retain native
last-wins behavior; ports 3/4 retain the native forced-gamepad defaults. A saved
disconnected port conflicting with that metadata is still rejected; automatic
reconciliation of forced but unused pads remains unfinished. FDS remains open.
No tests, builds or launches. Counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## NES saved-setup dispatch

Connected `nes-two`, `nes-four-score` and `nes-famicom-four` to the NES visual
profile, per-player calibration and native `.nes` dispatch. The setup-wide
adapter controls Four Score and Famicom expansion selection. All native ports
are explicit, rapid bindings are cleared, and iNES CRC device conflicts are
rejected before handoff. UNIF/FDS, special cartridge behavior and runtime
verification remain unfinished. No tests, builds or launches were performed.
Coverage remains 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone
integrations (3.6%).

## NES native checksum implementation

Added byte-based iNES CRC computation matching pinned `nes/ines.cpp`
`iNESLoad`: PRG/CHR padded separately with 0xff to power-of-two bank counts,
zero PRG count means 256 banks, trainer/header/trailing bytes excluded, and
short declared content rejected. The result feeds the native InputDB override
table. The pinned loader's legacy size interpretation is retained rather than
substituting NES 2.0 size semantics. Launch integration remains unfinished.
No tests, builds or launches; counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## NES ROM-device override layer

Added the pinned iNES InputDB CRC-to-device table, preserving null/inherit versus
explicit none, plus UNIF CTRL flag interpretation and a settings-conflict check.
The caller must supply the native loader CRC; whole-file hashes are not accepted
as equivalent evidence. Native CRC computation/content framing and dispatch
integration remain unfinished. No tests, builds or launches were performed;
coverage remains 93/94 RetroArch profiles (98.9%) and 9/249 partial standalone
integrations (3.6%).

## NES mapping layer in progress

Added eight-control native NES pad assignment generation for two-player,
Four Score and Famicom four-player routing. Each player has distinct joystick
ownership; rapid bindings are cleared and unused ports select none. Native
`nes.nofs` and `nes.input.fcexp` are set explicitly from the adapter choice.
Source: pinned `src/nes/input.cpp` (GamepadIDII, NESPortInfo and NESINPUT_Init)
and `src/nes/nes.cpp` (nes.nofs). ROM-selected DesiredInput can override these
settings and still needs resolution before connecting launch dispatch.
This mapping layer is not counted as additional coverage. No tests or launches.

## PC Engine Fast integration

Saved `pce-fast-two`/`pce-fast-six` choices now connect to native `pce_fast`
profiles, config layers and raw `.pce`/`.sgx` dispatch, with mixed two/six-button
pads on up to five ports. The pinned `src/pce_fast/input.cpp` lines 242–310
declare the same button names and mode switch as `pce`; setting prefixes remain
module-specific. No nonexistent fast-module multitap setting is emitted.
Cross-module player overrides are rejected. CD content remains incomplete.
No tests, builds or launches; counts remain 93/94 RetroArch profiles (98.9%)
and 9/249 partial standalone integrations (3.6%).

## PC Engine saved-setup integration

Saved `gamepad: "pce-two"` or `"pce-six"` now connects native calibration,
existing two/six-button visual layouts, per-port config and `.pce`/`.sgx` launch
dispatch for one to five players. Optional per-player `gamepad` overrides now
support mixed `pce-two`/`pce-six` pads in visual review and native assignments.
Omitted overrides inherit the existing setup default; cross-system overrides
are rejected. CD content and pce_fast remain separate work. Six-button mode is
only suitable for compatible games. Counts remain 93/94 RetroArch profiles
(98.9%) and 9/249 partial standalone integrations (3.6%). No tests or launches.

## PC Engine mapping layer checkpoint

Added native `pce` two/six-button setting generation for up to five distinct
players, explicit mode defaults, cleared rapid/mode-toggle bindings, disconnected
unused ports and explicit multitap selection. Source: pinned
`src/pce/input/gamepad.cpp` and `src/pce/input.cpp`. This is not `pce_fast`.
Saved setup, visual profiles and launch routing are not connected yet, so this
does not increase coverage. No tests, builds or launches were performed.

## Master System ports

Saved `gamepad: "master-system"` supports one or two players on distinct native
ports 1/2 with `.sms` dispatch. The same pinned `src/sms/system.cpp` declares
directions, Fire 1/2 and Pause for each port. A separate visual layout labels
Pause as a console action. Configuration captures both players into one private
layer set; unused port gameplay and rapid-fire bindings are cleared. Physical
identity, startup file-descriptor checks and health checks retain both devices.
No tests, builds or launches were performed. Counts remain 93/94 RetroArch
profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## Game Gear extension

Saved `gamepad: "game-gear"` connects the Game Gear visual layout to all seven
native controls and `.gg` launch dispatch. Pinned `src/sms/system.cpp` lines
313–358 define builtin directions, Button 1/2 and Start. Native setting-name
normalization lowercases `Start`; rapid Button 1/2 are cleared. Master System
uses different port names and remains separate work, not implicitly covered.
No tests, builds or launches were performed. Counts remain 93/94 RetroArch
profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## Virtual Boy extension

Saved `gamepad: "virtual-boy"` connects the existing Virtual Boy visual layout
to all fourteen native gamepad controls and `.vb`/`.vboy` launch dispatch.
Pinned `src/vb/vb.cpp` lines 956–1019 define both directional pads, A/B, L/R,
Start and Select on the builtin gamepad. Rapid A/B are cleared. The separate
system battery-voltage switch is preserved, not folded into the gamepad.
No tests, builds or launches were performed. Counts remain 93/94 RetroArch
profiles (98.9%) and 9/249 partial standalone integrations (3.6%).

## WonderSwan / Color extension

Saved `gamepad: "wonder-swan"` connects the existing WonderSwan visual layout
to eleven native controls and `.ws`/`.wsc` launch dispatch. Pinned
`src/wswan/main.cpp` lines 556–615 define physical X1–4/Y1–4, A/B and Start.
The private layers explicitly select `wswan.input.builtin gamepad`, preserving
physical identities instead of selecting the separate rotation-adjusted device.
Rapid A/B are cleared. Hyphenated setting names are preserved, matching the
frontend's CleanSettingName implementation in `src/drivers/input.cpp`.
No tests, builds or launches were performed. Emulator counts remain unchanged.

## Neo Geo Pocket / Color extension

Saved `gamepad: "neo-geo-pocket"` connects the existing NGP visual layout to
native calibration, private config layers and `.ngp`/`.ngc` launch dispatch.
Pinned `src/ngp/neopop.cpp` lines 347–377 define the single builtin gamepad:
directions, A/B and Option, with no separate Select. All seven controls are
required and rapid A/B bindings are cleared. This extends the existing partial
adapter; counts remain 93/94 RetroArch profiles (98.9%) and 9/249 partial
standalone integrations (3.6%). No tests, builds or launches were performed.

## Lynx gamepad extension

Saved `gamepad: "lynx"` now connects the existing Lynx visual layout to native
calibration, configuration overlays and `.lnx` launch dispatch. The pinned
`src/lynx/system.cpp` IDII/PortInfo (lines 405–438) defines nine controls:
directions, A/B, Pause and separate Option 1/2. All are required; rapid A/B and
rapid Option 1/2 are cleared. Native `lynx.rotateinput` behavior is preserved.
This extends the existing partial adapter and does not increase emulator counts.
No tests, builds, device captures or emulator launches were performed.

Implemented native input-expression serialization for 128-bit joystick IDs,
buttons and signed/full-range absolute axes with optional 4.12 fixed-point scale.
Indices are bounded to the documented 0–1023 range. Native IDs must be captured
from Mednafen rather than substituted with SDL GUIDs. The encoder emits one
explicit input expression, without inherited OR/chord alternatives.

Sources: official Mednafen input mapping documentation and source mirror commit
f0ee9d595db68ad5247ba5ac6a8367fdced9c3fc, src/drivers/input.cpp.

Pending: system-specific ordinary gamepad profiles, native inventory/calibration,
config staging, visual setup and launch dispatch. No tests, builds, device probes
or emulator launches run. Coverage remains 93/94 RetroArch profiles (98.9%) and
8/249 standalone candidates with partial dispatch (3.2%). Mednafen is not counted
as a ninth adapter yet.
Native GB and GBA gamepad assignment generation now uses source-derived
gb/gba.input.builtin.gamepad keys, including shoulder_l/shoulder_r on GBA.
Complete control sets and unique inputs are required; digital targets reject
full-range axis expressions. This does not yet clear rapid-fire alternatives,
stage configuration or connect physical calibration and launch dispatch.
Native flat-file rendering now appends complete selected mappings while retaining
original settings text, and clears source-derived rapid_a/rapid_b alternatives
for GB/GBA. The launch owner must still include per-game override layers before
claiming effective mapping. No file or emulator was touched by the renderer.
Native GB/GBA catalog profiles now reuse the existing Game Boy/GBA destination
layouts with a separate mednafen-settings transport. Catalog validation checks
the native control vocabulary and prohibits RetroArch dispatch on these profiles.
Profile identifiers are exposed for physical calibration and saved setup review.
Dedicated setup and launch integration remain pending.
Linux base-ID construction and native duplicate-ID allocation are implemented.
The base packs sysfs bus/vendor/product/version and legacy joystick axis/button
counts in big-endian order. Manager collisions increment the low 64 bits in
driver/cache order. Source evidence: Joystick_Linux.cpp and Joystick.cpp at the
same pin. Actual enumeration and physical identity capture remain pending; these
functions do not infer IDs from SDL GUIDs or execute device probes.
Physical control resolution now consumes original joydev button/axis maps and
kernel correction records, retaining hat axes in native numbering. Measured
axis endpoints are checked against the effective Mednafen fixed-point threshold
at unity binding scale. Raw capture, released device-state verification and
saved-profile integration remain pending; no probe has been executed.
Saved GB/GBA calibration now translates through the native physical map into
complete gamepad bindings. It requires every physical control's released state,
matches axis rest against saved measurements, and reuses the writer's native-ID,
completeness and competing-owner checks. The caller must still supply fresh
same-device capture and effective threshold; live capture/dispatch remain pending.
Saved native Mednafen setups now persist emulator/content identity, native base
directory, trusted executable hash, bubblewrap path, GB/GBA profile selection and
one physical controller. Defaults and whole-list validation are integrated into
settings. No-I/O review projects complete saved native calibration into existing
Game Boy/GBA source/destination layout views and labels launch integration pending.
The shared native setup dialog now loads, reviews and stages Mednafen records,
including GB/GBA profile selection. JSON input is bounded and the complete list
is validated/reviewed before staging. Existing mapping views display the selected
source controller and native destination. Live capture and dispatch remain pending.
Native flat-setting parsing now preserves literal values and last assignment
precedence, handles BOM/comments and rejects malformed key/value separators.
Effective axis-threshold extraction uses the native default of 75 percent when
absent and rejects invalid values; callers must merge actual override layers first.
The renderer validates captured settings syntax before appending replacements.
Ordered config-layer snapshots now capture bounded regular UTF-8 files, merge
effective values, reject canonical aliases, generate selected mapping replacements
for every layer and recheck original paths/bytes. Native override filename discovery
and absent-layer tracking remain the launch owner's responsibility; accepting a
caller-provided ordered list does not prove all native overrides were included.
Private config staging now materializes each captured layer into a retained
writable temporary file, supplies canonical-target bubblewrap mounts and rechecks
original/staged bytes before handoff. Child mount verification compares file
identities. Only configuration files are replaced; native save paths are retained.
Native layer discovery, physical capture and application dispatch remain pending.
Module/game override path generation now follows general.cpp: base/<system>.cfg
then effective filesys.path_pgconfig/<FileBase>.<system>.cfg. Relative directories
resolve against the native base; absolute ones remain absolute. Callers must use
the effective path after module overrides and resolve native FileBase first.
An absent-override guard now detects files appearing after preparation. Global
legacy-config migration selection and full discovery orchestration remain pending.
Modern config discovery now captures global -> module -> game overrides, using
the module-adjusted per-game directory and retaining absence guards. It requires
an initialized modern mednafen.cfg and rejects legacy headers before native
startup can rename/migrate originals. Native FileBase and command-line override
resolution remain caller requirements; legacy migration support is not claimed.
Combined preparation now discovers effective config/threshold, translates supplied
same-device calibration, and stages all selected mapping replacements. Private
staging retains the complete discovery owner, including absent-override guards,
through handoff. Native device capture and FileBase resolution remain prerequisites;
the supplied ID/map/state are not treated as physical identity proof by themselves.
Read-only native joydev map capture is now connected through a shared raw-map
reader that retains original kernel axis order, including hats, and correction
data. Existing SDL consumers still receive their prior reordered ClassicMap.
The raw reader supports /dev/input/jsN and legacy /dev/jsN, checks character-device
type and rereads maps/corrections. No capture or tests were executed. Native
enumeration, sysfs identity and released physical-state capture remain pending.
Native directory enumeration now selects /dev/input or legacy /dev by matching
joystick-entry counts (ties favor /dev/input), uses Linux version sorting, and
retains both lists for topology rechecks. It handles canonical js<digits> names;
noncanonical scanf-accepted names are not supported. Sysfs identity, successful
device-open filtering and released-state capture still need integration. No
enumeration/capture was executed during implementation.
Native sysfs identity capture now follows the pinned subsystem search order,
resolves the joystick's input parent, reads bounded bus/vendor/product/version
fields, and constructs the base ID using captured joydev counts. Identity can
be recaptured for change detection. Event-node association, physical released
state and complete inventory ownership remain pending; this code was not run.
Event association now selects the first native version-sorted event child and
checks its sysfs parent. Read-only EVIOCGKEY/EVIOCGABS capture supplies raw physical
state for every mapped joydev button/axis without consuming events, then rechecks
identity/association. Full inventory/node ownership and launch integration remain
pending. No state capture was executed during implementation.
Retained inventory now combines ordered native enumeration, sysfs identity,
joydev maps/corrections, duplicate-ID allocation and joystick/event node identity.
It supports cancellation between devices and full identity/map/order recapture.
Incomplete or inaccessible device capture is rejected rather than guessing native
failed-open cache positions. Other enabled native drivers still require separate
handling. This implementation was not executed; session/dispatch remain pending.
Native session preparation now resolves the saved physical controller against
kernel topology and native joydev inventory, captures raw state, translates the
saved calibration using effective settings, and owns private config staging.
It retains joystick/event paths and full identity/topology/config rechecks.
Native FileBase, launch command and child handoff remain pending; no session
preparation or device capture was run during implementation.
Native command preparation now validates exact saved emulator/content identity,
trusted executable hash and file hashes; resolves raw handheld ROM basenames;
selects the saved base via child-only MEDNAFEN_HOME; mounts private config layers;
and forces the selected gb/gba module. Compressed-content naming and custom
arguments remain unsupported. Child startup and application dispatch are pending.
Owned child startup now checks the intended executable, private config layer
mounts and selected joystick/event descriptors, with bounded cancellable waiting
and kill/reap on failure. These establish file/device handoff, not native
internal IDs or gameplay responses. Additional native drivers and child ID-log
confirmation remain incomplete. Application dispatch is still pending; no child
or probe was started during implementation.
