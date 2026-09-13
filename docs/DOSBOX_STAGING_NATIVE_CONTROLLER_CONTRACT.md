# DOSBox Staging native Linux controller contract

Source oracle: `dosbox-staging/dosbox-staging` commit
`d9135010ea56c2faa0bb8062aaf30a8541bf22f3` (the captured v0.83 source),
especially `src/gui/mapper.cpp`, `src/gui/mapper.h`, and
`src/config/config.cpp`. The captured platform record is
`emulator_details/records/dosbox-staging.json`.

DOSBox Staging has a native SDL mapper. Its `[sdl] mapperfile` selects a plain
text mapper file (default `mapper-sdl2-<version>.map`). `src/gui/mapper.cpp`
serializes each named emulator event as an event name followed by quoted
`BIND` strings such as `key <scancode>` or `stick_0 button 6`; joystick axes,
buttons, hats, keyboard modifiers, and SDL2/SDL3 event paths are mapper
grammar, not a fixed gamepad layout. The primary configuration and mapper
locations are resolved by `src/config/config.cpp`; a local `dosbox.conf` can
also override the primary configuration after it is loaded.

The native module emits and patches the source mapper grammar while accepting
explicit caller-supplied EVENT names and BIND strings. DOSBox Staging emulates
a PC and does not define a universal set of guest actions: one DOS title may use keyboard
scancodes, another may read one of several joystick types, and a third may
expect mouse input. A generic adapter cannot choose event names or claim that
an SDL pad is a particular guest joystick without a reviewed per-game control
contract. Emitting mapper lines for guessed names would be a misleading
partial mapping.

The runtime has no save-state feature (upstream issue #313 is closed
`not_planned`); game saves remain ordinary files in mounted directories or
disk images. A per-game caller may copy a declared primary/local configuration
and mapper into an isolated root, set an explicit mapper path,
hash executable/config/probe inputs, and preserve mounted save directories,
disk images, and the absence of BIOS files. Native Linux and Flatpak
`io.github.dosbox-staging` must remain separate launch contracts.
