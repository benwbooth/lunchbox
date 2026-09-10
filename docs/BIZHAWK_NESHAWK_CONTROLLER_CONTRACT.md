# Native NesHawk controller contract

Step 395 adds exact captured-definition comparison for the configured mixed
digital deck, using player_port/buttons identities and pinned NESControllers.cs
merging. NES.Core.cs ResetControllerDefinition appends Power/Reset, FDS Eject
and zero-based FDS Insert side controls, and VS coin/service controls. Callers
supply those content expectations independently; the bounded capture contract
accepts 1–256 FDS sides. Extra axes (including SubNesHawk Reset Cycle), duplicate,
missing and unexpected buttons fail, as do wrong NES system/content hashes.
An owned-capture wrapper compares after child cleanup and checks cancellation.
This does not configure FDS/VS actions, prove core/content provenance, or integrate
a launch gate. No tests or captures ran.

Step 329 adds the `nes-power-pad` catalog schematic. `NESControllers.cs` defines
the twelve native PP switches; `vpads_schemata/NesSchema.cs::PowerPad` arranges
1–4, 5–8 and 9–12 in three rows. The schematic preserves these identities and
does not claim native launch support or physical mat-side artwork fidelity.

Step 328 also implements `ControllerSNES`, defined in `NESControllers.cs` with
twelve SNES controls. It contributes one merged logical player per port. Mixed
NES/SNES-adapter decks select each player's layout from its owning port, including
save validation, normalization and preview. This remains untested implementation.

Pinned BizHawk source: `8c6b8958bbbe623eaaa36bc82af858b812893628`, inspected
under `/tmp/lunchbox-core-contracts.AmSobs/bizhawk`.

- `NES/NESControllers.cs`: `NESControlSettings` stores `Famicom`,
  `NesLeftPort`, `NesRightPort`, and `FamicomExpPort`. Device values are class-name
  strings, not numeric enums. Ordinary NES ports support `UnpluggedNES`,
  `ControllerNES`, and `FourScore` (among other peripherals).
- `ControllerNES` has A, B, Select, Start and four directions. `FourScore`
  contributes two such players per port half. `NesDeck` merges left then right
  using `ControllerDefinitionMerger`; logical numbering follows that order.
- `NES.ISettable.cs`: `NESSyncSettings.Controls` contains the control settings.
  The native core type is `BizHawk.Emulation.Cores.Nintendo.NES.NES` and
  `CoreNames.cs` names the core `NesHawk`.
- `NES.Core.cs`: deck-level Power/Reset, FDS disk controls and VS coin/service
  controls are separate from gameplay. Preserve existing bindings for these;
  do not assign them to gameplay buttons automatically.

Step 322 implements a separate encoder for NES-mode joypad port configurations,
including Four Score halves. It preserves unrelated config and clears obsolete
NES peripheral/analog/autofire bindings. Famicom microphone/expansion devices,
Zapper, Power Pad and other peripherals still need their own contracts.

Steps 323–324 add calibrated raw/logical SDL translation, multi-player composition
and owned temporary-config argument preparation. Saved setup selection, live
session routing and visual editor integration remain to be connected.
No build, tests or runtime verification have
been performed. This foundation does not increase catalog coverage.
