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
These two direct-config tests use deliberately hidden virtual devices. The
additional saved-calibration test below exercises normal discovery and `prepare`.

`controller_launch::oracle::brawler64_config_reaches_psx_hardware_through_retroarch`
uses the same round-tripped calibration and native-numbering path with Beetle PSX
device 1. An original MIPS executable asks the local BIOS to poll its controller
ports. The test checks all fourteen digital controls, their releases, and
Cross+Square / L1+R1 combinations through the BIOS buffer in emulated RAM.
Expected PlayStation bits are independent of the generated mapping: the Brawler
A/B pair becomes Cross/Square, C-down/C-left becomes Circle/Triangle, and the two
Z buttons become L2/R2.

For this additional test, set `LUNCHBOX_ORACLE_PSX_CORE` to the software Beetle
core with SHA-256
`767bb60bd96d3f19806a9311d96638c9ca39272d1236035a752952bb4b4c1968` and
`LUNCHBOX_ORACLE_PSX_BIOS_DIR` to a local directory containing `scph5500.bin`,
`scph5501.bin`, and `scph5502.bin`. BIOS files are copied only into the private
test directory; no firmware or game data is bundled. Run the named PSX test
with `--ignored --nocapture --test-threads=1`, using the private display described
below. `READ_CORE_MEMORY 00020000 36` returns the execution marker and the
first controller's four-byte BIOS response at offset `0x20`. The valid-response
prefix is `00 41`, followed by two active-low button bytes.

This test verifies the digital Brawler64 path on the software core, not disc
compatibility overrides or the complete automatic `prepare` entry point.
The separate `lunchbox-libretro-input --system psx` diagnostic checks both
Beetle variants directly, with DualShock on both ports and mixed digital /
DualShock ports, in individual and bitmask callback modes. Each run makes 90
observations. Its `contract_source_revision` describes the expected input
contract; observed core hash and version identify the binary actually tested.

## Saved-calibration launch path

