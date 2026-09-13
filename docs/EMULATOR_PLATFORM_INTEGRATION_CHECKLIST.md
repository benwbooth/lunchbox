# Emulator platform integration checklist

Source-capture and integration review: 2026-09-12. Runtime results are recorded
only in `emulator_details/runtime-test-results.json`; source checks, compilation,
and matrix generation are not runtime evidence.

This is the current checklist for the platform facts needed by controller
setup, firmware management, and save synchronization. Detailed paths, syntax,
names, checksum dispositions, and citations live in
`emulator_details/records/<slug>.json`. The generated all-catalog testing ledger
is documented in [EMULATOR_FEATURE_MATRIX.md](EMULATOR_FEATURE_MATRIX.md).

## Status definitions

- **Capture** means a cited platform record has controller configuration,
  firmware/keys, saves, and state dispositions for that host. A **partial**
  capture has at least one explicitly unresolved dimension. Both are research
  evidence, not runtime results.
- **Gap** means the record explicitly says `unsupported`,
  `no_verified_package`, or `unresolved` and cites why. A gap is dispositioned,
  but it is not a concrete path capture.
- Controller binding syntax uses purpose `input`; purpose `keys` is reserved
  for cryptographic firmware keys. General configuration remains `config`.
- **Native** means a partial standalone controller writer/session/launch route
  is registered by `controller_coverage.rs`. It does not mean all devices,
  modes, hosts, games, or peripherals work.
- **RetroArch** is the shared frontend platform record for 94/94 core names; it
  is not a standalone adapter. Each core has its own controller, firmware,
  save, state, and host-availability record.
- **Rule** means `sources/firmware-rules.json` has at least one runtime rule for
  that emulator. A captured firmware path without a rule is documentation-only.
- **Per-file** and **whole-image** are the save classifications consumed by
  `platform_locations.rs`. Enumeration is not synchronization.
- Standalone firmware names and checksum dispositions remain prose. RetroArch
  core records have structured per-file digest dispositions, but the app does
  not yet enforce those hashes at runtime.

## Current coverage

| Track | Current | Denominator | Percent | Meaning / remaining work |
|---|---:|---:|---:|---|
| Native controller source adapters | 32 | 249 standalone candidates | 12.9% | 217 catalog candidates still lack a registered partial adapter |
| RetroArch core source contracts | 94 | 94 core names | 100.0% | Source contracts are complete; runtime verification remains |
| Combined controller source entries | 126 | 343 runtimes | 36.7% | 32 native adapters plus 94 core contracts; record-only simple64 is tracked separately |
| Standalone platform records | 250 | 250 tracked runtimes | 100.0% | 249 catalog identities plus `simple64` as `record_only` |
| Record/host cells dispositioned | 1,000 | 1,000 | 100.0% | 576 host records plus 424 explicit gaps |
| Fully captured host cells | 419 | 1,000 | 41.9% | Every required dimension has a non-unresolved disposition |
| Partially captured host cells | 157 | 1,000 | 15.7% | At least one feature dimension remains explicitly unresolved |
| Host gaps | 424 | 1,000 | 42.4% | 223 no-package, 122 unsupported, and 79 unresolved |
| Records with host records on all four hosts | 54 | 250 | 21.6% | Includes fully and partially captured cells; other records carry at least one gap |
| Linux host records | 172 | 250 records | 68.8% | Includes full and partial source capture; runtime verification remains |
| Flatpak host records | 57 | 250 records | 22.8% | Flatpak requires package-specific evidence |
| macOS host records | 144 | 250 records | 57.6% | Includes full and partial source capture |
| Windows host records | 203 | 250 records | 81.2% | Includes full and partial source capture |
| Structured core firmware files | 399 | 399 dispositions | 100.0% | 181 have published digests; 218 explicitly have no published digest |
| Structured core save dispositions | 94 | 94 cores | 100.0% | 52 supported, 19 content-dependent, 15 unsupported, 8 unknown |
| Structured core state dispositions | 94 | 94 cores | 100.0% | 74 supported, 9 unsupported, 11 unknown |
| Live end-to-end save synchronization | 0 | 343 catalog runtimes | 0.0% | The provider-neutral engine, transport, versioning, conflict UI, and atomic local restore are implemented; real-provider plus emulator save/load evidence remains per runtime and host |

## Integration findings that remain distinct from completion

1. All 250 records are embedded by `platform_locations.rs`, and the controller
   coverage UI now obtains the record slug list from that loader instead of a
   stale QML literal.
2. The resolver now returns concrete paths only for the current host and only
   resolves Flatpak paths for discovered app roots. Windows `%APPDATA%`,
   `%LOCALAPPDATA%`, and `%USERPROFILE%` tokens resolve against caller-supplied
   bases on Windows; prose remains documentation-only.
3. Source review corrected the BlastEm isolated config root to
   `$HOME/.config/blastem`, Dolphin's new-install Windows firmware root to
   `%APPDATA%/Dolphin Emulator`, and macOS openMSX firmware to
   `~/.openMSX/share/systemroms`. These are source-derived changes and still
   need runtime verification.
4. The generated CSV has one row per 249 catalog standalone candidates, 94
   canonical non-BizHawk-only RetroArch core names, and the source-captured
   `simple64` runtime absent from the database, on all four hosts: 1,376 rows.
   It preserves manual test statuses across regeneration by exact runtime ID
   and host.
5. The save consumer now hashes and versions the exact captured emulator/runtime
   roots, synchronizes them through OpenDAL, performs three-way merge from an
   immutable common ancestor, requires explicit Local/Remote choices for
   conflicts, stages atomic local restoration with recovery copies, and recovers
   interrupted local transactions before allowing another write. Whole-image
   routes fail closed until an exact stopped-runtime snapshot implementation
   exists. RetroArch core routes also fail closed until Lunchbox owns matching
   per-core frontend directory overrides, so unrelated cores cannot share a sync
   namespace accidentally. Deterministic in-memory transport and coordinator
   tests plus a Linux offscreen Qt settings probe are green; no matrix
   `save_test_status` is promoted until a real provider and the exact
   emulator save/load behavior are exercised on that host. Save-state results
   are tracked separately in `state_test_status`; a state-only pass does not
   promote persistent-save synchronization.
6. Standalone firmware identities remain prose unless a runtime rule exists.
   RetroArch core firmware is structured separately, including published
   digests where the pinned core-info source provides them. Neither proves that
   an emulator accepted an asset.
7. Controller coverage counts source routing only. Exact runtime results for the
   32 catalog-native adapters and 94 core contracts are kept separately in the
   runtime-test ledger; an unlisted runtime/host remains unverified.
