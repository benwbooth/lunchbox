# Emulator feature matrix

`emulator_details/emulator-feature-matrix.csv` is the executable checklist for
controller configuration, firmware/keys, saves, and save states across native
Linux, Linux Flatpak, macOS, and Windows.

It contains one row for every host/runtime pair in the canonical database:

- 249 standalone emulator definitions x 4 hosts = 996 rows.
- 94 canonical non-BizHawk-only RetroArch core names x 4 hosts = 376 rows.
- 1 source-captured runtime absent from the canonical database (`simple64`) x 4
  hosts = 4 explicit `record_only` rows.
- Total: 1,376 rows.

The source set now contains 250 standalone platform records: every one of the
249 catalog identities plus the separately researched `simple64` record. Their
1,000 record/host cells are all explicitly dispositioned: 419 fully captured,
157 partially captured, 223 without a verified package, 122 unsupported, and 79
unresolved. There are no `missing_record` rows.

The 94 canonical RetroArch cores also have one structured record apiece. Every
record is linked to at least one exact controller profile; together they cover
399 firmware-file dispositions, save behavior, state serialization, and all
four frontend hosts. Core host availability is intentionally conservative:
eight official macOS arm64 buildbot artifacts were downloaded and executed on
an Apple Silicon host, two exact Flatpak-updater Linux cores were executed, seven
official Windows x86_64 buildbot cores were executed on hosted Windows Server
2025 VMs, and one Windows SteemSSE artifact is source-verified as available. Mesen-S is
explicitly unavailable on macOS and Windows because the current official
artifact paths returned HTTP 404. The other 356 core/host cells remain `unverified`
rather than inheriting availability from the RetroArch frontend.

Because the shared RetroArch frontend record supplies the platform paths for
all 94 core rows, the generated matrix contains 795 captured rows, 157 partial
rows, and 424 explicit host-gap rows.

Generate or refresh it with:

```console
cargo run -p lunchbox-db -- feature-matrix \
  --database build/lunchbox.db \
  --records emulator_details/records \
  --retroarch-cores emulator_details/retroarch-cores \
  --controller-catalog crates/lunchbox-app/data/controllers/catalog.json \
  --firmware-rules sources/firmware-rules.json \
  --test-results emulator_details/runtime-test-results.json \
  --output emulator_details/emulator-feature-matrix.csv
```

The generator preserves the five manually maintained feature-result columns and
the notes column from an
existing matrix, keyed by `runtime_kind`, `runtime_id`, and `host_os`:
`controller_test_status`, `firmware_test_status`, `save_test_status`, and
`state_test_status`, plus `test_notes`. Older matrices without the state column
load it as `not_tested`. Regeneration refreshes the research-derived fields without
discarding test results.

Reviewed results in `emulator_details/runtime-test-results.json` override the
same keyed cells from a prior CSV. The generator rejects duplicate results,
unknown statuses, unknown runtime/host keys, missing evidence notes, and schema
versions it does not understand. This ledger is the durable source for runtime
evidence; preserved CSV-only values remain supported for in-progress manual
testing.

Controller configuration is assembled from both `config` and `input` record
entries. `input` means controller-binding syntax; `keys` is reserved for actual
cryptographic firmware keys. Standalone firmware filename, hash,
dynamic-identity, and "no canonical checksum" dispositions remain prose, which
is why `firmware_checksum_status=captured_in_prose_unstructured` is not a
validation result. RetroArch core firmware is machine-structured in separate
records, but source metadata still is not runtime acceptance evidence.

`record_status` uses these source-capture values:

- `captured`: every required dimension has a resolved source disposition for
  this host.
- `partial`: a host record exists, but at least one dimension is explicitly
  `unresolved`.
- `unsupported`: the upstream project explicitly does not support the host.
- `no_verified_package`: a native runtime may exist, but no trustworthy package
  capture exists for that host.
- `unresolved`: primary evidence was insufficient to establish a host runtime.
- `missing_record`: generator safeguard for catalog drift; none are present in
  the current matrix.

Each path also has a status. Omitted status means `captured` for compatibility;
explicit values are `captured`, `not_supported`, `not_required`, and
`unresolved`. The app resolver only turns `captured` entries into local paths.

The controller, firmware, save, and state status columns describe research
capture only. The per-core columns are structured source metadata, not runtime
results. Neither form implies that a feature works, a firmware asset is
accepted, or that the new provider-neutral save-sync engine has been exercised
with this exact runtime, host, and real cloud account. Only live evidence may
promote a feature test status from `not_tested`. Saves and save states have
separate result columns so a state round-trip cannot be misreported as a
persistent-save pass.

## Status vocabulary

Use these values in each feature test-status column:

- `not_tested`: no live verification has been performed on that host.
- `pass`: the exact captured behavior was reproduced on that host.
- `fail`: the runtime was exercised and contradicted the captured behavior.
- `blocked`: testing could not reach the feature; explain the blocker in
  `test_notes`.
- `not_applicable`: the runtime or feature is genuinely unavailable on that
  host, with the reason and evidence in `test_notes`.

Do not use `pass` for a successful install, config-file existence, process
startup, or a source-code inspection alone.

## Acceptance evidence

For controller configuration, record the runtime version/hash, controller
identity/backend, effective config path, exact serialized binding, and observed
in-game response for every mapped control and player in scope.

For firmware/keys, record the runtime version, effective search directory,
exact file names, sizes and canonical hashes when available, import/install
method, and a boot or runtime diagnostic proving the asset was accepted. Never
copy proprietary firmware into the repository.

For saves, create a deterministic in-game save, identify the exact written files
or whole image, record names and hashes, relaunch and load it, then exercise
export/restore without losing a newer version. Whole images require an atomic
snapshot/copy while the emulator is stopped.

For states, create a deterministic state, identify its exact filename and hash,
mutate emulated state, load the saved state, and verify the earlier state was
restored. Exercise export/restore separately before promoting synchronization.

The matrix is evidence tracking, not an assertion that every runtime is
installable on every host. Unsupported cells remain explicit rows and become
`not_applicable` only after host-specific evidence establishes that fact.
