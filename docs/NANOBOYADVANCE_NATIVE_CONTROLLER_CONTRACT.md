# NanoBoyAdvance standalone controller contract

Source oracle: `nba-emu/NanoBoyAdvance` commit
`55b5cf0ae3d929582ac5bfd486558173502b8354`, especially
`src/platform/qt/src/config.hh`, `config.cc`, and
`src/platform/qt/src/widget/controller_manager.cc`.

The Qt frontend persists `config.toml`.  `input.controller_guid` stores the
SDL3 GUID selected by the input UI, while each `input.gba` key is a five-int
array `[keyboard, button, axis, hat, hat_direction]`.  Axis negatives set the
`0x100` flag; hats use SDL cardinal masks 1=up, 2=right, 4=down and 8=left.
`controller_manager.cc` opens the joystick by the exact GUID and polls the
selected button/axis/hat values.

`crates/lunchbox-app/src/controller_nanoboyadvance_native.rs` renders only the
`[input]`/`[input.gba]` fragment and validates the lowercase 32-hex-digit GUID
spelling emitted by SDL3, byte-sized
SDL control indices, and duplicate physical inputs.  It must be merged into a
copied baseline TOML, preserving all unrelated options, especially
`general.save_folder` and backup settings.  A GUID string alone is not a
complete device identity proof: the launch layer must verify that the GUID
identifies the intended runtime joystick.

The default config path is `<QStandardPaths::ConfigLocation>/NanoBoyAdvance/config.toml`;
portable builds use `config.toml`, and an app-bundle macOS build uses
`~/Library/Application Support/org.github.fleroviux.NanoBoyAdvance/config.toml`.
The core config defaults `general.bios_path` to `bios.bin`; the Qt UI requires a
valid 16 KiB GBA BIOS and the upstream README suggests `gba_bios.bin` only as
an unofficial BIOS filename.  Preserve the user-selected save folder and
ROM-basename `.sav` cartridge backup data, plus the ten numbered
ROM-basename `.01.nbss` through `.10.nbss` save-state files.  The writer is
source-backed but runtime-unverified; it does not prove SDL3 enumeration,
effective input, BIOS availability, or save/state round trips.
