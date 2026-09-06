# Real-core input diagnostics

`lunchbox-libretro-input` is an opt-in, separate-process test harness. It loads an
explicitly supplied trusted core, verifies its SHA256, and runs an original
diagnostic. The default mGBA backend runs an ARM program from memory, copying the
emulated GBA KEYINPUT register to EWRAM alongside an execution marker. The Game
Gear backend runs an original Z80 program in Genesis Plus GX. Those two backends
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
