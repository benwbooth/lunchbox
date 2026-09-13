# DEmul native controller contract

The captured platform record is `emulator_details/records/demul.json`.
The official upstream identity/distribution site is
`http://demul.emulation64.com/`. A byte-pinned community archive of DEmul
0.7a build 280418 is recorded at `https://archive.org/details/demul07_2017`
(SHA-256 `ae3f11ed5d36c4f327b3428b8947181284a7f9ae302d811852d4d7a4e9af9148`).
Static inspection finds a plugin-specific numeric `padDemul.ini`, but no
authoritative parser/source, complete input semantics, or stable
physical-device identity.

No Rust adapter is added. Linux and Flatpak must remain explicitly unavailable;
the official site provides no basis for a native controller API or launch
overlay. Windows remains unresolved as well: a Windows binary's undocumented
INI or registry behavior cannot be reconstructed safely from screenshots,
community mappings, or another arcade emulator. Parent integration must retain
the gap status and must not infer Dreamcast/Naomi/Atomiswave controller,
firmware, save, or state paths.
