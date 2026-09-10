# ScummVM controller contract

Pinned libretro source: `d79f8bb292c89d2f45f4e30eeed3b1222bc28329`.
The source-derived layout covers the full default RetroPad mapper, not an
assumed A/B mouse convention:

- D-pad and left analog stick move the cursor.
- Right analog directions emit keyboard arrow keys.
- L/R are left/right mouse buttons; R2 reduces cursor speed while held.
- A/B/X/Y emit Space/Return/F5/Escape, respectively.
- Select emits the virtual-keyboard event; Start emits the main-menu event.
- L2, L3 and R3 are unassigned and must be pinned as such in a fixed profile.

`libretro-os-inputs.cpp` consumes mapper events on frontend port zero. Cursor
motion has acceleration, analog deadzone/response and fine-control settings.
Mouse-button transitions generate down/up events; ordinary mapped keys go
through `processKeyEvent` with mapper-derived modifiers. Engine-specific actions
are not inferred from those key codes. The pointer-device option separately
selects physical mouse or touch input; neither is automatically covered by this
RetroPad layout.

One explicit launch profile is now enabled, with all 24 mapper slots, fixed
cursor settings, both analog sticks and the mapped digital controls. It accepts
lowercase `.scummvm` hooks and snapshots startup inputs. No builds, tests or
runtime probes have been run.

The catalog guard now enforces all 24 mapper entries and six cursor settings:
RetroPad-only pointer input, speed 1.0, acceleration 0.2 seconds, linear analog
response, 15 percent deadzone and fine-control divisor 4. The source actually
divides motion by 4; the option's displayed percentage is not used as proof of
the mathematical gain. L2/L3/R3 are explicitly unassigned rather than inherited
from saved mapper state. Native keybindings remain preserved as engine-specific
behavior rather than being overwritten.

The first startup guard now validates `.scummvm` hook content against the core's
actual parser: lowercase extension, one plain target/engine identifier on the
first line, representable parent directory and bounded command length. It does
not execute game detection or claim that a target exists. The core itself may update
native configuration during startup; Lunchbox must not describe its read-only
preparation as a guarantee that ScummVM never writes settings.

Hook and optional native-config snapshots are now implemented and wired into
calibrated launch preparation. They use bounded regular-file reads, SHA-256 and
canonical paths; a previously absent `scummvm.ini` appearing before launch is
also detected. The selected system directory is pinned in the launch-scoped
RetroArch config. Native RetroArch, an explicit existing system directory and
no custom environment or unresolved includes are currently required. Saved
engine keybindings are preserved; the contract labels emitted keys/events,
not a universal in-game interpretation. Physical pointer modes, game-specific
bindings and arbitrary key chords remain separate work.

Some-mode coverage is 88/95 cores (92.6%).
