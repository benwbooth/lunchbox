# MEMU standalone controller boundary

Pinned source: `Memotech-Bill/MEMU@86eb58dfaae32ab73d36297e24daf2b8929567c4`.
MEMU supports Linux, Windows, and SDL builds. It accepts `-joy`,
`-joy-buttons`, and `-joy-central`; Linux opens `/dev/input/js0`, SDL opens
enumerated joysticks, and Windows uses DirectInput. However
`src/memu/config.c` places the joystick config-save code under `#if 0`, so a
normal `memu.cfg` save does not persist those options.

The adapter refuses to synthesize persistence or a stable physical-device
selector. Disk/tape images, ROM files, keyboard remapping, and snapshot files
remain emulation-mode-specific and are preserved outside this controller
boundary.
