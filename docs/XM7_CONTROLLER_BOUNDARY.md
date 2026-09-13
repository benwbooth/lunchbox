# XM7 controller boundary

XM7 is represented by the XM7-for-SDL tree at commit
`37f83f77d94d7510162952755be40b7e6f5d0d45`.  Its SDL joystick backend opens a
device with `SDL_JoystickOpen(physNo)` and later compares SDL event device
indices with `SDL_JoystickIndex()`.  It also limits the exposed buttons and
uses fixed X/Y axes and hard-coded thresholds and guest key codes.

`XM7.INI` has a `[JoyStick]` section (`Type0`/`Type1`, `Rapid*`, and `Code*`),
but those values describe guest joystick mode and keyboard/rapid-fire choices;
they do not identify a physical device.  The same INI contains machine image,
font, and state-directory settings.  Lunchbox therefore refuses to synthesize
an XM7 controller profile or overwrite the user's INI.  A launch integration
would need to probe the exact SDL runtime's device order and verify the
hard-coded event interpretation before it could be considered.

The refusal does not relocate XM7 ROMs, disk/tape images, or state files.
