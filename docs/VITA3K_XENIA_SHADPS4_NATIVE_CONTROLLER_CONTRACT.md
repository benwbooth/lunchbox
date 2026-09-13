# Vita3K, Xenia, and shadPS4 native controller contract

The adapters are pinned to the upstream commits named in their Rust modules.

* Vita3K has a real native SDL3 configuration surface: `controller-binds` is
  the 15-entry vector consumed by `ctrl.cpp`, and `controller-axis-binds` is
  the six-entry axis vector. A staged YAML file may set `pref-path` to the
  existing Vita filesystem, preserving installed firmware and title saves.
* shadPS4 reads per-game input files from `<UserDir>/input_config/<GameId>.ini`.
  Its parser accepts one-based controller IDs on both output and input names;
  the adapter emits only this source-defined format. UserDir must remain the
  persistent root because it also contains savedata and system modules.
* Xenia has an SDL HID implementation and its pinned source exposes the
  authorable `SDL.mappings_file` cvar (default `gamecontrollerdb.txt`). The
  adapter writes exact SDL GameControllerDB records keyed by a measured
  32-hex-digit SDL GUID and requires the complete 20-field standard XInput
  surface; it does not invent Xenia-specific TOML keys. The
  official platform record still has no verified packaged Linux runtime/data
  root, so this is a serialization contract rather than a native Linux
  availability claim.

None of these contracts establish runtime compatibility by themselves. Exact
executable and SDL hashes, device identity, and a live launch capture remain
required before a caller can mark a setup launch-ready.
