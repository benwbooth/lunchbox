# Model 2 Emulator native controller contract

The captured platform record is `emulator_details/records/model-2-emulator.json`.
The only official upstream source/documentation identified is the
`nebula.emulatronia.com` distribution site. A byte-pinned v1.1a Windows
archive is also captured through the Batocera packaging mirror (archive
SHA-256 `5ffebe72d2885bde3fbfab816947475a9a2ce2795284b9d1e90344209bd4c65f`,
`EMULATOR.EXE` SHA-256
`2abe14866db96f4853b58bfaebc95d803c931a0e5ae33e95758c6b39520bccbc`). Static
inspection finds per-game little-endian `CFG/<game>.input` DWORD arrays and
INI switches, but no authorable text grammar, source-backed input semantics,
stable physical-device identity, firmware contract, save path, or state
format.

No Rust adapter is added. A native Linux controller overlay would require a
compatibility layer or Wine-specific behavior that is outside the captured
runtime contract and cannot be claimed as native support. The parent must keep
Linux, Flatpak, and macOS unavailable, and must not infer controller, firmware,
save, or state behavior from similarly named arcade emulators. Windows support
also remains unresolved until an exact official release/configuration source is
captured.
