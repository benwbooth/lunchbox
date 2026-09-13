# Amiberry native Linux controller contract

Source oracle: `BlitterStudio/amiberry` commit
`06ff25093b620deef734a395189a1c564ed8beac`, inspected 2026-09-12. The
contract is source-backed and runtime-unverified; rendering a mapping is not
evidence that an Amiberry binary or an Amiga title accepted it.

## Input format

Amiberry's `src/cfgfile.cpp` reads the per-machine `.uae` options
`joyport0` through `joyport3`. The value `joyN` selects the zero-based SDL
joystick identity. For the normal joystick ports, `joyportNmode=gamepad`
selects the SDL GameController path. `joyportfriendlynameN` and
`joyportnameN` are preserved as the device's friendly and unique names; the
source's matching policy can require exact values, so a missing or changed
identity must fail rather than fall back to another pad. Ports 2 and 3 are
parallel-adapter ports, not additional normal gamepad ports, and are outside
this contract.

The private controller database uses SDL's standard mapping line:

```text
<32-hex GUID>,<name>,platform:Linux,a:<button-or-axis>,b:<...>,x:<...>,leftx:<...>,lefty:<...>,dpup:<...>,dpdown:<...>,dpleft:<...>,dpright:<...>
```

Raw SDL button, axis and hat values use `bN`, `aN+`/`aN-`, and `hN.mask`
(cardinal masks up=1, right=2, down=4, left=8). The adapter emits either all
four D-pad directions or two opposite `leftx`/`lefty` axis pairs; mixed or
incomplete direction sets are rejected. SDL south/east/west (`a`, `b`, `x`)
feed Amiberry's fire, second-fire and third-fire controls in `gamepad` mode.
The classic joystick contract has seven controls: up, down, left, right,
fire, fire2 and fire3. CD32 six-button semantics, keyboard shortcuts and
mouse mapping are not silently represented as a classic joystick.

The corresponding module is
`crates/lunchbox-app/src/controller_amiberry_native.rs`; it only renders the
source grammar. Shared catalog registration, saved-setup persistence, probe
translation, launch dispatch and runtime evidence remain parent integration
work.

## Paths and persistence

The pinned source resolves ordinary Linux user content below `$HOME/Amiberry`
(configurable with `AMIBERRY_HOME_DIR`): `Configurations` for `.uae` files,
`Controllers` for controller databases, `Roms` for Kickstarts, and
`Savestates` for `.uss` files. The global Amiberry settings file is resolved
under the XDG config location (`~/.config/amiberry/amiberry.conf` in the
documented layout); `AMIBERRY_CONFIG_DIR` can override the configuration
directory. `saveimage_dir` and `nvram_dir` are independent options. Game saves
remain in mounted Amiga media, while CDTV/CD32 NVRAM is emulator-managed.

`--config`/`-f` selects a `.uae` file, `--statefile` loads a `.uss` state, and
`-s key=value` applies a source-recognized override. A future launcher must
stage a private `.uae` and controller database, hash the executable, content,
probe, SDL library and generated files, and leave the user's global config and
persistent data roots untouched. `amiberry.portable` and any
executable-relative portable layout must be rejected when XDG/home isolation
is required.

## Runtime boundary

No native Amiberry executable, SDL mapping database, controller identity,
Kickstart set or gameplay session has been exercised by this contract. Until a
real Linux session verifies the selected Amiberry build and the same SDL
library, mark the adapter partial/runtime-unverified. Do not infer Amiga
compatibility from a successful Rust unit test or a syntactically valid SDL
mapping line.

## Parent integration

1. Add `mod controller_amiberry_native` and an Amiga classic-joystick catalog
   profile only after selecting the exact target layout and firmware rules.
2. Route calibrated physical controls through the same SDL mapping database
   used by the child; verify GUID, SDL enumeration identity and all seven
   controls immediately before launch.
3. Pass the exact `.uae` configuration/content arguments and keep
   `AMIBERRY_HOME_DIR`/`AMIBERRY_CONFIG_DIR` scoped to the private session.
4. Add runtime evidence before changing the status from partial.
