# Controller setup

Lunchbox listens for controllers in desktop and Couch Mode. Desktop navigation
uses the D-pad/left stick, South to activate, East to go back, and bumpers to
move focus. Start opens controller settings. Navigation is suppressed while a
game runs or Lunchbox is inactive. The profile recorder temporarily suspends
navigation while it captures the next physical control.

## Smart setup

Each row now offers **Choose layout and calibrate**. Select a physical diagram,
then press each highlighted control. The wizard ignores other controllers,
requires release/neutral before advancing, and rejects assigning the same input
twice. Missing or duplicate hardware controls can be skipped. Stick directions
require analog events; C-buttons may be recorded as buttons or axis directions.
**Use calibration** stages the result; the main **Save settings** persists it.
Controller navigation is suspended while the wizard is open.

The system/emulator preview composes any calibrated layout with documented input
contracts. Horizontal four-button pads retain their lower A/B pair for NES/GB/PCE;
diamond pads use the left/bottom pair, and N64-style pads can use six-button
arrangements. The catalog distinguishes independent upper buttons from hardware
turbo repeats. These rules do not assume device button numbering.

**Apply saved calibrations at emulator launch** uses a temporary configuration
for Linux RetroArch (native or Flatpak) with FCEUmm, Gambatte, Genesis Plus GX,
Mupen64Plus-Next or Beetle PCE Fast. The PCE Fast adapter explicitly selects
two-button mode for its five ports; six-button games and mouse input need a
separate profile. The N64 adapter enables independent C-button mode. Only compatible
connected calibrations are assigned as players. Older preview-only calibrations
require one new recording to capture physical inputs. Automatic RetroArch
overrides/remaps and N64/PCE per-game core options are suspended for that launch;
your saved files are not overwritten. Disable this switch to retain native
emulator setup when an adapter is unavailable. Standalone and Windows/macOS
adapters remain unfinished. Existing advanced InputPlumber mappings remain a
separate path and are not applied on top of this adapter. See
`crates/lunchbox-app/data/controllers/README.md` for coverage and provenance.

Standard SNES joypads are also supported in the Snes9x and bsnes RetroArch cores
(two console ports, not multitap/mouse/lightgun or Super Game Boy). An N64-style
source uses physical B/A for SNES Y/B, preserving the run/jump thumb pair, and
C-left/C-down for X/A. Required controls such as L/R and Select must be calibrated;
a four-face-button pad without shoulders is not silently treated as a full SNES pad.

Launch contracts now declare exact platform aliases, libretro device IDs, and
player counts in the catalog. Extra connected pads remain available in Lunchbox
but only the highest-ranked compatible controllers fill the current input mode's
ports. A Game Boy game no longer attempts to create a second player just because
two calibrated controllers are connected. Preview-only contracts stay preview-only.

Open Settings → Controllers. The live input check shows the device and the
normalized control it reports. Known standard diamond devices are recognized;
unusual pads are not assigned wiring based solely on their product name.
Each controller row flashes for 1.2 seconds on input, highlights the reported
button, and retains its last reported control. Enable **Test controller input**
to pause menu navigation while identifying buttons; keyboard/mouse still work.
Type a friendly name below that row, then save settings. Names are also used in
the preferred-controller selectors; clearing a name restores the hardware label.
Linux feedback matches the exact evdev source path, not a shared Xbox name or
VID/PID. Portable backends refuse to highlight an ambiguous model-level identity
when identical devices cannot be distinguished; the global input readout remains.
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
- Legacy virtual-device remapping uses the Linux InputPlumber adapter. A running
  service alone is insufficient: it must manage the selected physical device.
- **Enable Linux controller routing** explicitly confirms a system-wide change
  to the running service. It can affect other applications and Steam Input.
  Close games first. This action does not install packages or drivers. Use
  **Use native controller routing** to disable that service-wide routing.
- Per-launch profiles, target devices, and player order use the existing
  capture/restore session. They do not overwrite emulator configuration files.
- Standalone emulator and Windows/macOS configuration adapters are not implemented.
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

`nix develop -c env QT_QPA_PLATFORM=offscreen target/release/lunchbox --controller-calibration-ui-probe --screenshot-output /tmp/lunchbox-calibration.png`
opens the real calibration UI with the Brawler64 diagram against a connected
controller and captures it without saving any calibration or changing routing.

Regression coverage includes controller-family selection, unplugged preferences,
explicit game overrides, per-device N64/Sega profiles, C-button YAML generation,
desktop grid navigation, buttons/checkboxes, combo-box selection, popup back
handling, and inactive-window suppression. Physical-device end-to-end testing is
separate from those deterministic tests.
