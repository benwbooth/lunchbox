# Oricutron, XRoar, and ZEsarUX native controller contract

These adapters are source-pinned contracts, not claims of emulator-wide
compatibility.

## Oricutron

`pete-gordon/oricutron@002279fce9fa756d1d63cdc40ae97939eb7de7ed` parses
`oricutron.cfg` beside its executable.  `joystick_a` and `joystick_b` select
`JOYMODE_SDL0` through `JOYMODE_SDL9`, and `joy_setup` calls
`SDL_JoystickOpen(mode - JOYMODE_SDL0)`.  The file has no GUID, serial, or
device-path selector. The writer patches `joyinterface`, both Atmos/Oric-1
selectors and both Telestrat selectors using only a runtime index measured for
that exact launch. A launch guard must recheck physical identity and SDL order;
the source's fixed axis/hat/button interpretation remains runtime-test work.

## XRoar

`xroar@0229f97a636c3c80d51fd27e7d145d792f0a8932` supports source-shaped
`joy`, `joy-axis`, `joy-button`, `joy-right`, and `joy-left` directives.
Its SDL3 physical module writes `physical:<device>,<control>` and opens the
device through the current SDL joystick/gamepad enumeration. That device
number is enumeration order, so the writer accepts it only as a same-launch
measurement. It appends complete two-axis/two-button SDL3 profiles, including
the source's `%<button-mask>` form, then selects them with `joy-right`/`joy-left`.
XRoar's ROM, tape, disk, and snapshot paths remain caller-owned.

## ZEsarUX

`chernandezba/zesarux@2e529034957cb61dec35d52d0008f03dcd35c019` parses a
command-style config file and exposes `--realjoystickpath` plus
`--joystickevent <button-or-signed-axis> <event>`.  Native Linux opens the
exact path with `open(2)`; the writer requires `/dev/input/by-id/` and emits
the Kempston `Up`, `Down`, `Left`, `Right`, and `Fire` event table.  Launch
with `--configfile` pointing at the private file.  The mapping writer does not
relocate ROM, firmware, tape, disk, snapshot, or autosnapshot roots; those
must be supplied and validated by the launch layer.

All three paths still require runtime verification against the exact
executable, SDL/input backend, device symlink, firmware, and content before
being presented as launch-ready.
