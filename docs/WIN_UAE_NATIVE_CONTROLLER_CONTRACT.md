# WinUAE native controller contract

This is a Windows-only source-backed input contract, pinned to WinUAE source
commit `d9a702b536283ea968c0de7c1991b44181120575`. The official distribution
identity is [winuae.net](https://www.winuae.net/); no Linux, Flatpak or macOS
native runtime is implied.

`crates/lunchbox-app/src/controller_winuae_native.rs` writes the source-defined
`input.1.joystick.N.name/friendlyname`, `axis.N`, `button.N` and
`joyportN=joyM`/`joyportNmode=gamepad` keys. The caller supplies measured
WinUAE unique/friendly names, zero-based device index, axis/button numbers and
the complete seven-control Amiga joystick map. The writer rejects duplicate or
incomplete devices and sets the source match mask to require both names.

The exact official executable, Windows input backend, Kickstart/media paths,
save/state roots and a real controller session still need runtime verification.
Running the Windows executable under Wine remains a Wine/Windows runtime, not
native Linux compatibility. A Rust build, download page, or `.uss` extension
alone does not establish effective input.
