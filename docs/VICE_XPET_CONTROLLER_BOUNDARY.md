# VICE xpet controller boundary

The standalone PET binary is pinned to the VICE source mirror
`VICE-Team/svn-mirror@d322f7a8d6c269b97162c74e73214c58eaad9a71` (trunk r46226).
`xpet` uses the PET keyboard matrix and SDL virtual keyboard; its
`KeyboardType` selects PET keyboard layouts. The generic VICE `.vjm` writer
targets SDL joystick ports and is not an xpet keyboard serializer.

Lunchbox therefore refuses a separate xpet controller writer. This record is
distinct from the `vice` native integration and from the `vice_xvic`
libretro-core identity; no writer or config file is shared implicitly.
