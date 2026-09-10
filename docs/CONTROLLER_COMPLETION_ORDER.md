# Controller completion order

Latest source checkpoint: [guided integration](CONTROLLER_GUIDED_INTEGRATION.md).
This supersedes historical counts below: 93/94 RetroArch core contracts and
15/249 partial standalone dispatches, or 108/343 source entries (31.5%), not
overall completion. Eight existing native adapters consume guided choices,
with runtime setup/backend restrictions. Testing is deferred again by request.

## Current scope override — 2026-09-08

The user's latest direction supersedes the historical peripheral queue below:
arcade mapping means standard six/eight-button fighting-style panels, Start and
Coin. Do not hold arcade completion for wheels, guns, keyboards, twin sticks,
individual cabinet layouts or exhaustive per-game inspection.

MAME, FBNeo and Flycast now have standard six/eight-button catalog profiles using
the existing visual layouts. Six is the default, eight is selectable. Ordinary
MAME launch no longer requires native per-game inspection; saved advanced setups
retain precedence. MAME's disabled default still preserves native configuration.
FBNeo selects its six-button-panel device; Flycast retains its native routing
(Atomiswave ignores buttons beyond five). Extra controls need not exist in a game.

Source implementation inventory: 93/94 RetroArch core names have launch profiles
(98.9%), up from 91/94. This is profile presence, not tested compatibility or
all-mode completion. Steem SSE remains a Windows-runtime exception, not a reason
to delay standalone work. No tests, builds or native probes have been run.

The standalone pass has started: coverage now inventories native candidates even
when they also advertise libretro cores. Existing BizHawk native dispatch is
reported as partial, rather than absent. Its digital decks include Snes9x,
NesHawk, SMSHawk, PceHawk, TurboNyma and GPGX; PlayStation modes use the separate
native path. DuckStation, PPSSPP and mGBA's SDL frontend also have partial native
Linux launch dispatch for saved calibrated setups. Dolphin 2606 now also has
partial native Linux GameCube ISO/GCM dispatch for saved evdev setups. Its system
data path is user-declared and not runtime-confirmed. Snes9x GTK 1.63 now also
has partial native Linux dispatch for saved SNES controller setups. FCEUX Qt 2.6.6
also has saved NES pad dispatch; ROM-selected device overrides remain unresolved.
SameBoy SDL v1.0.3 now has saved device-zero Game Boy dispatch, with user-declared
runtime ABI/data paths; axis-based mappings reject MBC7 tilt cartridges.
Mednafen now has saved native Linux GB/GBA joydev dispatch with private config
layers; child internal IDs and additional native input drivers remain unverified.
Standalone MAME now also has partial native Linux raw-SDL panel dispatch for
explicit saved setups with private controller and cfg files. It is untested.
Flycast now also has partial native Linux standard-panel dispatch.
PCSX2 now also has partial native Linux DualShock2 dispatch.
RPCS3 now also has partial Linux standard-pad file-boot dispatch.
melonDS now also has partial Linux standard-button dispatch.
That is 14/249 standalone candidates (5.6%), not complete platform/mode support
or tested compatibility.
Installation, host compatibility and per-platform readiness are not inferred.
Update 2026-09-10: Hatari standalone Atari ST joystick dispatch was added
(`HATARI_NATIVE_CONTROLLER_CONTRACT.md`), bringing the native adapter list to
nineteen, after VICE standalone Commodore joystick dispatch
(`VICE_NATIVE_CONTROLLER_CONTRACT.md`) brought it to eighteen, after Stella 7.x standalone Atari 2600 dispatch
(`STELLA_NATIVE_CONTROLLER_CONTRACT.md`) brought it to seventeen. Update 2026-09-09: bsnes v115 standalone SNES gamepad dispatch was added
(`BSNES_CONTROLLER_CONTRACT.md`), bringing the native adapter list to sixteen.
The probe also gained a generic read-only `--evdev-catalog` capability dump.
No runtime verification was performed.

Next: continue ordinary standalone gamepad configuration beyond these nineteen;
do not resume the arcade-peripheral audit.

The notes below are historical and do not reopen the excluded peripheral scope.

User direction, 2026-09-08: finish RetroArch cores before starting standalone
emulators. Testing remains deferred. Wheels remain paused. No new absolute-device
infrastructure work unless it directly closes a listed RetroArch contract.

## RetroArch queue

| Core | Current source evidence | Required closure |
| --- | --- | --- |
| MAME | Dynamic per-game inspection, ordinary arcade presets, saved setup and launch dispatch | Audit the complete ordinary joystick/button path and visual mapping; identify actionable unsupported field/channel cases. Keep peripheral limits separate. Do not manufacture a static profile to count this adapter. |
| FBNeo | Dynamic target inspection, saved assignments, relative input composition and launch dispatch | Audit ordinary gamepad mapping and all supported callback parts through UI, persistence and launch; resolve concrete missing parts and retain explicit unsupported peripheral diagnostics. |
| Steem SSE | Joystick contract and firmware-directory guard; Linux execution explicitly rejected | Actual compatible runtime and transport are prerequisites. The pinned wrapper is Windows-only; enabling a Linux profile without a port or Windows-runtime adapter is invalid. |

The 91/95 historical figure mixed static profile presence with a denominator
containing BizHawk's Nymashock. It was not a RetroArch completion percentage.
The coverage report now excludes BizHawk-only relationships from RetroArch
denominators and lists dynamic adapters separately. This correction completes no
additional input contract. Current totals must come from the actual database.

