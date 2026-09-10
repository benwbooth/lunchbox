# Dolphin controller contract

Pinned libretro source: `e1e6d25fa1392b7d1bc05bf800c71b807a2bd2e0`, under
`Source/Core/DolphinLibretro`. No builds, tests or runtime probes were run.

## Current status

The explicit raw-GameCube mixed-trigger profile is now launch-enabled for
`Nintendo GameCube`, with four frontend ports and unused-port disconnection.
It requires measured pressure controls, native RetroArch, fresh startup,
explicit system/save routing and retained content/configuration snapshots.
Raw ISO/GCM classification rejects Wii and Triforce. Save routing follows
RetroArch sorting before pinning the effective path; no saves are relocated.
`UICommon::SetUserDirectory` uses its explicit argument before consulting
portable/environment fallback paths. Runtime behavior remains untested.

Some-mode coverage is now **89/95 cores (93.7%)**, with **286 enabled profiles**.
This is not full Dolphin coverage: Wii variants, Triforce, compressed media,
microphone, force-feedback forwarding and runtime validation remain outstanding.

## Implementation history

The following records describe the sequence of implementation steps. Earlier
statements that a component or profile is pending are superseded by the current
status above; they do not represent current launch availability.

## GameCube

`retro_set_controller_port_device_gc` clears the emulated pad's configuration,
selects its frontend device and installs explicit expressions. A/B/X/Y map to
their namesakes, R to Z, Start to Start, and the D-pad to the four directions.
Both native analog sticks map to the main stick and C-stick.

Trigger behavior must not be simplified into two digital buttons:

- Full-press expressions read analog trigger values, with digital L2/R2 used
  only when the corresponding analog value is zero.
- Soft-press expressions read analog values OR L3/R3.
- Mixed-trigger thresholds and output behavior are established below from the
  downstream implementation; frontend transport is still pending.

Downstream `MixedTriggers` tracing now confirms default deadzone 25 percent and
activation threshold 90 percent after deadzone processing. Activating the click
also forces analog output to full scale. The soft expression can reach full
analog scale without independently setting the click. GCPad converts both
resulting analog values to byte-sized trigger fields. The catalog now has a
layout separating analog travel from optional digital fallback/soft controls;
this does not yet establish calibrated analog-trigger transport.

The two proportional controls use the catalog's `shoulder` pressure group.
This is functional metadata: calibration requires measured release/full-press
values, and launch preparation selects the owned pressure-normalizing bridge
for analog shoulders. The bridge maps released pressure to zero and captured
full pressure to 32767, while retaining a centered virtual axis range for
RetroArch. A generic `trigger` group would bypass these checks and normalization.
The optional digital fallbacks remain digital controls. A Dolphin output profile
still needs to bind the pressure axes and fallback buttons to the corresponding
RetroPad L2/R2 channels without aliasing physical inputs.

Catalog validation now permits exactly the Dolphin pressure/fallback pair on
each L2/R2 output, retaining the duplicate-output rejection for other contracts.
The launch writer independently rejects two assignments to the same concrete
axis/button channel: pressure plus a physical fallback button can coexist,
whereas a second axis cannot silently overwrite the pressure assignment.
This plumbing is formatted but untested and does not enable a Dolphin profile.

Layout policy version 4 recognizes `l2`/`trigger_left` and
`r2`/`trigger_right` as equivalent proportional shoulder roles only when both
controls are analog shoulders. Calibration promotion still requires recorded
physical-axis measurements; a digital button does not become pressure-capable.
The one-input-per-target assignment remains in force, so the same trigger
cannot also supply a separate fallback. C-stick directions use the standard
`stick` group with right-stick semantic IDs, preserving side and direction
without inventing a second incompatible group.

The catalog now includes a 24-control, explicitly selected GameCube preview
profile. Its L3/R3 soft-press controls use the ordinary digital stick-click
group and output names. Measured triggers classified as `rear` in physical
layouts can be promoted to proportional shoulders for pressure-requesting
plans; this does not change the stored calibration or digital-only plans.
The profile deliberately has no launch descriptor until startup guards exist.

The preview now pins six source-defined options, checked by
`controller_dolphin::validate_gamecube_options` during catalog validation:
load/prevent-save enabled, GameCube microphone disabled, rumble disabled and
alternate Wii GameCube ports disabled, disc-to-Wii-menu redirection disabled,
and the microphone hotkey set to the exact `Disabled` sentinel. Rumble is not advertised through the
pressure bridge, which lacks force-feedback forwarding. These options do not
identify the content as GameCube, exclude Triforce, resolve native paths or
protect against every native lifecycle write; those remain launch prerequisites.

