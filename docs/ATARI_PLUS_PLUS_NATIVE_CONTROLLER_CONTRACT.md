# Atari++ native controller contract

Source oracle: official `atari++_1.85.tar.gz` from
`http://www.xl-project.com/downloads.html` (inspected 2026-09-12), primarily
`machine.cpp`, `analogjoystick.cpp`, `sdlanalog.cpp`, `digitaljoystick.cpp`,
`main.cpp`, and `atari++.man`.

Atari++ has four emulated joystick ports and an abstract input-device layer.
The configuration files are plain `option = value` files loaded in this
order: `/etc/atari++/atari++.conf`, `$HOME/.atari++.conf`,
`./.atari++.conf`, then an explicit `-config` file and command-line options.
The port binding is `Joystick.<0..3>.Port = <device>`.

`controller_atari_plus_plus_native.rs` is a writer-only native overlay. For a native Linux overlay, the only deterministic physical path in this
source is `AnalogJoystick.<n>`, which opens `/dev/input/js<n>` and exposes
`HAxis.<n>`, `VAxis.<n>`, and `First_Button.<n>` through
`Fourth_Button.<n>`. The source also has `SDLAnalog.<n>` and `SDLDigital.<n>`;
those select SDL joystick units by numeric index and do not persist a stable
hardware identity. A writer must therefore require a live probe of the exact
`/dev/input/js<n>` device and preserve the user's other options. It must not
silently substitute SDL unit order, keyboard, mouse, paddle, `DigitalJoystick`,
or 5200 analog modes. Any nonstandard cartridge, paddle, lightpen, or console
panel behavior needs a separate measured contract.

Guest saves land in writable mounted disk/tape images. Machine snapshots are
plain-text files emitted by `SnapShotWriter`; the menu default is
`atari++.state`, while `-state <file>` loads an explicitly chosen file.
OS++/Basic++ are built in; external OS and BASIC paths are selected by the
documented `OsAPath`/`OsBPath`/`Os1200Path`/`OsXLPath`/`Os5200Path` and
`BasicAPath`/`BasicBPath`/`BasicCPath` options. No separate key file or
canonical firmware checksum is published by the upstream archive.

There is no official Flatpak package or verified macOS package in the
upstream download list. Linux and Windows source/binary behavior remains
runtime-unverified.
