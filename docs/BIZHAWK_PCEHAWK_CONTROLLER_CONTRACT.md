# Native PCEHawk controller contract

Step 396 adds exact captured-definition validation and an owned-capture wrapper.
Pinned PceControllerDeck.cs builds each connected controller with its physical
port number and concatenates those definitions; no compacting, Reset or Power
controls are added. Expected button names reuse the adapter's eight-button list.
The caller supplies expected runtime system and GameInfo.Hash independently;
PCEngine.cs assigns PCECD in its single-disc loading path. Wrong system/hash,
axes, duplicates and missing/extra buttons fail. This does not establish core
provenance or integrate a launch gate, and no tests or captures ran.

Pinned BizHawk source: `8c6b8958bbbe623eaaa36bc82af858b812893628`, inspected under
`/tmp/lunchbox-core-contracts.AmSobs/bizhawk`.

- `Consoles/PC Engine/PceControllerDeck.cs` has only Unplugged (0) and GamePad (1).
  StandardController exposes Up/Down/Left/Right, Select, Run, B2 and B1. Its names
  use fixed P1–P5 prefixes; disconnecting an earlier port does not renumber others.
- The aggregate deck is `PC Engine Controller`; no Reset/Power control is added
  by that deck. Do not invent console-button bindings or a six-button device.
- `PCEngine.ISettable.cs` stores Port1 through Port5 in PCESyncSettings.
- `PCEngine.cs` identifies core `PCEHawk`, type
  `BizHawk.Emulation.Cores.PCEngine.PCEngine`, with PCE/PCECD/SGX/SGXCD constructors.

Step 348 adds fixed-port native configuration encoding. Step 351 adds calibrated
translation using the shared raw/logical SDL contract: catalog A/I becomes B1,
B/II becomes B2, Start becomes Run, and directions/Select retain their names.
Snapshot-based configuration preparation requires every connected port exactly
once and rejects repeated SDL devices. Argument preparation uses the shared
transactional private-configuration owner. Step 352 connects PCEHawk to shared
session dispatch, including pre-probe port validation, runtime artifact and
device-routing guards, normalized-device logical calibration, and configuration
ownership. P1-P5 retain their fixed identities; five is the slot-ID bound, not
the connected-player count. Step 353 adds optional `pcehawk_ports` saved settings,
exclusive digital-core selection, PCE/TurboGrafx/SuperGrafx platform aliases,
the `pcehawk` logical-calibration scope, and saved two-button calibration checks.
Stored configurations now select the shared launch adapter. Step 354 adds the
editor core choice, fixed-port toggles, topology round-trip and complete-port
recording gate. Source/destination preview selects pce-2 and the pcehawk logical
calibration scope. None of these paths have been built or runtime verified. Native
six-button pads need a separate core contract such as TurboNyma; the existing
libretro six-button profile is not proof of PCEHawk capability. No tests or runtime
verification have been performed.
