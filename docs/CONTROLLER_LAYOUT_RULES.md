# Shared controller-layout mapping rules

Lunchbox does not store an M-by-N table of controller-model mappings. It stores
one semantic description per layout, one output contract per emulator/core mode,
and one deterministic assignment algorithm. The current catalog contains 152
layouts, defining 23,104 possible ordered source/target combinations. This is a
catalog count, not a completed audit: the current policy has not been tested yet.
Adding a layout within a supported family automatically participates in all pairs.

The composition is:

`calibrated physical inputs -> source layout -> target layout -> core output contract`

RetroPad is an output transport, not a physical Xbox-controller assumption. A
core contract translates target controls to RetroPad; the native launch writer
then translates the saved calibration to the connected driver's numbering. The
same layout assignment is reused by every contract requesting the same target
controls, including standalone contracts. A documented contract does not itself
mean its launch adapter has been implemented or runtime-verified.

## Policy version 6 (implementation phase; verification deferred)

The `arcade-rows` family adds six-button `1 2 3 / 4 5 6` and eight-button
`1 2 3 7 / 4 5 6 8` geometry. The extra right-hand column preserves the ordering
of the first six buttons. These are physical positions, not game-action labels.

For arcade targets, diamond/horizontal-four sources prefer
`Y X R / B A L`, with L3/R3 supplying the extra two digital positions where
available. Sega rows prefer `X Y Z / A B C`, then L/R. The reverse Sega mapping
uses the same rows. A Neo Geo target prefers the arcade lower row, taking its
fourth button from position 8 on an eight-button layout or position 3 on six.
The constrained solver still handles missing calibrated inputs explicitly.

These family preferences do not change a core's output contract. In particular,
MAME's native buttons 7/8 use RetroPad L3/R3, not L2/R2 analog triggers. MAME
profiles require only observed active controls, reject too-small presets and
leave unsupported fields visible. Analog, pressure, directional and repeat
constraints still apply before digital family preferences.

### Policy version 3 foundation

The shared two-button preference now uses the target pair's left/right positions
instead of assuming that every manufacturer prints B on the left and A on the
right. It selects the primary thumb pair of the source family and composes that
with the core's actual bindings. Neo Geo Pocket and NES therefore do not need
controller-specific exceptions. The lower left/middle pair is used on Sega rows.

The `auxiliary` group describes discrete emulated actions such as console reset,
difficulty switches or a core-provided shake event. Spare calibrated gameplay
buttons may supply these actions; host Home/system keys and directional inputs
are still not generic donors. A digital shake event does not imply motion-sensor
support. Per-port binding subsets keep player-one console panels out of player
two's required capabilities. Source inspection and implementation are in progress;
the recorded policy-v2 all-pairs counts are historical, not current policy-v6
results. No all-pairs, catalog-composition or runtime checks were run in this
implementation phase.

### Rules inherited from policy version 2

The source layout provides control IDs, groups, positions, analog capabilities,
and hardware-repeat relationships. The caller supplies the subset of controls
actually calibrated and the subset of target controls requested by the contract.
Absent calibration entries cannot win an assignment; unrequested target controls
cannot consume an input. Normalization uses the full declared face geometry, so
skipping calibration does not move the remaining buttons in the scoring model.

Candidate edges obey these shared rules:

- Preserve semantic identities, including D-pad orientation, stick side and
  direction, Start, and auxiliary menu controls. Select and Sega Mode are aliases;
  neither is silently substituted with Start or Home.
- Preserve existing ergonomic family presets: diamond west/south for two-button
  run/jump; N64 B/A for diamond west/south; C-left/C-down for north/east; the N64
  six-face projection onto Sega's two rows; horizontal-four's explicit B/A pair.
- A real right-stick direction can supply the corresponding digital N64 C input.
  Digital buttons cannot supply analog targets. A digital diamond without a right
  stick prefers C-left/C-down for its two remaining face buttons.
