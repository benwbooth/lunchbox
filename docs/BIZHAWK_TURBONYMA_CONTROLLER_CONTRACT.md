# Native TurboNyma controller contract

Step 398 adds exact captured-definition validation and an owned-capture wrapper.
Pinned NymaCore.Controller.cs uses fixed port prefixes, adds Power/Reset, and
adds Open Tray, Close Tray and the Disk Index axis when CDs are present.
TurboNyma.SystemId derives its CD suffix from that same state; expected system
and disc mode must agree. Each connected gamepad requires all fourteen native
controls, including both mode selectors. Unexpected/missing/duplicate buttons,
wrong axes and system/hash mismatches fail. The capture records axis names,
not ranges: disc-index bounds, provenance and launch integration remain pending.
No tests or captures ran.

Source inspected at BizHawk `8c6b8958bbbe623eaaa36bc82af858b812893628` under
`/tmp/lunchbox-core-contracts.AmSobs/bizhawk`:

- `waterbox/nyma/mednafen/src/pce/input/gamepad.cpp` declares I–VI, Select,
  Run, four directions, and a two-position Mode switch. Six-button mode is
  intended only for compatible games.
- `waterbox/nyma/NymaCore.cpp` exports each switch position's Name, separately
  from its SettingName. The names here are `2-button` and `6-button`.
- `NymaCore.Controller.cs` exposes switch positions as separate rising-edge
  selectors, `P{port} Mode: Set 2-button` and `P{port} Mode: Set 6-button`.
  A single toggle binding does not implement this interface.
- `NymaCore.Controller.ButtonNameOverrides.cs` preserves I–VI and normalizes
  Select/Run/direction names. Port prefixes retain the native one-based index.
- `Consoles/NEC/PCE/TurboNyma.cs` uses `PC Engine Controller`, loads turbo.wbx,
  and hides ports 2–5 when pce.input.multitap is disabled.

Step 356 adds the fourteen-control `pce-turbonyma` schematic and calibrated
raw/logical SDL translation. This is a distinct native target; it does not
replace the existing libretro pce-6 layout. Step 357 adds configuration encoding:
five zero-based PortDevices entries use gamepad/none, and MednafenValues stores
pce.input.multitap as 1/0. Ports 2–5 require the multitap. All fourteen bindings
are required per connected player; exact duplicate input strings are rejected.
Power/Reset/Open Tray/Close Tray and Disk Index bindings are retained; stale
player analog/feedback/autofire bindings are removed from the owned deck.
Step 358 adds snapshot composition and transactional private-configuration argument
preparation. Topology and duplicate SDL devices are checked before translation;
per-player warnings are retained. Step 359 connects shared digital-session
dispatch and multitap-aware fixed-port layout selection. The NymaCore.cpp override
hook reads Game->DesiredInput; no writer was found in the inspected pinned PCE
source. This does not verify the installed core or runtime content behavior.
Step 360 adds optional saved ports/multitap topology, platform/session selection,
the turbonyma calibration scope and saved fourteen-control completeness checks.
Conflicting PCEHawk/TurboNyma routes for one emulator ID are rejected. Step 361
adds editor/preview selection, fixed-port controls, explicit multitap settings
and recording gates. Preview uses the fourteen-control native layout and
turbonyma SDL scope. No tests, builds or runtime checks.

Step 362 classifies the two native mode selectors as auxiliary emulator actions.
The existing resolver can allocate distinct spare digital controls to them,
preserving its one-to-one constraint. Shortages remain missing assignments;
no single toggle or synthetic second input is introduced. Native translation
also rejects duplicate SDL input strings across the fourteen actions.