Read-only inventory refresh (2026-09-08): querying `build/lunchbox.db` and the
current controller catalog with the application's eight core-name aliases gives
95 database core names, 94 non-BizHawk-only core names, and 91 with static
`retroarch_launch` profiles: **96.8% static core-entry coverage**. The three names
without static profiles are `mame`, `fbneo`, and `steemsse`; the first two have
dynamic per-game adapters, not static profiles. The catalog contains 294 profiles
and 157 layouts. These are inventory counts, not runtime checks, all-mode coverage
or overall completion. No application, core or input device was executed.

### FBNeo ordinary-gamepad source audit (2026-09-08)

The following links were inspected in source; none was executed:

- `controller_fbneo.rs::mapping_targets_from_records` unions descriptors with
  observed queries without inventing action labels. `binding_parts` and
  `retroarch_field` cover the 16 joypad channels, bipolar analog halves and
  analog-button pressure; pointer and keyboard contracts stay distinct.
- `controller_launch.rs::analyze_fbneo_bindings` checks exact target ownership,
  analog pairing, channel collisions and shared-address/pressure aliases.
  `validate_saved_fbneo_assignments` rejects unresolved parts and external targets.
- `ControllerFbneoSetupDialog.qml` connects visual mapping to assignment editing
  and `save_fbneo_controller_setup`. The settings model validates all gamepad
  ports before replacing the staged setup; it does not substitute a generic pad.
- The saved FBNeo launch branch performs fresh inspection, validates the saved
  contract through `bind_saved_relative_sources`, resolves exact controllers,
  prepares each player and attaches the launch session. Relative-only ports use
  their separate prepared-source path.

No missing ordinary-gamepad wiring was identified in this audit. This is not
proof of correct runtime behavior, all games, or complete core coverage. Physical
gun/keyboard launch integration, unsupported callback contracts and verification
remain. The coverage UI's stale claim that relative routing was unimplemented
has been corrected; that correction adds no new mapping support.

### MAME ordinary-arcade source audit (2026-09-08)

- `controller_mame/arcade.rs` builds six/eight-button and Neo Geo profiles from
  active fields, packs sparse channels without changing native wires, rejects
  insufficient preset capacity and keeps twin-stick geometry distinct.
- `controller_mame/digital.rs::combined_profile` resolves explicit routes and
  validates their required outputs against generated geometry; analog pressure
  ownership is retained rather than silently duplicated into button positions.
- `ControllerMameSetupDialog.qml` connects source/destination views, preset and
  per-player overrides to review and save. `save_mame_controller_setup` rejects
  unresolved native fields and missing physical measurements before staging.
- `controller_launch.rs::prepare_with_cancellation` selects exact saved setups,
  verifies content/core identities and performs fresh native inspection. Without
  a saved setup it dispatches the explicitly configured arcade default to
  `controller_launch/mame_automatic.rs`.
- Automatic preparation handles named archives, saved native configuration,
  persistence and optional same-core sibling dependency discovery. Unsupported
  fields remain errors rather than being silently discarded.

Both ordinary mapping paths have now been traced through source. No execution
or complete game/mode inventory was performed, so neither core is declared fully
complete. Next substantive RetroArch work must close concrete peripheral/runtime
contracts, not recreate these ordinary paths or add more coverage bookkeeping.

### RetroArch keyboard integration checkpoint (2026-09-08)

FBNeo's explicit keyboard-channel plan is now connected to saved setup validation,
the physical assignment editor, calibrated player preparation and private launch
remap staging. Native descriptors/queries remain unchanged. The launch retains
the remap hash, uses the fresh core library identity for its private fallback path,
disables controller-name remap sorting and clears keyboard hotkeys only for
keyboard-mapped sessions. Ordinary sessions continue to disable remap loading.

This closes one save-to-launch wiring gap, not a whole-core coverage row. The
picker exposes 16 button channels; advanced assignments can use the remaining
eight analog directions as measured opposite pairs. More than 24 required keys,
independent simultaneous keyboards and unresolved absolute-pointer inputs remain
unsupported. Steem SSE's compatible-runtime prerequisite also remains open.
Standalone work has not started in this checkpoint. Overall completion percentage
remains unknown; the historical 91/95 is not a valid all-input denominator.

Rust formatting and `git diff --check` passed. No tests, builds, UI runs, native
probes or input-device operations were performed. Changes remain unreviewed and
uncommitted under the deferred verification policy.

### Native keyboard follow-up (2026-09-08)

Full keyboards no longer have to fit entirely into the gamepad mapper. FBNeo now
has explicit native frontend keyboard passthrough, confirmed in the editor and
connected to save/launch. The implementation checks the actual Linux key table:
unsupported punctuation stays required and can be composed with explicit mapped
keys. Fully supplied non-gamepad ports skip physical gamepad preparation, without
changing native inspection evidence. Multiple exclusive keyboards remain outside
this shared frontend state contract. Absolute-pointer inputs and the Steem SSE
runtime prerequisite remain open. No tests or runtime checks have been run.

## Then standalone

Latest RetroArch checkpoint: calibrated FBNeo Arcade Gun aim now has saved-device
selection, validation and owned virtual-gamepad launch composition, including
mixed buttons and aim-only ports. This is source implementation only. Other
absolute pointer/touch contracts and hardware offscreen/reload semantics remain
open; Steem SSE still needs its compatible-runtime decision. No whole-core count
or overall completion percentage is established by the new route.

Audit the already-implemented native BizHawk adapters (including Nymashock) and
other standalone runtime entries before describing them as absent or complete.
The previous hardcoded zero standalone-adapter count is not an implementation
inventory. Track emulator, modes, host/runtime restrictions, visual mapping,
persistence and launch independently.

## Acceptance and reporting

Report specific input contracts closed, unresolved runtime prerequisites, and
source-only versus verified status. Do not use static-profile percentage as
overall progress. No tests/builds/runtime execution or commits until the deferred
verification restriction and repository pre-commit gates are reconciled.
