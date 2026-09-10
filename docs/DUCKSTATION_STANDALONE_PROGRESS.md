# DuckStation standalone controller mapping

Source implementation checkpoint, 2026-09-08. Testing remains deferred.

`controller_duckstation.rs` now composes the existing visual controller planner,
saved native calibration, target-SDL translator and session-owned INI writer.
It handles DigitalController and AnalogController gameplay bindings, rejects
missing required controls and duplicate resolved inputs, and requires each stick
axis to use opposite measured halves of the same physical axis. Settings slots
and SDL player identities are separate explicit inputs. Multitap slot eligibility
and effective game/profile input layers remain owned by the native config writer.
All requested players are translated before any private controller is patched.
The user's original files, controller type, analog policy and motor settings are
not rewritten by this composition.

Native Linux launch dispatch is now connected. Saved setups round-trip through application
settings and an advanced JSON editor with physical-to-native mapping review.
Staging validates calibrated capabilities and setup identities; it performs no
device operations and does not mark the setup launch-ready. Empty lists remove
only these staged setup records after the normal main-page save.

`prepare_resolved` now consumes the target SDL snapshot, resolves saved physical
IDs through exact inventory paths, and composes all players with the observed
DuckStation player projection. It checks snapshot/schema/revision, dependency
hashes, native controller type and private mapping-database consistency.
`PreparedSession` owns the private configuration and provides startup-log checks
for both native folder routing and actual player assignment. Preparation alone
does not mark startup confirmed.

`native_command.rs` obtains the snapshot using the configured trusted helper,
SDL and dependency paths; checks the exact executable SHA-256 and pinned version;
and stages the matching SDL hints with a private XDG configuration root. It admits
only the exact saved content argument and ordinary display/batch flags. Portable
configurations and custom environments are rejected. The emulator's config and
SDL player logs must confirm actual startup routing, and its loaded process maps
must include the inspected SDL/dependency paths. `CalibratedLaunch` retains the
session until emulator exit. Helpers have cancellation, output and time bounds;
startup failure kills and reaps the child. Stdout/stderr are drained separately
so pipe buffers cannot stall the emulator or merge partial lines.

The serial and optional disc-group serial are explicit user setup inputs; they
are not derived from a title. Runtime acquisition and startup only occur during
an actual user launch, not staging or review. No such launch has been run here.
Remaining: Flatpak/Wine namespaces, additional runtime revisions, hotplug/runtime
verification, easier structured setup UI, and the deferred build/test gates.

Coverage stays at 93/94 RetroArch core entries (98.9%). BizHawk and DuckStation now
have partial named standalone calibrated launch dispatch: 2/249 candidate entries
(0.8%), not complete coverage of either emulator's platforms, modes or runtimes.
The database has 249 native candidates, not 249 verified usable runtimes. There
is no established denominator for all controller modes, so overall completion
percentage remains unknown. No tests, build, startup oracle or device probe ran.
