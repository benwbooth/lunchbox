# Native SMSHawk digital contract

Step 397 adds exact captured-definition validation and an owned-capture wrapper.
Pinned SMSControllerDeck.cs declares both non-GG controller ports plus Reset and
Pause, independently of selected frontend mappings. GG exposes only P1, its
Start button and Reset. The comparator uses the adapter's native button and
system IDs, an independently supplied expected hash, and rejects axes,
duplicates and missing/extra buttons (including unexpected keyboard controls).
This does not prove loaded-core provenance or integrate a launch gate. No tests
or captures ran.

Steps 334–339 connect calibrated translation, owned sessions, saved settings,
platform selection and preview, and add SG-1000 standard controls. In `SMS.cs`,
SG content sets `IsSG1000` while the controller deck uses the non-GG standard
ports. `GG_in_SMS` changes video behavior but leaves `IsGameGear_C` true, so its
controller deck remains Game Gear. No content-dependent runtime verification has
been performed. Generic SG controller geometry does not claim model-specific art.

Source: BizHawk `8c6b8958bbbe623eaaa36bc82af858b812893628`, locally inspected at
`/tmp/lunchbox-core-contracts.AmSobs/bizhawk`.

- `Consoles/Sega/SMS/SMSControllers.cs`: `SmsController` exposes Up/Down/Left/Right,
  B1/B2, prefixed by physical P1 or P2. `GGController` adds Start.
- `SMSControllerDeck.cs`: standard SMS deck is `SMS Controller`; Reset/Pause are
  console controls. GG uses `GG Controller`, Reset and only P1 controls. Port 2
  exists internally but is not exposed for GG.
- `SMS.ISettable.cs`: sync settings contain Port1/Port2 and UseKeyboard.
  `SMSControllerTypes.Standard` is enum value zero. Other devices require their
  own analog/peripheral contracts.
- `SMS.cs` identifies `BizHawk.Emulation.Cores.Sega.MasterSystem.SMS` and
  `CoreNames.SMSHawk`; NES/SNES encoders cannot be reused by renaming a deck.

Step 333 adds separate SMS/GG standard-controller configuration encoding.
Unassigned SMS standard ports receive no host bindings; they are not modeled as
physically disconnected devices. Existing Reset/Pause bindings are preserved
where the selected deck exposes them. Translation, settings, session selection
and UI remain unfinished. No build, tests or runtime verification were performed.
