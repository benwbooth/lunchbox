# VirtualBuddy controller and persistence boundary

VirtualBuddy is pinned to `insidegui/VirtualBuddy@683739a7e75921f83c492d15ece1ab89aa66e4cf`.
The upstream README requires an Apple Silicon Mac running macOS 13 or later
and describes a macOS 12+ virtualizer. `VBSettings.swift` places the default
library at `~/Library/Application Support/VirtualBuddy`, while
`VBVirtualMachine.swift` defines `.bundle/.vbdata/Config.plist` and the other
VM metadata files. Saved states are packages below `_SavedState` and are
paired with both the host and the VM.

The source defines VM keyboard and pointing-device hardware, but no portable
physical-gamepad profile grammar. A VM keyboard choice is not an emulator
controller mapping, and a saved-state package is not a portable game slot.
Lunchbox therefore fails closed and does not write VirtualBuddy settings.
Runtime launch and effective guest input remain unverified.
