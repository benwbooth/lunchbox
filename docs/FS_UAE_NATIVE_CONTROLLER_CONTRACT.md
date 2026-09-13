# FS-UAE native controller contract

This is a source-backed mapping contract, pinned to official FS-UAE commit
`f362278ccd4c60991caac3b4d240d4a3f751bea2` and matching Python launcher commit
`350a7df368e50b44b8a5f91b4cb5663ab82aa406`.

## Mapping grammar and identity

The C-side SDL backend opens joystick/gamepad devices and the Python launcher
defines the controller-profile grammar. Mapping lines are exactly
`devicename_eventname = actionname`. The writer in
`crates/lunchbox-app/src/controller_fs_uae_native.rs` supports universal
`controller_*` events, port-specific `joystick_port_N_controller_*` events,
measured raw `button_N`/`axis_N`/`hat_N_*` events, and measured normalized device
names. Names are lower-cased, non-alphanumeric runs collapsed to `_`, with
`_2`, `_3`, ... selecting duplicate devices. Callers must probe the same native
SDL runtime immediately before launch; the writer never derives a button map
from a product name.

Do not treat a `*.fs-uae` file, a controller `*.conf`, or a successful parser
test as native compatibility evidence. Runtime probing must verify the exact
executable, matching launcher, SDL device identity, resulting action path and
preserved media/state roots.

## Preserved paths

The source/docs identify `~/.config/fs-uae/` and per-game `*.fs-uae` options,
controller profiles under the Controllers data directory, Kickstarts under
`~/FS-UAE/Kickstarts/`, and `.uss` save states under `~/FS-UAE/Save States/`
(with `save_states_dir` as an override). Amiga application saves remain inside
mounted ADF/HDF or other Amiga media; there is no universal sidecar save file.
Any private config must preserve those declared media, firmware, save and state
roots and hash the exact executable, content, config, controller profile, probe
and SDL runtime.

The native writer is distinct from any libretro route. Keep those executable,
config and save roots separate, and do not claim effective gameplay input until
a real native session has been probed.
