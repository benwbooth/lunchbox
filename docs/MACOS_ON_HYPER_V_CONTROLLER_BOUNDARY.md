# macOS on Hyper-V controller and persistence boundary

`balopez83/macOS_On_Hyper-V` is an archived Windows Hyper-V/OpenCore support
project, pinned to
`cacf043c6b362c621037d44a85ad15feca9ef7aa`. It is not a conventional game
emulator. The repository's `EFI - Testing/EFI/OC/Config.plist` is a pinned
template (SHA-256
`b12badfafb109c1438a6080913671d9c2f60e018868db24ad1b707b8669e33fd`), while
the installation guide instructs the user to mount a downloaded `UEFI.VHDX`
and edit its `EFI/OC/config.plist`.

The guide documents Hyper-V Manager's Generation 2 VM settings, virtual
processor/memory/disk choices, firmware boot order, and Integration Services.
It identifies keyboard and trackpad/touchscreen/stylus input (the latter is
passed as mouse input) and explicitly says USB and Bluetooth passthrough do
not work. Those settings are VM-manager state, not a portable controller
mapping file, and the project does not define a stable physical gamepad
identity.

The macOS guest filesystem lives in the user-selected virtual hard disk; no
host save-game or emulator-state sidecar is defined. The guide explicitly
disables Hyper-V Checkpoints, so no project-defined state-slot contract
exists. `UEFI.VHDX`/OpenCore and the copied version-specific
`com.apple.recovery.boot` directory are boot/installation assets, not
cryptographic key files.

Accordingly Lunchbox adds no controller writer for this target. The Windows
captures in `emulator_details/records/macos-on-hyper-v.json` are documentation
of the source's VM/EFI boundary, not proof of a runnable macOS guest or
effective controller input. Linux, Flatpak, and macOS host cells are
unsupported because Hyper-V is a Windows host requirement.
