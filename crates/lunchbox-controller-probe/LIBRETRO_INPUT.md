# Real-core input diagnostics

`lunchbox-libretro-input` is an opt-in, separate-process test harness. It loads an
explicitly supplied trusted core, verifies its SHA256, and runs an original
diagnostic. The default mGBA backend runs an ARM program from memory, copying the
emulated GBA KEYINPUT register to EWRAM alongside an execution marker. The Game
Gear backend runs an original Z80 program in Genesis Plus GX. The NES backend
runs an original mapper-0 program in FCEUmm or Mesen. The SNES backend runs an
original 65C816 LoROM program in bsnes, Snes9x, or Mesen-S. Those four backends
need no BIOS. The PlayStation backend runs an original MIPS diagnostic through
user-supplied firmware. No commercial game or firmware bytes are included or
downloaded by this helper.

The GBA backend tests ten standard buttons, releases after every press, and
A+B/L+R combinations. It checks active-low hardware bits, not just whether a
configuration was accepted. Run both frontend callback paths:

    nix develop -c cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-input -- --core /absolute/trusted/mgba_libretro.so --sha256 EXPECTED_SHA256
    nix develop -c cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-input -- --core /absolute/trusted/mgba_libretro.so --sha256 EXPECTED_SHA256 --bitmask

Output records core identity/hash, callback request counts, reported memory size,
and expected/observed KEYINPUT values. The CLI's watchdog terminates its own
process after 15 seconds by default (`--timeout-seconds`, range 1–120). A core
that does not return from a callback cannot hang the Lunchbox GUI because this
helper is never loaded into that process. Native code is **not sandboxed** by the
helper: a matching hash identifies a file, not whether its publisher is trusted.
Use only trusted cores. Normal exits unload/deinitialize the core before releasing
ROM/callback storage. Save paths refer to a private temporary directory; system
paths do too except for PlayStation, which uses the explicitly supplied BIOS
directory. This is not a filesystem sandbox. Abnormal termination may leave the
temporary directory behind.

## Evidence and scope

