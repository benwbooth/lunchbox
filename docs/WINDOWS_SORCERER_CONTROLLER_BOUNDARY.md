# Windows Sorcerer controller boundary

Windows Sorcerer is pinned to the preserved v0.01 artifact
[`Windows-Sorcerer-v001.zip`](https://www.vincenzoscarpa.it/emuwiki/pmwiki/myscripts/downloademulwin.php?fn=729), SHA-256
`b3171b5cf2020884c66cf5f3dee6481d230737492d914cc3797e6733e2b6c693`.
Its bundled readme targets Windows 95/98 and DirectX, says joystick input is
working but not configurable, and says keyboard configuration is not working.
The archive has an INI file and DirectX runtime behavior, but no verified
controller-profile grammar or stable device identity.

Lunchbox therefore fails closed and does not write `WSORCER.ini` or invent a
DirectInput mapping. The record's `.snp`, ROM, cartridge and INI paths are
artifact evidence only; Windows launch and effective gameplay input remain
unverified.
