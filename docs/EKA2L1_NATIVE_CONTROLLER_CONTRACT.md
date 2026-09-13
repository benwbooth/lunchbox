# EKA2L1 native controller contract

The pinned upstream source is `EKA2L1/EKA2L1@8dd86cffc59d12c59661acecdfddfab5ffc810db`.
`src/emu/config/src/config.cpp` emits top-level `config.yml` options and a
complete YAML sequence at `bindings/<current-keybind-profile>.yml`; the
selected profile name comes from the `current-keybind-profile` option. Each
entry has `source.type` (`key`, `mouse`, or `controller`), source data, and a
guest `target`. The native writer patches only the selected profile scalar and
serializes caller-supplied guest actions/topology into that binding file.

`src/emu/drivers/src/input/backend/emu_controller_sdl2.cpp` loops over
`SDL_NumJoysticks()` and passes the enumeration index as `controller_id`.
Standard controller button IDs are 0 through 20; virtual axis/trigger events
use the source's 300-series codes. These indices are runtime process order,
not persistent physical identity, so the launch layer must probe and verify
the selected SDL2 devices immediately before exec.

Preserve `config.yml` options other than the selected profile, the complete
`data/drives` save tree, installed device/Z-drive firmware and ROM resources,
and unrelated profiles. EKA2L1 has no documented desktop save-state file;
this module does not claim executable startup, firmware installation, or guest
input success.
