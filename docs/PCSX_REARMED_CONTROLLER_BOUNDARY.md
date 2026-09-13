# PCSX-ReARMed controller boundary

Pinned upstream: `notaz/pcsx_rearmed@d7d741db1d974cf8dd05a23149a1f12758fc1894`.

The upstream README identifies a Linux CI build, BIOS HLE, and libretro support (lines 1-22). The standalone frontend's `frontend/plugin_lib.c` consumes input state (lines 704-726), but the pinned source does not establish a stable native controller-profile file or ownership of save-state/memory-card paths for the catalog entry. The adapter refuses to write RetroArch/frontend-owned settings under the PCSX-ReARMed core name. Runtime and package behavior remain unverified.