`controller_launch::oracle::saved_brawler64_calibration_prepares_and_controls_gba_through_retroarch`
uses two explicitly labeled Steam-compatible virtual gamepads. Their supplied
VID/PID (`28de:11ff`) select Lunchbox's existing supported virtual-gamepad class;
they remain `BUS_VIRTUAL` and are asserted to be virtual in the actual discovery
result. This does not impersonate a physical Brawler64 in the evidence: the
saved physical-layout choice is explicitly `brawler64`, independent of USB identity.
There is no injected inventory or test-only discovery exception. See the
[Linux uinput interface](https://docs.kernel.org/input/uinput.html) for device setup.

The test gets both stable IDs from real discovery, saves and reloads their
calibrations through a private `SettingsStore`, and prefers the later discovery
entry. It builds a production ROM launch plan and calls the public `prepare`
entry point. The generated controller arguments and Flatpak filesystem grants
are preserved when the isolated diagnostic process is launched; the test never
calls `player_config` on this path. It checks the selected joydev index and
single-player limit, then reads the same independent GBA hardware expectations
as the direct-config test. The non-preferred pad holds conflicting A/L inputs
throughout every selected-pad press/release check. Launch-session cleanup is
checked after the emulator has been stopped.

Run this test by replacing the test-name filter in the command below with
`saved_brawler64_calibration_prepares_and_controls_gba_through_retroarch`.
It still does not cover physical GUI button capture, automatic emulator/core
discovery (the trusted core is supplied explicitly), the desktop launch action,
mode-aware core preparation, standalone emulators, or other operating systems.
No user settings database or emulator configuration is rewritten. The production
preparation cache contains an owned temporary controller-config directory that is
removed when the launch session ends.

Verified on 2026-09-06 with the pinned mGBA binary and RetroArch 1.22.2
(`69a4f0ea1e`): the later-discovered `/dev/input/js8` was selected over `js7`, all
25 GBA hardware observations passed (initial release, ten individual controls,
and two combinations with releases), and the generated config directory was
removed after teardown. Joystick numbers describe that run, not saved assumptions.

## Prerequisites

The saved-calibration variant also checks that conflicting device CLI selections
(`--device=1:5`, `--nodevice=1`) and unresolved grouped/abbreviated flags fail
during production preparation without mutating the launch plan. It also rejects
two forms of duplicate append-config options. Its successful launch uses `-M`,
one pipe-separated list of preexisting configs, and the matching `--device=1:1`,
which RetroArch processes after the generated configuration. The preexisting
configs deliberately contain a mismatched device and an unbound B button, so the
hardware readback also verifies that Lunchbox's config is appended last.
These checks exercise the single-mode mGBA path;
unit tests separately check every launch contract and unfilled-port conflicts.

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
- The NES variant uses Nestopia source commit
  `4ed9d68bd251e122f1311895fabe68bde89da86a`, built from `libretro/Makefile`
  with its default Unix release flags. The reviewed binary has SHA-256
  `f6e2a6f96bd73385663b732324bff7832c168c4c2c1811732f7b0300c0de8376`.
  This revision publishes the NES CPU memory map needed for independent
  hardware readback. The installed updater core used by the state-path probe
  below is older (`473d307`) and deliberately is not accepted for this oracle.

After starting a private X server (for example `Xvfb :97 -screen 0 640x480x24
-nolisten tcp -ac`), run from the development shell:

```console
LUNCHBOX_ORACLE_DISPLAY=:97 LUNCHBOX_ORACLE_MGBA_CORE=/absolute/path/mgba_libretro.so cargo test -p lunchbox-app --lib --release brawler64_config_reaches_gba_hardware_through_retroarch -- --ignored --nocapture --test-threads=1
```

For the NES variant, set `LUNCHBOX_ORACLE_NESTOPIA_CORE` to the reviewed
`nestopia_libretro.so` and use the
`brawler64_config_reaches_nes_hardware_through_nestopia` filter.

Verified on 2026-09-12 with the reviewed Nestopia binary and the Flatpak
RetroArch build above: all 21 NES hardware observations passed (initial
release, eight individual controls, A+B, and an up+right diagonal, with a
release after every case). The diagonal deliberately avoids depending on
Nestopia's policy for impossible opposing directions. Software GL startup took
27.20 seconds for the complete run; startup has a separate 30-second bound,
while every running input transition remains bounded to 10 seconds.

The test does not start or stop the X server. Close controller-aware desktop
applications first if they might react to a synthetic joystick. In particular,
the saved-calibration test's Valve-class pads are visible to normal discovery and
can be observed by host controller daemons. Button codes use
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
The test verifies that the private X socket is listening before launching.
Inherited device access is reset and only input/shared-memory devices are
requested, so a missing X connection cannot fall back to direct DRM display access.
An owned process group is killed and the launcher is reaped on teardown.

`READ_CORE_MEMORY 02000000 8` is newline terminated. Its stdout reply starts with
`READ_CORE_MEMORY 2000000` and contains eight hexadecimal bytes. The test waits
for the original program's execution marker and expected active-low KEYINPUT
value. It retries across frames because a command can read RAM from the frame
before a newly injected input was consumed. Both startup and each assertion are
bounded; emulator exit, a missing response, or the wrong hardware bits fail.

Passing this test establishes only this explicitly tested core/runtime/input path.
It must not be reported as proof that every emulator or core is implemented.

## Direct NES core probe

`lunchbox-libretro-input --system nes` loads an exact-hash FCEUmm or Mesen
binary and runs an original NROM diagnostic directly through the libretro API.
Use `--nes-topology two-player` or `--nes-topology four-score`; add `--bitmask`
to exercise `RETRO_DEVICE_ID_JOYPAD_MASK` rather than individual button
callbacks. The probe selects each core's advertised standard-controller device,
checks its controller metadata and descriptors, independently stimulates every
connected player, and compares active-low bytes published from the emulated NES
controller registers. Deliberately nonzero state is also supplied on
disconnected frontend ports, so the hardware result detects accidental
topology leakage even when a core still queries those ports.

Verified on 2026-09-12 with the installed Flatpak updater cores:

- FCEUmm `(SVN) 5cd4a43`, SHA-256
  `e7a17d1a5dacaeb7e02067f73aafab168f6a249a5dc0059ff16a5df44784d65c`.
- Mesen `0.9.9`, SHA-256
  `552f8ab6ac1fd08bd555f589eb999be73a469c79ccfa929f884adb2cf3366b43`.

For each core, both callback modes passed 42/42 observations with two players
and 84/84 with Four Score (252/252 total per core). FCEUmm did not query
disconnected ports. In bitmask mode Mesen queried all five frontend ports, but
the nonzero disconnected states never appeared in the NES hardware bytes. The
Mesen binary needs its C++ runtime available when the probe is run outside the
Flatpak runtime. This direct-core evidence does not cover the RetroArch GUI,
physical calibration capture, firmware, persistent saves, save states, or
save-sync export/restore.

## Nestopia Flatpak state-path probe

A separate manual probe on 2026-09-12 exercised the installed Flathub
`org.libretro.RetroArch` 1.22.2 build (Git `69a4f0ea1e`, Flatpak commit
`9c51e2bcb6f7f29ecb327ee057b273c5b59efc22d35026e90aef601bc0052752`)
with the locally installed `nestopia_libretro.so` binary, SHA-256
`3d517a4aa301b37d9d65e2a984e7d4e4db2fae7ccdad17abed7093316f2efec4`.
The run used a private configuration and explicit save, state, and system
directories under `target/runtime-evidence/retroarch-flatpak`.

RetroArch loaded a real NES cartridge, redirected the save and state paths to
the private directories, and reported the Nestopia save directory requested by
the core. The udev driver enumerated four real host gamepads. A network-command
`SAVE_STATE` wrote the expected content-basename `.state` file and reported a
13,320-byte serialized payload; `LOAD_STATE` then loaded the same path and
payload size. The on-disk compressed state was 4,766 bytes with SHA-256
`5f355e158e8b7406e3dbc5261dabc7e61b8ff68cf19539f174274fd419452487`.

This is evidence for Nestopia's RetroArch Flatpak state path and a save/load
state serialization round-trip. It is not controller-mapping evidence because
no input response was observed, not persistent-save evidence because the game
did not create and reload save RAM, not optional FDS BIOS evidence, and not a
save-sync export/restore test. The matrix therefore records only
`state_test_status=pass` for this runtime/host probe.

## Source contract

- [RetroArch 1.22.2 memory command response](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/command.c#L1074-L1114)
- [RetroArch command syntax](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/command.h#L442-L457)
- [mGBA libretro memory maps and frame input](https://github.com/mgba-emu/mgba/blob/e31759b24e7a4e3899285ff720d7b573ac328ae7/src/platform/libretro/libretro.c#L1537-L1821)
- [Beetle PSX input and memory maps](https://github.com/libretro/beetle-psx-libretro/blob/56f4732070835bb81078dd8ecab7246e203612a1/libretro.c)
