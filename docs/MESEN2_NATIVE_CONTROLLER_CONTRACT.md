# Mesen2 standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

SourMesen/Mesen2 b9fa69ddc6d0a331fb103fdb5eef6904305703c2 (Linux frontend):

- `Core/Shared/Interfaces/IKeyManager.h` — `BaseGamepadIndex = 0x1000`;
  `Linux/LinuxKeyManager.cpp` builds every KeyMapping UInt16 as
  `0x1000 + pad*0x100 + buttonIndex` for up to twenty pads.
- `Linux/LinuxGameController.cpp` — buttonIndex resolves straight to kernel
  codes: 0..13 are BTN_A/B/C/X/Y/Z, BTN_TL/TR/TL2/TR2, BTN_SELECT/START,
  BTN_THUMBL/THUMBR (BTN_MODE is unhandled); 14..25 are ABS_X/Y/Z and
  ABS_RX/RY/RZ with the positive half on the lower index
  (`CheckAxis(code, true)`); 26..29 are ABS_HAT0X/Y directions with
  BTN_DPAD_* folding into the same indices.
- `Linux/LinuxKeyManager.cpp CheckForGamepads` — pads register from
  `/dev/input/event*` in directory-iteration order, so this contract
  requires the selected controller to be the single qualifying device
  (`EV_KEY+BTN_GAMEPAD` or `EV_ABS+ABS_X`, per
  `LinuxGameController::GetController`) and uses pad slot 0.
- `Linux/LinuxGameController.cpp CheckAxis` with
  `EmuSettings::GetControllerDeadzoneRatio` — at the default
  ControllerDeadzoneSize 2 (ratio 1.0) digital halves engage beyond 40% of
  the kernel-reported range around the captured default position; the
  binding checks require a near-middle rest and >40% travel.
- `UI/Utilities/JsonHelper.cs` / `UI/Config/InputConfig.cs` /
  `Core/Shared/SettingTypes.h` — settings serialize indented with PascalCase
  properties and string enums; one UInt16 per control; the NES standard
  controller is `NesController` and unset ports `None`; the configuration
  lives at `$XDG_DATA_HOME/Mesen2/settings.json`.

## Integration

- `controller_mesen2_native.rs` encodes the KeyMapping UInt16s (kernel
  key/axis tables with unit tests) and renders the private settings.json;
  `controller_mesen2_native/settings.rs` holds the saved launch setups;
  `controller_mesen2_native/session.rs` enumerates every `/dev/input/event*`
  node through the probe's read-only evdev catalog, applies Mesen2's
  acceptance test, requires the selected controller to be the sole
  qualifying device and translates the calibration's kernel codes with
  40%-range travel checks; `controller_mesen2_native/native_command.rs` owns
  the XDG_DATA_HOME-isolated launch.
- Guided target `mesen2:standalone-nes` (one player) reuses the existing
  `nes` layout for the NES/Famicom Disk System platforms. Saved per-game
  setups are reused for another game of the same emulator. The probe's
  evdev catalog types gained `Deserialize` for this consumption.

## Limits

Single player, NES standard controller only. Zapper, Arkanoid, Power Pad,
Subor mouse, Four Score, multitaps, Famicom expansion devices, SNES and
every other system's controllers, hotkeys and the Qt/other-OS frontends are
not covered. Runtime behavior is unverified.
