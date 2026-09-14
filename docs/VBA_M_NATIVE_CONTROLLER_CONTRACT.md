# VBA-M standalone-native controller contract

Source oracle: `visualboyadvance-m/visualboyadvance-m` commit
`fd13034143c128c8b68133a7a18bc785178ec4e4`.  The Qt and wx frontends both
persist controller values through the source `GameCommand::ToConfigString`
and `UserInput::ToConfigString` implementations.

The first player is represented by these logical INI keys (the Qt frontend
serializes the nested path with a backslash):

```ini
[Joypad]
1\Up=Joy1-Button11
1\Down=Joy1-Button12
1\Left=Joy1-Button13
1\Right=Joy1-Button14
1\A=Joy1-Button1
1\B=Joy1-Button0
1\L=Joy1-Button2
1\R=Joy1-Button3
1\Select=Joy1-Button4
1\Start=Joy1-Button6
```

The wx frontend writes the same logical keys as a `[Joypad/1]` section with
`Up=...` through `Start=...` entries; use the adapter's wx rendering for a wx
baseline and its Qt rendering for a Qt baseline.

`JoyN` is the SDL joystick enumeration slot plus one, not a stable device
identity. Axis values use `JoyN-AxisM+`/`JoyN-AxisM-`; hats use
`JoyN-HatMH` with `N`, `S`, `W` or `E`. The source engages a positive axis
above `0x1fff` and a negative axis below `-0x1fff`. Keyboard values use the
source's named or numeric `UserInput` grammar. The native adapter emits all ten
first-player controls and rejects duplicate physical inputs. It merges them
into a copied baseline, preserving other options, keyboard shortcuts, joypads,
save paths and state paths.

The wx frontend's default file is `vbam.ini`; the Qt frontend's default file
is `vbam-qt.ini`.  Both resolve under the source-selected user configuration
root (XDG on Linux, platform-standard user data/config roots on Windows and
macOS), and either frontend accepts an explicit configuration file.  A
runtime launch passes that private file with `--config`, forces
`SDLGameControllerMode=false`, and re-enumerates the intended SDL device
immediately before starting VBA-M. This matters because the option defaults to
logical SDL GameController events when enabled, while `JoyN-ButtonM`, axis and
hat values otherwise refer to raw joystick controls.

The Linux launch guard supports either an exact SDL2 runtime with captured
classic/evdev raw numbering or an exact SDL3 runtime forced onto its classic
`/dev/input/js*` backend. It pins the executable, helper, SDL library, content,
source and private configs by SHA-256; retains the selected kernel topology;
and rechecks routing and raw numbering immediately before spawn. The setup is
restricted to one controller and uncompressed `.gba` content because the
database relationship is Nintendo e-Reader. It configures the GBA host pad;
card scanning or loading is outside this contract.

An empty `General/BatteryDir` or `General/StateDir` means the ROM directory.
Absolute configured directories are accepted after `%s` is expanded to
`GameBoy Advance`, and must already exist and be writable. Relative configured
directories are rejected: Qt anchors them to its selected configuration root,
while the inspected wx Linux path helper applies a different normalization
chain, so the saved setup does not carry one common exact destination. The
guard records the resolved save and state directories by canonical path,
device and inode before launch and rechecks them during the session.

When `preferences/BootRomEn` is enabled, `GBA/BiosFile` must resolve from the
actual launch working directory to a readable 16 KiB file; its hash is pinned
for the session. Disabled BIOS settings are preserved without inventing a
firmware requirement. ROM-adjacent `.sav`/`.sgm` data and frontend-selected
state files remain native VBA-M data rather than copied into the private config
directory.

The writer tests prove Qt/wx INI patching, CRLF preservation, axis/hat syntax,
and malformed/duplicate rejection. They do not prove a particular VBA-M binary,
physical controller, game, save load, state restore, or e-Reader workflow at
runtime.
