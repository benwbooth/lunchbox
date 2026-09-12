# Save and save-state synchronization

Lunchbox uses Apache OpenDAL 0.59.1 as its cloud-storage abstraction. The
compiled providers are Google Drive, Dropbox, and OneDrive; the in-memory
backend is used as a deterministic storage oracle in tests. Lunchbox owns the
sync policy because a generic newest-file or two-way-copy policy is unsafe for
emulator data.

## Safety model

- Each sync scope is one exact emulator and runtime platform: `linux`,
  `linux-flatpak`, `macos`, or `windows`. Native and Flatpak installations are
  never merged implicitly.
- Only captured, machine-resolvable `saves` and `states` roots enter a sync.
  Unsupported, unresolved, documentation-only, bare-relative, and user-chosen-only
  roots are unavailable; Lunchbox never guesses an emulator-specific base
  directory. A declared fixed root that has not been created yet is an
  available empty route, allowing a fresh installation to receive remote saves;
  every existing path component is checked for symbolic links before local
  writes.
- Local inventories reject traversal, non-portable names, symbolic links,
  special files, unstable files, more than 4,096 files, more than 16 GiB per
  file, and more than 64 GiB per scope.
- File identity is SHA-256 plus size. Modification timestamps are recorded for
  the conflict UI but never decide which copy wins.
- Three-way comparison uses an immutable common-ancestor manifest. A local-only
  change uploads, a remote-only change downloads, and matching hashes converge.
  If both sides changed, including edit-versus-delete, the plan stops until the
  user chooses **Local** or **Remote** for every conflicting artifact.
- Whole-image save models such as Xemu HDD images and Saturn backup RAM remain
  atomic artifacts. Lunchbox does not attempt a byte-level merge.

## Remote layout

For a scope `<emulator>/<runtime>`, OpenDAL stores:

```text
saves/v1/<emulator>/<runtime>/
  blobs/<sha256>
  manifests/<manifest-sha256>.json
  devices/<device-id>.json
```

Blobs and manifests are immutable and verified by readback. Manifests are
canonical JSON objects whose ID is the SHA-256 of their body. Each installation
writes only its own device-head pointer. This avoids relying on a global
conditional write: OneDrive exposes conditional writes, but the selected
Google Drive and Dropbox backends do not. Diverged device heads are merged from
their nearest common ancestor and preserve both parent IDs.

OpenDAL operations have per-attempt timeouts and retry temporary failures with
jitter. The timeout layer is inside the retry layer, as required by OpenDAL's
stateful operation semantics.

## Verification status

The provider-neutral engine and in-memory OpenDAL transport are unit-tested for
first sync, one-sided edits and deletes, edit/edit and edit/delete conflicts,
mandatory exact conflict choices, unavailable roots, timestamp-independent
content identity, canonical/tamper-evident manifests, portable-path and symlink
rejection, immutable object verification, streamed multi-megabyte artifacts,
provider builder construction, device-head round trips, and common-ancestor
discovery. The application coordinator re-scans before applying a plan, stages
and verifies downloads, retains replaced files with a recovery journal, and
only publishes a new manifest and device head after local application succeeds.

The QML launch path now performs automatic synchronization before launch and
after the observed emulator session exits. Its blocking conflict dialog shows
both timestamps and requires an explicit **Local** or **Remote** choice for
every artifact. Device heads that share a manifest, are already incorporated,
or are ancestors of another remote head collapse to the unmerged history
frontier. When several divergent tips remain, a separate prompt makes the
remote history explicit; histories are merged one at a time without using
timestamps as a winner. Provider profiles and OAuth tokens are stored in the
operating system credential store; the settings panel verifies authenticated
write/read/delete I/O before accepting a connection. Lunchbox consumes tokens
issued for a registered OAuth application; it does not yet run the provider's
browser authorization flow itself.

This is not a claim of live provider interoperability. The three provider
builders compile and initialize without network I/O, but provider-account tests
require real user-authorized credentials and still need to be run and recorded
on Linux, Linux Flatpak, Windows, and macOS.
