# Hatari controller contract

Source: `libretro/hatari` revision
`ae4caa2e15ff1116fe5cb8e93b3a42a1aaa81580`.
The current database links core `hatari`, owned by Hatari, to Atari ST.
These source findings and layouts do not yet enable a launch profile.

## Joystick route

`src/retro/joy_ui.c::JoyUI_ReadJoystick` swaps ST ports 0 and 1 when reading
frontend inputs. Frontend player 1 therefore drives ST port 1; player 2 drives
ST port 0. It reads left analog X/Y, then overrides each direction from the
RetroPad D-pad. B becomes button 1, Y button 2, and A button 3.

`src/joy.c::Joy_GetStickData` establishes their actual meaning: B fires, Y
supplies Up or a Space shortcut according to `bEnableJumpOnFire2`, and held A
generates autofire on the single ST fire line. The descriptor's "Fire 3" label
does not establish a third independent button. Separate jump and Space layouts
now preserve these meanings.

The libretro `JoyUI_NumJoysticks` and `JoyUI_ValidateJoyId` functions return zero
and false, but the main joystick-read path does not call the validator.
`JoyUI_SetDefaultKeys` sets the default real-stick mode, and core options later
set ST ports 0/1 explicitly. This is source evidence, not runtime confirmation.
The no-op `retro_set_controller_port_device` does not disconnect core ports;
port options must establish topology. Enhanced STE joypad buttons use a separate
stubbed `JoyUI_GetRealFireButtons` path and are not equivalent to the ST joystick.

Required options include `hatari_joystick_port0/1` (`none`, `keys`, `real`),
`hatari_joystick_autofire` and `hatari_joystick_jump_fire2`. Global autofire and
the held-A autofire action are distinct. Configuration/load order, option
persistence, machine selection and startup requirements still need tracing
before enabling launch profiles.

`retro_init` resolves `system_directory/tos.img`, passes it as `--tos`, runs
`Main_Init`, and then applies runtime options. `Main_Init` loads global and user
configuration before boot options; `OPT_TOS` disables native autosave loading
when the explicit image is accepted. Libretro's nonempty save directory becomes
the Hatari home directory, so its user configuration is `save/hatari.cfg`.
Keyboard initialization registers the frontend keyboard callback; it does not
provide a virtual keyboard or a RetroPad menu shortcut.

The Rust preparation snapshot now requires existing explicit system/save
directories, captures the TOS image and required native config without rewriting
them, rechecks file contents and symlink destinations, and can pin RetroArch's
save-path sorting to the inspected directory. This is not firmware validation.
The calibrated-launch path now retains and rechecks this snapshot for Hatari,
resolves explicit save-directory sorting (including the core's lowercase
`hatari` library name), and appends the inspected paths to the launch-scoped
configuration. Native RetroArch, a resolved base configuration without includes,
and no custom launch environment are currently required. No Hatari launch profile
is automatic: the catalog now exposes four explicit ST joystick profiles for
one/two players and jump/Space shortcuts, using direct ST/MSA/DIM/STX floppy
content. These are source-grounded implementations, not runtime validation.

The catalog now has a dedicated `hatari_st_joystick` guard. It requires an
explicit fresh ST launch, six neutralizable frontend ports, one or two active
RetroPads, fixed ST port 1 joystick mode, and ST port 0 disabled for one player
or enabled for two. Jump/Space layout selection must match the fire-2 option;
global autofire must be disabled so only the labeled held-A action pulses fire.
Hatari launch preparation rejects profiles without this guard. The native config
must explicitly set `nJoystickMode = 0` in each `[Joystick2]` through `[Joystick5]`
section. These user-file assignments load after the global config and disable
extra routes that the libretro device setter does not disconnect. The read-only
resolver rejects missing assignments, nonzero modes, repeated extra-port
sections, ambiguous headers, BOM/NUL content and lines reaching the core's
fixed input-buffer limit. It does not rewrite the user's settings. Both initial
preparation and the pre-launch snapshot recheck apply this requirement.

## Other inputs still in scope

`src/retro/gui_event.c` reads actual frontend mouse deltas and three mouse
buttons; the middle button starts an emulated double-click sequence. Motion is
ignored during the first ten VBLs and adjusted for screen zoom. These are not
RetroPad stick-mouse controls. `src/retro/keymap.c` contains ST keyboard mapping;
keyboard callback registration and per-content keyboard requirements remain to
be traced. No virtual keyboard or controller-menu binding is inferred here.

No builds, tests or runtime probes have been run. Hatari now has an enabled ST
joystick mode; overall some-mode coverage is 86/95 cores (90.5%). Other Hatari
inputs, machines and media remain incomplete.