- Diamond/horizontal-four to Sega uses west/south/east for A/B/C and left
  shoulder/north/right shoulder for X/Y/Z. Six-button layouts can supply left/right
  shoulders or triggers from X/Z when needed. Same-hand shoulder/trigger fallback
  supplies remaining digital controls without reusing an already allocated input.
- Other face candidates use a lower-priority positional fallback. There is no
  arbitrary conversion of menu or D-pad inputs into face buttons.
- As a final, explicitly reported fallback, spare digital gameplay controls
  (faces, shoulders, triggers or stick clicks) can supply another gameplay button
  or Start/Select/Mode. This never consumes Home/system, menu or directional
  inputs as generic button donors. Thus an N64 pad's spare C buttons can supply
  PlayStation auxiliary controls instead of incorrectly reporting absent hardware.
- Hardware-repeat controls cannot stand in for independent gameplay buttons.
  They match only the same repeat control/relationship; normal core contracts do
  not request them.

These are family and capability rules, not per-controller-model exceptions or
per-emulator physical bindings. Partial/lossy conversions are not required to be
inverses. In particular, the previously reviewed two-button and horizontal-four
presets are preserved rather than redefining their primary buttons in this change.

## Global assignment

All allowed controls participate in a rectangular minimum-cost assignment
(Hungarian algorithm). One dummy unmatched slot per target makes missing hardware
explicit. Each physical control may be used at most once.

The objective is lexicographic:

1. Maximize mapped required controls.
2. Maximize mapped optional controls without sacrificing required coverage.
3. Minimize the sum of rule preference penalties.
4. Minimize position distance (normalized face clusters; whole-layout coordinates
   for digital cross-group fallbacks).
5. Break ties deterministically in canonical control-ID order.

Weights are derived from the target count so lower-priority costs cannot overwhelm
a higher-priority objective. Complexity is polynomial, `O(T²(S+T))`, with a validated
64-control resource bound per layout. There is no longer an eight-face exponential
search limit.

This is optimal **under this explicit policy and allowed candidate graph**, not a
claim of a universally best arrangement for every game, grip, or accessibility
need. Physically impossible pairs remain partial: e.g. NES cannot provide N64's
analog stick, and a standard N64 pad cannot provide a second analog stick.

Each mapping row carries an explanation. Missing rows distinguish incompatible
hardware, uncalibrated compatible controls, and insufficient distinct inputs.
Plans include `mapping_policy_version`; positional/shoulder/digital fallbacks also appear
in warnings. No calibration or existing emulator settings are rewritten.

## Verification and reproducible audit

Run the layout tests from the repository's development shell:

```console
nix develop -c cargo test -p lunchbox-app --lib controller_layout
nix develop -c cargo test -p lunchbox-app --lib controller_catalog
```

The existing test suite checks pairs for injectivity, capability/direction preservation, identity,
input-order independence, and complete result explanations. A separate augmenting-
path matcher verifies maximum required/optional coverage for every pair. Each
source input is also removed in turn across every pair to test partial calibration.
An exhaustive small-matrix oracle checks the assignment solver's optimal cost.
Family fixtures check actual expected mappings, not only structural invariants.
Catalog tests compose every source layout with every output contract and check that
the output contract is unchanged.

To export the current catalog's ordered mappings (23,104 with 152 layouts), use
a new absolute output filename (existing files are not overwritten). This command
is for the later testing phase and has not been run for policy 6:

```console
LUNCHBOX_LAYOUT_REPORT=/absolute/path/layout-pairs.json nix develop -c cargo test -p lunchbox-app --lib controller_layout::tests::export_all_pairs_report -- --ignored --nocapture
```

The JSON contains every assignment, its rule, missing reasons, and a separate
`required_complete` field per pair. Generating a result for all pairs does not mean
all pairs have sufficient hardware, or that every emulator/core has a launch
adapter. Runtime validation of the calibration/driver/core layers is separate.
