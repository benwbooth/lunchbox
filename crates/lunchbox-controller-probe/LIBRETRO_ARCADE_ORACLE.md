# Exact-core arcade input and state oracle

`lunchbox-libretro-arcade-oracle` is a bounded direct-libretro diagnostic for
the installed FBNeo and MAME cores. It does not replace either core, RetroArch,
or Lunchbox input routing. It executes an explicitly trusted native core in
hidden worker processes, so use it only with core files whose hashes you trust.

The oracle uses the user's existing `1943.zip` in place. It never copies the
ROM into this repository or its private evidence tree. Every worker receives
new mode-0700 system, save, content-root, and state directories. The retained
evidence contains JSON and one serialized emulator state, not a ROM copy.

## Pinned Linux inputs

The built-in profiles fail closed on all of these values:

| Profile | Core identity | Core SHA-256 | Exact source | State bytes |
| --- | --- | --- | --- | ---: |
| `fbneo` | `FinalBurn Neo` / `v1.0.0.03 260417 GITe923538` | `3555759523d6da5f78012c6846921ac27884b03264604387d07a4877579a177e` | [`e9235389cede90638ad2726cfe80660841b23425`](https://github.com/libretro/FBNeo/tree/e9235389cede90638ad2726cfe80660841b23425) | 14,256 |
| `mame` | `MAME` / `0.287 (a891bc3b)` | `8bcc096667a3c24baefece40b610f981a5c8de13447af486b0ad9381e28302e7` | [`a891bc3b98c5a9f00848c953c8768007c6d339cb`](https://github.com/libretro/mame/tree/a891bc3b98c5a9f00848c953c8768007c6d339cb) | 4,568,390 |

The byte-pinned user set is
`/mnt/roms/emudeck/Emulation/roms/arcade/1943.zip`, 805,560 bytes, SHA-256
`e44b89e80bf8bccc4f16476d7254d959d937a1668d27ec564d57103ac94fba6f`.
Both exact cores accepted that set. A basename, size, core version, or successful
load cannot substitute for either SHA-256 check.

## What is proved

The core supplies topology after loading the pinned game. FBNeo advertises two
`Classic` devices (`RETRO_DEVICE_ANALOG`, ID 5) and queries their buttons through
the joypad callback. MAME advertises one generic `RetroPad` controller-info entry
but publishes descriptors for eight ports; the game-relevant first two ports are
tested directly. MAME's controller array includes its null terminator in the
reported count, which this bounded reader accepts without dereferencing it.

The exact P1/P2 topology exercised in both cores is:

| Arcade input | Libretro address |
| --- | --- |
| Coin 1 / Coin 2 | port 0 / 1, joypad Select, ID 2 |
| Start 1 / Start 2 | port 0 / 1, joypad Start, ID 3 |
| Up, Down, Left, Right | each player port, IDs 4, 5, 6, 7 |
| Fire 1, Fire 2 | each player port, B / A, IDs 0 / 8 |

For every stimulus, the worker restores a serialized baseline, runs a neutral
continuation, then runs the input continuation twice. An input is promoted as
distinguished only when both input runs are byte-identical, differ from neutral
in exported system RAM or rendered frame pixels, and do not collide with another
control in that stage. A changed serialized state alone is recorded but is not
accepted as gameplay proof. The worker searches bounded 60-frame intervals until
P1 and then P2 demonstrably accept gameplay input; it does not assume a title
screen delay.

The state gate serializes the active two-player baseline and compares a 30-frame
continuation against same-process and new-process restoration. Report fields keep
system RAM, video, and reserialized-state comparisons separate, so a partial
restoration cannot be described as exact.

## Linux results, 2026-09-12

Both per-button and negotiated bitmask modes completed with deterministic results:

| Core / callback | Distinguished responses | Coin boundary | Same-process state | Fresh-process state |
| --- | ---: | --- | --- | --- |
| FBNeo / individual | 20 of 22 | Coin 1 and Coin 2 each respond, but both reach the same credit RAM/video outcome | exact RAM, video, and reserialized state | exact RAM, video, and reserialized state |
| FBNeo / bitmask | 20 of 22 | same shared-credit collision | exact | exact |
| MAME / individual | 20 of 22 | Coin 1 and Coin 2 each change state/video, but reach the same rendered credit outcome; MAME's exposed 2 KiB RAM segment does not contain that response | exact RAM, video, and reserialized state | exposed RAM exact; video and reserialized state differ |
| MAME / bitmask | 20 of 22 | same shared-credit collision | exact | exposed RAM exact; video and reserialized state differ |

The 20 distinguished cases are P1/P2 Start plus Up, Down, Left, Right, Fire 1,
and Fire 2 for each player in active gameplay. Coin is proven responsive, but
the two physical coin ports are not promoted as semantically distinct because
1943 merges them into one credit pool. MAME fresh-process restoration remains an
explicit partial boundary: `retro_unserialize` succeeds and the exposed 2 KiB
RAM continuation matches, but the 30th rendered frame and reserialized state do
not match the original/same-process continuation. No cross-process parity claim
is made for that core.

Evidence roots from the final runs were:

- `/tmp/lunchbox-fbneo-individual-evidence-4`
- `/tmp/lunchbox-fbneo-bitmask-evidence`
- `/tmp/lunchbox-mame-individual-evidence-3`
- `/tmp/lunchbox-mame-bitmask-evidence`

Each `report.json` carries the exact core/content identities, descriptors,
effective options, callback addresses/counts, neutral/input/repeat hashes,
collisions, and the three state-restore observations.

## Run the pinned Linux profiles

The installed Flatpak cores need the matching KDE runtime libraries on the
loader path on this host:

```console
export LD_LIBRARY_PATH=/var/lib/flatpak/runtime/org.kde.Platform/x86_64/6.11/3adaa41de78d95076617f743099ec3851b5ae4fcdadbaaa8b7df1412c705c56c/files/lib/x86_64-linux-gnu

cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-arcade-oracle -- \
  --profile fbneo \
  --core /home/ben/.var/app/org.libretro.RetroArch/config/retroarch/cores/fbneo_libretro.so \
  --content /mnt/roms/emudeck/Emulation/roms/arcade/1943.zip \
  --output /tmp/lunchbox-fbneo-evidence

cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-arcade-oracle -- \
  --profile mame \
  --core /home/ben/.var/app/org.libretro.RetroArch/config/retroarch/cores/mame_libretro.so \
  --content /mnt/roms/emudeck/Emulation/roms/arcade/1943.zip \
  --output /tmp/lunchbox-mame-evidence \
  --timeout-seconds 120
```

After each individual run succeeds, repeat the same command with `--bitmask`
and these new output paths (all other arguments remain byte-for-byte identical):

- FBNeo: `/tmp/lunchbox-fbneo-evidence-bitmask`
- MAME: `/tmp/lunchbox-mame-evidence-bitmask`

Add `--bitmask` for the negotiated mask path. Output directories must not exist.
The supervisor kills and reaps a worker at its deadline; a native crash, missing
report, identity change, content change, state-size change, callback-mode mismatch,
nondeterministic response, or same-process restore mismatch is an error.

## Strict non-Linux build profile

Official cores built for another host may have legitimate version, hash, and
state-size differences. Supply the exact expected identity rather than weakening
the profile:

```console
cargo run -p lunchbox-controller-probe --bin lunchbox-libretro-arcade-oracle -- \
  --profile fbneo \
  --core /absolute/path/to/fbneo_libretro.dylib \
  --expected-core-sha256 EXACT_64_HEX_SHA256 \
  --expected-core-version 'EXACT retro_get_system_info VERSION' \
  --expected-state-bytes 14256 \
  --expected-source-commit EXACT_40_HEX_SOURCE_COMMIT \
  --content /absolute/path/to/1943.zip \
  --output /tmp/lunchbox-fbneo-macos-evidence
```

`--expected-core-sha256` and `--expected-core-version` are an all-or-nothing
pair. The core name remains fixed by `--profile`; the content hash and all
behavior assertions remain fixed. `--expected-state-bytes` defaults to the Linux
value above. If the exact foreign build reports a different size, the first
worker fails before testing and prints both the observed and expected byte counts;
rerun only after adding that observed size as an explicit per-build expectation.
`--expected-source-commit` is optional only when the full commit is unavailable;
the report then leaves it null rather than borrowing the Linux provenance.

This is core-level evidence. It does not prove RetroArch configuration paths,
frontend hotkeys, Lunchbox player assignment, cloud persistence, or behavior of
another core/content hash.

## Apple Silicon results, 2026-09-12

The official Libretro arm64 buildbot dylibs were downloaded and executed on a
macOS 26.5.1 M1 host. Their exact profiles are:

| Profile | Exact identity | SHA-256 | Exact source | State bytes |
| --- | --- | --- | --- | ---: |
| FBNeo | `FinalBurn Neo` / `v1.0.0.03  GITa251c76` (two spaces before `GIT`) | `38f382d2c08c21491dbdc81839f36f0a3e995d7aa147e975bc9e03dc54bf7b61` | [`a251c76229f1637e433b93e29845039752771b6d`](https://github.com/libretro/FBNeo/tree/a251c76229f1637e433b93e29845039752771b6d) | 15,288 |
| MAME | `MAME` / `0.289 (4fc9a931)` | `de43859a4ab93ea7a1df3f920ae7b2657212b7357b31e6ed72754394198baa4b` | [`4fc9a9312baaf34963847f884961ad9793fbbc1d`](https://github.com/libretro/mame/tree/4fc9a9312baaf34963847f884961ad9793fbbc1d) | 4,568,435 |

The same 805,560-byte `1943.zip` used on Linux matched its pinned SHA-256 on
the M1. Both individual and negotiated-bitmask runs reproduced the Linux input
boundary: 20 of 22 channels were independently distinguished, while Coin 1 and
Coin 2 each responded but merged into the same shared-credit outcome. This game
exposes only two fire buttons, so these runs do not prove the complete contracted
six- or eight-button layouts.

FBNeo restored its arm64-specific 15,288-byte state exactly across both same-
and fresh-process continuations: system RAM, video, and the reserialized state
all matched. MAME restored its 4,568,435-byte state exactly in-process, and the
fresh process matched the exposed 2 KiB RAM, but the rendered frame and
reserialized state diverged in both callback modes. That repeats the Linux
cross-process failure and is not reported as deterministic state parity.

The retained reports are:

- `/Users/ben/lunchbox-runtime-audit-20260912/evidence/fbneo-macos-arm64/report.json`
- `/Users/ben/lunchbox-runtime-audit-20260912/evidence/fbneo-macos-arm64-bitmask/report.json`
- `/Users/ben/lunchbox-runtime-audit-20260912/evidence/mame-macos-arm64/report.json`
- `/Users/ben/lunchbox-runtime-audit-20260912/evidence/mame-macos-arm64-bitmask/report.json`

The first fail-closed runs, which established rather than guessed the two
arm64-specific state sizes, remain alongside them in directories ending in
`-state-size-mismatch`.

## Hosted Windows identity boundary, 2026-09-13

GitHub Actions run
[`34751199122`](https://github.com/benwbooth/lunchbox/actions/runs/34751199122)
executed the identity-only path on Windows Server 2025 x86-64. The official
archives and DLLs matched every pinned byte and reported identity:

| Profile | Archive SHA-256 | Exact DLL identity | DLL SHA-256 |
| --- | --- | --- | --- |
| FBNeo | `8389e62bbd370282ea408db1498e0e64b443c346530e7fee650d551c5aaffc76` | `FinalBurn Neo` / `v1.0.0.03 260904 GITa251c76` | `4e097b6a1587ad58c3292e885ce218ec8eb8fbecebdf82a51489eb16f95ac3ca` |
| MAME | `77d38183ba995e8bb3e2f514e680d30645eae3b8aad78d6b62026d166c664590` | `MAME` / `0.289 (4fc9a931)` | `b38c9ebb2cf679745701569c511f9c381e1f4f3f43e0212fe728397b0badbead` |

The repository did not have the private `LIBRETRO_ARCADE_1943_URL` secret, so
the workflow deliberately downloaded no game bytes and recorded both runtime
audits as `blocked`. The earlier local Wine preflight is not promoted as Windows
runtime evidence. Controller responses and save-state behavior therefore remain
blocked on hosted Windows until the byte-pinned set can be supplied without
publishing it. Identity reports are retained under
`target/runtime-evidence/libretro-arcade-windows-2026-09-13-run-34751199122`;
the retained artifact contains no ROM bytes or secret URL.
