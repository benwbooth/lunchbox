# ProSystem 1.3 standalone native controller contract

Source oracle: `gstanton/ProSystem1_3` commit
`3040295934c68024ecf89b45b50107ea1c271e66` (inspected 2026-09-12). The
project README identifies ProSystem 1.3 as a PC/Windows emulator written
against the Windows API and DirectX. It is a separate standalone runtime,
not a libretro Atari 7800 core and not the A7800 MAME fork.

## Input writer

`crates/lunchbox-app/src/controller_prosystem_native.rs` is a writer-only
overlay for a copied `ProSystem.ini`. The upstream `Win/Configuration.cpp`
uses WinAPI private-profile INI calls and reads/writes:

- `[Input] Key0..Key11` and `Device0..Device11` for controller 1 and 2,
  ordered by right, left, down, up, button 1, button 2 for each controller;
- `[Input] Key12..Key16` and `Device12..Device16` for reset, select, pause,
  right difficulty and left difficulty;
- `[Input] User_Key0..1`, `User_Device0..1` and `User_Modifier0..1` for
  menu/exit input, which this overlay deliberately leaves unchanged.

The module accepts only an explicit `(index, key, device)` tuple. In the
upstream UI, `device == 0` means Keyboard and nonzero values are the
DirectInput device-enumeration values; `key` is either a DirectInput keyboard
scan code or an `e_joy_value` (`JOY_AXIS_*`, `JOY_PAD_*`, `JOY_BUTTON_1` through
`JOY_BUTTON_12`) from `Win/Input.h`. The caller must capture those values from
the same Windows runtime. No SDL numbering, XInput assumption, axis direction,
or controller model is inferred.

## Paths and persistence

`Win/Main.cpp` derives `common_defaultPath` from the executable path.
`Win/Console.cpp` loads and saves `common_defaultPath + "ProSystem.ini"` and
the logger uses the same directory for `ProSystem.log`. A session overlay must
copy that INI to a private directory and point the Windows launch at the
private executable/config arrangement; the original file must not be edited.

ROMs are opened explicitly from the command line or the Win32 file dialog.
The upstream accepts `.a78` images with a valid header, raw binaries when the
database is enabled, and ZIP archives containing a ROM. No universal ROM or
BIOS directory is established by the source.

BIOS is optional. When enabled, `[Emulation] Bios.Filename` names a caller
selected file; `Core/Bios.cpp` accepts a raw file or an archive and the help
states that a BIOS image is not necessary to run the emulator. No canonical
BIOS checksum is published by this project. The optional database is likewise
selected by `[Emulation] Database.Filename` and must not be confused with
firmware.

Save states are user-selected files, not emulator-managed slots. `Ctrl+S`
opens a destination dialog and `Core/ProSystem.cpp` writes an uncompressed
binary `.sav` or a ZIP containing the member `Save.sav`; `Ctrl+L` loads a file
only when its embedded ROM digest matches the currently loaded cartridge.
`[Console] Save.Path` stores the last dialog path. There is no universal
per-game save sidecar directory.

The source is Windows-only. Linux, macOS and Flatpak platform entries remain
unsupported/unresolved until a distinct maintained native port and its
configuration/input/state/firmware roots are verified. Runtime application of
the writer and controller behavior remain unverified.
