# Native GPGX controller contract

Pinned source: BizHawk `8c6b8958bbbe623eaaa36bc82af858b812893628`, inspected
under `/tmp/lunchbox-core-contracts.AmSobs/bizhawk`.

`Consoles/Sega/gpgx64/GPGXControlConverter.cs` defines:

- Genesis3: Up/Down/Left/Right, A/B/C, Start.
- Genesis6: the same eight controls plus X/Y/Z and Mode.
- AddToController prefixes those names with the resolved native player number.

Step 363 adds native three/six-button calibrated translation using the existing
genesis-3/genesis-6 catalog layouts. The shared translator requires distinct
native SDL bindings and retains physical/logical calibration checks.
Step 364 adds ordinary/empty connector topology. GPGXSyncSettings.UseSixButton
is shared by all normal pads; ControlType None/Normal serializes as 0/1.
GPGXControlConverter increments player numbers for each real device but skips
DEVICE_NONE, so a right-only normal pad is P1. The topology validates complete,
unique compacted assignments and exposes their physical connector indices.
This model describes requested ordinary topology, not proof that content loading
retains it: the upstream settings explicitly warn about automatic control changes.
Step 365 adds native configuration encoding for GPGX Genesis Controller,
PreferredCores.GEN = Genplus-gx (CoreNames.Gpgx) and exact GPGXSyncSettings pad/port fields. Power,
Reset, Previous Disk and Next Disk retain existing bindings; stale player
analog/feedback/autofire bindings are removed. This encoder does not target
GPGX's SMS/GG/SG decks. Multitap topology and launch/session
routing and editor selection remain unfinished. Mouse, light-gun, Activator,
XE-1AP and other peripheral contracts are not covered by this pad translator.
No tests, builds or runtime verification have been performed.

Step 366 adds snapshot-based configuration composition and transactional argument
preparation. Complete unique native slots and distinct SDL devices are required
before translation. Warnings identify the requested physical connector and retain
the unresolved loaded-device/content-override boundary. This is preparation, not
verification that the launched game uses the requested controllers.

Step 367 connects shared digital-session dispatch, pre-probe compacted-slot
validation, per-pad normalization targets, device-routing guards and private
configuration ownership. Saved settings/editor selection, multitap contracts
and loaded-device override handling remain unfinished and unverified.

Step 368 adds optional gpgx_topology saved settings, the gpgx SDL calibration
scope, shared-session selection and three/six-button saved-calibration checks.
Platform routing is limited to Genesis/Mega Drive and Sega CD aliases, excluding
32X and the distinct SMS/GG/SG decks. Editor selection, multitaps and loaded-game
override handling remain unfinished. This is not runtime compatibility evidence.

Step 369 adds editor selection, normal-pad connector toggles, shared pad mode,
saved topology round-trip and three/six-button visual previews. UI shows compacted
player/connector routing and requires complete assignments before recording.
Saved SDL previews use the gpgx scope for either pad mode. Multitaps and loaded-game
override handling remain unfinished; no runtime behavior has been verified.

Step 370 models the post-load gpgx_get_control system[2]/dev[8] arrays used by
GPGX.SetControllerDefinition. Native system None/Gamepad are 0/1; device
None/Pad3B/Pad6B are 0xff/0x00/0x01. Validation rejects system, pad mode and count
mismatches and returns actual device indices in converter order. Acquisition,
content/runtime identity binding and launch enforcement remain unfinished;
deserializing this structure is not proof of a real loaded-core observation.

Step 371 inspects the BizHawk gitlink's Genesis Plus GX revision
`051d430d3d1b54625f9900c8f152d7f232e06daf`, core/input_hw/input.c, directly from
the TASEmulators upstream repository. No upstream implementation is vendored.
Normal pads occupy slots 0/4; Team Player occupies 0–3 or 4–7; paired 4-Way Play
occupies 0–3. BizHawk's osd.h sets MAX_INPUTS to eight. Added optional Team Player
and 4-Way Play topology fields, exact requested slot placement, and validation.
ControlType Teamplayer/Wayplay serialize as 4/5; runtime SYSTEM codes are 12/13.
4-Way Play requires both connectors and excludes Team Player. Loaded-device
validation now compares exact occupied indices. Editor support and J-Cart's
content-dependent extra pads remain unfinished, as does trusted runtime capture.

Step 372 connects Team Player and 4-Way Play to the editor, expands socket labels
and player capacity, and rejects conflicting adapter/connector choices before
recording. Pad-mode and connector edits preserve the adapter fields; retained
players are not silently removed or reassigned. No UI/runtime checks performed.

Step 373 adds per-player socket selection with connector/adapter labels, occupied
socket notices and explicit out-of-topology saved-player display. Selection edits
the same compacted virtual_port field; backend uniqueness/completeness checks
remain mandatory. No additional controller or runtime compatibility is claimed.

Step 374 includes dll/gpgx.wbx in shared-session runtime artifact capture and
rechecks, alongside the already guarded host artifacts. PathUtils in the pinned
source derives DllDirectoryPath from BIZHAWK_HOME or the executable directory;
conflicting effective BIZHAWK_HOME is rejected. The same guard covers turbo.wbx
and snes9x.wbx. This detects file changes, not semantic compatibility or actual
loaded input devices; wrapper behavior and runtime verification remain unproven.

Step 375 adds GPGXControlConverter's sixteen Activator channels (1L/1U through
8L/8U), with the genesis-activator channel-grid schematic and native SDL
translation. Each sensor must resolve to an independent physical identity and,
for recognized SDL controllers, an independent logical output. Opposite travel
on one axis is not treated as independent sensor capacity. Connector/config/
session and editor selection remain unfinished; physical ring orientation and
real Activator hardware behavior are not established by this grid.

Step 376 adds per-connector activators flags (absent defaults to false), exclusive
with adapters on the same connector. Native ControlType Activator is 3; runtime
system/device codes are 6/0x0a. Translation, encoding, saved calibration and
normalization target selection now resolve per player, allowing mixed Activator
and pad connectors. LoadedInputs validation checks the requested device type per
connector. Editor/preview selection and real runtime capture remain unfinished.

Step 377 connects per-port Activator editor controls and mixed-device per-player
preview selection. Invalid adapter topology blocks recording; physical and scoped
SDL previews surface independent-sensor mapping failures. These are saved-input
checks only, not a claim of physical Activator capture or loaded-game correctness.
