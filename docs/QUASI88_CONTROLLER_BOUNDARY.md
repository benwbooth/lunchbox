# QUASI88 controller boundary

QUASI88 is integrated here as the `libretro/quasi88-libretro` core at commit
`459bbc6e90caa3dc392ae8e64a9b0881b1e5ef77`. Its documented controller surface
is RetroArch's frontend-owned RetroPad/RetroKeyboard mapping. The core maps
those callbacks to PC-88 keyboard keys and exposes core options such as BASIC
mode and save-to-disk-image; it does not define a persistent physical-device
profile format.

The libretro port's configuration-file initialization is stubbed, while its
save and state locations are supplied by the frontend. Lunchbox therefore
refuses to invent a native controller file or mutate RetroArch's user config.
The refusal is intentional and remains valid on Linux, Flatpak, Windows, and
macOS until an authoritative per-device grammar is established.
