# Set up controllers

There are two separate jobs: controlling Lunchpail and controlling the game.
A pad can work in the library before its emulator mapping is configured.

## Choose the players

1. Open controller setup from Settings or the game's **Settings & mappings**.
2. Choose the controller for **Player 1** from its dropdown.
3. Use **Add player** for additional players, then choose their controllers.
4. Continue to the target system and review the mapping.

When opened from a game, player choices are remembered for that system.
For example, choosing a Brawler64 for arcade games leaves your NES controller
choice unchanged. Systems without a saved choice use your default players.
In Settings, switching the target system recalls that system's saved players.

Press a button to identify a connected controller. Generic names are common;
**Rename** gives a pad a name you will recognize next time.

Virtual controllers are hidden by default. Enable **Include virtual controllers
(Steam Input, etc.)** if that is the device you intend to use.

## Record buttons when needed

Choose the controller model or physical layout, then use **Record buttons** or
**Review / change buttons**.

Press the highlighted control and release it before moving on. Move sticks and
triggers through their range, then let them return to rest. Skip a control your
pad does not have rather than assigning another button twice.

Choose by the controller's physical layout, not just its reported Xbox-style
button names. Some USB modes expose the same pad differently.

Changing USB mode, swapping otherwise identical controllers, or moving a
device without a unique serial to another USB port can require reviewing
its assignment again.

## Save a mapping at the right level

In **Apply this button mapping to**, choose where you want the mapping to apply:

| Scope | Use it when |
| --- | --- |
| Game | This one game needs different buttons. |
| System | You want the same layout across a console or arcade system. |
| Emulator/core | You want a default for games using that emulator or core. |

Game mappings take precedence over system mappings, which take precedence
over emulator/core mappings. Use **Remove this override (inherit)** to go back
to the broader choice.

Player assignments and controller setup save as you go. Edited button mappings
have their own **Save Player…** button; changing that editor is not the same as
saving it.
Saving a **System** mapping also remembers that system's players, so changing
your default controller later does not replace them.

## D-pad, sticks, and extra buttons

For supported targets with only one directional control, the mapping can use
both the source D-pad and left stick for that same target. You can then use
whichever feels better. Targets with separate D-pad and analog controls retain
that distinction.

For DS/DSi stick-stylus profiles, both the D-pad and left stick control
movement, while the right stick remains dedicated to the stylus.

For a two-action-button target, a four-face-button source can use its spare
buttons as turbo where the emulator supports it, or as duplicates of the
main pair. This is not limited to one controller brand. Hardware turbo buttons
that merely repeat another input cannot be mapped as independent buttons.

The mapping review labels these extra connections **Turbo A/B** or
**A/B (duplicate)**. Hover a button to see its action. With the N30's independent
X/Y layout and RetroArch FCEUmm, X is Turbo A and Y is Turbo B.

See [Arcade controls](arcade.md#six-button-and-n64-style-pads) for six-button
layouts on N64-style pads.

## Steam Controller and Steam Input

A controller may expose physical input, mouse/keyboard input, and a virtual
gamepad. Those are not always interchangeable.

On Linux, Steam's gamepad mode can expose a virtual Xbox controller. If you use
that route, leave Steam running and select the intended virtual pad. Do not
assign both the physical and virtual versions as separate players.

If each press happens twice, check the controller's mode and any software that
also sends keyboard or mouse input. Use Lunchpail's input test to identify which
device is producing events before changing the game mapping.

## If a pad is missing or the wrong one controls the game

- Open the player dropdown, not just its currently selected label.
- Check **Include virtual controllers** if using Steam Input.
- Press a button to identify generic device names, then refresh the list.
- Confirm the player assignment and the saved mapping scope.
- Review the selected emulator's target input mode.
- Relaunch after changing a game mapping.

A saved controller that is disconnected can remain in your preferences.
Lunchpail asks you to reconnect it or choose a replacement instead of silently
giving its player slot to another pad. You do not need to delete your mappings.

## Navigating Lunchpail

Use the D-pad or left stick to move, the south face button to select, and the
east face button to go back. Bumpers page lists; supported triggers jump to
their beginning or end.

Navigation pauses during button recording and while a game owns input.
Native file pickers and text entry may still need a keyboard or mouse.
