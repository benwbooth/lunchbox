# 86Box native Linux controller contract

Source oracle: `86Box/86Box` commit
`189d9d003ad9670853cec6edac8db7d6ff63550f` (inspected 2026-09-13), especially
`src/config.c`, `src/game/joystick_standard.c`,
`src/include/86box/gameport.h`, `src/qt/sdl_joystick.c`, and `src/86box.c`.
The captured platform record is `emulator_details/records/86box.json`.

86Box does have a real joystick API, but it is a machine-level SDL joystick
contract rather than a stable gamepad semantic contract. `src/config.c`
loads per-machine `86box.cfg` `[Input devices]` values, including
`joystick_type`, `joystick_<n>_nr`, `joystick_<n>_axis_<n>`,
`joystick_<n>_button_<n>`, and `joystick_<n>_pov_<n>`; POV values are two
integers (`x, y`). The selected
machine's joystick type determines the number and meaning of those slots.
`src/86box.c` accepts `-C`/`--config` for an explicit config file.

The implemented slice is deliberately narrower: native Linux, an explicitly
trusted SDL2 build, and the source-defined `2axis_2button` topology. That type
has two emulated axes, two buttons, no emulated POV, and at most two joystick
slots. Each selected physical direction must resolve to opposite halves of one
centered raw SDL axis or to the matching cardinal directions of one raw SDL hat;
hat indices are encoded using upstream's `POV_X`/`POV_Y` flags. A and B must be
distinct raw SDL buttons. The writer rejects indices outside the fixed upstream
arrays: 8 platform joysticks, 4 emulated slots, 16 axes, 32 buttons, and 4 POVs.

The saved setup treats the selected catalog content as that machine's exact
absolute `86box.cfg`. Preparation copies the file, replaces only
`joystick_type` and `joystick_<n>_*` keys in its unique `[Input devices]`
section, and keeps every unrelated setting. A Bubblewrap bind overlays the
private writable copy at the original config path, then starts the exact hashed
executable as `86Box -C <original-config-path>`. Keeping the path unchanged
preserves relative machine, ROM, and disk references while preventing 86Box
from persisting controller edits into the user's real config.

The retained launch session rejects virtual, missing, duplicate, or ambiguous
controllers and more than eight enumerated SDL devices. It hashes the real and
private configs, executable, probe, exact SDL2 library, and Bubblewrap binary;
captures kernel topology; and reopens every selected device immediately before
spawn to compare SDL order, paths, mappings, raw control counts, and classic or
evdev numbering. Guided setup and the JSON review surface consume the same
one/two-player profile. Review itself opens no devices.

This is still a partial adapter, not runtime evidence. No 86Box executable,
physical controller, firmware set, guest program, disk write, or save behavior
has been exercised. Writing gameport values does not prove that the emulated
machine has a gameport or that the selected guest program reads it. Guest saves
remain files inside user-selected writable disk images, and the reviewed runtime
has no documented portable save-state contract; neither is relocated or claimed
sync-ready here. Other gameport types, SDL3 builds, Flatpak
`net._86box._86Box`, macOS, and Windows require separate contracts.
