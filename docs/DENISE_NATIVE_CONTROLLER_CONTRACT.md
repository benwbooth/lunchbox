# Denise standalone-native controller contract

This adapter is source-backed for settings serialization, but remains a
mapping-layer contract rather than a runtime-compatibility claim.

## Pinned evidence

The reviewed upstream is `piciji/denise@1cb7d45117893c76025d32260edb6008001344f9`.
`program/program.h` defines the native file name as `settings.ini`;
`program/program.cpp` resolves it under Denise's user-data folder (or the
install folder in portable mode). `guikit/tools/setting.cpp` writes one
`identifier:value` record per line using CRLF line endings.

`program/input/mapping.cpp` serializes each input assignment as
`anded|device-id|group-id|input-id|qualifier`. `program/input/global.cpp`
constructs mapping identifiers from the emulated device name and input index.
The Linux udev backend (`driver/input/udev.cpp`) derives its physical
`device-id` from the device path plus vendor/product data; this is the exact
identity value the writer requires, not a host enumeration slot.

## Writer boundary

`controller_denise_native.rs` renders only validated mapping records. The
caller must resolve the emulated identifiers and physical device ids from the
same pinned Denise build, merge the fragment without dropping unrelated
settings, and launch with the intended user-data/install-root mode. The writer
does not relocate or rewrite firmware/ROM selections, mounted media,
guest-save locations, snapshots, or states. The fragment alone does not prove
Denise startup, device discovery, or effective gameplay input.

Native Linux, Flatpak, Windows and macOS launch contracts remain separate;
runtime verification against the exact executable and input backend is still
required.
