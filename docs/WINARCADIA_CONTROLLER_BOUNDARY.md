# WinArcadia controller and persistence boundary

WinArcadia 36.64 is pinned to the official source archive
[`WinArcadia-src.rar`](https://amigan.1emu.net/releases/WinArcadia-src.rar),
SHA-256
`061544e10b182ad06e0e084ecd1460303917e52f4877fee9ca4fb4ab221565f`. The
official release page describes a Windows DirectX 9+ build. The source sets
the default configuration path to `Configs\\WinArcadia.ini`, polls host
devices through DirectInput, and provides an interactive game-specific
“Rearrange Gamepad Buttons” dialog. Saved states use `.COS`, including
`AUTOSAVE.COS` and `QUICKSAV.COS`.

Those facts do not establish a safe Lunchbox serializer: device identity,
guest-port assignment, and the per-game button table need runtime probing and
round-trip verification against the installed build. Lunchbox therefore
fails closed and does not mutate WinArcadia configuration. Windows launch
and effective controller input remain unverified.
