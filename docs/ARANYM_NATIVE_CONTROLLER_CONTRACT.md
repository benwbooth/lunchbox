# ARAnyM standalone-native controller contract

This is a source-backed, writer-only physical-controller contract. The reviewed
upstream is [ARAnyM](https://github.com/aranym/aranym) at commit
`5f4ebed6b039ddf42eef1122a315ad9608d20f6e`.

`src/parameters.cpp` defines the `[JOYSTICKS]` keys `Ikbd0`, `Ikbd1`,
`JoypadA`, `JoypadAButtons`, `JoypadB`, and `JoypadBButtons`. `src/input.cpp`
passes those four numeric selectors to `SDL_JoystickOpen`; the source exposes
no GUID, serial, or device-path key. `src/joypads.cpp` verifies that the
`Joypad[A/B]Buttons` values are only button-permutation strings.

`crates/lunchbox-app/src/controller_aranym_native.rs` now patches those exact
keys in a copied config. It accepts SDL indices only as explicit same-launch
measurements, validates unique guest ports and live indices, and validates each
Joypad A/B string as a complete permutation of button values 0 through 16. It
does not mistake the integer for durable physical identity.

Before enabling this writer as a launch adapter, pin the exact executable and
SDL backend, probe a real native session immediately before launch, and verify
that every measured index still resolves to the selected physical device.
Linux, Flatpak, Windows and macOS remain separate runtime contracts. Preserve
`~/.aranym/`, TOS 4.04 or EmuTOS, disk images and GEMDOS folders (where guest
saves live), and user-selected snapshot paths.
