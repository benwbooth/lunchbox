# VirtualC64 standalone-native controller contract

VirtualC64 has a source-backed macOS controller writer. The contract is pinned
to `dirkwhoffmann/virtualc64` commit
`daab78ff6059df44d3bf33d944b3fb2014c91338`; it is no longer an inferred
NSUserDefaults refusal.

The inspected source exposes two cooperating persistence surfaces:

- `GUI/Defaults.swift` resolves the application-support directory, loads and
  saves `VirtualC64/virtualc64.ini`, and registers
  `Peripherals.ControlPort1` / `Peripherals.ControlPort2`.
- `VCCore/Infrastructure/Defaults.cpp` proves the INI grammar: section headers
  become dotted prefixes and keys are emitted as `key=value`.
- `GUI/Input/DeviceDatabase.swift` reads and writes `Devices.Schemes` as
  `Data` containing `JSONEncoder` output for `[GUID:String]`. Each string is an
  SDL-gamecontrollerdb descriptor. The source recognizes axes `a0` through
  `a5`, buttons `b0` through `b16`, axis reversal, and the logical directions
  and fire inputs used by the writer.
- `GUI/Input/HIDExtensions.swift` defines the `Codable` key as
  `struct GUID { var guid: String }`. Foundation therefore represents the
  non-string-keyed dictionary as an alternating JSON array of
  `{"guid":"<32 hex>"}` objects and descriptor strings.
- `GUI/Input/GamePadManager.swift` reserves slots 0 through 2 for the mouse and
  two keyboard keysets, then assigns attached HID devices to the first free
  slots 3 through 6.

`controller_virtualc64_native.rs` now generates that exact encoded device
database, the hexadecimal `Data` argument used by macOS `defaults`, and a
preserving `virtualc64.ini` patch for the two control-port keys. It validates
GUIDs, SDL axis/button bounds, duplicate devices, and duplicate port
assignments. The baseline INI remains authoritative for ROM paths, mounted
media, drives, captures, audio/video, server settings, workspaces, saves and
states.

This is a writer-only contract until macOS runtime integration is exercised.
The remaining boundary is specific: physical slot numbers depend on the live
IOHID enumeration order. A launch adapter must stop VirtualC64, write and read
back `Devices.Schemes` in bundle domain `de.dirkwhoffmann.VC64`, probe the same
devices immediately before launch, confirm their slots 3 through 6, patch a
copied `virtualc64.ini`, and verify the application actually consumed both
artifacts. Linux, Flatpak and Windows remain separate unsupported runtime
paths; they do not invalidate the macOS writer.
