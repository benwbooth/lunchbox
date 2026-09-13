# Odyssey Now: HAL controller boundary

The source is pinned to
`Vibrant-Media-Lab/OdysseyNowHAL@7eb0c1da4f46728dd22eab189840f8f51f03118b`.
The repository is a Unity 2018.3.14f1 project and its README advertises
Windows and macOS builds through the [official itch page](https://pathealy.itch.io/odyssey-now-hal).

`OdysseyNow/Assets/Scripts/CardDirection/LocalInputManager.cs` stores the
selected schemes and AI levels in Unity `PlayerPrefs` keys `P1Input`, `P2Input`,
`ai1`, and `ai2`.  `Actors/PlayerTargetController.cs` reads those keys, uses
InControl `ActiveDevices[(player - 1) % 2]` for the Traditional scheme, and
uses the serialized keyboard `KeyCode` fields for keyboard control.
`HardwareInterface/ConsoleMirror.cs` connects the OriginalConsole scheme to
Ardity's serial controller and sends/receives JSON over the selected serial
port.  `ProjectSettings/InputManager.asset` contains Unity joystick indices and
axes, not a stable physical identity.

These are source-backed runtime paths, but they are not a portable persistent
controller profile that Lunchbox can safely author: PlayerPrefs storage is
Unity/platform-specific, InControl device ordering is runtime state, and the
Arduino serial port is selected by the Unity/Ardity runtime.  The adapter
therefore fails closed.  No Unity build launch, hardware serial session, or
effective gameplay input has been verified here; save-state, BIOS, and key
contracts are likewise not claimed.
