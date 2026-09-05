# DuckStation controller adapter: evidence and remaining work

Status: the original PlayStation digital layout and DuckStation DigitalController
contract are implemented in the calibration preview. **Automatic DuckStation
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
or Xbox button numbers. AnalogController, lightguns, and specialty controllers
need separate contracts.

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
startup has now been verified for a **fresh test root** (not a production overlay).
Flatpak overrides XDG paths at sandbox entry, so the override must happen inside
the sandbox, before executing DuckStation. The Rust startup oracle confirms the
logged root and unchanged original settings. Preservation of user data and
effective per-game/profile configuration still needs implementation/verification.

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
retain first-seen order. Other physical backends, axis rest/range interpretation,
analog target conversion, final player assignment, isolated configuration, and
launch integration remain.

The opt-in event-order player probe now keeps all opened devices alive and
projects the verified revision's collision/fallback rules. Its matching-runtime
mode requires explicit target libudev (and libcap dependency), rejecting an
unverified loader fallback. Snapshot schema 3 records hashes for those libraries.
The Rust `duckstation_startup` oracle verifies the actual installed executable's
verbose device-open records, not merely the helper's own calculations. On
2026-09-05 it confirmed three gamepads with the two selected generic pads in
DuckStation slots **1 and 2**. Both opened names are `Xbox 360 Controller`, so
names alone still cannot identify them. Log validation includes event order,
instance IDs and slots; paths come from the matching helper observation.
Live hotplug between processes and production launch-time confirmation remain.

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