On 2026-09-05 the official Libretro Linux x86_64 nightly mGBA core reported
`0.11-219-e31759b`, SHA256
`768921964037e0a40e8eab9e0d6eccad1b8a13d74bc37e9cae5543bb167d18c4`.
The initial individual and bitmask runs each passed all 26 observations.
The referenced source is [mGBA's libretro frontend](https://github.com/libretro/mgba/blob/e31759b24e7a4e3899285ff720d7b573ac328ae7/src/platform/libretro/libretro.c).
Its system-memory API reports 32 KiB even for the GBA EWRAM pointer; the harness
reads only eight bytes within the reported bounds, not an assumed 256 KiB span.

This verifies the **frontend RetroPad → core → emulated hardware** portion only.
It does not test physical-device calibration, OS/SDL enumeration, RetroArch's
configuration/remap processing, other cores, cartridge sensors, or rumble.
Those layers require separate evidence. Audio/video callbacks intentionally
discard output: this is a headless input diagnostic, not a game frontend.

## Game Gear / Genesis Plus GX

Add `--system gamegear` and supply a trusted `genesis_plus_gx_libretro.so` with its
expected SHA256. Run once normally and once with `--bitmask`, as above. This core
requires a file path, so the helper writes its original 32 KiB diagnostic cartridge
only into the private temporary directory and removes it on normal exit. The
`TMR SEGA` bytes are the cartridge format signature, not imported game artwork or
code. The helper selects Game Gear hardware and disables optional BIOS loading.

The Z80 program samples port DC bits 0–5 (directions and buttons 1/2) and port 00
bit 7 (Start), storing both bytes plus an execution marker in work RAM. Output's
`diagnostic` field distinguishes `gamegear-dc-00` from `gba-keyinput`; the existing
`expected_keyinput`/`observed_keyinput` fields contain masked register samples.
For Game Gear the low byte is DC, the high byte is 00, and the mask is `0x803f`.
Region/link-port bits are excluded. Cases cover all seven controls, releases,
1+2 and 1+Start combinations, and the absence of a Select gameplay action.

On 2026-09-05 the official Libretro Linux x86_64 nightly core reported
`v1.7.4 a7985a9`, SHA256
`a6da7c738dfa87708d173b2034b71b84368d6adf5a53126ebb5791933ce929bd`.
Both callback modes passed all 22 observations using explicit device 769
(MS Joypad 2 Button); the core exposed 8192 bytes of work RAM.
The pinned [frontend mapping](https://github.com/libretro/Genesis-Plus-GX/blob/a7985a9c4278ac352f8ca7bb4d3cc6b36e9e3e7d/libretro/libretro.c)
and [hardware I/O](https://github.com/libretro/Genesis-Plus-GX/blob/a7985a9c4278ac352f8ca7bb4d3cc6b36e9e3e7d/core/io_ctrl.c)
provide the source contract. This does not verify physical input, RetroArch
configuration processing, other Genesis Plus GX systems, or link-cable hardware.

## NES / FCEUmm and Mesen

Select `--system nes` with an exact-hash FCEUmm or Mesen core. Choose
`--nes-topology two-player` or `--nes-topology four-score`, and run each once
normally and once with `--bitmask`. The probe selects each core's advertised
standard-controller device, validates controller choices and input descriptors,
then independently stimulates every connected port while holding conflicting
inputs on the others. Completed serial samples are published from emulated RAM
only after all four controller reads finish; bytes are compared against an
independent active-low NES register mapping.

On 2026-09-12 all four modes passed against each installed Flatpak updater core:
FCEUmm `(SVN) 5cd4a43`, SHA256
`e7a17d1a5dacaeb7e02067f73aafab168f6a249a5dc0059ff16a5df44784d65c`,
and Mesen `0.9.9`, SHA256
`552f8ab6ac1fd08bd555f589eb999be73a469c79ccfa929f884adb2cf3366b43`.
Each core passed 42/42 observations per two-player run and 84/84 per Four Score
run. Both exposed 2048 bytes of system RAM. FCEUmm left disconnected frontend
ports unqueried. Mesen's bitmask path queried all five ports, but deliberately
nonzero disconnected-port states never appeared in the emulated hardware bytes.
The Mesen binary requires its Flatpak C++ runtime when loaded directly from the
host. These runs do not establish physical-device capture, RetroArch remap or
launch behavior, firmware handling, saves, states, or sync behavior.

## SNES / bsnes and Snes9x

Select `--system snes` and choose `--snes-topology two-player` or
`--snes-topology multitap`. The latter means one joypad on physical port one and
a four-player Super Multitap on physical port two, for five frontend players.
The probe selects the exact advertised devices after loading content, validates
the core's physical-port choices and every connected frontend port's descriptors,
and holds different nonzero masks on the other players while checking each target.

The original 32 KiB LoROM manually latches and clocks `$4016`/`$4017` in the
hardware order B, Y, Select, Start, directions, A, X, L, R. It toggles `$4201`
bit 7 to sample both halves of the port-two multitap. Five completed 16-bit
words are published to WRAM with a generation marker; the frontend never
manufactures the expected hardware values. Snes9x exposes the diagnostic bytes
through its 128 KiB standard system-RAM region. bsnes 115 deliberately exposes
no standard memory region, so the probe takes a bounded `retro_serialize`
snapshot, requires exactly one diagnostic WRAM marker, and reads the adjacent
generation and controller words from that emulated-state snapshot.

On 2026-09-12 the installed Flatpak updater cores passed every supported mode:

- bsnes `115`, SHA256
  `ffd2898ddf27fbcac962e9e10a65e7562e8c1a7c2b795a0d93dc32a3ebfb8e8b`,
  passed 58/58 two-player and 145/145 multitap observations in individual mode.
  This revision does not negotiate libretro joypad bitmasks; `--bitmask` is
  rejected explicitly rather than reported as exercised.
- Snes9x `1.63 185488c`, SHA256
  `6e2d5fb3bbf57ef0a24834b36914187bea1a0b20f373da3adfdd5aebf75a4a99`,
  passed 58/58 two-player and 145/145 multitap observations in both individual
  and bitmask modes.

The source contracts are [bsnes v115](https://github.com/libretro/bsnes-libretro/tree/8e80d2f8a43e34a82931e25143b279e5fbcfaedc)
and [Snes9x `185488c`](https://github.com/snes9xgit/snes9x/tree/185488cd83aaf274752a742c94d45561cbecb7af).
The exact core hash identifies the binary that actually ran; these source
links define expected frontend/controller behavior and are not binary provenance.

As a supplemental bounded check, installed Mesen-S `0.4.0`, SHA256
`d43d7875316dc3f505ec160d5d34cc541fa899225c9227fed839697a7936cb5a`,
passed all 58 two-player individual-mode observations using its
128 KiB WRAM interface. Its advertised standard and multitap subclasses are
257 and 513, not the generic 1 and 257 used by the other two cores. It does not
negotiate bitmask input. Its five-player run was **not** promoted: the released
state read back `[0000, 0000, 0000, ffff, 0000]`, so the probe failed before
button cases. Its descriptor table also omits Start and Select even though the
two-player hardware path maps and passed both buttons. The expected adapter
contract is [Mesen-S 0.4.0](https://github.com/libretro/Mesen-S/tree/dd0287088c53e1e96e5818ed81160f6a958646a8).

This is direct-core evidence only. It does not cover physical controllers,
RetroArch configuration/remaps, the GUI launch path, game compatibility,
firmware, persistence, states, rumble, or other SNES peripherals.

## Exact macOS arm64 verification on 2026-09-12

The same original hardware-register diagnostics ran on macOS 26.5.1 on the M1
test host against official Libretro arm64 buildbot dylibs:

| System | Exact core | Modes and observations |
| --- | --- | --- |
| GBA | mGBA `0.11-219-e31759b`, SHA-256 `085350861044d9d2ef37634a7c201f57b4816fd343bdf29cdcd09bfb754b9218` | 26/26 in individual mode and 26/26 in bitmask mode |
| Game Gear | Genesis Plus GX `v1.7.4 c2838c7`, SHA-256 `0f4367774eddca7f6eb569648f9adc85cd62662577184634034be326199500d3` | 22/22 in individual mode and 22/22 in bitmask mode |
| NES | FCEUmm `(SVN) 236ccdf`, SHA-256 `8afebce8967bb81c4c11fc9c930756e304c3ea81db89cc9607d38a2744da861a` | 42/42 two-player and 84/84 Four Score observations in each of individual and bitmask modes |
| NES | Mesen `0.9.9`, SHA-256 `3849098df9baf3b37fb58e27049c05d39ff4c4ffa63f0d739294188ba601c5d5` | 42/42 two-player and 84/84 Four Score observations in each of individual and bitmask modes |
| SNES | bsnes `115`, SHA-256 `18900517569b4bd4a08c5a37dc8d2e3f1d8cf6891ab1bb786699867ecb635abf` | 58/58 two-player and 145/145 multitap observations in individual mode; bitmask mode unsupported |
| SNES | Snes9x `1.63 890b5d4`, SHA-256 `0f8fe5bf4e9ee72f8a73b00439884126c5358b98f4509fd19dd3d7aac26b3e54` | 58/58 two-player and 145/145 multitap observations in each of individual and bitmask modes |

Snes9x printed `Map_LoROMMap` from an exit handler after the four original JSON
reports. Every assertion exited successfully and the complete JSON prefix was
retained. The current Unix helper temporarily isolates native-core stdout; a
subsequent exact-hash M1 two-player rerun produced clean JSON and repeated all
58 observations. These are direct-core arm64 results; they do not promote
RetroArch GUI/remap behavior or physical-device capture.

## PlayStation / Beetle PSX

Select `--system psx`, a trusted Beetle PSX or Beetle PSX HW core and its expected
SHA256, and `--bios-dir /absolute/path/to/bios`. The directory must contain
`scph5500.bin`, `scph5501.bin`, and `scph5502.bin`, each 512 KiB. The helper records
their hashes and sizes; these checks identify the supplied files, not their
authenticity or redistribution rights. Obtain firmware independently and legally.

The original PS-X EXE polls both emulated controller ports and records packet
bytes in emulated RAM. Cases cover DualShock buttons and signed axes on both
ports, then a mixed digital/DualShock configuration. Digital mode is checked not
to request analog axes. Run both individual and `--bitmask` callback paths.
Results appear in `psx_observations`, with firmware identities in `firmware`.

This is a core-level input diagnostic, not a test of every game's compatibility
rules or of physical-controller discovery and launch-time configuration. The
application's prepared-disc compatibility checks and RetroArch launch oracle
cover different boundaries; see [the launch oracle](../../docs/CONTROLLER_RETROARCH_ORACLE.md).

## Game Boy / SameBoy

Select `--system gameboy` with a trusted SameBoy core and its expected SHA256.
The original 32 KiB LR35902 program samples both active-low halves of the Game
Boy's JOYP register into WRAM, along with an execution marker. Buttons occupy
the low byte's low nibble; directions occupy the high byte's low nibble. The
comparison mask is `0x0f0f`. The hardware expectation comes from
[Pan Docs' JOYP definition](https://gbdev.io/pandocs/Joypad_Input.html), independently
of the application's generated mappings.

The helper selects ordinary Game Boy hardware in its private diagnostic options.
It allows up to 240 frames for SameBoy's built-in open-source boot ROM to finish,
without modifying CPU state or supplying manufactured readback. The diagnostic
cartridge has a blank logo area: no Nintendo logo, commercial game, or external
firmware is included. A warning about absent `dmg_boot.bin` is expected before
the core falls back to its built-in boot ROM.

Each run checks all eight controls, releases, A+B, A+Right, and unassigned
shoulders twice: once with device 1, then with advertised joypad subclass 257.
The observation names record the selected device; there are 48 observations.
Run both callback modes:

```console
nix develop -c cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-input -- --system gameboy --core /absolute/trusted/sameboy_libretro.so --sha256 EXPECTED_SHA256
nix develop -c cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-input -- --system gameboy --core /absolute/trusted/sameboy_libretro.so --sha256 EXPECTED_SHA256 --bitmask
```

On 2026-09-06 the official Libretro Linux x86_64 nightly core reported
`1.0.3 8230189`, SHA256
`26b3de38033e14cb2185f47811d38340bd47f56d55dd82cb14cdbd39a88a8208`.
Both callback modes passed all 48 observations and exposed 8192 bytes of WRAM.
The individual run made 4432 individual input requests and zero mask requests;
the bitmask run made 277 mask requests and zero individual requests.
The pinned [upstream frontend](https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c)
matches the reported abbreviated source revision. The binary hash identifies the
actual runtime artifact; the report's source fields identify the expected contract.

This checks ordinary Game Boy emulated input, not SGB multiplayer, Game Boy Link,
all model variants, RetroArch's generated configuration processing, physical
controller calibration, or the desktop launch action. The application has
separate [option-dependent topology tests](../../docs/SAMEBOY_CONTROLLERS.md).

Reports additionally include the latest `input_descriptors` notification from
the core: port, device, index, id and copied UTF-8 description. Capture requires
a terminating record within 4096 entries and labels no longer than 1024 bytes.
Malformed capture prevents a successful report. This additive collection code
is exercised by the direct-core diagnostics above; it does not expand the
supported diagnostics to FBNeo or establish that every core control is described.
