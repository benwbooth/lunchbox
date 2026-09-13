# Retro8 controller boundary

The source is pinned to `libretro/retro8@ddc06a142398ee9755894b3f0bb17c8dc428151d`.
Retro8's libretro frontend receives two ports of runtime `RETRO_DEVICE_JOYPAD`
state and exposes directional plus A/B descriptors. The source has no
persistent host-controller profile grammar. Its libretro serialization API is
also an unimplemented zero-size stub, so it does not establish a core-owned
save-state format.

Lunchbox fails closed and does not invent a Retro8 mapping file. RetroArch
frontend configuration/autoconfig and save/state paths belong to the
frontend, not to this core record.
