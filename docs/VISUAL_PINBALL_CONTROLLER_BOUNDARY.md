# Visual Pinball controller boundary

Visual Pinball is recorded from the pinned upstream source `vpinball/vpinball@93d93d9653d7fdc295967cb5f30fff33b734e1fa`. The standalone runtime has native Windows, Linux, and macOS builds. `FileLocator.cpp` establishes the per-user `VPinballX/10.8/VPinballX.ini` location (Windows AppData, macOS Application Support, or Linux `~/.local/share`), while `InputManager.cpp` and `InputAction.cpp` establish device/action mapping syntax and device sections.

The input grammar is not enough to make a safe generic writer: device IDs are runtime SDL identities, mappings can combine buttons/axes with OR/AND and thresholds, and table actions are table-specific. Per-table `<table>.ini` files and `VPReg.stg` are persistence artifacts, not a portable host-controller profile. No BIOS or cryptographic key file is required by this contract. VP table saves and NVRAM are table/ROM-owned (`pinmame/nvram/<rom>.nv`); no deterministic filesystem savestate API was established.

Lunchbox therefore refuses to synthesize or overwrite VP input files. The refusal is implemented in `crates/lunchbox-app/src/controller_visual_pinball_standalone.rs`. Linux, Windows, and macOS paths are captured in `emulator_details/records/visual-pinball.json`; no verified Linux Flatpak package is recorded.
