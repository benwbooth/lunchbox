# RMG native Linux controller contract

This contract is pinned to the official Rosalie241/RMG source commit
`3e8b366be91ea96329db0567b038a31785f33468` (the `v0.9.0` source snapshot).
The source is the oracle; a successful Rust build or config render is not a
runtime-compatibility claim.

## Source facts

`Source/RMG-Input/main.cpp` iterates `i = 0..NUM_CONTROLLERS` with
`NUM_CONTROLLERS == 4` and constructs these sections:

```text
Rosalie's Mupen GUI - Input Plugin Profile 0
Rosalie's Mupen GUI - Input Plugin Profile 1
Rosalie's Mupen GUI - Input Plugin Profile 2
Rosalie's Mupen GUI - Input Plugin Profile 3
```

The UI presents those as players 1–4. The adapter therefore accepts one-based
Lunchbox ports 1–4 but writes section `port - 1`; it does not write a fictional
profile 4. `DeviceType=4` selects the explicit SDL joystick path. The source's
`open_controller` requires exact equality for `DeviceName`, `DevicePath`, and
`DeviceSerial`; the adapter uses the exact SDL path and name returned by the
trusted probe and fails closed when those cannot be represented. The current
probe does not expose SDL serials, so it writes the empty serial. A target that
reports a non-empty serial will fail to open rather than fall back to a name or
GUID.

The profile mapping fields are four semicolon-list settings per control:
`<field>_Name`, `<field>_InputType`, `<field>_Data`, and
`<field>_ExtraData`. For the raw joystick path used here, the source enum is:

| InputType | Meaning | Data | ExtraData |
| --- | --- | --- | --- |
| 2 | `JoystickButton` | SDL joystick button index | 0 |
| 3 | `JoystickAxis` | SDL joystick axis index | 0 = negative, 1 = positive |
| 4 | `JoystickHat` | SDL joystick hat index | SDL mask: up 1, right 2, down 4, left 8 |

The four N64 stick directions must resolve to two distinct SDL axes, with
opposite halves for each pair. RMG reads digital axis mappings at
`abs(value) >= SDL_AXIS_PEAK / 2`, i.e. 16,383 for its 32,767 SDL axis peak;
the adapter verifies both released and pressed measurements against that
threshold. All eighteen N64 gameplay controls are required: A, B, Start,
D-pad, four C buttons, L, R, Z, and the four analog-stick directions.

## Isolation and persistence

RMG's `Source/RMG-Core/Directories.cpp` resolves its config directory as
`$XDG_CONFIG_HOME/RMG` and its data directory independently as
`$XDG_DATA_HOME/RMG` (falling back to the normal home-based XDG locations).
`Source/RMG-Core/Core.cpp` force-sets the core's `SaveSRAMPath` and
`SaveStatePath` from that data directory. A prepared launch sets only
`XDG_CONFIG_HOME` to a session-owned temporary root; it intentionally does not
set `XDG_DATA_HOME`, so native `Save/Game` and `Save/State` remain persistent.
The copied config replaces the four base profile sections and removes all
`Profile <0..3> Game <id>` sections. The latter is required because the ROM MD5
used by RMG to select a per-game override is not available at config-build
time; retaining one could silently replace a calibrated mapping. GUI/core/plugin
sections, hotkeys, optional PIF/64DD paths, and unrelated settings are copied
unchanged. The private profile starts with `Pak=3` (no pak); Transfer Pak/Game
Boy authoring is outside this adapter.

RMG portable mode is rejected. In the source this mode is activated by
`portable.txt` or a beside-the-executable `Config/mupen64plus.cfg`, either of
which would override the XDG isolation boundary.

## Runtime gates

The Linux session requires all of the following before spawn and rechecks them
immediately before spawn:

* the selected executable's SHA-256 equals the saved setup hash;
* the probe program and SDL library hashes, the ROM hash, the declared source
  config hash, and the generated private config hash are unchanged;
* the probe reports the exact SDL library and effective
  `SDL_JOYSTICK_LINUX_CLASSIC=1` hint;
* each selected physical `/dev/input/js*` node still has the captured kernel
  topology identity and resolves to exactly one SDL path;
* each selected `/dev/input/js*` node's verified classic map is unchanged, and
  calibrated values still resolve to the same native controls; and
* the current SDL inventory and exact device paths match the preparation
  snapshot.

This is source-backed launch plumbing, not a claim that an RMG binary, SDL
build, graphics plugin, or native runtime has been exercised. A parent
integration must add the module declaration, catalog profile, settings-model
storage/review path, guided setup routing, and launch dispatch separately. The
adapter file deliberately does not mutate those shared registration files.

## Parent integration steps

1. Add `mod controller_rmg_native;` to `crates/lunchbox-app/src/lib.rs`.
2. Add an `rmg:standalone-n64` catalog profile targeting `n64`, with native
   launch metadata for Linux and the eighteen controls listed above. The source
   URL should point at this pinned commit and mark the profile as partial until
   an actual RMG binary is exercised.
3. Add the `SavedSetup` vector and validation/review/staging plumbing to the
   settings and settings model, following the existing native adapters.
4. Add guided setup and `controller_launch` dispatch that calls
   `controller_rmg_native::native_command::prepare`; preserve the exact ROM
   positional argument and do not set `XDG_DATA_HOME`.
5. Add focused runtime evidence with the same RMG executable, SDL library,
   probe hash, and isolated config. Until that evidence exists, keep the
   launch status partial/not runtime-verified.
