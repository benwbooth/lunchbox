# Steem SSE controller contract

Primary source is SourceForge SVN revision 1501, repository UUID
`dd3fbd3b-ef5d-4b78-98e8-da144fa02e5b`, under
`trunk/steemsse/steem/libretro`. The exported wrapper was inspected without
building or running it. This is not the older Steem Engine GitHub tree.

`libretro-core.h` defines two ports. `update_input` maps frontend port zero to
ST port one and frontend port one to ST port zero. D-pad inputs set the four
direction bits; A and B both assert the single fire bit. Hatari's jump, Space and
held-autofire bindings are not present. The catalog now describes that joystick
layout, without enabling a launch profile yet.

The wrapper reads real mouse deltas on frontend port zero and supplies them to
the IKBD. Mouse left/right buttons share the fire bits of ST ports zero/one.
Those bits are refreshed from the mouse before joystick fire is ORed into them.
`retro_set_controller_port_device` only logs the request, so disconnecting a
frontend device is not itself proof that the core stops polling it.

The reported library name is `SteemSSE`. Initialization sets its RunDir to
`system_directory/SteemSSE` and may create that directory. Firmware, core options,
ST/STE selection, downstream joystick handling and configuration/load ordering
must be traced before enabling fixed launch profiles. Keyboard and mouse remain
part of the overall coverage goal.

Startup tracing establishes that `retro_load_game` inserts the supplied disk and
calls `retro_reset`, which reapplies core options. There is a significant model
interaction: `get_default_tos` installs embedded EmuTOS and unconditionally sets
`ST_MODEL = STE`. Consequently an embedded-ROM profile cannot truthfully promise
STF just by setting `sse_st_type = STF`. The new catalog guard requires STE and
`sse_st_os = Emu`, explicit fresh selection and two joystick ports.

The `Emu` option still first attempts `RunDir/tosEmu.img` before falling back to
the embedded image. The forthcoming directory guard must account for that file
before claiming embedded-ROM identity. Native TOS-image/STF profiles remain
separate work. The guard is not launch-enabled yet.

The Rust directory snapshot is now implemented and connected to calibrated
launch preparation. It requires existing explicit system and `SteemSSE`
directories, rejects any `tosEmu.img` entry (including symlinks), pins the inspected
system path in the launch-scoped configuration, and rechecks directory identity
and override absence before launch. It creates no directories and alters no
firmware. Native RetroArch without custom environment or unresolved config
includes is currently required. Downstream joystick/configuration tracing and
profile activation remain outstanding.

The pinned `ikbd.cpp` excludes `joy_read_buttons` and `joy_get_pos` under
`SSE_LIBRETRO`, preserving wrapper-supplied joystick bits. Macro playback and
dongle modes can still alter them; the `SSE_LIBRETRONUKE` build condition also
controls native port-activity and shortcut handling. Those conditions need to
be resolved for the exact supported artifact, not inferred from the wrapper.

The wrapper explicitly describes a DLL and marks the Linux core as TODO.
Current preparation now rejects native non-Windows execution with a specific
explanation. No Windows DLL is presented as a native Linux core, and no launch
profile is activated. Completing Linux coverage requires a real native port or
a Windows-runtime adapter with controller transport and correct path handling;
the directory guard alone is not such an adapter.

Some-mode coverage remains 87/95 cores (91.6%). No builds, tests or runtime
probes have been run.
