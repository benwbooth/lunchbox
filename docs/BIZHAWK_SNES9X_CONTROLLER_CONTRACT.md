# Native BizHawk Snes9x adapter boundary

Step 394 adds a captured-definition comparator, using the pinned
Snes9xControllers.cs compacted joypad list and appended Reset/Power controls.
It expects all twelve buttons for each connected logical player, exact SNES
system and an explicitly supplied GameInfo.Hash, with no axes or extra/duplicate
buttons. An owned capture wrapper performs this comparison after child cleanup
and checks cancellation. This is not loaded-core provenance or a launch gate;
ROM identity derivation, runtime integration and physical behavior remain
unverified. No tests or captures ran.

Source-grounded implementation checkpoint, not runtime-verified coverage.
Pinned BizHawk revision: `8c6b8958bbbe623eaaa36bc82af858b812893628`.
Local source: `/tmp/lunchbox-core-contracts.AmSobs/bizhawk`.

The existing libretro Snes9x profile cannot configure native BizHawk. The current
Nymashock adapter selects PSX and writes a PSX deck. A separate Snes9x adapter now
selects SNES and writes its own deck; it does not rename the Nymashock output.

## Source-confirmed input contract

- `src/BizHawk.Emulation.Cores/Consoles/Nintendo/SNES9X/Snes9xControllers.cs`
  constructs the `SNES Controller` deck from left-port devices followed by
  right-port devices. A joypad contributes one controller; a multitap contributes
  four. Both multitaps therefore produce eight logical players.
- The joypad exposes twelve controls: B, Y, Select, Start, Up, Down, Left, Right,
  A, X, L and R. Internal fragment names have a leading `0`.
- `src/BizHawk.Emulation.Common/ControllerDefinitionMerger.cs` allocates those
  fragments into one-based `P{number} {control}` names. Physical right-port player
  numbering depends on how many controllers precede it on the left port.
- Reset and Power are deck-level controls, not per-player face buttons. Their
  policy must be explicit; do not consume gameplay buttons automatically.
- `Snes9x.cs` defines sync settings `LeftPort` and `RightPort` using distinct
  enums in `LibSnes9x.cs`. Only the right-port switch constructs mouse, Super Scope
  and Justifier devices. These need separate input contracts, not joypad bindings.

## Implementation status

Steps 311–320 implement joypad encoding, physical/logical translation, explicit
port topology, launch-owned configuration, normalized transports, settings/editor
selection, source/destination preview and saved-input completeness checks. These
are source changes only: compilation, UI behavior and runtime acceptance have not
been checked. Mouse, Super Scope and Justifier contracts remain unimplemented.

The requirements below remain the acceptance checklist, not a claim of verified
completion:

1. Model the native core choice separately from libretro identities and preserve
   Nymashock behavior when adding a native Snes9x selection.
2. Encode the exact Snes9x sync-settings type, selected core preference and SNES
   deck while preserving unrelated configuration. Trace serialization before
   choosing numeric versus named enum representations.
3. Compose calibrated physical/logical SDL inputs into the twelve native button
   names using the existing source selection and translation machinery.
4. Resolve logical player numbering from explicit left/right topology; do not
   assume physical port two is always player two.
5. Connect saved settings, editor, launch ownership and preview to that contract.
6. Keep missing native translations and peripheral modes explicit. Builds, tests
   and runtime verification remain deferred by user instruction.

This checkpoint changes neither the canonical core denominator nor coverage.
