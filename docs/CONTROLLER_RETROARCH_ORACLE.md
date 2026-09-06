# RetroArch controller hardware oracle

`controller_launch::oracle::brawler64_config_reaches_gba_hardware_through_retroarch`
is an opt-in Linux x86-64 integration test. It creates a temporary kernel joystick,
round-trips a Brawler64 calibration through JSON, reads the actual joydev button
numbering, and passes the production `player_config` output to real RetroArch.
An original ARM diagnostic ROM copies the GBA KEYINPUT hardware register to RAM;
RetroArch's stdin command interface reads it back. The test checks all ten GBA
buttons, their releases, and A+B / L+R combinations. Expected hardware bits are
independent of the generated configuration.

This covers the native-numbering and RetroPad intermediary path, not physical
GUI calibration capture, automatic device selection, the complete `prepare`
entry point, standalone emulators, other operating systems, or all cores.
The deliberately virtual device is not offered by normal physical-pad discovery.

## Prerequisites

- A writable `/dev/uinput` and Linux joydev support.
- Installed Flatpak `org.libretro.RetroArch` (reviewed runtime: 1.22.2,
  Git `69a4f0ea1e`).
- A separately running local Xvfb display, distinct from the desktop display.
  The desktop `DISPLAY` must be set to a local display; the oracle accepts only
  `:<number>` and rejects equivalent desktop numbers even with screen suffixes.
- The reviewed mGBA core, version `0.11-219-e31759b`, SHA-256
  `768921964037e0a40e8eab9e0d6eccad1b8a13d74bc37e9cae5543bb167d18c4`.
  The test rejects other bytes before loading them. The core is not distributed
  with this test; no game or BIOS is needed.

After starting a private X server (for example `Xvfb :97 -screen 0 640x480x24
-nolisten tcp -ac`), run from the development shell:

```console
LUNCHBOX_ORACLE_DISPLAY=:97 LUNCHBOX_ORACLE_MGBA_CORE=/absolute/path/mgba_libretro.so cargo test -p lunchbox-app --lib --release controller_launch::oracle -- --ignored --nocapture --test-threads=1
```

The test does not start or stop the X server. Close controller-aware desktop
applications first if they might react to a synthetic joystick. Button codes use
the nonstandard trigger-happy range, not Guide/keyboard events, and the virtual
device is destroyed on return or unwind.

## Isolation and readback

The Flatpak invocation disables network and Wayland, resets inherited filesystem
grants with `--nofilesystem=host:reset`, and adds a grant for the private temporary
test directory. Flatpak's built-in per-app directories remain accessible; this
is not an empty-home sandbox. `env` inside the sandbox
sets XDG config/data/cache/state locations: Flatpak overwrites equivalent `--env=XDG_*`
options before execution. HOME is not changed. All writable RetroArch content,
core-option, save, state, history, remapping, recording, and log paths are private;
automatic overrides/remaps and config/remap saving are disabled. SRAM is neither
loaded nor saved. Companion desktop UI, microphone initialization, and screensaver
control are disabled. The Flatpak launcher inherits the private X display before
socket exposure is selected; Qt is explicitly set to X11 inside the sandbox.
An owned process group is killed and the launcher is reaped on teardown.

`READ_CORE_MEMORY 02000000 8` is newline terminated. Its stdout reply starts with
`READ_CORE_MEMORY 2000000` and contains eight hexadecimal bytes. The test waits
for the original program's execution marker and expected active-low KEYINPUT
value. It retries across frames because a command can read RAM from the frame
before a newly injected input was consumed. Both startup and each assertion are
bounded; emulator exit, a missing response, or the wrong hardware bits fail.

Passing this test establishes only this explicitly tested core/runtime/input path.
It must not be reported as proof that every emulator or core is implemented.

## Source contract

- [RetroArch 1.22.2 memory command response](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/command.c#L1074-L1114)
- [RetroArch command syntax](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/command.h#L442-L457)
- [mGBA libretro memory maps and frame input](https://github.com/mgba-emu/mgba/blob/e31759b24e7a4e3899285ff720d7b573ac328ae7/src/platform/libretro/libretro.c#L1537-L1821)
