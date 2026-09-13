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

`JoyN` is the SDL enumeration slot (`N` is one-based), not a stable device
identity.  Axis values use `JoyN-AxisM+`/`JoyN-AxisM-`; hats use
`JoyN-HatMH` with `N`, `S`, `W` or `E`.  Keyboard values use the source's
named or numeric `UserInput` grammar.  The adapter emits only the first-player
section and must merge it into a copied baseline, preserving all other
options, keyboard shortcuts, joypads, save paths and state paths.

The wx frontend's default file is `vbam.ini`; the Qt frontend's default file
is `vbam-qt.ini`.  Both resolve under the source-selected user configuration
root (XDG on Linux, platform-standard user data/config roots on Windows and
macOS), and either frontend accepts an explicit configuration file.  A
runtime launch must re-enumerate the intended SDL device immediately before
starting VBA-M; this writer does not prove that a slot still denotes the same
physical controller.

Preserve ROM-adjacent `.sav`/`.sgm` data, user-selected state files, optional
`gba_bios.bin`, `gb_bios.bin` and `gbc_bios.bin` firmware, and the existing
configuration root.  The source-backed writer is not a runtime claim about a
particular VBA-M binary or game.
