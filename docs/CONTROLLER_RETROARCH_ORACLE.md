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
`c718ba34de4548937bce76efbd4130399c6c5fb25c335833080b391b92034674` and
`LUNCHBOX_ORACLE_PSX_BIOS_DIR` to a local directory containing `scph5500.bin`,
`scph5501.bin`, and `scph5502.bin`. BIOS files are copied only into the private
test directory; no firmware or game data is bundled. Run the named PSX test
with `--ignored --nocapture --test-threads=1`, using the private display described
below. `READ_CORE_MEMORY 00020000 36` returns the execution marker and the
first controller's four-byte BIOS response at offset `0x20`. The valid-response
prefix is `00 41`, followed by two active-low button bytes.

The companion `brawler64_config_reaches_psx_hw_hardware_through_retroarch`
test uses `LUNCHBOX_ORACLE_PSX_HW_CORE` and pins SHA-256
`25176f77c060cf74c4561f745bab181d9bb6b620591f92f82ad0d53c1cc7fb56`.
It exercises the same digital hardware assertions through the PSX HW launch
profile. The isolated Flatpak run negotiated an OpenGL Core 3.3 shared hardware
context and executed it with Mesa llvmpipe; this proves the frontend/core HW
context handshake in software rendering, not physical-GPU behavior.

This test verifies the digital Brawler64 path on the software core, not disc
compatibility overrides or the complete automatic `prepare` entry point.
The separate `lunchbox-libretro-input --system psx` diagnostic checks both
Beetle variants directly, with DualShock on both ports and mixed digital /
DualShock ports, in individual and bitmask callback modes. Each run makes 90
observations. Its `contract_source_revision` describes the expected input
contract; observed core hash and version identify the binary actually tested.
The exact Linux x86_64 and macOS arm64 core identities and retained report
locations are recorded in
[`LIBRETRO_INPUT.md`](../crates/lunchbox-controller-probe/LIBRETRO_INPUT.md);
these direct-core runs do not promote the separate Linux-only RetroArch launch
oracle to another frontend or host.

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

## Direct SNES core probe

`lunchbox-libretro-input --system snes` runs an original 32 KiB LoROM through
an exact-hash bsnes, Snes9x, or Mesen-S core. `--snes-topology two-player`
selects ordinary joypads; `--snes-topology multitap` selects a joypad plus a
port-two Super Multitap. The 65C816 diagnostic manually clocks `$4016` and
`$4017`, switches the multitap halves through `$4201`, and publishes five
completed serial words to emulated WRAM. Each connected frontend port is tested
independently with different asserted state on all other ports.

Verified on 2026-09-12 against these installed Flatpak updater binaries:

- bsnes `115`, SHA-256
  `ffd2898ddf27fbcac962e9e10a65e7562e8c1a7c2b795a0d93dc32a3ebfb8e8b`:
  58/58 two-player and 145/145 multitap observations in individual mode.
  Because this revision has no standard memory exposure, readback came from one
  uniquely marked WRAM region in a bounded `retro_serialize` snapshot. It does
  not negotiate the joypad bitmask API, so bitmask mode is explicitly rejected.
- Snes9x `1.63 185488c`, SHA-256
  `6e2d5fb3bbf57ef0a24834b36914187bea1a0b20f373da3adfdd5aebf75a4a99`:
  58/58 two-player and 145/145 multitap observations in each of individual and
  bitmask modes. Readback used its reported 131,072-byte system-RAM region.

A supplemental run against Mesen-S `0.4.0`, SHA-256
`d43d7875316dc3f505ec160d5d34cc541fa899225c9227fed839697a7936cb5a`,
passed 58/58 two-player observations in individual mode. It does not negotiate
bitmasks. Its multitap run failed the initial released-state gate with words
`[0000, 0000, 0000, ffff, 0000]`, so there is no five-player compatibility
claim for that binary. Mesen-S also omits Start and Select from its advertised
descriptor table despite mapping both on the hardware-verified two-player path.

