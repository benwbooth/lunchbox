# DOSBox Pure controller contract evidence

Implementation source: `schellingb/dosbox-pure` revision
`7f6e8fb7385fa446d1444d671063268520bf9b54`, especially
[`dosbox_pure_pad.h`](https://github.com/schellingb/dosbox-pure/blob/7f6e8fb7385fa446d1444d671063268520bf9b54/dosbox_pure_pad.h).
Seventeen explicit keyboard, mouse and joystick catalog launch profiles now use this evidence.
These are source-grounded implementations, not tested launch contracts.

## Deterministic preset selection

`retro_set_controller_port_device` delegates to `SetPortMode`. Explicit joypad
subclasses select fixed presets; the ordinary joypad selects the mutable mapper.
`SetInputDescriptors` applies saved `PADMAP.DBP` and detected game mappings only
in mapper mode. Explicit preset mode skips those saved mappings, applies its
preset and then fills remaining controls with generic keyboard bindings.

| Libretro device | Preset |
| --- | --- |
| 257 | Generic keyboard |
| 513 | Mouse on left analog stick |
| 769 | Mouse on right analog stick |
| 1025 | Gravis four-button gamepad |
| 1281 | First two-button DOS joystick |
| 1537 | Second two-button DOS joystick |
| 1793 | Thrustmaster flight stick |
| 2049 | Both DOS joysticks |

All eight frontend ports must be accounted for. The presets target shared DOS
devices, so the number of frontend ports is not an independent-player capacity.
Use explicit selection and fresh starts, with game-specific key requirements
visible rather than labeling the generic keyboard as universal gameplay input.

## Generic keyboard, frontend port one

D-pad and left stick send arrow keys. B/A/Y/X send left Ctrl/left Alt/left
Shift/Space. Select/Start send Escape/Enter. L/R/L2/R2 send 1/2/3/4. The right
stick sends Home/End on X and Page Up/Page Down on Y. These analog input pairs
generate keyboard keys, not proportional joystick motion.

With `dosbox_pure_on_screen_keyboard = false`, L3/R3 send F1/F2. With the
menu enabled (`keyboard`, `true` or `onlyosk`), L3 is reserved for the overlay;
the preset collision chain shifts F1 to R3 and drops the direct F2 mapping.
The corresponding layouts must differ. Fix `dosbox_pure_auto_mapping = false`
and `dosbox_pure_keyboard_layout = us` for a reproducible explicit contract.

Mouse and joystick presets also fill unused controls with generic keyboard
bindings through a collision chain, not simply a same-button fallback. Derive
the complete resulting mapping before publishing their layouts.

## Two-player topology

The two-joystick profile uses frontend devices 1281 and 1537, targeting separate
DOS axis/button registers. Player two supplies only its two buttons, D-pad and
native left stick; keyboard fallback and menu ownership stay with player one.
This does not imply two Gravis pads or two flight sticks: those presets consume
both DOS joystick register sets. The guest application must support the selected
hardware and may need its own calibration.

The core sends D-pad joystick endpoints and native stick axes as changed events.
If both are used simultaneously, the last changed event wins; the mapping does
not promise a synthesized axis combining both inputs.

## Remaining implementation

The keyboard, mouse and joystick layouts and explicit profiles are implemented
with one active controller and all eight frontend ports accounted for. The seven
mouse/joystick layouts include the collision-resolved keyboard fallback with L3
reserved for the menu. Keyboard analog pairs use signed magnitude 12000 for
press/release; native mouse/joystick pairs retain analog motion. Remaining work
includes further multi-controller topologies, mutable per-game mappings,
and expanded overlay documentation. All eight fixed presets now have separate
menu-enabled and no-menu layouts; no-menu fallback does not reserve L3. Extend
media/launch boundaries. The core reports
`DOSBox-pure` as its library name. DOS-only platform aliases must not accidentally
claim PC-98 coverage from the database's DOSBox-X relationship. Configuration,
playlist, Windows and directory entrypoints need their own decisions. No tests,
builds or runtime launches were performed for this evidence checkpoint.
