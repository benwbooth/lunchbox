# M88kai standalone controller boundary

The adapter intentionally refuses to serialize a controller profile.

## Verified artifact and source-family evidence

M88kai is treated as a Windows-only artifact. The official distribution site
(`http://nenecchi.kirara.st/`) returned HTTP 403 during capture, so no exact
M88kai source or executable metadata could be verified. The available
M88-family source oracle is `rururutan/m88@1c48d83070202eef43ab00db757131d0cc4768cc`.
The corresponding mirrored archive was captured from
`https://www.emu-france.com/?wpfb_dl=7265`, SHA-256
`305bf4a6918f472ba29169425048354d6d324244b93d0fe97a0abb35c9c4b3f7`.

That source family persists an `M88.ini` joystick-enable/port mode and uses a
fixed Win32 joystick backend (`JOYSTICKID1`, X/Y axes, and buttons). It does
not expose a remappable physical-device identity or a generic binding-file
grammar. The family evidence is explicitly not promoted to exact M88kai
semantics.

## Refusal boundary

The adapter does not synthesize an INI schema, stable joystick identity, or
Linux/macOS configuration path from an inaccessible Windows artifact. Save
files, snapshots, BIOS/font files, and key bindings remain unresolved until
an exact M88kai artifact/source oracle is available.
