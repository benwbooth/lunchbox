# 8-Bit Wonders, ADAM+, and AdViEmulator controller contract

This batch is pinned to real upstream artifacts:

- 8-Bit Wonders `eightbitwonders/app@3b72d139099a513eb2d3fe5fe9b775fd11aeb06f`.
- ADAM+ `dvdh1961/ADAMP@f82570765bc97a2b95138cbeb33df57172d7ab94`.
- AdViEmulator v1.0 SourceForge Linux32 archive (SHA-256
  `fd4d515ae78c6cd524f7300958c98c495f43834d1a3a24c5316a4de22dff25ad`) and
  Win32 archive (SHA-256
  `f6694923cc10358e608e436f6c792d7fc6990e5494fddad5b73acdd79bca38f6`).

## 8-Bit Wonders

The pinned README identifies an Android application built with Android Studio
or the Android SDK. It documents hardware joysticks/gamepads, configurable
button mappings, and save states, but does not define a native desktop package
or portable desktop mapping file. `controller_8_bit_wonders_standalone` keeps
this as an explicit refusal; a desktop writer would be an invented VICE/Android
configuration contract.

## ADAM+

The pinned Qt source uses `settings.ini` beside the executable. `joypadwindow.cpp`
and `inputwidget.cpp` read/write the logical fields `input/p1/0..17`,
`input/p2/0..17`, their `type` values, and `controller/joystickType` (0 General,
1 PS, 2 Xbox). `controller_adam_plus_standalone` patches those QSettings INI
fields, preserving unrelated sections and values. The source's
`SimpleJoystick` always starts polling runtime device index 0 and persists no
physical identity; launch code must re-enumerate and verify the caller-selected
device immediately before startup.

The same source establishes a user-selected `statePath` and state files named
`<rom-basename>.sta`. BIOS overrides are `bios/coleco`, `bios/eos`, and
`bios/writer`, with embedded BIOS arrays also present in the source. No
portable cartridge-save or external key-file contract is claimed.

## AdViEmulator

The official v1.0 archives are legacy 32-bit Linux and Win32 binaries. Their
README describes the Adventure Vision emulator, while the binary exposes a Qt
options dialog, `AdViEmulator.ini`, and key-capture labels for four buttons and
four stick directions. The published artifact does not expose a stable,
reviewable serialized key-name schema, so `controller_adviemulator_standalone`
refuses to fabricate one. The binary references `avbios.bin` and `avsound.bin`
firmware resources, but the archives publish no external checksum contract.

None of these modules proves device discovery, executable startup, effective
gameplay input, firmware validity, or save/state round trips. Those remain
launch-time/runtime oracle responsibilities.
