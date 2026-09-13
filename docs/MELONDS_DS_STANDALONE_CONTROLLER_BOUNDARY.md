# melonDS DS standalone controller boundary

Pinned source: `JesseTG/melonds-ds@bc4e4b67d2d470d7c682810a1e892cafd6f9082b`.
melonDS DS is a libretro core. Its source registers one
`RETRO_DEVICE_JOYPAD` Nintendo DS port and consumes libretro joypad, pointer,
mouse, and analog APIs. It has no standalone executable or native TOML/INI
controller profile. Host binding belongs to RetroArch (or another libretro
frontend), so the native standalone adapter refuses serialization; the
existing RetroArch core route remains the applicable integration.
