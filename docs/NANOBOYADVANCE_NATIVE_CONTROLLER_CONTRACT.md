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

`crates/lunchbox-app/src/controller_nanoboyadvance_native.rs` now parses a
copied baseline TOML and replaces only `input.controller_guid` plus the
controller halves of the ten `input.gba` arrays. Keyboard values and all
unrelated options remain intact. The writer validates lowercase 32-hex-digit
GUID spelling, byte-sized SDL control indices, cardinal hats, threshold-crossing
axis halves, completeness, and duplicate physical inputs.

The native Linux session resolves the chosen physical controller through the
kernel topology, inventories the exact declared SDL3 library with
`SDL_JOYSTICK_LINUX_CLASSIC=1`, translates the saved physical calibration into
raw SDL joystick numbering, and rechecks the inventory, GUID, classic control
map, dependency hashes, and topology before spawn. Upstream opens the first
joystick whose GUID matches, so the session rejects a selected GUID that occurs
more than once. The private config is mounted over the selected real config
path with bubblewrap; this covers both ordinary and portable Linux builds
without relocating any persistent data.

The default config path is `<QStandardPaths::ConfigLocation>/NanoBoyAdvance/config.toml`;
portable builds use `config.toml`, and an app-bundle macOS build uses
`~/Library/Application Support/org.github.fleroviux.NanoBoyAdvance/config.toml`.
The core config defaults `general.bios_path` to `bios.bin`; the Qt UI requires a
valid 16 KiB GBA BIOS and the upstream README suggests `gba_bios.bin` only as
an unofficial BIOS filename. The launch resolves and hashes that configured
BIOS, but deliberately asserts no universal BIOS digest. It also captures and
rechecks the selected save directory identity. Preserve the user-selected save folder and
ROM-basename `.sav` cartridge backup data, plus the ten numbered
ROM-basename `.01.nbss` through `.10.nbss` save-state files. The executable
adapter and deterministic writer tests are source-backed but runtime-unverified;
they do not prove emulator startup, effective input, or save/state round trips.
