# DCHector, DEmul and Dolphin Triforce source contract

## DCHector

The preserved Windows DCHector v0.1 archive (`sha256
eb4c69ff01e033523d11113b64cef38a78b804c56abe71cb2788129737b28c1d`) is the
only inspectable artifact found; the official site is currently challenge
protected. Its UI mentions keyboard/joystick settings and its changelog
mentions HRX cassette read/write, but no serialized config, controller identity,
save-state or external ROM path is exposed. The record therefore captures the
artifact and refuses to invent a writer.

## DEmul

The Windows 0.7a build 280418 archive is pinned by SHA-256
`ae3f11ed5d36c4f327b3428b8947181284a7f9ae302d811852d4d7a4e9af9148`. It has
plugin-owned INIs, including numeric `padDemul.ini` sections and observed
`nvram/` data. Numeric values have no stable physical-device or authoritative
parser contract in the closed artifact, so the input writer remains refused;
state hotkey fields are not treated as a state-file grammar.

## Dolphin Triforce

The pinned `Hancock33/dolphin-triforce` commit
`842a088dcecbc99348b24557396f56456ceaf2ac` exposes Dolphin's real
`Common::IniFile` and `ControllerEmu` grammar. Its roots are the normal
Dolphin `Config/`, `GC/`, `Triforce/`, `StateSaves/` and `Sys/GC/<region>/`
tree, with portable `User/` and `DOLPHIN_EMU_USERPATH` overrides. Existing
Dolphin native controller/profile handling is the authoritative writer
boundary; Triforce service/coin/test semantics must not be guessed as ordinary
GameCube controls.
