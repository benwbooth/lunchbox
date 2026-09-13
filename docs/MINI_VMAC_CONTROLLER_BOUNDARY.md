# Mini vMac controller boundary

The official Mini vMac project at <https://www.gryphel.com/c/minivmac/>
publishes generated Macintosh emulator builds for several host platforms.
Build identity depends on the selected Macintosh model, ROM, and frontend;
the project does not publish a stable host-gamepad profile or persistent
device-selector grammar. The adapter therefore refuses to synthesize one.
Keyboard, ROM, disk, save, and state behavior must be verified per generated
build and are outside this controller boundary.
