# eSCV native controller contract

This is a Windows-only, writer-only contract for the official eSCV Common
Source Code Project archive (`source.7z`, SHA-256
`63d598a29cabf2d9212bd8a1699d8bcdddfc2416e711c20581a0d665c96f2399`). The
official eSCV page requires Windows 10/11.

The Win32 executable loads `scv.ini` from its current directory. Its source
writer emits `[Input]` keys `JoyButtonsEx1_1` through `JoyButtonsEx8_16`.
Each value is either a negative Win32 virtual-key code or a nonnegative packed
joystick value `(stick << 5) | position`; the runtime polls joystick indices
0 through 3 and positions 0 through 31. `controller_escv_native.rs` patches
that complete source-generated shape while retaining unrelated INI content.
It does not serialize a physical device name: WinMM joystick order must be
measured and verified in the same launch.

The source loads `BIOS.ROM` from the working directory, derives cartridge
battery saves by replacing the selected cartridge extension with `.SAV`, and
uses `scv.sta0` through `scv.sta9` (gzip by default) for state slots. These are separate from
the controller profile. Linux, Linux Flatpak, and macOS have no verified eSCV
artifact in the official distribution and therefore have no native contract.
