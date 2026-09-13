# BGB, Gambatte, and GBE+ standalone controller boundary

This batch records the native writer boundaries established from pinned source.
No libretro configuration is promoted to a standalone runtime.

## BGB

BGB's official BGB 1.6.6 x64 archive (`bgbw64.zip`, SHA-256
`38b97e4496ad85106f59c87a6b0386b7405fbebb3bccc90650279762bd10478c`) contains
`bgb64.exe`, a generated `bgb.ini`, and the test ROM. The inspected INI has
general settings and no joypad mapping records. The manual documents Joypad
configuration through the GUI, legacy controller IDs 1–16 and XInput IDs
17–20, but no portable key grammar. The adapter therefore refuses to invent
BGB mappings; the record's ROM-relative save and explicit state-path semantics
remain the source-of-truth roots.

## Gambatte

The Qt standalone frontend is pinned to
`EclipseEmu/gambatte@04e7ddf85ff23032cb7132f155c42c7d1857f474` (the original
upstream repository is now private). `main.cpp` sets QSettings organization
`gambatte` and application `gambatte_qt`; `InputDialog` reads and writes the
`input` group with `Game<Control>Key1/Value1` and `Key2/Value2` entries. A
keyboard entry uses Qt's `0x7fffffff` value sentinel. A joystick entry uses
the packed SDL event id (`type | device << 8 | number << 16`) and value 1/2
for the frontend's positive/negative axis states, or SDL hat bits. The writer
serializes the eight Game Boy controls and clears the unused alternate slot.
The device number is SDL's runtime joystick index; it is not a stable
physical-device identity. The SDL command-line frontend's `--input` mapping
is runtime-only and has no persistent profile.

## GBE+

GBE+ is pinned to
`shonumi/gbe-plus@05a05e931b3993ff3e6316b0d841a1fb4d3ac7a7`. The native
configuration parser and saver recognize the compact
`[#gbe_joy_controls:A:B:X:Y:START:SELECT:LEFT:RIGHT:UP:DOWN:L:R]` section.
The settings UI generates button values `100 + button`, axis values
`200 + axis * 2` plus one for positive polarity, and hat values
`300 + hat * 4` plus the cardinal direction code. The writer emits all twelve
fields in that exact order. `config::joy_id` is an SDL joystick index used by
the runtime and is not persisted by the native configuration, so physical
device resolution remains a launch-time boundary. The config file is
`$HOME/.gbe_plus/gbe.ini` on Unix (unless a portable/install folder wins) and
`%LOCALAPPDATA%\gbe_plus\gbe.ini` on Windows.

## Runtime boundary

BGB remains an intentional refusal because its official executable/archive and
manual expose GUI configuration but no stable portable grammar. The Gambatte and GBE+ writers
are serialization milestones only: they do not establish device discovery,
executable startup, effective mappings, firmware availability, gameplay input,
or save/state round trips. Those claims require launch-time logs and artifacts
from the actual executable.
