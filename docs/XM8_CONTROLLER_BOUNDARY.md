# XM8 controller boundary

XM8 is represented by the `xm8m` tree at commit
`2c4bf025840711a696d7d9ef55f3be3d0f3c84f9`.  Its SDL input backend opens every
joystick using `SDL_JoystickOpen(loop)`, where `loop` is the current SDL
enumeration index, then reads the first two axes and a fixed button table.
The optional joystick-to-key map is part of the private binary settings file,
not a documented physical-device profile.

`setting.cpp` uses `SDL_GetPrefPath("retro_pc_pi", "xm8")` and persists the
settings blob as `setting.bin`; the blob also contains unrelated display and
emulation options.  XM8 has no source-defined text profile grammar or stable
device identity selector.  Lunchbox therefore refuses to synthesize a native
XM8 controller profile or patch `setting.bin`.

This boundary does not relocate the adjacent ROM set, disk/tape media, or the
ten global save-state slots described by `README-XM8.txt`.
