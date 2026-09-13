# Fuse standalone-native controller contract

The writer is pinned to `fuse-emulator/fuse@5ba7804a44483466d7403a6e646a228da562ed5d`. `settings.dat`, the generated `settings.pl` parser/writer, `peripherals/joystick.{c,h}`, `input.{c,h}`, and `ui/sdl2/sdl2_joystick.c` establish the exact configuration and event semantics.

`controller_fuse_standalone.rs` supports both source-defined encodings: key/value `fuserc`/`fuse.cfg`, and the libxml2 `<settings>` document. It preserves the baseline and patches only `joystick1output`/`joystick2output` plus all fifteen `joystickNfireM` values. Guest output values are the exact enum order: Cursor 1, Kempston 2, Sinclair 1/2 as 3/4, Timex 1/2 as 5/6, and Fuller 7. Each physical fire button can target no key, a supported Spectrum key, or the source-defined `KEYBOARD_JOYSTICK_FIRE` value `4096`.

Fuse does not persist physical identity or reorder the SDL devices. The pinned SDL2 frontend opens runtime slots 0 and 1; axes 0/1 and hat 0 supply directions, while buttons 0 through 14 become fire events 1 through 15. The writer accepts only those two measured slots. The launch layer must verify their physical identities against the exact child frontend immediately before launch.

Preserve the ROM directory/model ROMs, writable `.tap`, `.tzx`, `.dsk` and TR-DOS media, `.sna`/`.z80`/`.szx` snapshots, and `.rzx` recordings. Linux, macOS, and Windows frontends remain separate runtime contracts, and no first-party Flatpak artifact is asserted. A valid written file is not proof of startup, effective input, firmware availability, or save/state round trips.
