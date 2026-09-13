# Caprice32 standalone-native controller contract

The writer is pinned to `ColinPitrat/caprice32@6c12c4c92360065cdc229ac9ada7551f941436b8`. Source inspection of `src/cap32.cpp`, `src/keyboard.cpp`, and `src/configuration.cpp` establishes the real `cap32.cfg` grammar and the fixed SDL input path.

`controller_caprice32_standalone.rs` preserves a copied config and patches only `[system]` `joysticks=1`, `joystick_emulation=0`, `joystick_menu_button`, and `joystick_vkeyboard_button`. The latter two are one-based in the file and converted to zero-based SDL button numbers by Caprice32.

There is no physical-device selector in the config. The pinned executable opens the first eight enumerated SDL joysticks, but its input mapper recognizes event instance 0 as CPC joystick 0 and instance 1 as CPC joystick 1. Axes 0/2 and 1/3 are fixed to horizontal and vertical; buttons 0/1 are the two fires. The adapter therefore accepts the measured instance topology and fails unless it is exactly `0` and optionally `1`. The launch layer must repeat that probe against the exact SDL backend immediately before starting the exact executable. This is a usable fixed-order writer contract, not a stable identity claim.

Preserve all unrelated config, writable `.dsk` media, and `.sna` snapshots. The Caprice32 libretro core is a separate RetroArch contract. The source documents native Linux, macOS, and Windows builds, but does not establish a first-party Flatpak artifact. Generated config and a successful probe are not proof of startup or effective gameplay input on any host.