The probe validates core controller choices and descriptors, callback mode and
per-port queries in addition to hardware readback. This still bypasses the
RetroArch frontend and therefore does not establish physical-device capture,
generated remaps/configuration, GUI launch behavior, game compatibility,
persistence, save states, or other SNES peripherals. Full command and source
contract details are in [the direct-core diagnostic guide](../crates/lunchbox-controller-probe/LIBRETRO_INPUT.md).

## Direct Stella core probe

`lunchbox-libretro-input --system atari2600` executes an original 4 KiB 6507
cartridge and compares both joysticks, both trigger lines, and the Atari 2600
console switches at the emulated RIOT/TIA register boundary. The exact Linux
buildbot core passed 36/36 observations in individual and bitmask modes both on
the host and from inside the installed Flathub RetroArch runtime. The Flatpak
updater directory contained no Stella binary, so the sandbox run used an
explicit read-only grant for that exact external core; it does not establish an
installed-core or RetroArch frontend launch path.

The macOS arm64 buildbot dylib repeated both 36-observation modes on the M1,
and the official Windows x86-64 DLL repeated them on hosted Windows Server
2025 in GitHub Actions run
[`34750404594`](https://github.com/benwbooth/lunchbox/actions/runs/34750404594).
The two current build streams advertise different exact controller-choice
tables, which the probe binds to their respective pinned source revisions.
Only the diagnostic's automatically detected Joystick pair was exercised;
Genesis-pad, BoosterGrip/Joy 2B+, and all non-digital peripherals remain
unverified. Exact hashes, query counts, state results, and retained evidence
locations are recorded in
[`LIBRETRO_INPUT.md`](../crates/lunchbox-controller-probe/LIBRETRO_INPUT.md) and
[`LIBRETRO_PERSISTENCE.md`](../crates/lunchbox-controller-probe/LIBRETRO_PERSISTENCE.md).

## FCEUmm and Mesen save/state behavioral probe

A separate bounded probe on 2026-09-12 exercised the same exact installed
FCEUmm and Mesen core binaries through the libretro API. Every worker was a
fresh process with private `save`, `state`, and `system` directories under
`target/runtime-evidence/retroarch-flatpak/<core>`. It verified memory after
reload; file creation or a successful serialization return was not accepted by
itself.

Persistent save RAM used an original 24,592-byte battery-backed NROM diagnostic,
SHA-256
`0ae807dfe80d7178f082e0d6297e94b145d2880ff5a974fa5c1b26f6bf1beaaa`.
The first process executed the cartridge and published `LBSR01` from the core's
8,192-byte save-memory region. Its `.srm` had SHA-256
`35f57192c251084b91b1a5fbfda019c4294db3266df377689c96a5769317745e`.
A second process loaded those bytes before running the cartridge; the executing
program observed the retained signature, incremented it, and published
`LBSR02`. The updated `.srm` had SHA-256
`de5232a4b6205e474f7c169fda3dc0f1a22f3edd770c798340f7ee4571d2e124`.
Both exact cores produced the same independently checked transition.

State behavior used the real Faxanadu NES image already available on the test
host, SHA-256
`9e5f12c5fb7f6aa2b360538038abc682488511293e79b9bd2ef199877a8c6d1d`.
After 180 frames, the probe wrote `LBSTATE1` to NES RAM, serialized the core,
replaced those bytes with `MUTATED!`, and required an immediate load to restore
`LBSTATE1`. A second fresh process first observed unrelated bytes at the same
address and then required the persisted state to restore `LBSTATE1` again.
FCEUmm's 13,772-byte state had SHA-256
`d7c665ad5e6cf0d876edc2c5b8653edf883dc7313dcd1f334b3633fc1de5fffa`;
Mesen's 35,840-byte state had SHA-256
`779f3ebb5a906fda3bd17cbcd7a6e1e61be0c5546dfc07cc529620d95c65495e`.

These results establish core-level save-RAM persistence and behavioral state
restoration for these exact Linux-installed binaries. They do not establish
RetroArch GUI or frontend-path behavior, sync export/restore, firmware handling,
or compatibility with other games, cores, versions, or hosts.

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
- [Beetle PSX input and memory maps](https://github.com/libretro/beetle-psx-libretro/blob/82d8e051d1c7741a18d930be90e458b48abaa9a1/libretro.c)
