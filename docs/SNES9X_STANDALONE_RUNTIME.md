# Snes9x standalone runtime evidence

Tested 2026-09-12 on Linux/x86_64 against the system Flatpak
`com.snes9x.Snes9x` version 1.63. This document deliberately separates the
installed emulator's observed behavior from Lunchbox's complete production
adapter integration.

## Result boundary

| Gate | Status | Evidence |
| --- | --- | --- |
| Installed runtime identity | `passed` | Flatpak commit `79ac5baaad32c5f7289f36277f2f643bb51085c4ecb41fe653679f885612b725`, runtime `org.freedesktop.Platform/x86_64/24.08`; `/app/bin/snes9x-gtk` SHA-256 `6b8566f1ffc718058fdc3f322bf56b5b5556fd00f3b69ad9db94c4a08802fd2c`. |
| Production configuration renderer | `passed` | The ignored `write_isolated_runtime_oracle_config` test rendered two complete player banks from `configuration::render`; the pre-launch config SHA-256 was `8e3c5f2f17a36464c97ef8a7d89bb2aaa67324ae533f06c1c69894ed89af3740`. |
| Direct runtime controls | `passed` | The generated ROM observed the expected ordered one-hot SNES words for all 12 named controls on both players, not merely their aggregate mask. |
| Direct runtime players | `passed` | Player 1 and player 2 produced separate 12-entry hardware logs with the same expected sequence; both aggregate masks were `fff0`. |
| Ordinary startup without BIOS | `passed` | A plain LoROM launch with no BIOS argument reached `Port 1: Pad #1`, `Port 2: Pad #2`, `Map_LoROMMap`, and created SRAM. |
| Fresh-process SRAM reload | `passed` | Three separately launched Snes9x processes observed persisted boot counts 1, 2, and 3 before state restoration. |
| Save-state mutate/restore | `passed` | Slot 000 restored the ROM-observed boot and mutation bytes from `03 5a` to `01 00`; the slot hash did not change and an exact undo file was produced. |
| Lunchbox production adapter with Flatpak | `blocked` | The adapter rejects every `EmulatorExecutable::Flatpak` before launch. Removing that guard is unsafe because the existing host-side probe and target Flatpak enumerate different joystick indices (details below). |
| Multitap players 3–5, hats/axes, hotplug, physical pads | `not_tested` | The oracle intentionally covered two 12-button virtual pads and a stable topology only. |

The proposed shared-ledger values are therefore: renderer `passed`; direct
runtime controls `passed`; direct runtime players `passed`; no-BIOS startup
`passed`; SRAM reload `passed`; state restore `passed`; production adapter
integration `blocked`; unexercised variants `not_tested`.

## Original diagnostic

The checked-in assembly in
`crates/lunchbox-app/src/controller_snes9x/runtime_oracle/diagnostic.s` reads
the SNES automatic joypad registers `$4218` and `$421a` during NMI. It records
current and cumulative values, isolated one-bit values, previous values, and a
bounded ordered rising-edge log in cartridge SRAM. It also stores an `LB`
signature, a boot counter, and a `$5a` mutation marker when SNES B is seen.
This is console-visible state; it does not inspect Snes9x memory or the
generated host configuration.

The final 32 KiB LoROM was built from the checked-in `.s` and `.cfg` files with
cc65 `ca65`/`ld65`. Its SHA-256 is
`8d42522de6b9889a90ae608c98804d5dd7aaccdd80384067424606ead969852f`.
The internal checksum/complement words are `$8a11`/`$75ee` and sum to `$ffff`.
No commercial ROM, firmware, or BIOS was used.

The companion `virtual_pads.rs` creates two Linux/x86_64 uinput devices with
12 buttons apiece. `pulse-all` presses and releases buttons 0 through 11 in
order on player 1 and then player 2, with 120 ms between edges. It requires
write access to `/dev/uinput` and is test tooling, not application code.

The checked-in artifacts can be rebuilt independently with:

```sh
nix shell nixpkgs#cc65 -c ca65 diagnostic.s -o diagnostic.o
nix shell nixpkgs#cc65 -c ld65 -C diagnostic.cfg diagnostic.o -o diagnostic.sfc
rustc --edition 2024 virtual_pads.rs -o virtual-pads
```

Run those commands from the `runtime_oracle` directory. The opt-in renderer
test additionally requires distinct existing source/output-parent paths below
the canonical `TMPDIR`, one-based joystick numbers in
`LUNCHBOX_SNES9X_ORACLE_JOYSTICKS`, and a not-yet-existing output file.

## Controls and player association

The production renderer was given the target runtime's observed one-based
slots 3 and 6. Its isolated source was
`/tmp/lunchbox-snes9x-runtime-01a098ce/production-render/source/snes9x.conf`
(SHA-256 `dbbf0010fc60e0f9b73aae38f5885a9d9fee47d186a9983006b0c7ba4d1f43d9`).
The rendered config was
`/tmp/lunchbox-snes9x-runtime-01a098ce/production-render/config/snes9x/snes9x.conf`.
Snes9x was run with private HOME/XDG roots, `--nofilesystem=host:reset`,
`--nofilesystem=home`, access only to the oracle's `/tmp` tree, and input/shm
devices. Snes9x was free to rewrite only this private rendered config; its
post-run SHA-256 is
`bc084ed1c66bddec5011ac51f4f2429f8c477c348d55ec2f58a042b72fc45a57`.

