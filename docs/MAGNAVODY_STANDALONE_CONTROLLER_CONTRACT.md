# Magnavody standalone controller boundary

Magnavody is a Godot 4 simulator for the Magnavox Odyssey. The record is
pinned to `dodgyville/magnavody@b07ecc571f26ba6c934a7b9cc05557e396284984` and
the official `0.8.5` itch distribution lists Linux, macOS, and Windows builds.

## Source-backed persistence

The running scene stores its general settings at `user://settings.json`.
`Globals.save_settings()` writes a JSON object containing `settings_version`,
`fullscreen`, and `hide_ui_all`; Godot resolves `user://` to the project's
default per-platform data directory. The source also contains a `Settings`
Resource with `user://settings.tres`, but the active scene calls the JSON
implementation, so the unused Resource is not presented as an active config
file.

The default InputMap is in `project.godot`. It defines keyboard controls for
both players and gamepad dial/reset actions using Godot `InputEventKey`,
`InputEventJoypadMotion`, and `InputEventJoypadButton` objects. The bundled
`luke-input-remapper` addon optionally writes
`user://luke_input_remapper_overrides.tres` using Godot `ResourceSaver`.
Joypad selection is a runtime index: auto-detection chooses the first
connected joypad for player 1 and the second for player 2. No stable physical
controller identifier is persisted.

## Writer boundary

Lunchbox does not write Magnavody's `.tres` Resource. Generating one requires
Godot's resource serialization and the source's object/resource IDs, while a
caller-selected device would still be only a runtime Godot index. A generic
SDL or RetroPad profile would therefore not be the native Magnavody contract.
The source does not implement guest save games or emulator save states;
`user://Games/` is only the destination for extracted homebrew ZIP contents.
No external BIOS, firmware, or cryptographic-key file is required.

The three native desktop paths are source/documentation captures, not proof of
package startup or effective gameplay input. A future adapter needs a live
Godot launch oracle and an exact ResourceSaver fixture before it may modify a
user profile.
