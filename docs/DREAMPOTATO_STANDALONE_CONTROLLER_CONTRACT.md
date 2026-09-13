# DreamPotato standalone controller contract

The adapter writes DreamPotato's native MonoGame input schema. It does not
claim that a saved gamepad index identifies the same physical controller after
re-enumeration.

## Verified source contract

Pinned source: `RikkiGibson/DreamPotato@ba03ef47622ee105f127d47bc359f9c47bb431c2`.
`src/DreamPotato.MonoGame/Configuration.cs` serializes
`configuration.json` with `System.Text.Json` source generation. `Configuration`
contains `PrimaryInput` and `SecondaryInput`; each `InputMappings` object has
`KeyMappings`, `ButtonMappings`, and `GamePadIndex`. `SourceKey`,
`SourceButton`, and `TargetButton` are string enum values. The verified
`VmuButton` names are `Up`, `Down`, `Left`, `Right`, `A`, `B`, `Mode`, `Sleep`,
`InsertEject`, `Pause`, `FastForward`, `LoadState`, `SaveState`, and
`TakeScreenshot`.

The default data/config root is the executable's `Data` directory. Linux
AppImage and macOS app-bundle builds instead use the platform local-data
`DreamPotato` directory. VMU states are
`SaveStates/<loaded-file-stem>_<id>.dpstate`; the VMU ROM is
`Data/american_v1.05.bin`.

## Writer boundary

The native writer patches only `PrimaryInput` and `SecondaryInput` in a copied
`configuration.json`. It validates `GamePadIndex` as `-1` or `0..15`, matching
MonoGame DesktopGL 3.8.4's sixteen SDL slots, and emits the exact PascalCase
property and enum spelling consumed by the pinned source. It accepts only
exact `Keys` and `Buttons` names from MonoGame 3.8.4 tag source commit
`f34200720b558125964273fd3e7ab44cde0b0429`, not arbitrary strings that merely
resemble C# identifiers.
`GamePadIndex` is passed to MonoGame's `GamePad.GetState(index)` and is a
runtime enumeration slot. Probe and recheck the slot immediately before
launch. VMU data, saves, states, ROM files, and unrelated configuration keys
remain outside the writer's scope.
