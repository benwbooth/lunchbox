# Devector standalone controller boundary

The adapter intentionally refuses to serialize a controller profile.

## Verified source contract

Pinned source: `parallelno/Devector@ba0790a4a4a2b7300d6eb4869d11caf2b7047122`.
The Vector-06C emulator accepts a settings JSON path and ROM/FDD/REC paths.
`resources/settings.json` contains UI, boot, RAM-disk, and recent-file
settings, but no controller mapping schema. `core/keyboard.cpp` has a
hard-coded SDL scancode table. The frontend initializes SDL gamepad support
for ImGui navigation only; no gamepad/joystick events are translated into the
emulated machine's input and no persistent host-device selector exists.

## Refusal boundary

The adapter does not invent a mapping file, SDL slot identity, or joystick
binding syntax. Devector's verified persistent settings are not controller
settings. This boundary does not claim ROM/FDD/REC completeness, recording
round trips, runtime gameplay input, or platform parity.
