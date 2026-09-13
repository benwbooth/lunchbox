# DOSBox-X native Linux controller contract

Source oracle: `joncampbell123/dosbox-x` commit
`532909c4e84160a5ac2185fbf9c4c97dbe07f85d` (inspected 2026-09-12), especially
`src/gui/mapper.cpp`, `src/misc/savestates.cpp`, and
`dosbox-x.reference.full.conf`. The captured platform record is
`emulator_details/records/dosbox-x.json`.

DOSBox-X provides a native mapper file selected by `[sdl] mapperfile` (and
SDL2-specific `mapperfile_sdl2`). `src/gui/mapper.cpp` loads and writes named
EVENT/BIND records for keyboard, joystick axes/buttons/hats, and optional
Mod1/Mod2/Mod3 combinations. The reference configuration also exposes
`joysticktype` and explicit joystick axis/dead-zone settings. This is a
machine/input event API, not a universal gamepad action schema.

The native module emits and patches the source mapper grammar while accepting
explicit caller-supplied EVENT names and BIND strings. DOSBox-X supports arbitrary DOS
programs and PC-98/other machine configurations whose controls are chosen by
the guest software. Without a reviewed per-game action vocabulary and exact
joystick topology, a generic host calibration cannot safely decide which
EVENT names to bind. A guessed mapper would appear configured while leaving
the actual guest controls unknown.

DOSBox-X does implement save states: `-savedir` selects the state directory,
`[dosbox] saveslot` covers slots 1–100, and `savefile` can override the slot
filename; `src/misc/savestates.cpp` writes the implementation-defined state
files. The per-game caller must therefore isolate only copied config and mapper
files, preserve the declared `-savedir`/mounted guest-save roots and
optional machine resources, and hash the exact executable, config, mapper,
probe, and content before launch. Native Linux and Flatpak
`com.dosbox_x.DOSBox-X` remain separate contracts.
