# EightyOne standalone-native controller contract

The Windows writer is pinned to `charlierobson/EightyOne@4de3cdb8cff0060ad03e43ae2681b0628a9c5f5d`. `Source/main_.cpp` reads and writes the real INI keys, while `Source/Joystick/Joystick.cpp` establishes WinMM enumeration and polling behavior.

`controller_eightyone_standalone.rs` preserves a copied `%APPDATA%\EightyOne\<executable-basename>.ini` (or the executable-local/explicit `.INI` alternative) and patches only `[MAIN]` `ConnectJoystick1/2`, `EnableJoystick1/2AutoFire`, and `Joystick1/2Controller`. A missing port writes controller `-1` and disables connection/autofire. A connected port accepts a distinct measured WinMM ID from 0 through 15.

The stored IDs are runtime slots, not stable controller identities. `InitialiseJoysticks` calls `joyGetDevCaps` for IDs 0 through 15, and the read paths call `joyGetPosEx` for the configured slot with source-fixed axes, trip points, and the first ten buttons as fire. The launch layer must resolve and recheck the intended devices for the exact native Windows process immediately before launch. The selected emulated machine and guest joystick interface remain user/baseline configuration and must not be overwritten by this physical binding writer.

Preserve `[HARDWARE]` programmable guest keys, all other application configuration, media, saves, and user-selected state paths. Windows is the only established native host; Wine is a Windows runtime, not native Linux support. A patched INI is not proof of executable startup or effective gameplay input.
