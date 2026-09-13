# Libretro persistent-save and save-state oracle

`lunchbox-libretro-persistence` is an isolated diagnostic for an exact, trusted
libretro core. It proves two narrow contracts with original test programs:

- the emulated program creates battery-backed save memory, a fresh process loads
  those bytes before its first frame, and the emulated program consumes and
  updates them; and
- the core serializes and restores system RAM in the same process, then a fresh
  process restores the persisted state and the same RAM marker.

This is runtime evidence for the supplied core binary. It does not configure
RetroArch, exercise RetroArch's file naming or autosave policy, prove atomic
writes, or enable Lunchbox persistence. Loading a native core executes code, so
never point the tool at an untrusted library.

## Isolation and failure behavior

The public driver hashes the core first, creates a new private evidence tree, and
runs four bounded subprocesses: save creation, save reload, state creation, and
state reload. Every phase loads the same hash and exact core identity. The driver
kills only its own worker on timeout and rejects truncated output, nonzero exit,
unknown JSON fields, changed ROM/core/artifact hashes, unexpected memory sizes,
or missing observations. System, save, and XDG directories all point inside the
evidence tree. No user ROM, save, state, or RetroArch configuration is read.

The GBA program executes ARM instructions against the SRAM bus at `0x0E000000`
and publishes its result to EWRAM. Its cartridge contains the conventional
`SRAM_V113` identifier, but the proof depends on the bus transaction and returned
save-memory bytes, not the string alone. On blank SRAM it writes `LBSG01`; after
a frontend-style reload it increments that to `LBSG02`.

The Game Boy program is an original 32 KiB LR35902 cartridge with a valid
Nintendo header and an MBC1+RAM+battery declaration. It enables external RAM,
selects MBC1 RAM-banking mode and bank zero, performs the transaction at
`$A000`, and copies the result to WRAM at `$C000`. The five profiles pin each
core's exact save-memory capacity, system-memory view and offset, serialized
state size, and identity. No copyrighted game code or data is present.

