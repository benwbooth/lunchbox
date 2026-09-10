# Snes9x GTK standalone mapping

## Current status

Partial native Linux launch dispatch is connected for explicitly saved GTK 1.63
SNES setups: source-to-destination visual review, calibration translation,
standard pads/multitap configuration, private config mounting, exact runtime
hashes, startup mount/library/device checks, cancellation/cleanup and live kernel
topology checks. No tests, builds, probes or emulator launches were run.
Qt, Wine/Flatpak, custom launch arguments and runtime compatibility remain open.
Coverage is 93/94 RetroArch profiles (98.9%) and 6/249 standalone candidates with
partial dispatch (2.4%). This supersedes historical pending-dispatch notes below.

## Implementation history

Conflict handling now resolves effective last-write-wins entries and native
hash-comment escaping before inspecting old routes. It clears collisions in
other players' banks as well as shortcuts, comparing native routing identities
without threshold bits. Unrelated joystick shortcuts are preserved rather than
clearing every shortcut on an assigned device. No tests were run for this edit.

Source pin: Snes9x 1.63, `921f9f7b83660eb44ad263022a57a4a029057c37`.
Primary contracts: `gtk/src/gtk_binding.cpp`, `gtk_binding.h`, `gtk_config.cpp`.

Implemented native joystick binding encoding and configuration-string rendering,
including one-based device numbers, button/axis separation, threshold validation,
and threshold-independent routing identity. This is the GTK frontend contract,
not the libretro core or Qt frontend. Native configuration stores mappings in
zero-based `Joypad N` sections; alternate banks map back onto five SNES players.

The configuration writer now emits all twelve standard controls for selected
players 1–5, clears their alternate binding bank and turbo/sticky bindings, and
disables joystick shortcuts on their assigned physical devices while retaining
keyboard shortcuts. It rejects duplicate native input ownership, including axis
bindings that differ only by threshold. Output is a private replacement string;
the original file is not edited. Native port selection now sets port 0 to joypad
for player 1, and port 1 to joypad for player 2 or multitap for players 2–5 when
any player above 2 is selected. Unneeded physical SNES ports are left unchanged.
Continued-line configuration is explicitly rejected pending normalization,
rather than allowing continuation syntax to hide conflicting input entries.

Raw SDL input translation now handles buttons, measured axes and cardinal hats.
Hat axes follow GTK's native appended vertical/horizontal order, with up positive.
Measured axes select a representable integer-percent threshold that separates
release from press under native inclusive comparisons; unrepresentable endpoints
are rejected instead of silently using a generic deadzone.

The native SNES catalog profile now uses the existing visual layout with GTK
output names and a separate validated transport. Calibration translation is
connected to supplied SDL2 snapshots and physical maps, verifies released button,
axis and hat state, and builds a complete player mapping. Snapshot capture and
launch-time device identity ownership are not connected yet.

Saved setup persistence and a no-I/O visual-review model are implemented for
one to five distinct players/controllers, exact emulator/content identity,
native config/runtime paths and trusted executable hash. Settings validation
rejects duplicate setups and invalid assignments. UI actions now load, visually
review and stage Snes9x setups through the shared native mapping dialog. The
editor explicitly labels launch integration as pending; review/staging perform
no device capture. JSON size limits and full-list validation precede staging.

Private configuration ownership is implemented: bounded original-file capture,
full mapping render, a retained temporary snes9x.conf and Linux file-mount
arguments. Source bytes and canonical path are rechecked before handoff. Native
GTK config-path resolution mirrors XDG/HOME/cwd precedence without creating
directories; saves and other paths are not relocated. Launch integration must
check this resolved path against the saved source_config.

Native session preparation now captures SDL2 inventory through the existing
helper, resolves every selected physical kernel device, captures measured raw
controls per player, constructs private configuration, and retains topology and
ordered routing for launch-time rechecks. Native GTK has ten joystick slots;
preparation requires a complete contiguous fresh inventory within that limit.
Capture code has not been executed. Runtime executable ownership, child startup
confirmation and launch dispatch remain pending.

Native command preparation now verifies the exact saved ROM and GTK config path,
pins hashes for the executable/helper/SDL/bubblewrap/content, and builds the
private config-mount command while retaining the input session. Only a single
ROM argument is accepted until other launch overrides are modeled. GTK's main
initializes controllers and configuration before parsing normal arguments, so
preparation deliberately does not run a --version subprocess against original
settings. Runtime identity rests on the saved trusted GTK executable hash.

Startup confirmation and spawn ownership are implemented: find the owned native
child, require its mapped SDL library, private config inode and all selected
controller descriptors, then recheck configuration/runtime/input routing.
Cancellation, early exit and the bounded startup timeout kill and reap the
spawned child. The app dispatcher still needs to call this adapter; none of
these startup functions have been executed during implementation.
No tests, builds, probes or emulator launches were run.

Coverage remains 93/94 RetroArch profiles (98.9%) and 5/249 standalone candidates
with partial dispatch (2.0%). Snes9x is not counted as a sixth adapter yet.
