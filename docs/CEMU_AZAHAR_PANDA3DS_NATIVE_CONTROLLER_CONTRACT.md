# Cemu, Azahar, and Panda3DS native Linux controller contract

These adapters are source-pinned contracts, not claims of emulator-wide
compatibility. The inspected upstream revisions are Cemu
`3310f3b8b184d64a62b89fd59088c799432badf5`, Azahar
`ec8201d42cd3d8e2ec1d69d5832be0389490ea47`, and Panda3DS
`5aaa1d26565c834a6f1999026260e559f54aacf1`.

* **Cemu** writes source-shaped `emulated_controller` XML for an SDLController
  VPAD. The UUID is the SDL GUID occurrence (`0_` plus the lower-case 32-hex
  GUID for the first matching device). Controller profiles can be staged under
  private XDG config while the normal Cemu data/MLC root remains persistent.
* **Azahar** writes a QSettings `Controls/profiles` array using native
  `engine:sdl` ParamPackages with `guid`, `port`, `api:controller`, and either
  `button`, trigger `axis`/`direction`/`threshold`, or analog
  `axis_x`/`axis_y`. The profile covers all 18 native buttons (including the
  debug, GPIO14, Home, and Power entries) and both native analogs. Isolating
  `XDG_CONFIG_HOME` leaves
  `XDG_DATA_HOME/azahar-emu` (NAND, SDMC, keys and saves) unchanged.
* **Panda3DS native** has no authorable gamepad mapping table in the pinned
  source: it opens SDL gamepad 0 and hard-wires standard controls. Its Qt/SDL
  keyboard map is authorable, however, through the exact `controls_qt.toml`
  `InputMappings::serialize` format; the adapter can emit that keyboard map
  while refusing to claim gamepad remapping. A private cwd/config may isolate
  settings, but it does not change the positional SDL gamepad selection.

All three paths require runtime verification against the exact executable and
SDL library before being presented as launch-ready.
