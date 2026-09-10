# VICE standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

VICE trunk r46226 (official mirror VICE-Team/svn-mirror
d322f7a8d6c269b97162c74e73214c58eaad9a71; the saved executable SHA-256 pins the
actual binary):

- `joyport/joystick.c` `mapping_dump_header`/`mapping_dump_map` — a `.vjm`
  joystick map holds `#` comments, the `!CLEAR` keyword and entries
  `<device> <inputtype> <inputindex> <action> <params>`:
  inputtype 0=axis, 1=button, 2=hat; buttons use the plain button index,
  hats `hat*4 + (0=up,1=down,2=left,3=right)`, digital axes
  `axis*2 + (0=positive,1=negative)`; action 1 (joystick) takes the pin
  bitmask 1=up, 2=down, 4=left, 8=right, 16=fire, 32=fire2, 64=fire3.
  The device column is the host device index in SDL enumeration order.
- `joyport/joystick.c` `set_joystick_device` — resource
  `JoyDevice1`/`JoyDevice2` binds a host device to a control port with the
  value `4 + host device index` (`JOYDEV_REALJOYSTICK_MIN`); config files are
  `ResourceName=ResourceValue` lines (`resources.c`).
- `arch/sdl/joy.c` — `-joymap <file>` selects the mapping, `-config <file>`
  a private config file; `-joythreshold` (default 10000) and `-joyfuzz`
  (default 1000) drive digital axis engagement at |value| > 10000 with the
  fuzz window added on release.

## Integration

- `controller_vice_native.rs` renders the `.vjm` body (`!CLEAR` plus grammar
  lines) and the private `JoyDeviceN` config with unit tests;
  `controller_vice_native/settings.rs` holds the saved launch setups;
  `controller_vice_native/session.rs` probes the trusted SDL2 runtime per
  player (SameBoy observation path), translates calibrated controls through
  the classic physical map with 10000-threshold and 1000-fuzz travel checks,
  and stages the private `-config`/`-joymap` pair in a session-owned tempdir;
  `controller_vice_native/native_command.rs` owns the launch.
- Guided target `vice:standalone-vice-joystick-panel` (two players) registers
  a Commodore digital-joystick layout (up/down/left/right/fire plus optional
  fire2/fire3) covering Commodore 64/128/Plus4/VIC-20/PET/MAX platforms.
  Saved per-game setups are reused for another game of the same emulator.

## Limits

Keysets 1/2, paddles, potentiometer (paddle-analog) axes, mouse, light pens,
userport adapters, the 4-player adaptoids and per-machine extra control ports
beyond port two are not covered. Which VICE binary runs per platform follows
the application's existing executable discovery; this contract only maps the
two standard control ports. Runtime behavior is unverified; the threshold and
fuzz resources are assumed at their defaults.
