# Cxbx-Reloaded native controller contract

The pinned upstream source is
`Cxbx-Reloaded/Cxbx-Reloaded@585c49a50af1255ab155099e06f24505f9c5a800`.
`src/common/Settings.cpp` uses `settings.ini` sections `input-port-0` through
`input-port-3` with `Type`, `DeviceName`, `ProfileName`, `TopSlot`, and
`BottomSlot`. Profiles are `input-profile-N` sections with a numeric `Type`,
quoted `ProfileName`, `DeviceName`, and source-defined control names. The
writer accepts those control names and values from the caller because Duke/S,
arcade, lightgun, and Steel Battalion devices have different guest layouts.

`InputManager::AddDevice` assigns an ID per `(API, device name)` and
`GetQualifiedName` serializes the host identity as `API/id/name`.
`BindHostDevice` looks up that exact qualified name before attaching it to the
configured Xbox port. XInput, SDL, DirectInput/raw input, and libusb devices
are enumerated at runtime; the launch layer must enumerate the same process
inventory and verify the selected qualified name immediately before exec.

Preserve the copied `settings.ini`, XBE/XISO media, emulated `EmuDisk`
partitions and saves, HLE firmware/kernel data, and unrelated settings. No
general-purpose save-state file is documented by the pinned source. This is a
writer contract, not proof of Windows/Wine startup or title compatibility.
