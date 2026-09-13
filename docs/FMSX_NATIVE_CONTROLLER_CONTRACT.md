# fMSX standalone-native controller boundary

The adapter intentionally refuses to serialize a controller profile.  The
upstream standalone binary has command-line joystick selection, but no native
persistent input configuration or stable host-device identity.

## Verified source contract

Pinned source: `lvitals/fMSX@fd5fac3cc4d8b95a9c9ec22fddc18fc326f71b01`.
`fMSX/fMSX.c` recognizes two `-joy <type>` options, one for each MSX port:
`0` no joystick, `1` normal joystick, `2` mouse in joystick mode, and `3`
mouse in real mode.  The same file recognizes `-home <dirname>` for the
system-ROM directory and `-state <filename>` for the emulation state file.
`fMSX/Help.h` documents these options and the `.STA` state behavior.

For the Unix SDL2 frontend, `EMULib/Unix/LibSDL2.c` opens available SDL game
controllers and assigns the first two opened controllers to ports A and B.
Generic joystick fallback events use runtime joystick instance/slot handling;
the source has no profile file or serialized controller GUID/name selector.
The standalone program also does not parse an `fmsx.ini` file.

## Refusal boundary

`-joy` can select an emulated port type but cannot identify a physical device,
and the SDL mapping is compiled/runtime behavior rather than a persisted
profile.  The adapter refuses to invent an INI path or map a Lunchbox device
id to an enumeration slot.  This does not claim startup, effective gameplay
input, ROM/BIOS completeness, state round trips, or platform parity.
