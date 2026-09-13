# Clock Signal standalone controller contract

`CLK (Clock Signal)`, `Clock Signal`, and `Clock Signal (CLK)` are catalog
aliases of one upstream project, `TomHarte/CLK`, pinned at commit
`096de57445920ecf16cf066979422e34aceb843a`. They therefore share one behavior
and one controller boundary.

The official README states that source and macOS releases are hosted on GitHub
and that the Linux build is Qt-based (with an SDL build also available). In
`OSBindings/SDL/main.cpp`, the SDL frontend opens every `SDL_Joystick` by
enumeration index and forwards axes 0/1, hats, and each raw button index
directly to the emulated machine. It has no persistent controller profile or
device selector. The Cocoa frontend in
`OSBindings/Mac/Clock Signal/Machine/CSMachine.mm` repeats that direct mapping
and contains the explicit TODO “configurable mapping from physical joypad
inputs to machine inputs”.

Accordingly, `controller_clock_signal_standalone` refuses to fabricate a
mapping writer. This is an artifact-backed serialization limitation, not a
refusal to work with externally enumerated devices: a future launch adapter
must measure and recheck the selected device indices/identifiers immediately
before startup, using the same SDL environment as CLK.

The three records intentionally keep the same source pin, paths, input
boundary, and platform claims. No separate behavior is inferred from the
catalog aliases.
