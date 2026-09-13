# Virtual APF controller boundary

Virtual APF is represented by the official Windows archive
`VAPF_20100509.zip` (SHA-256
`a2a4b5a2781e3858251deb0aea6d6d07b48099b61870a405539d8a590bd933ee`). The
binary offers Configure Emulated Keyboard / Joystick dialogs and stores legacy
settings in `default.ini`, but no source-backed portable input grammar is
published. Its INI paths and keyboard values are not a safe serializer
contract.

Lunchbox therefore refuses standalone Virtual APF controller serialization.
This is an explicit refusal, not an inferred mapping or a claim of support on
Linux, Flatpak, or macOS.
