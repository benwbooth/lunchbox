# BigPEmu, ColEm, and CUzeBox standalone controller contract

This batch is source-backed at the following exact artifacts:

- BigPEmu 1.221 Linux archive, whose official download page identifies the
  Linux archive as `BigPEmu_Linux64_v1221.tar.gz` (archive SHA-256
  `c0ff610f21d5f55c01c404ef991d89f769a5b3b893c8570c033bcef44121ee57`).
- ColEm 5.6 `ColEm56-Source.zip` (SHA-256
  `11e536e20892b89140c24e3163bbe7b2c032e31535f6df0334768144b1deb351`).
- CUzeBox source `Jubatian/cuzebox@adcea412e18cca8a4bb9e94af94097a420c2f5cc`.

## BigPEmu

The published archive and manual expose `.bigpcfg` properties, the Input GUI,
and `-setcfgprop`, but do not publish a complete stable controller-property
schema. `controller_bigpemu_standalone` therefore refuses to fabricate a
writer. The caller may use the official GUI or a version-specific executable
oracle, with `-cfgpath`/`-cfgpathabs` and a copied `UserData` root as needed.

## ColEm

The pinned source's Unix port (`EMULib/Unix/LibUnix.c`) maps X11 keyboard
events directly into joystick bits; it does not load a portable persisted
controller mapping file. The Windows physical joystick selection is outside
the portable source. `controller_colem_standalone` explicitly refuses a
cross-platform writer. The same source does establish `<cartridge-basename>.sav`
for EEPROM and `<cartridge-basename>.sta` for emulation state.

## CUzeBox

`ginput.c` loads `gamecontrollerdb.txt` from the executable base directory via
`SDL_GameControllerAddMappingsFromFile`, then opens at most the first two
SDL game controllers. `controller_cuzebox_native` writes standard SDL2
GameController database records keyed by the caller-measured 32-hex-digit
device GUID, preserves comments and unrelated records, rejects malformed or
duplicate fields, requires the complete source-consumed SNES button set, and
limits the patch to two mappings. The source has no
persisted physical-slot setting; launch must re-enumerate and verify selected
devices immediately before startup.

The pinned CUzeBox source writes EEPROM to the fixed `eeprom.bin` file in the
game virtual filesystem (2048 bytes). Its README lists emulator state saves
(snapshots) as still lacking, so no state writer or state filename is claimed.
