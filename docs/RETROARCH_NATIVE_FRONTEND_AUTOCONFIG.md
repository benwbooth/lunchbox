# Native RetroArch frontend-autoconfiguration boundary

Lunchbox supports exactly two native macOS/Windows RetroArch controller launch
profiles through this path:

- `retroarch:nestopia:nes-2player`
- `retroarch:nestopia:nes-4player`

Both are explicit ordinary `.nes`, `.unf`, or `.unif` cartridge modes using
Nestopia Gamepad device 257. They do not infer cartridge peripherals from the
filename or attached-controller count. FDS, NSF, VS coin, Famicom microphone,
Auto device 1, and every non-pad peripheral remain outside this adapter.

## Audited binary boundary

The production path fails closed unless both native binaries match the audited
pair for the host:

| Host | RetroArch frontend | Nestopia core |
| --- | --- | --- |
| macOS | official universal Metal 1.22.2, `ed90b54434a2899de0ddbfed59f335255fb462c8691bd970a1a761ebb5656d65` | `31bdba996461c5214e706ca1c674138976f6932af0a4da1de431b031cf706540` |
| Windows | official x86_64 1.19.1, `738ca659d2360cedbc62bab7b53c6e9bb20c7d92dfe3de743fa4f3b1fa218e7b` | `58445c86e4f1858bbe5a68eb4f7b120419f1418a1a9f7743d81076321fafa296` |

The executable and `-L` core are hashed during preparation and again immediately
before spawn. The prepared content is hashed too, and the final program, argv,
core path, and content path must remain identical. Windows additionally pins the
length and SHA-256 of all 59 executable/runtime files in the official 1.19.1
distribution closure. It rejects any extra `.dll` or `.exe` beside
`retroarch.exe`, and any extra loadable file under `platforms`, both during
preparation and at the pre-spawn check. Ordinary mutable config/data files and
directories such as `retroarch.cfg`, cores, assets, logs, saves, and states may
still coexist with the audited closure. Exact frontend pinning is also the macOS
non-Steam proof: a generic build cannot safely reveal the compile-time
`HAVE_STEAM` portable-layout branch.

## Configuration and ownership

RetroArch retains ownership of its input driver, joypad driver, autoconfiguration
database, manual physical bindings, and device enumeration. Lunchbox writes no
`input_playerN_*_btn`, axis, joypad-index, driver, autodetection, save, or state
keys. Its private append layer disables remaps/overrides and save-on-exit, records
the reviewed frontend topology, and points at a private copy of the effective
core options with `nestopia_button_shift=disabled` overlaid. Because RetroArch
does not consume `input_libretro_device_pN` from an ordinary global append layer
at startup, the same four exact device choices are also passed as
`--device=1:...` through `--device=4:...`: 257/257/0/0 for two players and
257/257/257/257 for Four Score. User-supplied device arguments remain rejected.

Windows main-config precedence is captured by successful complete reads beside
`retroarch.exe` and then `%APPDATA%\retroarch.cfg`; unreadable/directory/dangling
local candidates fall through. macOS captures the adjacent `portable.txt` and
`Contents/Info.plist` decision inputs; `:/` expands from the `.app` bundle root.
The exact inherited `HOME` is required. The complete prepared launch environment
is retained and must be byte-for-byte identical at the pre-spawn boundary; a
late `HOME`, `APPDATA`, or unrelated-variable addition/removal/change aborts.
Every applicable game/folder/core/global options candidate is captured before
selection so later appearance, disappearance, symlink replacement, or byte drift
also aborts launch.

Private directories/files are `0700`/`0600` on Unix. Windows applies and reapplies
a protected owner-plus-LocalSystem DACL at the pre-spawn boundary. The session
retains all private files until the child exits. Existing physical mapping,
savefile, and savestate paths are preserved.

## Evidence status

The checked-in Windows native oracle passes through the exact production helper
against the pinned official RetroArch 1.19.1 frontend, its complete 59-file
runtime closure, and the pinned Nestopia core. It generates both diagnostic ROMs
from the shared Rust builders. The two-player profile passes baseline, player-one
A, player-two B, and simultaneous input; the Four Score profile passes independent
P1/P2/P3/P4 input with observed active-low bytes FE/FD/FE/FD, all four frontend
ports at device 257, and the pre-existing `nestopia_select_adapter=ntsc` option
preserved. The same run proves 8 KiB SRAM across a fresh process and
same-process/fresh-process RASTATE1 restoration. It rejects frontend, core,
runtime-file, content, argv, environment, source-config, private-config, and
unexpected runtime-DLL drift and proves child/private-session cleanup. The
retained Windows report is
`C:\lunchbox-oracle\rust-native-010\retroarch-native-windows-report.json`
(5,221 bytes, SHA-256
`c48aad1a575e5efaf2aa3c335681b9e6250ffa45114737420575d3c50e94847c`).

The checked-in macOS oracle passes through the same production helper against the
pinned official RetroArch 1.22.2 bundle and Nestopia core. Eight launches prove
the two-player baseline then independent P1 A and P2 B (FF/FF, FE/FF, FF/FD),
and the Four Score baseline then independent P1 A, P2 B, P3 Start, and P4 Right
(FE/FD/F7/7F). All four Four Score ports use device 257 and the existing
`nestopia_select_adapter=ntsc` option is preserved. The run also proves 8 KiB
SRAM generation one to two across processes; a 5,408-byte state restores
`LBSTATE1` after same-process mutation and in a fresh process; sealed bundle
resource and late environment drift are rejected; source paths remain unchanged;
and no owned process or temporary root remains. The retained macOS report is
`/Users/ben/lunchbox-runtime-audit-20260913-macos-retroarch-frontend-oracle.Q8gJrN/evidence/retroarch-native-macos-automated-report.json`
(24,853 bytes, SHA-256
`06ac244a5231dce41a3dd5a20ca18c01de8703db38a6e4b2663fab8bce3f6af5`).
The oracle uses deterministic BSV replay, and all eight observed launches require
bounded forced termination, so it does not claim physical-controller mapping or
a normal user-requested shutdown path.

Direct-core Nestopia input and persistence also pass for both pinned cores, but
those runs alone do not prove this frontend adapter. Neither native frontend
oracle tests physical-controller enumeration/mapping, optional FDS firmware, or
real-provider save export/restore.
