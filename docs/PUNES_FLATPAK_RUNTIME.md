# puNES 0.111 Flatpak controller runtime

Status: the app/probe wiring is integrated, and the exact installed-runtime
production oracle passed on 2026-09-13. The verified boundary remains the pinned
Flatpak on isolated X11 with two stable uinput source pads; the limitations below
are still outside the claim.

## Audited boundary

The target is the installed system Flatpak `io.github.punesemu.puNES`, version
0.111, stable x86_64 deployment
`335dff06f700b1d21f600fa8b77e34080f577536a40df9d8c2b319be5d4c86e9`.
Its runtime is `org.kde.Platform/x86_64/5.15-25.08` at
`1e9b0aa4623015cebd20350b36dc515124567b0f43c2235bd4ea8259fc2b18de`.
The implementation refuses drift in all of these identities:

| Object | SHA-256 or identity |
| --- | --- |
| `/app/bin/punes` | `ae027e3b7bc5396407f82db665c6dd9507dc65e2e41c7f4d9830165f14a15c45` |
| puNES ELF build ID | `bc9cd365de9aa78dd668a0be5d366654d111e4fd` |
| runtime `libudev.so.1.7.10` | `ebee10d92fdc8bfb3d7b8df59771dbc8bed79552c3a0c4cf7d30989ac0d01e4d` |
| runtime loader | `d204c9ce43348a62ec1cefad8e7a04eec7d365deb9e9dc3d881a236f2994dba1` |
| Flathub source archive | `438c23a7b69e303b36a13e82505b66e2bd376177c29aa2b13f6a64ccf03772cb` |
| upstream tag/commit | `v0.111` / `613b7b44baddbe7bf2ff79e72348c3ee35f3e70b` |

The installed manifest builds with `DISABLE_PORTABLE_MODE=ON`. In that mode,
`gui_config_folder()` uses Qt's generic config location and
`gui_data_folder()` uses its generic data location. The latter owns `bios`,
`diff`, `prb`, `save`, and `screenshot`. The runtime therefore makes only the
config/cache/state/home roots session-private while retaining the installed
Flatpak's real data root as `XDG_DATA_HOME`. Battery RAM and native save states
remain in the user's existing Flatpak data tree.

The installed permission set includes `devices=all` and `/run/udev:ro`. puNES
does not use SDL for Linux joystick discovery. `os_jstick.c` enumerates udev
input devices, opens `/dev/input/event*`, and reads evdev identity/capabilities.
`js_guid_create()` constructs its GUID from eight native-endian 16-bit words.
The JSC grammar maps `BTN01`, `BTN02`, `BTN11`, `BTN12`, and `BTN16` through
`BTN19` to Linux `BTN_A`, `BTN_B`, `BTN_SELECT`, `BTN_START`, and the four
`BTN_DPAD_*` codes respectively.

## Runtime contract

For each selected physical controller, the adapter requires Linux calibration
for every ordinary NES control: A, B, Select, Start, Up, Down, Left, and Right.
It verifies the exact physical evdev/sysfs identity, then creates one
session-owned uinput pad per player. The pads have fixed identities:

| Player | input ID | puNES GUID |
| --- | --- | --- |
| 1 | bus `0006`, VID `1209`, PID `4c51`, version `0001` | `{FE12FF9C-1209-1141-4C51-4B250001FE71}` |
| 2 | bus `0006`, VID `1209`, PID `4c52`, version `0001` | `{FE12FF9C-1209-1141-4C52-4B260001FE71}` |

The Flatpak's `devices=all` permission leaves the physical source event nodes
visible inside the sandbox. Before creating a target or launching puNES, the
bridge takes an exclusive `EVIOCGRAB` on the same identity-checked source FD it
reads. A failed grab aborts preparation, while a successful grab prevents other
clients, including puNES, from receiving source events until that FD is dropped.
The bridge emits only the eight declared target buttons, maps measured axis
half-travel to button state, begins from a complete physical-device snapshot,
and neutralizes the target on exit or failure. The in-Flatpak probe independently
opens each target, checks its exact
input ID, name, GUID, complete key-capability set, device/inode identity, and
then holds those descriptors while starting puNES.

The private `puNES.cfg`, `input.cfg`, and per-GUID JSC files are mode 0600 under
a mode-0700 root. Unowned settings are copied from the user's current files.
The final Flatpak plan replaces the ordinary launcher's directory grant with an
exact read-only grant for the ROM directory; native game data stays separate.
The owned fields force NES controller mode, disabled cheat mode, an empty Game
Genie ROM path, ordinary controller ports one/two,
disabled ports three/four, standard expansion port, distinct target GUIDs,
opposing-direction rejection, explicit F1/F4 state-shortcut bindings, multiple
instances, and no write-back of private settings. Hashes and the original user
files are rechecked through handoff. The accepted state oracle used the exact
State-menu actions described below: XTEST delivery of the configured F1/F4
shortcuts did not trigger either action reliably in the isolated session, so it
is not claimed as a verified automation path.

The supervisor does not write its readiness receipt until `/proc` proves that
the child is the audited executable, maps the audited libudev, sees the exact
private config files, has the five intended user-root environment variables,
and has opened every target evdev descriptor. Any identity, topology, config,
deployment, library, or bridge-health drift fails closed and terminates the
child on startup failure.

This contract is only for one or two standard NES cartridge pads and `.nes`,
`.unf`, or `.unif` content. FDS requires a separately reviewed persistent BIOS
contract. NSF, Four Score, Famicom expansion hardware, Zapper, paddle, keyboard,
mouse, microphone, and other special peripherals are deliberately excluded.

