# Neko Project II Kai input contracts

Source revision: `ac05f003bf44f58a235f5b31de0cd42dabdbf1a8` in
`libretro/NP2kai`. The five fixed keyboard presets and three mouse profiles are
implemented in the controller catalog. Builds and runtime tests remain deferred.

## Native joystick is board-dependent

`sdl/joymng.c` polls only frontend port zero. Its direction bits are 0–3;
A/Y/B/X clear bits 6/5/7/4 respectively. Those four bits are not sufficient
evidence of four independent game buttons.

In `sound/fmboard.c`, `fmboard_getjoy`:

1. Gates joystick input using OPNA register 15 and can merge keyboard-joystick
   state according to `np2cfg.KEY_MODE`.
2. Applies `np2cfg.BTN_RAPID` to the high nibble.
3. Combines bits 6/7 into trigger bits 4/5, alongside X/Y's direct bits.
4. Swaps the two triggers when `np2cfg.BTN_MODE` is enabled, then replaces the
   high two bits with OPNA interrupt status.

Thus the normal FM-board route has two guest triggers, with alternate frontend
button sources and configuration-dependent rapid/swap behavior. Sound Blaster
and PC-9801-118 gameport code uses a different transformation; the sound-board
choice must be part of the contract, not inferred from the name Atari Joypad.

Before enabling a native-joystick profile, resolve the effective sound board and
legacy `btnRAPID`, button-swap and keyboard-joystick settings. The existing
private `np2kai_joymode` option selects the polling mode but does not alone fix
those downstream hardware transformations. User INI files must be preserved.

## Existing modes

Fixed keyboard presets use twelve explicit key mappings and L3 for the menu.
They bypass Manual Keyboard's INI mapping array but replace physical-keyboard
polling; there is no additional physical-keyboard typing path in those modes.

Mouse mode retains physical-keyboard polling. D-pad movement accelerates while
held. Optional left/right native-stick motion is integer-scaled; stick press
clicks left, or right while L2 is held. The menu uses the other stick button to
avoid a click/menu collision. Physical mouse input is disabled in these mouse
profiles, although the core still recognizes physical middle-click for its menu.

Native joystick and custom INI keyboard modes are unfinished. Current core
coverage counts NP2kai because of its eight implemented keyboard/mouse profiles,
not because every input mode is covered.
