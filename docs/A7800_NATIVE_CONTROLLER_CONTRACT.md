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
- Each value must already be a provider-resolved `JOYCODE_*` or `KEYCODE_*`
  token. The module does not guess SDL indices, keyboard layouts, controller
  types, axis orientation, or the console switches.

The target also implements paddles, lightguns, driving wheels, keypads,
trackballs, Amiga mice and ST mice. Those devices have different source input
ports and analog semantics; they are intentionally refused by this overlay
until a caller supplies a separately measured contract. The existing MAME
controller-profile machinery remains the source of truth for stable device
identity (`<mapdevice>`), provider choice and native item resolution.

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
surface. Session launch should pass private copies/roots and leave the user's
existing A7800/MAME tree untouched.

There is no verified official A7800 Flatpak identity in the pinned upstream
repository. A native overlay must not claim Flatpak behavior without a
separately verified package manifest and sandbox roots.

Runtime application of the new writer and gameplay parity remain unverified;
this document is a source-backed configuration contract, not an activation
claim.
