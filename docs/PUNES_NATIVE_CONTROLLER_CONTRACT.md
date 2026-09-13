# puNES standalone-native controller contract

The source oracle is `punesemu/puNES` commit
`b1758ec26a3d6da10abf50dc184fa837236c5f61`. The native Qt frontend persists
per-port `P?J GUID` values in `input.cfg`, but that value selects a joystick;
the frontend then applies its compiled per-device defaults from `jstick_db.h`.

The writer in `crates/lunchbox-app/src/controller_punes_native.rs` emits that
exact GUID selection and, optionally, all ten source-defined `P?K` keyboard
fallback fields. It does not pretend that an arbitrary host pad has a
portable per-control mapping grammar. The GUID must be measured from the same
native frontend and correspond to a known, source-supported joystick database
entry; an unknown GUID remains unsupported and must fail the runtime probe.

The patcher changes only selected `[port N]` fields and preserves unrelated
config, media, firmware, save and state settings. Preserve `input.cfg` and
`puNES.cfg`, `$XDG_DATA_HOME/puNES/prb/` battery/NVRAM files,
`$XDG_DATA_HOME/puNES/save/` state slots, and `bios/disksys.rom` when FDS is
used. Keep Linux, Flatpak and Windows roots separate. A rendered config or
successful parser test is not proof of startup or effective gameplay input.
