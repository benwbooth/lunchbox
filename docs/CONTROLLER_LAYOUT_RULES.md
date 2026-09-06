# Shared controller-layout mapping rules

Lunchbox does not store an M-by-N table of controller-model mappings. It stores
one semantic description per layout, one output contract per emulator/core mode,
and one deterministic assignment algorithm. The current catalog contains 16
layouts, so the all-pairs audit exercises 256 ordered source/target combinations.
Adding a layout within a supported family automatically participates in all pairs.

The composition is:

`calibrated physical inputs -> source layout -> target layout -> core output contract`

RetroPad is an output transport, not a physical Xbox-controller assumption. A
core contract translates target controls to RetroPad; the native launch writer
then translates the saved calibration to the connected driver's numbering. The
same layout assignment is reused by every contract requesting the same target
controls, including standalone contracts. A documented contract does not itself
mean its launch adapter has been implemented or runtime-verified.

## Policy version 2

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

Tests cover every pair for injectivity, capability/direction preservation, identity,
input-order independence, and complete result explanations. A separate augmenting-
path matcher verifies maximum required/optional coverage for every pair. Each
source input is also removed in turn across every pair to test partial calibration.
An exhaustive small-matrix oracle checks the assignment solver's optimal cost.
Family fixtures check actual expected mappings, not only structural invariants.
Catalog tests compose every source layout with every output contract and check that
the output contract is unchanged.

To export all 256 mappings, use a new absolute output filename (existing files are
not overwritten):

```console
LUNCHBOX_LAYOUT_REPORT=/absolute/path/layout-pairs.json nix develop -c cargo test -p lunchbox-app --lib controller_layout::tests::export_all_pairs_report -- --ignored --nocapture
```

The JSON contains every assignment, its rule, missing reasons, and a separate
`required_complete` field per pair. Generating a result for all pairs does not mean
all pairs have sufficient hardware, or that every emulator/core has a launch
adapter. Runtime validation of the calibration/driver/core layers is separate.
