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
startup and effective settings still need verification before enabling this path.

## Implementation requirements

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
