# RetroArch Beetle VB controller contract

Beetle VB is the libretro core from `libretro/beetle-vb-libretro` commit
`83ed42608601fb7b01d41e4f8fb2007a37b8c84e`, hosted by RetroArch. The core
declares Virtual Boy channels in `libretro.cpp`: left D-pad, right D-pad,
A/B/L/R, Select/Start, low-battery toggle, and right-analog X/Y. RetroArch
owns their authoring through the existing `input_playerN_*` RetroPad bindings
and per-driver autoconfig profiles; core options (`vb_3dmode`, color,
direction and CPU options) are separate `vb_*` key/value options.

This record preserves that existing controller-profile contract. It does not
add a standalone Beetle VB writer, because the core has no native config file
or device-identity grammar apart from its RetroArch host.
