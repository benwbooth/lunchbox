# SkyEmu standalone-native controller contract

Source oracle: `skylersaleh/SkyEmu` commit
`01516d6798e3652b583e6a366085bb51c43b528d`, chiefly `src/main.c` and
`src/sb_types.h`.

The native SDL frontend persists a selected controller's bindings as raw
host-endian `int32_t[64 * 2]` (512 bytes; little-endian on supported desktop
hosts): the first 64 values are key bindings and the second 64 are analog
bindings.  The file is named
`<SDL_GetPrefPath("Sky", "SkyEmu")><controller-name>-bindings.bin`.  Key
indices and analog indices are source-defined; unused slots are `-1`.

Key bindings encode a raw SDL button index, an axis index plus
`1<<17` (positive) or `1<<18` (negative), or a hat as `(1<<16) | (hat<<8) |
SDL_HAT_*`.  The controller name is part of the filename, so the adapter
rejects path separators/control characters and limits it to the source's
128-byte name buffer.  The name and SDL GUID are discovered at runtime; a
controller file must be written for the exact selected name while SkyEmu is
stopped, and the device must be rechecked before launch.

The SDL preference root is platform-owned by SDL2: on Linux it normally lies
under the per-user data root (`$XDG_DATA_HOME` or `~/.local/share/Sky/SkyEmu/`),
on macOS under `~/Library/Application Support/Sky/SkyEmu/`, and on Windows
under `%APPDATA%\\Sky\\SkyEmu\\`.  Portable/alternate SDL builds may choose
another root, so the launch layer must use the path returned by SDL rather
than hard-code one.

The writer changes only the controller binding file.  Preserve
`user_settings.bin`, `keyboard-bindings.bin`, recent games, ROM-adjacent
`.sav`, and the four persistent state files
`<save-data-base>.slot0.state.png` through `.slot3.state.png` (including their
screenshot payloads).  BIOS lookup remains source-defined: native builds use
`gba_bios.bin` and GB boot aliases such as `dmg0_rom.bin`, `dmg_rom.bin` and
`gb_bios.bin`, with bundled replacement BIOS support.  A source-backed binary
writer does not prove a particular SkyEmu executable, SDL enumeration, game
input behavior or save/state round trip.
