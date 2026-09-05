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
  Genesis Plus GX/MD six-button, Mupen64Plus-Next/N64 independent C-button mode,
  Beetle PCE Fast/PC Engine two-button mode (maximum five players), and
  Snes9x/bsnes standard SNES joypads (two ports).
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

The PlayStation digital and dual-stick analog gameplay contracts are available
for preview using `duckstation-settings`. This transport contains DuckStation
setting keys rather than RetroPad outputs; it cannot opt into the RetroArch writer. DuckStation automatic
launch remains unavailable pending verified SDL identity/binding resolution and
configuration isolation. See `docs/DUCKSTATION_CONTROLLERS.md` for pinned evidence
and the remaining adapter requirements. Mapping rows expose stable control IDs
separately from display labels.

The analog preview requires proportional physical controls for its eight stick
directions. Horizontal pads and N64 C-buttons cannot supply a missing analog
stick, regardless of whether their drivers encode buttons as axis events. The
24 gameplay keys do not include analog-mode toggle or motor routing; the private
configuration writer preserves those existing bindings and controller options.

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

### SwanStation mode-aware adapter (in progress)

The catalog distinguishes digital device 1 from DualShock device 261 using
SwanStation source revision `7f69c199ed88d5723f71dd3a6e9c1b7a45b535a6`.
These IDs are core-specific: Beetle PSX uses a different DualShock ID.
The adapter composes each port independently from its configured mode and a
compatible calibrated physical layout; missing analog controls never silently
change the emulated controller to digital. Command-line device overrides take
precedence over the base configuration. Saved override/remap files and include
chains still need effective-mode resolution and currently produce an error.

The Flatpak BIOS-only runtime probe loads SwanStation and boots the BIOS. The
initial null-display probe timed out; an OpenGL display completes the eight-frame
limit successfully for both digital and DualShock configurations (2026-09-05,
RetroArch 1.22.2; core SHA256
`b0dcc6c4f939fe814693a59d03895ab8a8ba8d3c2cc1e84bb79c14d33de9899e`).
This is startup evidence, not physical controller gameplay
verification. The probe uses private configuration/save/history paths and
explicitly supplied existing BIOS files, without downloading game content.
Runtime observation also showed that RetroArch can save a per-core `.opt` under
`rgui_config_directory` despite a private `core_options_path`; both the production
options writer and the probe now redirect that additional path into the session.

Functional driver/configuration facts checked against RetroArch's
[linuxraw driver](https://github.com/libretro/RetroArch/blob/master/input/drivers_joypad/linuxraw_joypad.c),
[configuration reader](https://github.com/libretro/RetroArch/blob/master/configuration.c),
Linux `joystick.h` read-only ioctl ABI, and GilRs 0.11.2/0.6.8 Linux event-code
and normalization implementations. Core-specific functional mappings are linked
in the catalog; Mupen64Plus-Next's independent mode is verified against its input
implementation, not assumed from generic RetroPad labels.

Beetle PCE Fast's I/II input descriptors, input polling table, and five initial
joypad-mode options were checked against upstream commit
`95fbd9c51bf0e35aa376a7774e9c02ff927cd991` on 2026-09-05 (`libretro.c` and
`libretro_core_options.h`). Its two-button contract leaves mode-switch and
software-turbo hotkeys unbound; six-button games require another profile.

The initial adapter suspends RetroArch automatic config overrides/remaps for a
session. N64 and PCE use a copy of global core options with independent C-buttons
or two-button mode selected (per-core/per-game options are suspended). Custom N64/PCE config/include
chains are rejected until their effective options can be resolved. No existing
config file is overwritten. Temporary files are deleted when the launched child
exits, including spawn/error/cancel paths. A frontend crash can leave a private
cache directory but cannot persist these mappings into RetroArch's base config.

Snes9x's input descriptors, `BTN_*` definitions, polling, and joypad device selection
were checked at `7a8878f1306f65594c30b7d86dee41d972c2e495` in
`libretro/libretro.cpp`. bsnes's `joypad_mapping`, input polling and device selection
were checked at `63ee22f7af8f5d1ada295c2c98fd7bf261fa62a7` in
`bsnes/target-libretro/{program,libretro}.cpp` on 2026-09-05. The standard two-port
joypad mode is covered; multitap, mouse, lightgun, Super Game Boy and other core
variants require explicit separate contracts. These are source/configuration
checks, not physical-controller gameplay verification.

`retroarch_launch` explicitly opts a documented contract into the Linux writer.
It records exact `platforms` aliases (case-insensitive, trimmed caller input),
the libretro joypad `device` ID, and the input mode's `max_players`. Validation
rejects missing aliases, non-joypad modes, invalid port counts, and overlapping
core/platform defaults. Names are not matched by substring. Omission means
preview-only, regardless of a contract's documentation status. The settings
coverage text is generated from these entries instead of a second hard-coded list.

Only compatible, calibrated, connected devices enter the player list; explicit
ordering and preferred-device settings rank those devices. Excess devices are
left out after applying that order, up to the mode's declared port count.
Current limits are GB 1, standard NES/MD/SNES 2, N64 4, and PCE Fast 5.
Multitap modes require a separate contract. The legacy InputPlumber
launch adapter is not stacked on the calibrated RetroArch adapter. Existing
externally configured Steam Input/InputPlumber routing is not modified.

Read actual Linux device numbering without changing devices:

    cargo run -p lunchbox-app -- --controller-numbering-probe

Remaining work includes broader core/mode coverage, preserving effective per-game
overrides while replacing only input settings, backend/version fingerprints,
runtime input verification, hotplug during a game, analog-axis calibration
corrections, and standalone/Windows/macOS adapters. The all-emulator goal remains
incomplete.
