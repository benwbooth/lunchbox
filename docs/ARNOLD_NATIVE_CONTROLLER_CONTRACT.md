# Arnold native controller contract

Pinned to `rofl0r/arnold` commit `e5dce08964f94add100f7db992a6d0e49fe74f01`.

The Unix port reads/writes `$HOME/.arnold` as plain `key=value` lines (with
`/etc/arnold` as a fallback). The authorable keys are directory/media paths,
including `rom0` through `rom15`; SDL keyboard/joystick selection is runtime
state and has no persistent physical-device grammar. The Windows port uses
`HKCU\Software\Arnold` and persists settings such as `sSnapshotPath`, but a
cross-platform registry writer is outside this native module.

`controller_arnold_native.rs` only provides a field-preserving Unix key writer
and `.sna` snapshot normalization. It deliberately does not claim a stable
joystick identity or write Windows registry data; its explicit controller
refusal records that the Unix frontend opens SDL runtime indices 0 and 1 and
the Windows frontend enumerates runtime WinMM ids.
