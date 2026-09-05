# Controller catalog

`catalog.json` is Lunchbox's versioned controller-layout and emulator-contract
database. Layouts contain stable semantic control IDs, approximate front-view
positions, control groups, and analog/digital requirements. Calibration records
map those IDs to the user's observed device inputs; they are private settings,
not distributed hardware assumptions.

The SVG renderer is original Lunchbox code. No vendor artwork or third-party
SVG is redistributed. Export reproducible diagrams with:

    cargo run -p lunchbox-app -- --export-controller-svgs /chosen/output/directory

The exporter only updates the named layout SVGs whose generated content changed;
it does not delete other files. Adding a layout automatically adds its diagram.
These are schematics, not exact shell measurements. Brawler64 diagrams cover
the common gameplay controls; auxiliary controls vary by edition.

## Provenance and rights

The catalog is original structured data describing control identities and
functional mappings. It does not reproduce source prose, illustrations, or
software implementations. The catalog and generated schematics use the project's
license. External source URLs document the facts; they do not grant rights to
reuse upstream artwork. Any future imported asset needs an explicit per-asset
license/provenance record before it can be committed.

The initial RetroArch contracts were checked against the official FCEUmm,
Genesis Plus GX, and Mupen64Plus documentation on 2026-09-04 (URLs in each entry).
They are unversioned documentation observations, not runtime-verified emulator
adapters. Controls/options can vary by core version and controller mode.

## Verification levels and rollout

- Calibration and diagram support: implemented for the listed layouts.
- Mapping composition: deterministic global face-button assignment for every
  catalog layout pair, with reviewed ergonomic preferences and capability checks.
  Missing analog sticks/buttons remain missing. This is optimal under the stated
  cost policy, not a claim of universal ergonomic optimality.
- Linux RetroArch launch writer: native and Flatpak, FCEUmm/NES, Gambatte/GB,
  Genesis Plus GX/MD six-button, Mupen64Plus-Next/N64 independent C-button mode.
  Per-launch configs use kernel-reported joystick indices, not Xbox numbering.
  Saved calibrations from the preview-only prototype need one recalibration to
  capture physical evdev bindings, including synthesized D-pad inputs.
- The old Mupen64Plus right-stick contract and MD three-button contract remain
  preview-only. Other cores, standalone emulators and Windows/macOS still need
  verified configuration adapters. Writer availability is not hardware/game
  end-to-end verification.
- Standalone emulators need their own input-backend, mode, version, configuration
  writer, isolation/restore, and runtime test evidence. Never inherit an Xbox
  default just because another emulator uses it.

The initial catalog does not cover every system or controller variant. Extend it
with reviewed layouts and contracts, then verify each adapter against the actual
emulator before enabling automatic launch. Lightguns, keypads, wheels, motion
controls, pressure sensitivity, and nonstandard analog devices need explicit
capabilities rather than being squeezed into a standard joypad profile.

Raw GilRs event codes are OS-specific. Calibrations record their OS/backend and
must not be silently reused on a different input backend. Linux row identification
uses the exact evdev source; portable model IDs with multiple matches are refused.

## Physical layout versus hardware behavior

`horizontal-four` describes independent X/Y buttons above a horizontal B/A pair;
it is not a diamond pad. `n30-turbo` describes the N30 variants whose upper buttons
repeat lower A/B inputs (see the 8BitDo FAQ recorded in the entry). Do not infer
which mode a device exposes from its marketing name. The wizard excludes known
hardware repeat controls from independent calibration. PC Engine two-button and
NES/GB mappings use the lower pair; independent four-button targets can use all
four on a device that actually reports them. Hardware turbo is never treated as
an extra independent face input.

## Launch adapter evidence and limitations

Functional driver/configuration facts checked against RetroArch's
[linuxraw driver](https://github.com/libretro/RetroArch/blob/master/input/drivers_joypad/linuxraw_joypad.c),
[configuration reader](https://github.com/libretro/RetroArch/blob/master/configuration.c),
Linux `joystick.h` read-only ioctl ABI, and GilRs 0.11.2/0.6.8 Linux event-code
and normalization implementations. Core-specific functional mappings are linked
in the catalog; Mupen64Plus-Next's independent mode is verified against its input
implementation, not assumed from generic RetroPad labels.

The initial adapter suspends RetroArch automatic config overrides/remaps for a
session, and N64 uses a copy of global core options with independent C-buttons
enabled (per-core/per-game options are suspended). Custom N64 config/include
chains are rejected until their effective options can be resolved. No existing
config file is overwritten. Temporary files are deleted when the launched child
exits, including spawn/error/cancel paths. A frontend crash can leave a private
cache directory but cannot persist these mappings into RetroArch's base config.

Only compatible, calibrated, connected devices enter the player list; explicit
ordering and preferred-device settings rank those devices. The legacy InputPlumber
launch adapter is not stacked on the calibrated RetroArch adapter. Existing
externally configured Steam Input/InputPlumber routing is not modified.

Read actual Linux device numbering without changing devices:

    cargo run -p lunchbox-app -- --controller-numbering-probe

Remaining work includes broader core/mode coverage, preserving effective per-game
overrides while replacing only input settings, backend/version fingerprints,
runtime input verification, hotplug during a game, analog-axis calibration
corrections, and standalone/Windows/macOS adapters. The all-emulator goal remains
incomplete.
