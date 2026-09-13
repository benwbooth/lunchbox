# Nestopia UE Flatpak production-controller boundary

This adapter targets one installed deployment, not Nestopia UE or Flatpak in
general. The saved-setup editor, guided-controller reuse, launch registry, and
controller-probe supervisor dispatch are integrated for that deployment. Every
other package, version, host, controller topology, and display backend remains
outside this contract.

## Audited identity

- Upstream: `0ldsk00l/nestopia` 1.53.2, source commit
  `4470a2e99199d8010322eef4bf680fb3760f6eda`.
- The installed manifest fetches the `1.53.2` tag archive with SHA-512
  `b6bc3b464e4b160830963a1ff7fd97603883f3c95bada6d0cf46f759cc6d9e18974110a1737ab55223b4f3b06c3272510d2137d493ed16796c4ed3b16edbd04f`.
- Flatpak app: `ca._0ldsk00l.Nestopia`, commit
  `0f3d30d71419cee53f903b77944821d919fcfbe98aa3648bf8d12765bddd6169`.
- App executable SHA-256:
  `1b63638f9e19e007900ac7c81451dbaa734661f79e3033d7013dbec2509543ea`.
- Runtime: `org.freedesktop.Platform/x86_64/25.08`, commit
  `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`.
- Runtime SDL2 is `lib/x86_64-linux-gnu/libSDL2-2.0.so.0.3200.70`, SHA-256
  `6a2edbdfb43cea4c8e8d44033f05a626336123e426793f34a43d8d9576f9068d`.
- Runtime loader SHA-256:
  `a3b79fc634bbfdc51b5f1c6dc0013b14a7ceb98068efbd1f0055d9ca96d27cf6`.

Preparation and every launch re-read the deployed app/runtime commits and hash
the executable, SDL2 library, loader, source controller probe, and private
launch inputs. A mismatch is a refusal, not a compatibility fallback.

## FDS firmware boundary

Famicom Disk System content is a separate, firmware-gated path. For the exact
platform UUID `d01f03eb-cbf9-5847-92a6-f5fb9ba80b15`, canonical platform name
`Nintendo Famicom Disk System`, standalone emulator UUID
`73ad4eb8-0f5f-56ca-b839-8398db3e8d77`, emulator name `Nestopia UE`, Flatpak
app ID `ca._0ldsk00l.Nestopia`, and canonical `.fds` or `.FDS` content, Lunchbox
accepts a user-selected file only when it is an exact raw 8,192-byte image with
one of these SHA-256 identities:

- `99c18490ed9002d9c6d999b9d8d15be5c051bdfa7cc7e73318053c9a994b0178`
- `a0a9d57cbace21bf9c85c2b85e86656317f0768d7772acc90c7411ab1dbff2bf`

Both values come from the primary-source Mesen2 allowlist in
`UI/Interop/FirmwareTypeExtensions.cs` at commit
`b9fa69ddc6d0a331fb103fdb5eef6904305703c2`. The known CRC32 values
`5e607dcf` and `4df24a6c` are retained only as diagnostics; CRC32 never admits a
file. Lunchbox does not discover, download, or bundle a BIOS or disk image.

The installer rejects relative paths, symlinks, path-identity drift, wrong
ownership, and an existing unrecognized target. It publishes
`~/.var/app/ca._0ldsk00l.Nestopia/data/nestopia/disksys.rom` atomically without
replacement, with mode `0600`, effective-user ownership, and one hard link.
The status probe and general pre-spawn path reopen that installed file and
recheck its directory identities, ownership, mode, link count, size, and
SHA-256. The pre-spawn check runs after controller preparation and immediately
before either calibrated or ordinary process spawning; cancellation is checked
on both sides.

The catalog rule remains host-agnostic, so other Nestopia packages are exposed
as an explicit non-blocking manual configuration. They are never reported as
managed-ready and never use this installer. The exact Flatpak implementation is
covered by deterministic security and integration tests, but no lawfully
obtained BIOS plus FDS game has been run through Nestopia here. Firmware runtime
status therefore remains `not_tested`.

## Controller and configuration contract

The production path requires exactly two distinct, non-virtual Linux
controllers and complete saved native calibration for the eight standard NES
controls. It inventories the controllers inside the target Flatpak using that
runtime's SDL2 with both `SDL_LINUX_JOYSTICK_CLASSIC=1` and
`SDL_JOYSTICK_LINUX_CLASSIC=1`. The second spelling is set explicitly because
SDL adds it during initialization; requiring both keeps repeated inventory and
child-environment checks deterministic. Physical paths are resolved through the
captured host topology and are rechecked before and during launch.

