# DuckStation, melonDS, and standalone Flycast controller contract

This is a source-backed boundary for the three native adapters. It records
what Lunchbox may serialize and what still requires a real launch-time check.

## Pinned upstream contracts

- DuckStation is pinned to `stenzek/duckstation@ed6720bbf59011b276faafb098ecc1c2407e74a3`.
  Controller sections are `[Pad1]` through `[Pad8]`; bindings use the native
  `SDL-<player>/ButtonN`, signed half-axis, and full-axis spellings. The
  adapter preserves measured bipolar stick endpoints and rejects ambiguous
  serials, duplicate pads, and unsafe paths.
- melonDS is pinned to `melonDS-emu/melonDS@906e9ebb27da8c6a715cd7abab4abfe8a8d29427`.
  The native joystick encoding distinguishes button indices, hat cardinal
  directions, and raw axis activations. DS saves default beside the ROM as
  `<basename>.sav`; states use `<basename>.ml1` through `.ml8`; configured
  firmware paths remain explicit (`bios9.bin`, `bios7.bin`, `firmware.bin`,
  and DSi paths where applicable).
- Standalone Flycast uses the recorded core source pin
  `flyinghead/flycast@eddf2635867f0f16f64bebd3185db151c99c551c` for roots and
  port persistence. Its mapping writer is checked against the pinned mapping
  implementation `fb286f777ce690ef8acf3359a75ab84b61566ad9`; Lunchbox emits
  version 3, separates digital and analog binds, and records resolved trigger
  polarity rather than relying on Flycast's SDL auto-discovery.

## Root preservation

Native user roots are staged privately for configuration. DuckStation keeps
its configured memory-card, save-state, and BIOS directories; melonDS keeps
ROM-adjacent saves/states unless its TOML overrides redirect them; Flycast
keeps the XDG config/data split, `dc_boot.bin`/`dc_nvmem.bin`, VMU files, and
state directory semantics. No adapter treats a staged config as proof that
firmware exists or that a save/state was successfully written.

## Verification boundary

The focused Rust tests cover serialization invariants and reject malformed
topologies. They do not prove SDL discovery, device identity, firmware
availability, native process startup, effective configuration, gameplay input,
or save/state round trips. Those claims require launch-time logs and emulator
artifacts from the actual runtime.
