# Emulator adapter roadmap

The authoritative row-by-row status is now
[EMULATOR_PLATFORM_INTEGRATION_CHECKLIST.md](EMULATOR_PLATFORM_INTEGRATION_CHECKLIST.md).
It tracks controller configuration, firmware/keys, and save/save-state facts
separately across Linux, Flatpak, macOS, and Windows.

Current source-only counts are 32/249 partial native controller adapters (12.9%),
94/94 RetroArch core contracts (100%), 250 platform records, and 419/1,000 fully
captured host/emulator cells (41.9%). These are not runtime-verification
percentages; see the checklist for partial captures and explicit host gaps.

Kronos, jgenesis, Gopher64, RMG, and Yaba Sanshiro 2 are no longer data-only
candidates: each has a registered native controller adapter. simple64 also has
a source-backed adapter, but remains outside the 249-candidate denominator
until the canonical catalog gains its currently record-only runtime. The
checklist is the source of truth for the remaining captured and uncaptured work.

FS-UAE remains deferred: its live mapping format is in a bundled Python layer
that was not available in the inspected repository, so a source-pinned contract
still requires a runtime oracle or the missing source.
