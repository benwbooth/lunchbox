# PX68k controller boundary

PX68k is integrated here as the `libretro/px68k-libretro` core at commit
`0ad84d7058a12b7db4f7f7a906e87fad4e2f26f6`. Its controller inputs are sampled
through the libretro `RETRO_DEVICE_JOYPAD`, keyboard, mouse, and analog
callbacks. The core options select the emulated device type and button
layout, but do not provide a stable physical-controller profile grammar.

The core's native `system/keropi/config` file is an INI-like persistence file
for `StartDir` and previously loaded FDD/HDD paths. It is not a controller
profile: the `JOY_TYPE` and button behavior are supplied by core options and
runtime libretro callbacks. Lunchbox therefore refuses to write this file or
RetroArch's frontend-owned `retroarch.cfg`/autoconfig profiles.

PX68k also writes `system/keropi/sram.dat` and may modify loaded disk images.
Those media side effects are outside controller serialization and must not be
treated as a controller writer.
