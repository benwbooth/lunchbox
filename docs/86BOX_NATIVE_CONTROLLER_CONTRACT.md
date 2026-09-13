# 86Box native Linux controller contract

Source oracle: `86Box/86Box` commit
`189d9d003ad9670853cec6edac8db7d6ff63550f` (inspected 2026-09-12), especially
`src/86box.c` and `src/config.c`. The captured platform record is
`emulator_details/records/86box.json`.

86Box does have a real joystick API, but it is a machine-level SDL joystick
contract rather than a stable gamepad semantic contract. `src/config.c`
loads per-machine `86box.cfg` `[Input devices]` values, including
`joystick_type`, `joystick_<n>_nr`, `joystick_<n>_axis_<n>`,
`joystick_<n>_button_<n>`, and `joystick_<n>_pov_<n>`; POV values are two
integers (`x, y`). The selected
machine's joystick type determines the number and meaning of those slots.
`src/86box.c` accepts `-C`/`--config` for an explicit config file.

The native module writes only the source-defined `[Input devices]` keys after
the caller supplies the exact machine joystick type, emulated slot topology,
and measured one-based host joystick number. It preserves a copied baseline
configuration and accepts no guessed machine or guest action names. Writing
joystick values still does not establish that a game actually reads the mapped
controls. 86Box's
guest saves remain files inside user-selected disk images, and the official
runtime has no documented portable save-state format; neither can be safely
wrapped by a generic controller adapter.

The caller must supply the exact `86box.cfg` baseline, joystick type/slot
topology, host SDL/raw-input numbering, and guest-facing actions. It must copy
and patch the declared machine config through `-C`, hash the
executable/config/probe inputs, reject ambiguous or virtual devices, and leave
the configured ROM directory,
firmware ROM set, disk images, and guest save paths untouched. Flatpak
`net._86box._86Box` is a separate sandbox launch mode and is not implied by a
native Linux adapter.
