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
- Mapping composition: preview only, with missing controls and assumptions shown.
- Emulator configuration writers and automatic calibrated launch: not implemented
  yet. A documented contract must not be represented as launch-ready.
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