`Boot.cpp` can replace even GameCube disc boot parameters with the installed
Wii System Menu when disc-menu redirection is enabled. Disabling alternate
GameCube ports alone would not prevent that platform change. The microphone
mapping update also reads the hotkey expression independently of the accessory
enable option, so the no-microphone mode pins both settings.

## Raw disc classification implementation

`validate_raw_gamecube` reads the GameCube magic only after rejecting Wii magic
(matching `DiscIO/Volume.cpp` precedence), then reads the bounded filesystem
table at header offsets 0x424/0x428. It checks entry/name bounds, directory
ancestry and file extents, and locates root `boot.id` case-insensitively.
The `BTID` marker rejects Triforce as required by `VolumeGC.cpp`; a GameCube
magic word alone does not establish the ordinary-pad contract. Duplicate root
markers and malformed filesystems are rejected rather than guessed.

This reader is implemented but not yet connected to launch. It accepts decoded
raw streams; RVZ/WIA/GCZ/CISO and other containers still require their decoding
adapters, and filesystem integrity is not a claim of game playability.

The native reload audit also located `SConfig::OnESTitleChanged`, which calls
`Pad::LoadConfig`. Its call sites and applicability to GameCube still require
tracing before asserting that settings cannot replace the fixed mappings.

The located caller is IOS ES `TitleContext::Update`, after valid title metadata
and ticket checks on the first title change following an IOS reload. This is a
Wii title path; it is not evidence of an ordinary GameCube pad reload. The broad
partial-clone search was stopped after fetching stalled, so this finding does
not claim an exhaustive audit of every caller.

Raw ISO/GCM preparation now retains a content snapshot: canonical path, file
identity/state and the classified six-byte game ID. Rechecking repeats full
classification and rejects path replacement or metadata changes, including Unix
device/inode/change-time differences. This is a prelaunch race check, not a
whole-disc cryptographic digest or protection against writes after launch.
Native configuration preparation and the launch connection are still pending.

Native path resolution now follows explicit frontend paths without creating or
redirecting saves: `save/User`, or `system/dolphin-emu/User` when no save path is
supplied, with assets under `system/dolphin-emu/Sys`. It enumerates
`Config/Dolphin.ini`, `Config/GCPadNew.ini`, and eight game-INI candidates in the
core's merge order: system then user, each with one-character prefix,
three-character prefix, full ID and full ID plus `r<revision>`. Disc-supplied IDs
must be six ASCII alphanumeric bytes before they can form paths. Reading,
snapshotting and resolving references are separate from path enumeration.

Native snapshot implementation now reads the ten enumerated files without
writing them, limits each to 8 MiB, records canonical paths and SHA-256 digests,
and checks file identity/state before and after reads. Missing files are recorded
as absent; permission errors and dangling symlinks are not treated as absence.
A verification pass rejects changed content, path replacement and newly appearing
files. Ordered source bytes are retained for the next native-INI interpretation
step. The content snapshot also retains disc revision byte 7 for revision-specific
INI selection. Referenced controller profiles, effective setting interpretation,
directory identity checks and launch integration are still pending.

Native INI merging is now implemented from `Common/IniFile.cpp`: first-column
section recognition, UTF-8 BOM and CRLF handling, case-insensitive section/key
lookup, last assignment wins, first-equals splitting and paired double-quote
removal. Inline `#`/`;` are not stripped from values. Raw `$`/`+`/`*` lines are
excluded from the settings map while original bytes remain retained. Invalid
UTF-8, NUL and an ambiguous lone quote are rejected. Snapshot game settings are
merged in source-defined order; controller-profile reference resolution is next.

Source tracing of `InputProfile::GetProfilesFromSetting` establishes that a
setting is a comma-separated list: each trimmed choice is appended to the
user `Config/Profiles/GCPad/` root, directories are recursively expanded, and
non-directory choices get `.ini` appended. The first resulting profile is the
startup choice; this must not be approximated as a single filename.

Profile resolution is now implemented for all four effective `PadProfileN`
settings. Each comma-list entry preserves core ordering; directory expansion
sorts and deduplicates paths, matches `.ini` case-insensitively, and does not
recurse through directory symlinks. Missing choices are skipped, but a list with
no existing candidate is rejected. The first selected file is parsed and added
to the native snapshot; the entire candidate list is recomputed on verification
to detect changes in selection. Traversal is bounded to 256 list entries,
10,000 directory entries per choice and depth 64. Permission errors are surfaced
instead of silently choosing a different profile. Launch integration and
effective native configuration checks remain pending.

The combined preparation object now retains content, native files/profile
selection and directory identities. Missing native directories are anchored to
their nearest existing ancestor, so appearance and symlink retargeting are
detected on recheck. Existing Unix directories additionally retain device/inode
identity. Explicit existing system/save directories are rendered into the
session's RetroArch configuration with unsupported quoting/control characters
rejected; no directory or native file is created or rewritten by preparation.
This object is not yet connected to the launch writer, and native-setting
interpretation remains required before enabling the profile.