Nestopia's native joystick grammar has several non-obvious limits enforced by
the adapter:

- the player index is a single decimal digit;
- button and axis suffixes must remain below 100 or they alias another player;
- digital axes use the frontend's strict `-16384`/`16384` thresholds; and
- the 1.53.2 event path ignores an SDL hat number, so only hat zero is safe.

The adapter parses the effective case-insensitive mINI maps and writes complete
private copies of `nestopia.conf` and `input.conf`. It forces two ordinary NES
pads, disables the other native ports and expansion port, owns all eight
bindings for both selected pads, clears turbo, and clears exact native-event
conflicts elsewhere in the private input copy.

The temporary root and all of its directories are mode `0700`; the two config
files, supervisor request, and readiness receipt are mode `0600`. The original
Flatpak profile configs are hashed and checked for byte-for-byte preservation.
`HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `XDG_STATE_HOME` point into
this private root. `XDG_DATA_HOME` deliberately remains the app's persistent
`~/.var/app/ca._0ldsk00l.Nestopia/data` directory so cartridge RAM and save
states retain normal production semantics.

The target-sandbox supervisor starts `/app/bin/nestopia` only after validating
the private inputs and a fresh target-SDL snapshot. Readiness is acknowledged
only after `/proc` proves the exact executable and SDL mapping, both private
config inodes, exact environment values, and open descriptors for both chosen
controller nodes. The launch fails closed if any proof changes.

The accepted display path is X11 only. Both observer and production launches
pass `--nosocket=wayland --socket=x11` and `FLTK_BACKEND=x11`. Readiness also
requires one nonempty `DISPLAY` entry and no `WAYLAND_DISPLAY`. An earlier
Wayland attempt reached the generated game but crashed in FLTK/libdecor shared
buffer handling after the F5 action and produced no `.nst`; it is failure
evidence, not a supported fallback. The retained acceptance run therefore used
a fresh private Xvfb display.

## Runtime oracle assets

`lunchbox-nestopia-flatpak-oracle-pads` creates two deterministic Linux uinput
joydev pads with eight buttons each and pulses every player/control route.
`lunchbox-nestopia-flatpak-oracle-rom` generates a reproducible battery-backed
NROM cartridge. The ROM records every nonzero `(player 1, player 2)` input pair
in SRAM and increments a boot generation without erasing the prior log. The
production oracle runs the generated ROM twice through the ordinary
prepare/spawn path, targets only the unique exact-title window, and closes the
window cleanly after each run.

## Accepted runtime result

On 2026-09-13 the ignored installed-runtime test passed under isolated Xvfb
`:144` against the exact deployment above. Target SDL found oracle P1 and P2 at
instances 1 and 4 on both fresh launches; these non-adjacent indices demonstrate
that the adapter used the target inventory rather than assuming host ordering.
The supervisor proved the pinned executable/runtime inputs, private config
inodes and environment, and both open controller nodes before each launch.

The first run recorded P1 B, sent F5 and observed the slot-0 `.nst`, recorded P2
B, loaded the state with F7, then recorded P1 A. The resulting SRAM contained
only the expected pre-save P1 B and post-load P1 A events, which proves that the
load restored emulated behavior rather than merely accepting a hotkey. The
second fresh process loaded generation 1, advanced to generation 2, and appended
all eight controls for P1 followed by all eight for P2. The final exact result
was:

```text
Nestopia oracle passed: save_sha256=f192459776e12b272154ffc2381f84aaa99615b73e4ff007d1e7c3866cfa6481 state_sha256=43972ad216487190be8f73b44205a78e1e4f453c5f5c14c6b1724db7a1357c3f generations=1,2 events=18
```

The test passed 1/1 in 105.95 seconds. It rechecked that the user's source
`nestopia.conf` and `input.conf` hashes were unchanged, reaped its emulator,
supervisor, and pad-driver children, and removed only the oracle's unique `.sav`
and `.nst` outputs.

This proves the exact Flatpak/X11 production path for two stable eight-button
uinput joydev pads, ordinary battery-backed NROM persistence across fresh
processes, and slot-0 F5/F7 state restoration. It does not prove Wayland,
physical controllers, hats or axes, hotplug, other player/device modes, other
Nestopia packages or versions, FDS firmware runtime behavior, or save-provider
export/restore.
The shared ledger therefore records controller `pass`, state `pass`, firmware
`not_tested`, and save sync `blocked` despite the proven native `.sav` reload.
