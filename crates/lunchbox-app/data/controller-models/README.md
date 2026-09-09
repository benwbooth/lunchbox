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

Imported button indices are not automatically applied to GilRs capture codes.
Backend translation and physical-layout correspondence must be verified before
these records can replace calibration. USB and Bluetooth modes can have separate
profiles; a shared VID/PID or model GUID is not a unique physical-unit identity.