The Game Gear program is a 64 KiB Sega-mapper cartridge. It executes
`$FFFC = $08`, mapping cartridge SRAM at `$8000`, performs the same transaction,
and copies the result to work RAM at `$C000`. This follows the exact
[Genesis Plus GX c2838c7 mapper implementation](https://github.com/libretro/Genesis-Plus-GX/blob/c2838c7dc4236fc2fe94e5dbd08b41486067918e/core/cart_hw/sms_cart.c).
The mGBA expectations follow the exact
[libretro mGBA e31759b source](https://github.com/libretro/mgba/tree/e31759b24e7a4e3899285ff720d7b573ac328ae7).

The NES program is an original 6502 NROM-128 cartridge with an iNES battery
flag and one 8 KiB PRG-RAM unit. It performs the transaction at `$6000` and
copies `LBSR\x01`, or `LBSR\x02` after a fresh-process reload, to zero-page RAM.

The SNES program is an original 65C816 LoROM cartridge whose internal header
declares ROM, 8 KiB RAM, and a battery. It accesses SRAM in bank `$70` and
copies `LBSG01`, or `LBSG02` after reload, to WRAM bank `$7E`. The diagnostic
ROMs contain no copyrighted game code or data.

## Usage

Build and run from the repository root:

```console
nix develop -c cargo build -p lunchbox-controller-probe --bin lunchbox-libretro-persistence

./target/debug/lunchbox-libretro-persistence \
  --system gba \
  --core /absolute/path/to/mgba_libretro.so \
  --sha256 EXPECTED_64_HEX_DIGITS

./target/debug/lunchbox-libretro-persistence \
  --system gameboy-gambatte \
  --core /absolute/path/to/gambatte_libretro.so \
  --sha256 EXPECTED_64_HEX_DIGITS

./target/debug/lunchbox-libretro-persistence \
  --system game-gear \
  --core /absolute/path/to/genesis_plus_gx_libretro.so \
  --sha256 EXPECTED_64_HEX_DIGITS

./target/debug/lunchbox-libretro-persistence \
  --system nes-fceumm \
  --core /absolute/path/to/fceumm_libretro.so \
  --sha256 EXPECTED_64_HEX_DIGITS \
  --expected-version '(SVN) 236ccdf' \
  --expected-state-bytes 13758

./target/debug/lunchbox-libretro-persistence \
  --system snes9x \
  --core /absolute/path/to/snes9x_libretro.so \
  --sha256 EXPECTED_64_HEX_DIGITS \
  --runtime-library /absolute/path/to/libz.so.1 \
  --runtime-library /absolute/path/to/libstdc++.so.6
```

The other accepted system values are `gameboy-mgba`, `gameboy-sameboy`,
`gameboy-skyemu`, `gameboy-vbam`, `nes-mesen`, `mesen-s`, and `bsnes`.
`--expected-version` is required when the core's reported version differs from
the pinned Linux identity in the verification tables below. If it is omitted,
the driver uses that pinned identity. An override changes only the expected
version string: the system-specific core name, caller-supplied SHA-256,
full-path contract, memory sizes, diagnostic observations, artifact hashes, and
four-worker lifecycle checks remain exact. The chosen version is passed to and
independently checked by every worker, so the option is not a wildcard or a
request to accept any build.

`--expected-state-bytes` is likewise required when an exact core build reports
a serialized-state size different from the pinned value in the tables. It must
be between 1 and 33,554,432 bytes. The selected size is forwarded to all four
workers, enforced during both state creation and fresh-process reload, and
recorded as `expected_state_bytes` at the top level, as
`save_state.expected_bytes`, and in every private worker report. This keeps a
platform- or revision-specific override auditable without allowing arbitrary
state sizes. For example, the observed arm64 FCEUmm `(SVN) 236ccdf` build uses
13,758 bytes; that observation still requires confirmation by the complete
four-worker run with the exact arm64 core SHA-256.

Some cores depend on libraries that are absent from the host process, or do not
declare all symbols that the host must provide. Pass each trusted library with
`--runtime-library` in dependency order. The driver canonicalizes and hashes
each dependency, loads it before the core with immediate, process-global symbol
visibility on Unix, and records it in every worker report. It does not search
for or select libraries implicitly. This global loading is required by the
verified Linux SkyEmu binary, which imports `pow` without a `libm` dependency.

Without `--output`, the retained results use a private
`/tmp/lunchbox-libretro-persistence-*` directory. `--output` accepts only a path
that does not yet exist. `--timeout-seconds` defaults to 15 and applies to each
worker independently.

The final JSON is printed and also stored as `results.json`. It records the
canonical core path, hash and identity; ROM/save/state paths, sizes and hashes;
the selected expected state size; the initial and fresh-process save
observations; and the same-process and fresh-process state markers. A top-level
`"status": "pass"` is emitted only after the driver re-hashes every retained
artifact.

## Exact buildbot verification on 2026-09-12

The Linux x86-64 `latest` artifacts staged under
`target/runtime-evidence/retroarch-buildbot-2026-09-12` passed:

| System | Core identity | Core SHA-256 | Save transition | State bytes |
| --- | --- | --- | --- | ---: |
| GBA | mGBA `0.11-219-e31759b` | `768921964037e0a40e8eab9e0d6eccad1b8a13d74bc37e9cae5543bb167d18c4` | 128 KiB autodetect capacity to 32 KiB SRAM; `LBSG01` to `LBSG02` in a fresh process | 430,144 |
| Game Boy | Gambatte `v0.5.0-netlink d9d6cd0` | `b8fba61ecbb840723a7c64d771196b37931c50ba4d98874cccedfd69a8aa27a6` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 59,650 |
| Game Boy | mGBA `0.11-219-e31759b` | `768921964037e0a40e8eab9e0d6eccad1b8a13d74bc37e9cae5543bb167d18c4` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 202,816 |
| Game Boy | SameBoy `1.0.3 8230189` | `26b3de38033e14cb2185f47811d38340bd47f56d55dd82cb14cdbd39a88a8208` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 252,666 |
| Game Boy | SkyEmu `adacd0788964ed89f5c43dcbc1f3cc26deec996c` | `bd5bf1f727d14e274a7f71b29e541d4d9188797c781a1557236aa94d54ed7c85` | 128 KiB save buffer; `LBSG01` to `LBSG02` in a fresh process | 246,416 |
| Game Boy | VBA-M `2.1.3 115defb` | `156dee1827dee4c36b8f88ab9ef6a9918b1e4e89a62195a4228fe0b1b982e31c` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 115,948 |
| Game Gear | Genesis Plus GX `v1.7.4 c2838c7` | `30abab06a9e1cfc26766a864fab83ee986cec1d6156c48dec13f9d03b46b2a6c` | 64 KiB pre-run capacity to six modified bytes; `LBSG01` to `LBSG02` in a fresh process | 1,036,288 |

The retained GBA and Game Gear reports are in
`target/runtime-evidence/libretro-persistence-mgba-2026-09-12-final` and
`target/runtime-evidence/libretro-persistence-game-gear-2026-09-12-final`.
Game Boy reports are under
`target/runtime-evidence/libretro-persistence-gameboy-CORE-2026-09-12`. SkyEmu
used host `libm.so.6` SHA-256
`95aafdf744c5bd6df251264d6a0b864159ba6f77c1f21997dd79dfb8ecc2e2bf`.
These ignored local artifacts are evidence, not repository fixtures. Re-run the
oracle after a core hash or embedded revision changes; the exact identity and
size contracts intentionally fail closed rather than treating a new build as
already verified. State restoration is a behavioral claim: state hashes for
Gambatte, SameBoy, and SkyEmu varied across valid reruns.

## Exact installed Flatpak verification on 2026-09-12

The following Linux x86-64 cores were read from
`~/.var/app/org.libretro.RetroArch/config/retroarch/cores`. Each passing row used
four fresh worker processes and a newly created mode-`0700` evidence root:

| System | Core identity | Core SHA-256 | Save transition | State bytes |
| --- | --- | --- | --- | ---: |
| NES | FCEUmm `(SVN) 5cd4a43` | `e7a17d1a5dacaeb7e02067f73aafab168f6a249a5dc0059ff16a5df44784d65c` | 8 KiB; `LBSR\x01` to `LBSR\x02` in a fresh process | 13,726 |
| NES | Mesen `0.9.9` | `552f8ab6ac1fd08bd555f589eb999be73a469c79ccfa929f884adb2cf3366b43` | 8 KiB; `LBSR\x01` to `LBSR\x02` in a fresh process | 35,840 |
| SNES | Snes9x `1.63 185488c` | `6e2d5fb3bbf57ef0a24834b36914187bea1a0b20f373da3adfdd5aebf75a4a99` | 8 KiB; `LBSG01` to `LBSG02` in a fresh process | 823,407 |
| SNES | Mesen-S `0.4.0` | `d43d7875316dc3f505ec160d5d34cc541fa899225c9227fed839697a7936cb5a` | 8 KiB; `LBSG01` to `LBSG02` in a fresh process | 550,912 |

Mesen and Mesen-S required Flatpak `libstdc++.so.6` SHA-256
`efca9ca0397af47196d837603f6ea29155ec3f150155559583c3ab696d2497b0`.
Snes9x required Flatpak `libz.so.1` SHA-256
`08b646c80eafa289f68199f2422e935219c0618e29d9027329a202bd28dcbac9`
followed by that `libstdc++`. Their canonical immutable runtime paths and hashes
are retained in `results.json`; the `active` symlinks used on the command line
are not treated as identity.

The retained passing reports are in:

- `target/runtime-evidence/libretro-persistence-nes-fceumm-2026-09-12-final3`
- `target/runtime-evidence/libretro-persistence-nes-mesen-2026-09-12-final3`
- `target/runtime-evidence/libretro-persistence-snes9x-2026-09-12-final3`
- `target/runtime-evidence/libretro-persistence-mesen-s-2026-09-12-final3`

## Exact macOS arm64 verification on 2026-09-12

The official Libretro macOS arm64 `latest` archives were downloaded on the
Apple Silicon test host and exercised there on macOS 26.5.1. Each row below is
a complete four-worker pass with private system, save, and state roots (schema
2 for the earlier systems, schema 3 for Game Boy):

| System | Core identity | Core SHA-256 | Save transition | State bytes |
| --- | --- | --- | --- | ---: |
| GBA | mGBA `0.11-219-e31759b` | `085350861044d9d2ef37634a7c201f57b4816fd343bdf29cdcd09bfb754b9218` | 32 KiB; `LBSG01` to `LBSG02` in a fresh process | 430,144 |
| Game Boy | Gambatte `v0.5.0-netlink d9d6cd0` | `19f088f910a89ffef80a26766f682dd01aa5ae81c95adca6926a3d9c10733a50` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 59,650 |
| Game Boy | mGBA `0.11-219-e31759b` | `085350861044d9d2ef37634a7c201f57b4816fd343bdf29cdcd09bfb754b9218` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 202,816 |
| Game Boy | SameBoy `1.0.3 8230189` | `581f35441d3263f53769d1a542cb3904656eff7ec2121fce3a17cacb8a282cb3` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 252,666 |
| Game Boy | SkyEmu `adacd0788964ed89f5c43dcbc1f3cc26deec996c` | `64778cf538f741dc5a55de2130390c196083fdb5346bec916b83d32accb9a24c` | 128 KiB save buffer; `LBSG01` to `LBSG02` in a fresh process | 246,416 |
| Game Boy | VBA-M version string ` 115defb` | `880d40f6338a7c60544b9c4662f8a950ea457bcd337dbc17240a03078328087f` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 115,948 |
| Game Gear | Genesis Plus GX `v1.7.4 c2838c7` | `0f4367774eddca7f6eb569648f9adc85cd62662577184634034be326199500d3` | six modified bytes; `LBSG01` to `LBSG02` in a fresh process | 1,036,288 |
| NES | FCEUmm `(SVN) 236ccdf` | `8afebce8967bb81c4c11fc9c930756e304c3ea81db89cc9607d38a2744da861a` | 8 KiB; `LBSR\x01` to `LBSR\x02` in a fresh process | 13,758 |
| NES | Mesen `0.9.9` | `3849098df9baf3b37fb58e27049c05d39ff4c4ffa63f0d739294188ba601c5d5` | 8 KiB; `LBSR\x01` to `LBSR\x02` in a fresh process | 35,840 |
| SNES | Snes9x `1.63 890b5d4` | `0f8fe5bf4e9ee72f8a73b00439884126c5358b98f4509fd19dd3d7aac26b3e` | 8 KiB; `LBSG01` to `LBSG02` in a fresh process | 823,407 |

The retained earlier reports are under
`/Users/ben/lunchbox-runtime-audit-20260912/evidence` on that host. Game Boy
reports are under
`/Users/ben/lunchbox-runtime-audit-20260913-macos-gameboy/evidence` on that
host. The FCEUmm
state-size drift from the Linux binary is why the exact
`--expected-state-bytes 13758` override exists; every worker enforced it. The
Snes9x and Genesis Plus GX serialized-state hashes changed across hosts or
reruns even though the behavioral marker restoration passed, so no byte-level
cross-build state determinism is claimed.

bsnes `115`, SHA-256
`ffd2898ddf27fbcac962e9e10a65e7562e8c1a7c2b795a0d93dc32a3ebfb8e8b`,
is deliberately unsupported by this direct-memory oracle. The exact installed
core returned null pointers and zero sizes for both save RAM and system RAM, so
the oracle stopped in its first worker and emitted no passing report. This
matches the exact upstream implementation: both libretro memory functions
unconditionally expose nothing and bsnes relies on its own file-save path
([source](https://github.com/bsnes-emu/bsnes/blob/7d5aa1e656b9171524d01b1b22917197d8121cb4/bsnes/target-libretro/libretro.cpp#L771-L783)).
This is an oracle-interface limitation, not evidence that bsnes cannot persist
SRAM or serialize state. Supporting bsnes requires a separate file-based
diagnostic with equally strict isolation and fresh-process checks.

## Exact Windows x86-64 verification on 2026-09-12

GitHub Actions run
[`34743651414`](https://github.com/benwbooth/lunchbox/actions/runs/34743651414)
executed the schema-3 four-worker Game Boy oracle against official Libretro
Windows x86-64 `latest` artifacts on separate Windows Server 2025 VMs. The
downloaded evidence archives were then independently checked: each core DLL and
source ZIP matched `download.json`, and each retained ROM, save, and state file
matched its report hash.

| System | Core identity | Core SHA-256 | Save transition | State bytes |
| --- | --- | --- | --- | ---: |
| Game Boy | Gambatte `v0.5.0-netlink d9d6cd0` | `c15eb6dc323b08610e8241ace11135d9fe8e1c8a3190541394f15457cdac59e1` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 59,650 |
| Game Boy | mGBA `0.11-219-e31759b` | `d5a3fcc915609ab5c81ede3cd1d0a9ea7a7670d3a7e325990297a16d6e987a33` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 202,816 |
| Game Boy | SameBoy `1.0.3 8230189` | `5b184f0bfa4a0bcf614c996cfa12985e60144722df815be2e9c8cd90bc259bfb` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 252,666 |
| Game Boy | SkyEmu `adacd0788964ed89f5c43dcbc1f3cc26deec996c` | `a9f020507fa90551107a40fb00c05e9d20f9bfb2140319aae9f5a9892c2973bc` | 128 KiB save buffer; `LBSG01` to `LBSG02` in a fresh process | 246,416 |
| Game Boy | VBA-M `2.1.3 115defb` | `a88130470c10aa4f4e34fd39c07f56630af7b596cc81ebd211796326b8f3af8f` | 32 KiB MBC1 save; `LBSG01` to `LBSG02` in a fresh process | 115,948 |

Every Windows state size exactly matched the pinned Linux and macOS size for
the same core revision. This is still behavioral state-restoration evidence,
not a claim that serialized bytes remain identical across processes, reruns, or
platform builds. Save synchronization remains separately blocked until a real
provider export and restore is exercised.
