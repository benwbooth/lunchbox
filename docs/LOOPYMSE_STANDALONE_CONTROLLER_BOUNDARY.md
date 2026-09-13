# LoopyMSE standalone controller boundary

The adapter intentionally refuses to serialize a controller profile.

## Verified source contract

Pinned upstream: `PSI-Rockin/LoopyMSE@f6ad0bb9e40eafb0907bcd74223ad5b17c7f66cd`.
The README documents command-line launch as `<game ROM> <BIOS> [sound BIOS]`
and a `.sav` sidecar beside the game ROM. `src/sdl/main.cpp` translates the
fixed keyboard bindings Z/X/A/S/Q/W, the arrow keys, and Enter. The input
module has no configuration-file writer, profile schema, or host joystick
identity selector.

## Refusal boundary

The adapter does not invent an SDL slot, gamepad mapping file, or device GUID.
The documented `.sav` is a game save sidecar, not a controller profile. This
boundary does not claim save-state, BIOS checksum, runtime gameplay, or
cross-platform completeness.
