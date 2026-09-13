# AppleWin native controller contract

The pinned upstream source is `AppleWin/AppleWin@3e8054b4627624398e4589f7f27b3d40a6b9718e`.
`source/Configuration/Config.cpp` loads joystick values from the
`[Configuration]` section, and `Registry.cpp` shows that `-conf <pathname>`
switches persistence from HKCU to an INI file. The exact input keys are
`Joystick0 Emu Type v3`, `Joystick1 Emu Type v3`, `PDL X-Trim`, `PDL Y-Trim`,
`Autofire`, `Joystick Centering Control`, `Joystick Cursor Control`, and
`Swap buttons 0 and 1`; the native writer patches only these keys.

`Joystick.cpp` calls WinMM `joyGetNumDevs`, `joyGetDevCaps`, and `joyGetPos`,
then chooses the first two usable device IDs. The GUI's `PC Joystick #1/#2`
values therefore select ordinal discoveries, not durable physical identities.
The launch layer must probe the same WinMM inventory immediately before exec
and verify the selected IDs. Keyboard-cursor, keyboard-numpad, mouse, and
disabled modes are also explicit guest choices.

AppleWin is a Windows runtime. Preserve the copied INI, selected Apple II
model, disk/hard-disk media, firmware directory, and `SaveState.yaml` path;
the module is a low-level configuration writer and does not claim startup,
firmware, or gameplay-input success.
