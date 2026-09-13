# Retro Virtual Machine controller boundary

Source inspection is pinned to the public Retro Virtual Machine 1.1.x source
`jcgamestoy/retrovm1@e2b3cbeda8f96d92947a1a1d004e2548da95db33`. The current
2.1.x desktop releases are distributed as binaries; the official site says
the documentation section is still being developed.

The v1 source stores HID gamepad mappings in
`~/Library/Application Support/Retro Virtual Machine/gamepads.json`, keyed by
`<vendor>-<product>` and containing base64-encoded seven-byte mapping data.
That is an Apple HID-specific implementation detail from an older release,
not a verified v2 cross-platform contract. Lunchbox therefore refuses to
synthesize a controller file.

The public v1 source does establish `.rvm` document bundles and gzip/JSON
machine snapshots, but those are full document state rather than a portable
host-controller profile. Current v2 persistence and effective runtime input
remain unverified.
