# Controller setup

Lunchbox listens for controllers in desktop and Couch Mode. Desktop navigation
uses the D-pad/left stick, South to activate, East to go back, and bumpers to
move focus. Start opens controller settings. Navigation is suppressed while a
game runs or Lunchbox is inactive. The profile recorder temporarily suspends
navigation while it captures the next physical control.

## Smart setup

Open Settings → Controllers. The live input check shows the device and the
normalized control it reports. Known standard diamond devices are recognized;
unusual pads are not assigned wiring based solely on their product name.
In particular, generic Xbox 360 identity `045e:028e` does not establish a diamond
layout: the connected USB pads both use it with different hardware revisions.
Those entries display their revision and device node to distinguish them.
Without a unique serial, saved identities are tied to the USB port. Keep each
pad on the same port; moving or swapping pads requires checking its saved layout.

1. Choose a physical layout for each nonstandard device. For horizontal pads,
   `East` is the reported Xbox-style B and `South` is A; confirm the left/right
   wiring with the live check before selecting the normal or swapped preset.
2. Enable automatic selection, then optionally choose a preferred connected
   controller for each system family. Disconnected preferences remain saved.
3. For N64 or six-button layouts, create a custom profile in the editor. Select
   the virtual target and use **Record a physical button / C-button**. Right-stick
   directions are supported as both sources and targets. Assign the saved
   profile in the appropriate system-family row. One controller can use
   different profiles for N64 and Sega/arcade.
4. Save settings. Explicit player order wins over automatic ordering. In
   automatic mode, game overrides win over device/system profiles, which win
   over device defaults. Hidden devices are excluded. Unknown devices retain
   native mappings unless explicitly configured.

For two-button systems, a standard diamond pad uses West for the emulated
run/B input and South for jump/A. A verified horizontal pad retains its
left/right arrangement. These are control-layout defaults, not semantic
knowledge of every game's run/jump actions. Individual games may need overrides.

## Runtime support and boundaries

- Frontend navigation uses GilRs and is not dependent on InputPlumber.
- Game-launch remapping currently uses the Linux InputPlumber adapter. A running
  service alone is insufficient: it must manage the selected physical device.
- **Enable Linux controller routing** explicitly confirms a system-wide change
  to the running service. It can affect other applications and Steam Input.
  Close games first. This action does not install packages or drivers. Use
  **Use native controller routing** to disable that service-wide routing.
- Per-launch profiles, target devices, and player order use the existing
  capture/restore session. They do not overwrite emulator configuration files.
- Standalone Windows/macOS emulator-configuration adapters are not implemented.
  Device-specific N30, Retro Fighters, and Steam Controller mode/receiver
  verification still requires the actual hardware. Do not assume product names
  or printed A/B labels prove a particular mapping.
- System-native file pickers retain the host's input behavior. Desktop menu
  navigation does not synthesize global keyboard input or provide controller
  text entry.
- Simultaneous opposing C-buttons cannot be recovered if a hardware mode exposes
  them as a single axis and drops that information. Use a mode with independent
  button reporting where available.

## Verification

Regression coverage includes controller-family selection, unplugged preferences,
explicit game overrides, per-device N64/Sega profiles, C-button YAML generation,
desktop grid navigation, buttons/checkboxes, combo-box selection, popup back
handling, and inactive-window suppression. Physical-device end-to-end testing is
separate from those deterministic tests.
