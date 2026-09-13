# VICE xvic controller contract

The native VIC-20 binary (`xvic`) is pinned to
`VICE-Team/svn-mirror@d322f7a8d6c269b97162c74e73214c58eaad9a71`. Its SDL UI
creates one joystick port (`uijoyport_menu_create(1, 0, 1, 1, 1, 0)` in
`src/arch/sdl/xvic_ui.c`). The machine uses VICE's normal SDL `.vjm` grammar:
`!CLEAR`, followed by `<device> <inputtype> <inputindex> <action> <params>`;
axis/button/hat input types are 0/1/2 and action 1 is the joystick pin mask.
`JoyDevice1=4+<SDL host-device-index>` binds the host device.

Lunchbox reuses the shared native VICE `.vjm` renderer, but limits xvic to its
single port. The private resource file is selected with `-config`; VICE's SDL
defaults are `sdl-vicerc` on Unix and `sdl-vice.ini` on Windows. The default
joymap is `$XDG_CONFIG_HOME/vice/sdl-joymap-xvic.vjm` on Unix and
`<VICE boot directory>\\sdl-joymap-xvic.vjm` on Windows (or a private path
selected with `-joymap`). This is a native xvic contract, not the PET keyboard
boundary of xpet and not a libretro-core mapping.
