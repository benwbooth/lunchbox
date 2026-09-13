# Virtual Jaguar controller boundary

The maintained Virtual Jaguar implementation is the libretro core pinned to
`virtualjaguar-libretro@f9a3c89f58836cb2c45a42ad4edfac047050f30a`. Its
`virtualjaguar_libretro.info` declares the core's input and savestate
capabilities, while RetroArch owns controller remapping, configuration,
save-directory naming, and state slots.

The standalone `virtual-jaguar` record is consequently distinct from
`emulator_details/retroarch-cores/virtual_jaguar.json`. Lunchbox refuses a
standalone native writer and uses the shared RetroArch profile only when the
RetroArch-hosted record is selected.
