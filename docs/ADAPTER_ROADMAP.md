# Emulator adapter roadmap

The authoritative row-by-row status is now
[EMULATOR_PLATFORM_INTEGRATION_CHECKLIST.md](EMULATOR_PLATFORM_INTEGRATION_CHECKLIST.md).
It tracks controller configuration, firmware/keys, and save/save-state facts
separately across Linux, Flatpak, macOS, and Windows.

Current source-only counts are 28/249 partial native controller adapters (11.2%),
94/94 RetroArch core contracts (100%), 250 platform records, and 432/1,000 fully
captured host/emulator cells (43.2%). These are not runtime-verification
percentages; see the checklist for partial captures and explicit host gaps.

Kronos, jgenesis, and Yaba Sanshiro 2 are no longer data-only candidates: each
has a registered native controller adapter. simple64 and RMG do have platform
records now, but still lack native controller adapters. The checklist is the
source of truth for the remaining captured and uncaptured work.

FS-UAE remains deferred: its live mapping format is in a bundled Python layer
that was not available in the inspected repository, so a source-pinned contract
still requires a runtime oracle or the missing source.