Ordinary cartridge execution has no external firmware dependency. In the exact
0.111 source, FDS instead reads an 8 KiB `disksys.rom`, searching the configured
`fds bios file`, the current directory, the disk-image directory, and finally
`$XDG_DATA_HOME/puNES/bios` in that order. Upstream does not make those fallback
locations or the BIOS content trustworthy. A future FDS setup must therefore
select and hash one exact BIOS, eliminate competing fallbacks, account for
writable disk/diff behavior, and use an FDS/peripheral oracle. `gamegenie.rom`
under the BIOS tree is likewise outside this ordinary-cartridge contract.

## Deterministic end-to-end oracle

The two opt-in binaries are
`lunchbox-punes-flatpak-oracle-rom` and
`lunchbox-punes-flatpak-oracle-pads`. The first creates a deterministic
battery-backed NROM-128 image with no copyrighted content. The second creates
two deterministic uinput *source* pads at PIDs `4c61` and `4c62`, deliberately
distinct from production's `4c51`/`4c52` target pads, and accepts `pulse PLAYER
BUTTON`, `pulse-all`, and `quit` on stdin. Its tab-separated identity lines
contain player, joydev path, evdev path, and source GUID. Button indices are A=0, B=1,
Select=2, Start=3, Up=4, Down=5, Left=6, Right=7.

The accepted production test performs the following sequence:

1. Generate a new oracle `.nes`, refuse existing output files, start two
   deterministic source pads, and launch through the ordinary production
   prepare/spawn path. Require successful exclusive grabs of both still-visible
   source evdev nodes, guarded private main-config values, and the readiness
   receipt for the distinct `4c51`/`4c52` target pads before sending input.
2. Send P1 A, activate the exact `State` / `Save state` menu action, and require
   the slot-0 `.p00` file. Send P2 A, wait for the ROM to observe the mutation,
   activate `State` / `Load state`, then send P1 Select and quit via the exact
   `File` / `Quit` action.
3. Require both
   `$XDG_DATA_HOME/puNES/save/<oracle-base>.p00` and
   `$XDG_DATA_HOME/puNES/prb/<oracle-base>.prb` in the real Flatpak data tree.
   The PRB is raw 8 KiB NVRAM. After the first launch its header must be
   `4c 42 50 55 01 02`, and its two records at offset `0x10` must be P1 A
   (`01 00`) followed by P1 Select (`04 00`). The post-save P2 A record must be
   absent, proving the load restored emulated RAM rather than merely repainting
   the screen.
4. Launch again in a fresh process, send `pulse-all`, and quit normally. The PRB
   header must become `4c 42 50 55 02 12`. Its 18 records must retain P1 A and
   P1 Select first, followed by P1 masks 01 through 80 and P2 masks 01 through
   80. The `.p00` hash must remain unchanged because the second run performs no
   save-state action. This proves native battery reload and resave through the
   persistent data root.
5. Recheck the original `puNES.cfg` and `input.cfg` hashes, reap every child,
   and remove only the unique oracle `.prb` and `.p00` outputs.
6. For negative testing, contend an exclusive source grab, change one staged
   config byte, remove a target, or use a different Flatpak deployment. Each
   case must fail before readiness and leave the original user configs unchanged.

Passing source-shaped unit tests, generating the ROM, or observing target pads
is not equivalent to passing this end-to-end oracle.

## Accepted runtime result

On 2026-09-13 the post-fix ignored installed-runtime test passed 1/1 in 112.09
seconds under isolated Xvfb `:177` and Openbox against the audited deployment.
The ordinary production adapter launched twice. On both launches the bridges
successfully acquired and held exclusive grabs on both source evdev FDs before
creating targets or spawning puNES; the private main config forced cheat mode
disabled and an empty Game Genie ROM path. The in-Flatpak supervisor proved the
exact executable and runtime libudev, the private config inodes and environment,
and open descriptors for both separate production target pads before readiness.

The first clean exit produced SRAM header `4c 42 50 55 01 02` and event bytes
`01 00 04 00`: P1 A was present before the save, the P2 A mutation made after
the save was absent after loading, and P1 Select was observed after loading.
The second fresh launch loaded that SRAM, advanced the generation to 2, and
retained it before appending all eight P1 events followed by all eight P2
events. The final exact result was:

```text
puNES oracle passed: battery_sha256=b5326ae3bdb6402c9e4768e460283e11d8a98b4d8ccccba0803c19c4b9b720d8 state_sha256=118ef114d0303140284e1ae5d2039c46b743b98d7692e9267f9b200ec6f656e9 generations=1,2 events=18 elapsed_ms=112033
```

The test preserved the original main/input config hashes, reaped the emulator,
supervisor, target bridges, and source-pad process, and removed only its unique
`.prb` and `.p00` outputs. The shared saved-setup model, guided setup UI,
catalog, launch dispatch, module exports, probe commands, and coverage reporting
are integrated for this exact target.

This proves the pinned Flatpak/X11 production path with two stable uinput source
pads, successful exclusive source capture while their nodes remained visible,
independent source-to-target routing for all eight ordinary controls per player,
native battery-backed NROM persistence across fresh processes, and slot-0 state
restoration through the exact application menu actions. It does
not prove physical controllers, axes or hats, hotplug, Wayland, other puNES
packages or versions, one-player runtime behavior, special peripherals, FDS
firmware, function-key injection, or save-provider export/restore. The runtime
ledger therefore records controller `pass`, state `pass`, firmware
`not_tested`, and save sync `blocked` despite the proven native `.prb` reload.
