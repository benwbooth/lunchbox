# A7800 standalone native controller contract

Source oracle: `7800-devtools/a7800` commit
`7a2afdc1ea08fc331b16b750d8c1f02d4ef62fc8` (inspected 2026-09-12). This is
the standalone A7800/MAME-derived executable, not a libretro Atari 7800
core. `src/mame/mame.cpp` sets the application/config identity to `a7800`;
`src/mame/drivers/a7800.cpp` registers two controller ports and the
`vcs_joystick` and `proline_joystick` devices.

## Input boundary

The source-backed writer is
`crates/lunchbox-app/src/controller_a7800_native.rs`. It emits MAME v10 XML
`<port>` entries for only the controls declared by the caller:

- `P1_`/`P2_` `JOYSTICK_UP`, `JOYSTICK_DOWN`, `JOYSTICK_LEFT`, and
  `JOYSTICK_RIGHT` come from the controller device's `IPT_JOYSTICK_*` ports.
- `P1_`/`P2_` `BUTTON1` is available on both source joystick devices;
  `BUTTON2` is available only on `proline_joystick`.
- Launch capture translates measured Linux controls through the exact declared
  SDL2 library. Raw buttons become source-numbered `BUTTON1`–`BUTTON32`, the
  first eight absolute axes become MAME half-axis switches, and the first four
  hats accept only cardinal directions. The configured threshold must separate
  every measured axis rest/press pair, all controls must be released, and no
  two controls may share a token.

The target also implements paddles, lightguns, driving wheels, keypads,
trackballs, Amiga mice and ST mice. Those devices have different source input
ports and analog semantics; they are intentionally refused by this overlay
until a caller supplies a separately measured contract. This A7800 fork
predates the current MAME identity contract: its SDL provider uses
`SDL_JoystickNameForIndex` after removing whitespace, not the joystick GUID.
Lunchbox reproduces that exact transformation, requires ASCII-safe names, and
refuses duplicates or substring-overlapping names because `<mapdevice>`
matching is substring based.

## Configuration and roots

The target inherits the pinned MAME configuration manager. Relative defaults
are beside the executable:

- `cfg/default.cfg` and `cfg/a7800.cfg` are MAME v10 XML configuration files;
  `-cfg_directory <dir>` relocates them. Controller profiles are XML files
  under `ctrlr/` and are selected with `-ctrlr <name>`/`-ctrlrpath`.
- `sta/` is the default `-state_directory`; with the default `-statename %g`,
  states are `sta/<machine-short-name>/<slot>.sta`. The source driver declares
  `MACHINE_SUPPORTS_SAVE` for every NTSC/PAL A7800 target.
- `nvram/` is the default NVRAM root for device-managed nonvolatile data. The
  A7800 driver does not establish a universal per-ROM sidecar save file, so
  this root must not be presented as a game-save substitute.
- `roms/` and `bios/` are media/ROM search roots. The NTSC driver declares
  optional `7800.u7` (SHA-1
  `d9d134bb6b36907c615a594cc7688f7bfcef5b43`) and optional prototype
  `c300558-001a.u7` (SHA-1
  `14584b1eafe9721804782d4b1ac3a4a7313e455f`); PAL targets declare optional
  `7800pal.rom` (SHA-1
  `5a140136a16d1d83e4ff32a19409ca376a8df874`). The source README says BIOS
  files are optional, so a missing file is not silently replaced by another
  ROM.

The generic MAME `-inipath`, `-rompath`, `-cfg_directory`,
`-nvram_directory`, and `-state_directory` options are the relocation
surface. Session launch accepts only the base `a7800` (NTSC) and `a7800p` (PAL)
machines and rewrites the generic one-ROM launch into `<machine> -cart <ROM>`.
It emits a session-owned controller profile, copies only `default.cfg` and the
exact machine cfg into a private cfg directory, and removes only the selected
P1/P2 direction and button overrides from those copies. It forces
`-joystickprovider sdl`, `-nosixaxis`, contiguous `-joy_idx1`/`-joy_idx2`
mapping and `proline_joystick` devices. Other INI, ROM, NVRAM and state roots
are not relocated.

There is no verified official A7800 Flatpak identity in the pinned upstream
repository. A native overlay must not claim Flatpak behavior without a
separately verified package manifest and sandbox roots.

Before spawn, Lunchbox rechecks the executable/content/probe/SDL hashes,
physical topology, SDL enumeration, raw controls, source cfg snapshots and all
private files. Startup succeeds only after the exact child executable has
loaded the declared SDL library and opened every selected kernel device. This
is still a partial native Linux adapter: no A7800 binary or game was run, and
gameplay, save/state behavior, firmware behavior, other packages, other hosts,
and alternate machines/peripherals remain unverified.