The native loader has an additional modern reference path in
`GameConfigLoader::LoadControllerConfig`. Preparation now also snapshots
`[GCPad.Controls]` `PadProfile1..4` and `GBAProfile1..4` independently from each
system/user layer. These append `.ini` to a literal name under `Profiles/GCPad`
or `Profiles/GBA`; comma lists and directory expansion do not apply. A local
override therefore cannot conceal a missing or changing system-layer reference.

The GameCube setter uses `SetBaseOrCurrent` for serial-interface device type:
an existing game-layer value causes a current-run assignment rather than a
base-only change. This supports the fixed device selection, but is not a claim
that all unrelated game settings or Wii input references have been resolved.

Startup snapshots now also include `GBA.ini`, `GCKeyNew.ini`,
`FreeLookController.ini` and `WiimoteNew.ini`, matching the controllers initialized
by the libretro wrapper. Legacy profile lists cover the corresponding GBA,
GCKey, FreeLookController and Wiimote prefixes as well as Pad. Modern per-layer
`[Wiimote.Controls]` references are also snapshotted; `Wiimote`, not `WiiPad`,
is the serialized configuration-system name. Accounting for these startup reads
does not enable those devices as GameCube gameplay inputs or claim Wii coverage.

Frontend save-path preparation now applies RetroArch's content-directory and
core-library sorting in source order before resolving Dolphin's `User` path.
Content-directory saves are supported. All three routing booleans must be
explicit, and the resulting directory must already exist so creation/fallback
behavior cannot silently select a different save location. The session pins
the resulting path and disables repeated sorting only in its temporary config;
the user's saved RetroArch configuration and save locations are not changed.

Launch plumbing now retains the combined Dolphin snapshot in `CalibratedLaunch`,
rechecks it before process launch, and appends its explicit system/save paths to
the session configuration. The new raw-GameCube guard requires native RetroArch,
no custom environment or unresolved includes, fresh startup, the exact
`dolphin-emu` library identity, four accounted frontend ports and ISO/GCM content.
Generic content validation cannot bypass native preparation. The catalog profile
remains preview-only while final effective-path/topology constraints are audited;
this plumbing therefore does not yet increase enabled coverage.

For Triforce, L is Test, Select is Coin, and L3+R3 is Service. These are not
ordinary GameCube controls and require a separate content/topology contract.
Microphone and rumble add further option-dependent routes.

The core can expose four GameCube ports, or eight frontend slots for Wii plus
alternate GameCube ports. Disconnect logic explicitly updates serial-interface
devices; topology depends on detected Wii content and the alternate-port option.

## Native settings

`dolphin_save_load_settings` defaults to disabled. The mapping function then
calls `Pad::GetConfig()->SaveConfig()`, so selecting fixed bindings can overwrite
native controller files. The GameCube filename is `GCPadNew.ini` (the
`InputConfig` constructor in `Core/HW/GCPad.cpp`), not `GCPad.ini`.

The startup ordering narrows the required guard: `Input::Init` calls
`Pad::Initialize`, which loads saved configuration and game-specific
`Controls/PadProfile1` through `PadProfile4` profiles. Later, the GameCube
device setter clears each selected pad with an empty section and installs
the explicit expressions. Its `SAVE_LOAD_SETTINGS` branch only decides
whether to save; it does not reload the saved GameCube bindings. Consequently,
enabling `dolphin_save_load_settings` prevents this setter's native-file write
without replacing its fixed GameCube expressions with saved expressions.
The Wii path has a separate native-profile loader and must not inherit this
conclusion.

The input hotplug callback only refreshes control references, not configuration.
A full `ResetControllers` reruns the setters, while option-specific GameCube
updates only update the microphone mapping. Startup configuration loading still
reads game profiles before the setter and can surface missing-profile errors.
Other lifecycle reloads and writes remain to be audited before enabling launch;
these source findings are not a runtime preservation guarantee.

Boot path resolution chooses `save_directory/User`, falling back to
`system_directory/dolphin-emu/User` when no save directory is supplied. System
assets normally come from `system_directory/dolphin-emu/Sys`. This means that
redirecting the user directory also moves save/configuration lookup; isolation
cannot discard the user's saves or silently reset unrelated settings.

## Remaining families

The wrapper has plain/sideways Wiimote, Nunchuk, Classic, Classic Pro and
MotionPlus variants, plus GameCube-on-Wii and real-Wiimote modes. Motion sensors,
IR and passthrough remain in the full goal. They are not covered by a prospective
GameCube mode.

The raw GameCube contract is enabled as described above. These remaining
families remain in the full controller-coverage goal.
