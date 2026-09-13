# Supermodel native Linux controller contract

Source oracle: `trzy/Supermodel` commit
`24d2ffcfc7f14229337f05f4920fe26b56633d9d` (inspected 2026-09-12), primarily
`Src/Inputs/InputSystem.cpp` and `Src/OSD/SDL/Main.cpp`. The platform record is
`emulator_details/records/supermodel.json`.

The new `controller_supermodel_native.rs` module is a source-backed config
overlay writer, not a universal Model 3 gamepad profile. It emits only action
names explicitly supplied by the caller and preserves unrelated INI content.
The accepted tokens are the upstream parser's `JOY<n>_BUTTON<m>`,
`JOY<n>_{X,Y,Z,RX,RY,RZ,S1,S2}AXIS[_POS|_NEG]`, and `JOY<n>_POV<m>_<direction>`
forms. Device numbering is one-based, button slots are 1–32, and POV slots are
1–4, matching the pinned source's lookup tables.

`Src/OSD/SDL/Main.cpp` resolves `Supermodel.ini` beneath the platform config
root and stores mappings as quoted comma-separated `Input*` values in
`[ Global ]`; `-config-inputs` is the upstream interactive editor. The module
does not add keyboard fallbacks, select a game-specific action family, or
assume that a fighting, racing, gun, fishing, ski, or sports title shares the
same controls.

Parent integration must provide a reviewed per-game/per-ROM-set action map,
the exact config baseline, and a native Linux SDL device probe/topology guard.
The launch overlay must isolate only the copied config while preserving the
declared `XDG_DATA_HOME/supermodel/{Saves,NVRAM}` roots. Supermodel save states
are `<game>.st<slot>` (up to ten slots) and NVRAM is `<game>.nv`; neither should
be redirected or mixed across game sets. Model 3 ROM components remain
game-specific (`epr`, `mpr`, `crom`, `vrom`, `srom`) and no universal BIOS
checksum is implied. There is no verified official Linux Flatpak package.

Runtime mapping and gameplay remain unverified.
