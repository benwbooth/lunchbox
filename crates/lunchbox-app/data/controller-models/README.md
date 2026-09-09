# Controller model database

Derived, normalized snapshots of SDL_GameControllerDB (zlib license) and
libretro/retroarch-joypad-autoconfig (MIT profiles, with upstream's additional
zlib notice). Redistribution is permitted subject to the accompanying notices;
the complete upstream license files are included here. This is not an unmodified
upstream distribution. Exact revisions and URLs are recorded in models.json.

Regenerate from checkouts at those revisions with:

```text
cargo run -p lunchbox-db --example import_controller_models -- SDL_CHECKOUT RETROARCH_CHECKOUT crates/lunchbox-app/data/controller-models
```

The importer retains original profiles and parsed bindings. It includes desktop
SDL profiles and RetroArch udev/linuxraw/dinput/xinput/hid/sdl2 profiles. These are
driver-specific records, not 2,779 distinct physical controller models.

Recognition requires matching hardware identifiers and reported name, excludes
virtual devices and generic Xbox/XInput aliases, and declines ambiguous matches.
Users may explicitly select a profile. Selections use the existing per-device
identity, not the model GUID, and do not overwrite nicknames or calibration.
Manual search includes all profiles alphabetically, including other platforms,
so a retro model with only a Windows profile remains selectable on Linux.
This does not establish compatibility with that platform's button numbering.

Controller cards expose the hardware unique ID when the input backend reports
one. If absent, the UI explicitly says so and labels the existing Lunchbox
connection ID separately. A connection ID may be port/session-dependent and
must not be described as a manufacturer serial number.

Imported button indices are not automatically applied to GilRs capture codes.
Backend translation and physical-layout correspondence must be verified before
these records can replace calibration. USB and Bluetooth modes can have separate
profiles; a shared VID/PID or model GUID is not a unique physical-unit identity.
