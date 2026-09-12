# Cemu, Azahar, and Panda3DS native Linux controller contract

These adapters are source-pinned contracts, not claims of emulator-wide
compatibility.

* **Cemu** writes source-shaped `emulated_controller` XML for an SDLController
  VPAD. The UUID is the SDL GUID occurrence (`0_` plus the lower-case 32-hex
  GUID for the first matching device). Controller profiles can be staged under
  private XDG config while the normal Cemu data/MLC root remains persistent.
* **Azahar** writes a QSettings `Controls/profiles` array using native
  `engine:sdl` ParamPackages with `guid`, `port`, `api:controller`, and either
  `button` or `axis_x`/`axis_y`. Isolating `XDG_CONFIG_HOME` leaves
  `XDG_DATA_HOME/azahar-emu` (NAND, SDMC, keys and saves) unchanged.
* **Panda3DS native** has no authorable gamepad mapping table in the pinned
  source: it opens SDL gamepad 0 and hard-wires standard controls. The adapter
  therefore refuses to claim native mapping support. Use Panda3DS through
  libretro, or expose the limitation to the user. A private cwd/config may
  isolate settings, but it does not create a mapping.

All three paths require runtime verification against the exact executable and
SDL library before being presented as launch-ready.
