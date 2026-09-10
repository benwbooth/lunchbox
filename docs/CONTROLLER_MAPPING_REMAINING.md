# Controller mapping remaining work

Source-only backlog refreshed at Step 593. No builds, tests, device capture or runtime
verification were performed. This is a backlog, not an acceptance report.

## Counting boundaries

The progress ledger reports 91/95 catalog cores, 289 profiles and 157 layouts.
The core percentage is not a peripheral, game, topology or native-runtime
completion percentage. Dynamic adapters must not become fake static profiles
merely to raise that number. Full coverage still has no established denominator.

## Implementation gaps confirmed in source

| Area | Existing boundary | Remaining work |
| --- | --- | --- |
| MAME ordinary buttons | Shared presets and per-player overrides; sparse native actions; explicit assignments; profile generation and partial review | Inventory exact unresolved game/mode contracts; resolve independent-channel limits without silently sharing or disabling actions |
| MAME relative motion | Explicit relative-axis/button planning, saved exact sources, logical source/destination visuals, owned transport/frontend routing, private UI-binding guards and launch dispatch are implemented in source | Unbuilt/unverified patched-runtime integration and UI isolation; model-specific device geometry where supported; do not describe stick velocity as physical capture |
| MAME absolute aim | Controller-driven aim plus a separate hardware-independent rectangular absolute calibration transform | Physical gun capture, calibration acquisition, perspective/display transforms and runtime routing; preserve offscreen/reload semantics per game |
| MAME keyboard/unknown inputs | `controller_mame.rs::unresolved_field_guidance` retains explicit key evidence requirements and unknown contracts | Exact native contract work for unresolved fields; no generic button fallback for unknown inputs |
| FBNeo | Per-game adapter includes source-level mixed/relative-only mouse selection, preparation, owned routing and visual review | Unverified relative runtime fidelity; physical gun/keyboard adapters and unsupported game/device binding parts |
| Nymashock | Coverage registry records native modes and launch integration as in progress | Physical pointer capture/calibration, ambiguous HID/SDL routes, force feedback, custom/Nix runtime contracts and runtime fidelity |
| Steem SSE | Coverage registry distinguishes the embedded-ROM guard from an input adapter | Supported Linux core port or Windows-runtime adapter before claiming controller integration |
| Standalone catalog entries | Coverage registry explicitly reports missing automatic standalone adapters | Inventory each actual runtime contract; catalog presence is not host compatibility |

## Work order and scope

### Current relative-input boundary (Step 572)

Steps 551–572 add button composition/editing, the opt-in core isolation patch,
mode declarations, private configuration validation and button-aware launch
preparation. The blanket button launch restriction is replaced with platform and
core-mode requirements. Publication still waits for confirmed owned routing and
focus; source errors and teardown release outputs. No packages have been built
or applied and no runtime behavior has been verified. The Step 550 source findings
below explain why the isolation requirements exist, not missing wiring today.

Steps 530–549 implemented the owned command channel, bootstrap routing, launch
dispatch, relative-only ports and axis mapping/tuning editor. The historical
notes below describe earlier gaps; those are no longer all missing in source.
No build, patch application, UI or runtime evidence establishes acceptance.

At Step 550, physical mouse buttons remained an implementation gap. The pinned MAME source
[`input_retro.cpp`](https://github.com/libretro/mame/blob/4fc9a9312baaf34963847f884961ad9793fbbc1d/src/osd/modules/input/input_retro.cpp)
registers mouse buttons as consecutive native items at lines 1201–1210. At
1213–1221, the first mouse also supplies native UI select/back/clear and page
actions. Left-button polling at 871–928 independently emits native UI pointer
events. Removing default UI sequences alone is therefore not proof of isolation.

Before enabling physical clicks, implement explicit exact-field button ownership,
resolve native UI/pointer-event isolation without disabling intended game input,
and compose button routes with axis, digital and analog assignments. Preserve
source identity and restrict forwarding to explicitly assigned buttons. Then wire
saved settings, review, staging and launch consistently. The current
`prepare_relative_sources` deliberately clears every mouse button and retains
only assigned physical axes; neither saved device buttons nor axis diagrams imply
click or scroll support. Runtime validation remains deferred by user instruction.

Continue substantive MAME contract work before additional cosmetic refinements.
Source follow-up at Step 503 found an existing transport in
`controller_axis/relative.rs`: identified event reader, complete relative packets,
virtual mouse, session/group/bridge lifetime and failure shutdown. Reuse it; do
not create a second capture foundation. Shared saved-output capability checks now
live in `relative_settings.rs` and are consumed by FBNeo. The next MAME boundary
is native relative-field routing and exact frontend mouse-index ownership, not
capture existence. Inspect those contracts before exposing a new supported mode.
Step 504 located the existing pinned udev startup-table parser and added batch
player resolution in `relative_frontend.rs`. It rejects absent/absolute endpoints
and implicit sharing. Launch integration must still establish startup provenance,
revalidate owned device identities and monitor topology; parsed indices are not
persistent routes. MAME's native snapshot now optionally records the enabled
mouse class and exact RetroMouse identities. Relative-axis native planning and
source restriction are implemented, but are not a completed physical launch adapter.
Capture must preserve device identity,
relative units, session ownership and cancellation. A gamepad approximation is
not a replacement for this remaining physical-input contract.

### Effective frontend routing gap (Step 527)

Source inspection of RetroArch commit
`69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576`,
[`command.c::command_get_config_param`](https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/command.c#L424),
establishes that GET_CONFIG_PARAM accepts a fixed set of queries, none for the
input driver or player mouse indices. Unsupported queries return `unsupported`.
Do not implement a client assuming arbitrary configuration queries work.

The relative bridge now requires complete effective routing state, including
disabled unused ports. The generated private configuration describes desired
state; it cannot populate this receipt as proof. Before connecting the bridge to
launch, identify a source-supported way to obtain effective state from the same
frontend startup, or explicitly choose a runtime extension and its packaging.
Step 528 adds an opt-in source patch at
`packaging/retroarch-relative-routing.patch` for this exact frontend commit. Its
`GET_CONFIG_PARAM lunchbox_mouse_routes_v1` query reads the active driver and all
eight mouse indices in one command handler. The Rust reply parser rejects stock
`unsupported` responses and incomplete/malformed states. The patch has not been
built, applied to a runtime or activated. Step 529 adds the opt-in Linux flake
package `retroarch-relative-routing`, pinning the exact archive and enabling the
command/udev build features. The default app/runtime is unchanged, and this
package definition has not been evaluated or built. An isolated owned command
channel, fresh-reply acquisition and launch integration remain outstanding.
Startup-log provenance, endpoint revalidation, focus/cancellation ownership and
protection against subsequent route changes also remain launch responsibilities.

Wheels remain paused. This backlog does not authorize running devices, native
inspection, tests or builds while the user's no-testing instruction remains.
Verification and fresh review remain outstanding for all recent changes; do not
commit or publish them before the repository's required checks are permitted.

Primary source entry points: `crates/lunchbox-app/src/controller_coverage.rs`,
`controller_mame.rs`, `controller_mame/analog.rs`, `controller_mame/digital.rs`,
and `controller_steemsse.rs` under the same source directory.
