# DuckStation controller adapter: evidence and remaining work

Status: PlayStation digital and dual-stick analog gameplay contracts are
implemented in the calibration preview. **Automatic DuckStation
launch is not implemented or enabled.** This is not a substitute for the full
standalone launch adapter.

## Verified interfaces

The installed Flatpak on the development host reports
`0.1-9482-g0a53bc47c`, app ID `org.duckstation.DuckStation`. Its actual `-help`
output was inspected on 2026-09-05: no settings-file or input-profile CLI option
is advertised. The current upstream argument parser likewise has no such option.

The digital controller's fourteen setting names were checked against
[the installed revision's controller definition](https://github.com/stenzek/duckstation/blob/0a53bc47c/src/core/digital_controller.cpp).
The catalog stores these functional configuration keys, not copied implementation
or Xbox button numbers. The AnalogController preview separately records its 24
gameplay inputs from the [installed analog definition](https://github.com/stenzek/duckstation/blob/0a53bc47c/src/core/analog_controller.cpp),
including eight proportional stick directions. Analog-mode toggle and motor
routing are not assigned by this preview. Lightguns and specialty controllers
still need separate contracts.

[The installed SDL backend](https://github.com/stenzek/duckstation/blob/0a53bc47c/src/util/sdl_input_source.cpp)
distinguishes gamepad names, raw button/axis indices, hats, and player IDs. Raw
events consumed by a gamepad mapping are suppressed. Therefore a Linux joydev
button number alone cannot be written as a DuckStation binding. SDL player IDs
also need runtime resolution; a Linux `jsN` suffix is not an SDL player ID.

Current upstream was additionally inspected at
`f5ffb6b51f1baf0aefe0f2f32b8d12ab53339a67`:

- [Argument parsing](https://github.com/stenzek/duckstation/blob/f5ffb6b51f1baf0aefe0f2f32b8d12ab53339a67/src/duckstation-qt/qthost.cpp)
- [Data-root selection](https://github.com/stenzek/duckstation/blob/f5ffb6b51f1baf0aefe0f2f32b8d12ab53339a67/src/core/core.cpp)
- [SDL input handling](https://github.com/stenzek/duckstation/blob/f5ffb6b51f1baf0aefe0f2f32b8d12ab53339a67/src/util/sdl_input_source.cpp)

On Linux, current upstream checks portable configuration beside the executable
before XDG configuration-directory routing. Changing XDG_CONFIG_HOME is thus a
possible isolation mechanism, not proof of isolation for every installation.
The installed revision's [data-root implementation](https://github.com/stenzek/duckstation/blob/0a53bc47c/src/core/host.cpp)
also prioritizes portable files before XDG routing on Linux. Actual redirected
startup has now been verified for both a fresh test root and a private copy of
the user's settings with rebased data-folder paths (no game launched).
Flatpak overrides XDG paths at sandbox entry, so the override must happen inside
the sandbox, before executing DuckStation. The Rust startup oracle confirms the
logged root and unchanged original settings. Game-specific input interpretation
and production Play integration still need runtime verification.

The Rust `duckstation_config::LaunchConfig` stages global settings plus separate
copies of game settings, input profiles, and any data-root SDL mapping database.
It rebases all sixteen configured data folders, keeping BIOS, memory cards and
saves at their original destinations while isolating the two configuration
directories. Its owner retains the temporary directory until the child exits.
Source settings are never written. The line-preserving editor retains unrelated
repeated keys, comments and newline style; replacing a binding clears all of its
old list entries and repeated application is idempotent.

Input layer selection follows the [installed system implementation](https://github.com/stenzek/duckstation/blob/0a53bc47c/src/core/system.cpp):
exact disc-set identity, optional separate-disc settings, the per-game controller
flag, and named profiles. Missing separate-disc settings or missing profiles
fall back as the emulator does. The caller must supply verified game/disc-set
identity, not a guessed serial. There is no global type fallback for an active
game/profile input layer. A digital patch cannot change an AnalogController to
DigitalController. The analog gameplay writer likewise requires AnalogController;
it preserves analog-mode options, dead zones, sensitivity, toggle and motor
bindings. Those preserved controls are not yet automatically reassigned.

On 2026-09-05, `duckstation_startup --preserve-config` exercised the installed
Flatpak with the user's real global settings copied privately. Its Dev output
confirmed all sixteen folder destinations, and the three device-open records
matched a fresh runtime projection. Original configuration bytes were unchanged.
The global Pad1 type remained AnalogController. Per-game/profile selection and
digital patches have fixture tests, **not yet actual game-start verification**.
Other data-root-relative files and persistent emulator databases still require
an audit before enabling production isolation; a no-game startup does not prove
all user state is preserved through gameplay.

The pinned-source data-root audit found specific remaining state to preserve:
`playtime.dat` and `custom_properties.ini` in `core/game_list.cpp`, memory-scanner
files in `watches/`, and a relative `UI/GameListBackgroundPath`. The playtime file
uses in-place locked updates, whereas custom properties use atomic replacement
through `INISettingsInterface::Save`; simply symlinking both files is not a
write-preserving solution. Data-root logs, crash output, shader/CPU dumps and
updater staging also need explicit destination policies. Achievement databases
and game-list caches use the rebased Cache folder. The private-root helper does
not yet handle these remaining direct-root state paths, so it remains unsuitable
for production game launches. Source inspection covered `src/` at `0a53bc47c`,
not just the global settings definitions; no upstream implementation was copied.

## Implementation requirements

The [target-runtime SDL probe](../crates/lunchbox-controller-probe/README.md) now
provides the enumeration stage of requirement 2. It successfully queried SDL
3.2.20 inside the installed DuckStation Flatpak with the packaged mapping DB and
the launcher's `SDL_JOYSTICK_LINUX_CLASSIC=1` setting. Distinct joystick paths
identified the generic pads despite identical names/GUIDs. Additional mouse/LED/
virtual-pointer joystick interfaces were also visible. Reported player-index
hints must not be mistaken for DuckStation's final fallback assignments.
**Correction:** those earlier six-device inventories omitted the target runtime's
libudev dependency; they do not match the real emulator's filtered device set.
See the actual startup verification below before using any player projection.

The probe can now optionally open explicitly selected gamepads to copy SDL's
resolved bindings, including input/output axis ranges and hats. Default
inventory queries still do not open devices. It never writes emulator settings.
The Rust digital-input translator uses these bindings to produce DuckStation
binding suffixes. For SDL 3.2.20's Linux classic backend, physical evdev-code
lookup is also implemented from the kernel's read-only joystick maps, verified
against SDL control counts. Hat axes are removed from the axis sequence and hats
retain first-seen order. Other physical backends, final launch-time player
assignment, full user-state isolation, and launch integration remain.

The Linux calibration wizard now records physical axis rest/peak values plus
kernel bounds and flat/fuzz/resolution. It samples through release and waits for
the raw value to settle, then persists the measurement with the binding and
corrects its raw direction. This supplies evidence needed for the range
translation above; it does not itself normalize those values to SDL or enable
DuckStation launch. Old calibrations remain readable but do not acquire invented
measurements. The numbering probe also reports read-only evdev axis metadata for
runtime comparison with saved measurements.

The opt-in event-order player probe now keeps all opened devices alive and
projects the verified revision's collision/fallback rules. Its matching-runtime
mode requires explicit target libudev (and libcap dependency), rejecting an
unverified loader fallback. Snapshot schema 3 added hashes for those libraries.
The Rust `duckstation_startup` oracle verifies the actual installed executable's
verbose device-open records, not merely the helper's own calculations. On
2026-09-05 it confirmed three gamepads with the two selected generic pads in
DuckStation slots **1 and 2**. Both opened names are `Xbox 360 Controller`, so
names alone still cannot identify them. Log validation includes event order,
instance IDs and slots; paths come from the matching helper observation.
Live hotplug between processes and production launch-time confirmation remain.

Schema 4 additionally records the actual kernel joystick correction for every
physical axis, checked before and after SDL device opens. The new
`physical_digital_binding` conversion composes measured evdev rest/pressed values
through these corrections and the resolved SDL mapping. Numbering alone was
insufficient: the [Linux joystick ABI](https://github.com/torvalds/linux/blob/v6.12/include/uapi/linux/joystick.h)
exposes calibration used by the [kernel value converter](https://github.com/torvalds/linux/blob/v6.12/drivers/input/joydev.c),
and [SDL 3.2.20 classic input](https://github.com/libsdl-org/SDL/blob/release-3.2.20/src/joystick/linux/SDL_sysjoystick.c)
forwards those already-corrected values. No calibration ioctl writes are used.
Corrections with unsupported arithmetic or type are rejected rather than guessed.
Old snapshots without corrections remain valid for button numbering, not axis
conversion. The actual kernel oracle matched all eight axes on both selected
USB pads on kernel 7.2.2, including physical trigger rest 0 -> joystick -32767.
Current physical-bounds validation and end-to-end launch application remain
unfinished.

The proportional translator now composes measured axis endpoints with kernel
correction and SDL axis mappings. It checks the complete movement interval, not
just the endpoint: preceding digital ranges, shared outputs and non-neutral
release boundaries cannot silently turn a stick into an on/off control. Separate
positive/negative mappings of the same physical axis are supported when they
retain independent output halves and a neutral release. Unconsumed raw axes use
the verified revision's suppression rules. Buttons and hats cannot become analog
inputs. This has deterministic regression tests, not actual analog gameplay
verification; it does not enable automatic DuckStation launch.

The installed `0a53bc47c` SDL backend marks raw axis events consumed using the
**output** axis index, unlike its button/hat handling, which uses the input index.
The translator models this version-specific behavior explicitly. Do not silently
"correct" it or apply it to other revisions without verification. Whole hats
are suppressed once any direction is mapped. Shared normalized outputs cannot
stand in for independent physical controls. SDL's own binding order, inverted
ranges, button-driven axes and axis-driven buttons also affect emitted events.

Live verification on 2026-09-05 queried both selected generic pads inside the
installed Flatpak: each returned 21 resolved bindings, 11 buttons, six axes, and
one hat. The read-only physical maps matched those counts and retained distinct
device paths. Missing-device and non-gamepad selections failed as expected.
This verifies enumeration and numbering, not button presses in a running game.

1. Resolve the exact launched executable/version, data root, game settings,
   selected input profile, and controller device type. Do not silently replace
   an analog controller with a digital controller to make a calibration fit.
2. Resolve SDL device identity/player IDs and physical-to-SDL bindings using the
   launched runtime's backend and mapping state. Handle mapped gamepad events,
   unconsumed raw inputs, signed axes, and hats separately. The existing Linux
   calibration contains physical evdev codes; no new button-label assumptions
   should be introduced. Host SDL and a Flatpak's SDL are not interchangeable
   without verification.
3. Create a private effective configuration for the launch. Preserve BIOS, memory
   cards, saves, and non-input settings, including relative paths and per-game
   overrides. Scope Flatpak access to required paths for that process only.
   Never edit the user's controller files as a shortcut.
4. Verify configuration isolation and binding interpretation with the installed
   emulator before opting the contract into automatic launch. Keep temporary
   configuration alive for the complete child lifetime and cover failures/exits.
5. Test actual inputs in digital and analog games with multiple connected pads,
   including device reordering, duplicated reported model names, and reconnects.

## Physical layout conversion

The digital PlayStation layout has four face controls, four independent shoulder
controls, D-pad, Select and Start. Brawler64's B/A pair maps to Square/Cross;
C-left/C-down supplies Triangle/Circle. Z/Z-right supplies L2/R2 when independently
reported and calibrated. A controller with one shared Z signal cannot provide
two independent shoulder inputs. Missing controls remain visible in the preview.

Mapping rows now include stable target/physical IDs and the output transport.
Display-label changes must not change capability checks or configuration identity.
