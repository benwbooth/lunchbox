# ares controller mapping

The guided controller setup now feeds the native ares launch, not just its
mapping preview. Player order and per-target button overrides come from the
same saved physical calibrations shown in the UI.

## Contract

- Requires ares 148+ and the SDL input driver. Older executables are rejected
  without running them: old ares writes its normal settings even for `--help`.
- Probes the target runtime's SDL3 library in an isolated helper. Linux evdev
  recordings are translated through the current kernel joystick table with
  SDL's classic backend enabled for both the probe and emulator. SDL3 gamepad
  recordings are translated through SDL's resolved raw joystick bindings.
- Matches an exact device path first, then uses ares's GUID/same-GUID-slot
  identifier. Names, model GUIDs alone, and process-local SDL instance IDs do
  not select players. Reconnecting controllers requires fresh launch discovery.
- Writes a private, home-backed `settings.bml` and passes `--settings-file`.
  This also works across the separate Lunchbox/emulator Flatpak `/tmp` mounts.
- Preserves the original file, firmware and save paths, and video/audio options.
  Suspends conflicting per-system input overrides and gamepad hotkeys in the
  private copy; preserves keyboard hotkeys. Clears inactive player bindings.
- Runs the installed ares settings parser against the private file before
  launching; every generated input key and binding must survive that check.

Source update after `d52297d`: guided target choice now persists through launch,
and an eight-button arcade panel is available. The current tree builds and its
focused source tests pass, but the installed-runtime probe below exercises only
the N64 mapping; see [guided integration status](CONTROLLER_GUIDED_INTEGRATION.md).

Default target modes: N64 (four players), NES/SNES/Master System/SG-1000/
Mega Drive-family/PlayStation digital (two), GB/GBC/GBA/Game Gear/NGP/NGPC/PCE
(one), and ordinary six/eight-button arcade panels plus Start/Coin (two).
These are default attached gamepad modes, not a claim of peripheral coverage.
PCE multitaps, Mega Drive six-button attachments, PS analog-mode switching,
and other non-default attachments are not configured by this adapter yet.

## Verification

`controller_ares` tests cover device identities, SDL enum-to-raw translation,
axis inversion, hats, configuration preservation, idempotence, target filtering,
player limits and rejecting pre-148 executables. The opt-in installed-runtime
test reads an explicitly selected real saved controller and roundtrips its N64
mapping through ares while checking that the original configuration is unchanged:

```text
LUNCHBOX_CONTROLLER_PROBE=/absolute/path/lunchbox-controller-probe
LUNCHBOX_TEST_CONTROLLER=<exact saved controller ID>
cargo test -p lunchbox-app --lib controller_ares -- --include-ignored
```

Verified locally with ares Flatpak 148, its SDL 3.2.30, and the saved Brawler64
Linux calibration. Parser acceptance is not an interactive gameplay test.
Windows/macOS gamepad translation is implemented but not runtime-verified;
non-SDL3 calibrations on those OSes are explicitly rejected. An emulator whose
bundled SDL cannot see a device (including newer hardware such as SC2) needs an
updated runtime; Lunchbox does not substitute another controller with its name.

Primary references: [ares v148 input configuration](https://github.com/ares-emulator/ares/blob/v148/desktop-ui/settings/settings.cpp),
[SDL device identity](https://github.com/ares-emulator/ares/blob/v148/ruby/input/joypad/sdl.cpp),
[input evaluation](https://github.com/ares-emulator/ares/blob/v148/desktop-ui/input/input.cpp),
[system gamepad wiring](https://github.com/ares-emulator/ares/tree/v148/desktop-ui/emulator).

## Remaining project-wide work

This change does **not** finish all emulators. At the preceding published commit
`2d68327`, 12 RetroArch core contracts were in the release; the working tree had
93 plus native adapters requiring separate advanced setup. Source/profile
presence is not evidence of working guided launch configuration. Those adapters
still need integration into the player-first flow, bounded publication, and
runtime verification. Keep ordinary arcade work to 6/8-button panels.