For both players, the ordered log count was 12 and the exact words for
Lunchbox's declared order `Up, Down, Left, Right, Start, Select, A, B, X, Y,
L, R` were:

```text
0800 0400 0200 0100 1000 2000 0080 8000 0040 4000 0020 0010
```

The aggregate and isolated-one-bit masks for each player were both `fff0`, and
the B mutation byte was `5a`. The retained 32 KiB evidence file is
`/tmp/lunchbox-snes9x-runtime-01a098ce/production-render/sram/ordered-mapping-evidence.srm`,
SHA-256 `085b77d34a586f19f88ea5c795559adefb62d7a43170e0c73eff6de4cc893398`.
This exact ordered comparison is what establishes name-to-bit association;
the aggregate `fff0` values alone do not.

## SRAM and state paths

The persistence/state sequence used the same final ROM and rendered private
config, starting without an SRAM file:

1. Process 1 booted with signature `4c 42`, boot byte `01`, and byte `00`,
   saved slot 000, and exited. The 32 KiB SRAM at
   `/tmp/lunchbox-snes9x-runtime-01a098ce/production-render/sram/ordered-diagnostic.srm`
   then had SHA-256
   `fe77c641f512336c015ee931323ca78fca401af0b746dec2e2b1b536550e97bc`.
2. A fresh process loaded that SRAM, incremented the boot byte to `02`, and a
   player-1 B pulse changed the mutation byte to `5a`. After exit the same path
   had SHA-256
   `60ae2121311f6bb382be78b145da12a3f30c6e4cc7c01fee9a2a5faf7f908bd0`.
3. A third fresh process reached `03 5a`. Quick-load F6 restored the slot's
   ROM-visible state to `01 00`, then exit wrote a final SRAM SHA-256 of
   `d1d4ca64bfc6616bbd41fdc501e1ea7b668c495a7bebfed67048b58442a6b32e`.

Slot 000 is
`/tmp/lunchbox-snes9x-runtime-01a098ce/production-render/savestate/ordered-diagnostic.000`,
2,684 bytes, SHA-256
`dcf82a564b0151db3ef04a9d9166320a974720a86d81dac3e742e2b52e128448`.
Its hash was identical before and after loading. The load created
`ordered-diagnostic.undo` in the same directory, 2,693 bytes, SHA-256
`0e43e8c105be80fc1c189b42ce7dbfdbb43587e672068f7038c0fd5ee33bd38c`.

## Why production Flatpak integration remains blocked

`controller_snes9x/native_command.rs` currently accepts only
`EmulatorExecutable::Native` and returns
`Snes9x GTK calibrated launch requires native Linux, not Wine/Flatpak` for the
installed target.

There is a deeper runtime reason not to delete that check. Inside the installed
Flatpak, Snes9x reported these one-based slots:

```text
1 ASRock LED Controller              /dev/input/js0
2 ASUSTeK ROG CHAKRAM X              /dev/input/js1
3 Lunchbox Snes9x hardware oracle P1 /dev/input/js3
4 Microsoft X-Box 360 pad            /dev/input/js4
5 Microsoft X-Box 360 pad            /dev/input/js5
6 Lunchbox Snes9x hardware oracle P2 /dev/input/js6
```

The current host-side controller probe, forced to load the exact Flatpak SDL2
2.32.10 library, instead reported four zero-based devices: P1 index 0, Xbox
indices 1 and 2, and P2 index 3. The sandbox's different udev visibility makes
the same SDL binary enumerate a different set. The existing session would
therefore render P1/P2 as `Joystick 1`/`Joystick 4`, while the target actually
requires `Joystick 3`/`Joystick 6`.

The current host probe also cannot load that runtime library in its ordinary
environment (`libwayland-egl.so.1` is absent). Adding the Flatpak library
directory alone to `LD_LIBRARY_PATH` selects the Flatpak libc ahead of the Nix
probe's libc and fails on the GLIBC_PRIVATE symbol `__nptl_change_stack_perm`.
A diagnostic search path with the Nix libc directories first made the probe
run, but did not fix the enumeration mismatch.

A safe implementation must run the retained controller probe inside the
target Flatpak environment, validate the app/runtime deployments, inject a
launch-scoped private `XDG_CONFIG_HOME` plus its filesystem permission at the
Flatpak app boundary, and teach startup confirmation to verify the sandboxed
executable, config, SDL mapping, and device descriptors. Those changes span
the session, isolation, native-command, and startup modules; they are outside
this batch's permitted files. Production adapter integration therefore stays
`blocked`, while the direct installed-runtime gates above remain `passed`.

## Preservation check

The user's Flatpak config
`~/.var/app/com.snes9x.Snes9x/config/snes9x/snes9x.conf` had SHA-256
`c84749a20943926909346b0dd9140aa13dfa999080d3ba6ac1df0642d31fd231`
before testing and the same hash after all runs. No user ROM path was supplied
to the sandbox. All generated ROM, config, SRAM, and state paths were below
`/tmp/lunchbox-snes9x-runtime-01a098ce`.
