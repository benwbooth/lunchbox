# Real-core input diagnostics

`lunchbox-libretro-input` is an opt-in, separate-process test harness. It loads an
explicitly supplied trusted core, verifies its SHA256, and runs an original
diagnostic. The default mGBA backend runs an ARM program from memory, copying the
emulated GBA KEYINPUT register to EWRAM alongside an execution marker. The Game
Gear backend runs an original Z80 program in Genesis Plus GX. No BIOS or commercial
game bytes are present, and no ROM or firmware download is needed.

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
ROM/callback storage. System/save paths refer to a private temporary directory;
abnormal termination may leave that temporary directory behind.

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
