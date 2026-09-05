# Real-core input diagnostics

`lunchbox-libretro-input` is an opt-in, separate-process test harness. It loads an
explicitly supplied trusted mGBA core, verifies its SHA256, and runs an original
ARM diagnostic from memory. The program copies the emulated GBA KEYINPUT register
to EWRAM alongside an execution marker. No Nintendo logo, BIOS, or commercial
game bytes are present, and no ROM or firmware download is needed.

The first backend tests ten standard buttons, releases after every press, and
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
