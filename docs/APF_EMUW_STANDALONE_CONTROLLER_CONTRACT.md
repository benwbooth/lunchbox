# APF_EMUW standalone controller contract

The boundary is pinned to the official APF_EMUW 2.0.1 Windows archive,
`https://orphanedgames.com/APF/apf_emulation/apf_emuw.zip`, SHA-256
`adb6c061094478d8b2b44d51b8ee08858f60e61e786daba89721266ec6b41629`.

The bundled `APFEMU-Eng.txt` documents `APF_EMU.KYS` beside the executable.
Each line contains two numbers: the value produced by Windows for the key and
the corresponding APF key value. The bundled `TECLADO` is an illustration of
the default keyboard/joystick layout. The same manual says configuration is
stored as `apf_emuw.ini` in the Windows directory, but it does not publish a
portable settings schema. Joystick identity and keyboard-layout values are
runtime-owned.

`controller_apf_emuw_standalone.rs` emits only the documented two-column KYS
file, preserving the values as opaque unsigned 16-bit numbers. It refuses an
empty or oversized override list and does not invent a gamepad mapping,
emulator state format, or save-data sidecar.
