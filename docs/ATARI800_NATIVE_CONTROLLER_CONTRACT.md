# Atari800 native controller contract

Source oracle: `atari800/atari800` commit
`fe1d2890d9f05fcecb2fd033d09a5c43f534bebf` (7.2.0, inspected 2026-09-12),
principally `src/sdl/input.c`, `src/sdl/input.h`, `src/cfg.c`, `src/ui.c`,
and `src/statesav.c`.

Atari800 is a four-port Atari 8-bit/5200 emulator. Its native SDL2 input
configuration is a plain `KEY=VALUE` file selected with `-config` (default
`~/.atari800.cfg`). The per-port source keys are
`SDL2_JOY_PORT_<n>_MODE`, `_NAME`, and `_SLOT`; mode 4 is a host SDL joystick,
while 0/1/2/3/5 mean none, keyboard 1, keyboard 2, parallel, and paddle.
`_USE_HAT` accepts -1 (automatic), 0 (axes), or 1 (hat), `_AXES` selects the
first axis in the pair, and `_DIAGONALS` selects the upstream diagonal-zone
enum. `_BUTTON_ACTIONS` and `_BUTTON_KEYS` are comma-separated arrays for up
to 15 buttons. Host identity is the SDL display name plus duplicate-name slot;
it is not a stable hardware identifier.

The native Linux adapter now implements that conservative overlay. It maps one
through four calibrated physical controllers to contiguous Atari ports, accepts
only the source-supported raw SDL axis pairs 0/1 or 2/3 or cardinal hat 0, and
requires Fire on raw SDL button 0..14. The trigger is written using the exact
`JoystickUiAction`/`AKEY_CONTROLLER_BUTTON_TRIGGER` pair (1/-100). It preserves
all unrelated configuration lines and refuses paddle, parallel, keyboard and
5200 analog modes.

Preparation captures kernel topology plus the exact target SDL2 library's
ordered device paths, names, raw controls, and duplicate-name slots. All are
rechecked immediately before launching the exact executable hash. Bubblewrap
mounts the copied config at the selected original `-config` path; this keeps
the source config untouched without changing Atari800's config-directory data
root.

Guest saves remain inside writable mounted media images. Explicit emulator
states are user-selected gzip-compatible binary files beginning with the
`ATARI800` header; the quick-save file is exactly
`.atari800-quicksave.state` beside the selected config file. OS/BASIC/5200 ROMs
are optional when the built-in Altirra revisions are selected; external ROM
paths are passed with the documented `-osa_rom`, `-osb_rom`, `-xlxe_rom`,
`-5200_rom`, and `-basic_rom` switches. No separate key file is used, and no
canonical user-ROM checksums are published by this source revision.

No Flatpak package is identified by the upstream project. Windows and macOS use
the same source grammar but have no Lunchbox launch adapter yet. Linux launch,
effective gameplay input, firmware selection, media write-back, quick-save,
explicit state round trips, hotplug, and multiple identical controllers remain
runtime-unverified.
