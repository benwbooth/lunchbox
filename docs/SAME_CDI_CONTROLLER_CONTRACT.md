# SAME CD-i controller contract

Pinned libretro source: `9a589f6ba8c35f5310853f63420d9dab1df63492`.
Database platform identity: `Philips CD-i`. No builds, tests or runtime probes.

The machine driver declares empty top-level input ports; the slave device owns
the pointer inputs in `src/mame/machine/cdislavehle.cpp`. X/Y are relative mouse
inputs with sensitivity 100 and key delta 2. Motion accumulates into a bounded
device position (X 0..767, Y 0..559). This is not a frontend absolute-coordinate
contract and must not be wired directly to a normalized lightgun axis.

Buttons 1 and 2 set protocol bits 0x02 and 0x04. Button 3 sets both bits (0x06).
A third frontend button therefore needs a combined-button route, not an invented
third independent native wire. Simultaneous buttons 1+2 have the same protocol
meaning, but the dedicated physical-button assignment is still part of coverage.

`src/osd/modules/input/input_retro.cpp` polls mouse X/Y and left/right only.
It registers four mouse buttons, but does not populate the third one from the
libretro middle-button input. The existing receiver alone does not provide a
dedicated third-button mapping. The frontend's generic joypad descriptors do
not prove gameplay pointer support. The launcher forces `-mouse`, while the
input poller separately checks the `mouse_enable` option; both layers matter.

The new seven-control catalog layout records relative motion and the three
button actions. There is no output/launch profile yet. Controller-driven pointer
conversion, physical pointer capture, native MAME input sequences/configuration,
BIOS/media startup and the combined-button route remain implementation work.
CHD, ISO and BIN/CUE formats remain in scope; BIOS discovery has not been
implemented by this controller step.

## Native sequence implementation

`controller_same_cdi::retropad_input_xml` now generates the exact `:slave_hle`
port overrides for left-stick velocity or D-pad relative motion, plus all three
button fields. The native engine's `analog_field::frame_update` uses absolute
joystick input as velocity for relative fields (`rawvalue / 8`); it does not
teleport the CD-i pointer to the stick position. D-pad mode uses increment and
decrement sequences with the driver's key delta 2. Both modes pin sensitivity
100 and explicitly clear unused axis sequences.

The wrapper registers MAME joystick buttons 1/2/3 from RetroPad A/B/X (the nearby
comment describes a different ordering and must not be used as the contract).
Binding the third CD-i field directly to joystick button 3 produces the native
combined-button action without depending on the unpopulated mouse-middle slot.
Physical mouse middle-button capture is still separate work.

The writer returns only an input subtree, not a replacement native config.
Preserving unrelated settings, arranging an owned cfg directory, startup guards
and launch profiles remain to be connected. Nothing has been built or tested.

Two explicit catalog preview profiles now bind the sequence-writer inputs:
`same-cdi-stick-pointer` uses standard left-stick directions and
`same-cdi-dpad-pointer` uses D-pad directions. Each has three distinct button
assignments mapped to RetroPad A/B/X. The physical-relative-pointer layout
remains separate, so stick velocity is not confused with mouse deltas. Neither
preview profile has a launch descriptor or increases enabled-core coverage.

## Owned configuration routing

The core's `.cmd` loader reads one bounded line and passes it to
`execute_game_cmd`. That path appends explicit arguments after its normal
save/system path setup. The command builder selects `cdimono1`, overrides
`cfg_directory` and the BIOS search path with session-owned directories,
and keeps the original disc as the last argument so per-game NVRAM naming is
preserved. It does not redirect the frontend save directory.

The builder supports CHD/ISO/CUE paths, quotes paths containing spaces, rejects
embedded quotes/control characters/search-path separators, and enforces the
core's 511-byte first-line capacity. It does not accept arbitrary commands.
Owned `.cmd` file creation, native XML merging and default configuration handling
are implemented below. Launch integration is pending. No command has been executed.

## Native configuration preservation

The core has a SAME-specific fallback in `src/emu/config.cpp`: when
`cdimono1.cfg` is absent it writes a version-10 config with the standard 4:3
video view, D-pad pointer sequences, and button fields mapped to native
joystick buttons 4/2/1. The calibrated profiles intentionally replace those
five fields with their explicitly declared sequences, not those defaults.

`merge_configuration` now edits only the identified pointer/button port nodes
inside the `cdimono1` input section and preserves all other original bytes,
including unrelated inputs, device maps, video settings and comments. Missing
input/system sections and self-closing sections are expanded. A missing source
file receives the core's standard video-view baseline plus calibrated input.
Ambiguous duplicate target systems/input sections, unsupported versions/DTDs,
and malformed or oversized documents fail rather than discarding user settings.

The core option parser accepts later values at equal priority, confirming the
command builder's late `cfg_directory` override. The preparation helper below
writes the merged result to an owned directory and retains source snapshots.

Owned preparation is now implemented: `prepare_configuration` snapshots the
original `default.cfg` and `cdimono1.cfg`, copies the default file byte-for-byte
when present, writes the merged machine config and bounded launch command into
a retained temporary directory, and rechecks source/owned hashes and frontend
directory paths. Original files are never opened for writing. Missing files
remain absence-sensitive, so a newly created source config invalidates the
preparation. Files are limited to 8 MiB each and permission errors are surfaced.

These are session-only config files; changes made by the core to this owned
configuration are not automatically written back to the user's originals.
Disc-content checks and launch wiring remain pending. The preparation function
has not been run or tested.

The two preview profiles now pin nine source-defined core options and catalog
validation checks them: native INI reading/writing, automatic state restoration,
arbitrary CLI boot, inherited path routing, physical mouse polling, lightgun
mode and four-way filtering are disabled; the fixed button-profile initializer
is enabled. Owned `.cmd` loading works independently of the arbitrary CLI-boot
option. Save-state naming and per-game NVRAM preferences are left unchanged.

Source `ioport_manager::load_config` applies device renumbering/remap tables
only to controller (`ctrlr`) configs, not to copied `default.cfg` or game CFGs.
Keeping INI reading off prevents an inherited `ctrlr` selection in this fixed
mode. This mode therefore does not claim compatibility with custom native INI
setups; those settings are not deleted or rewritten. Launch integration remains
pending, and none of these options has been applied to a running core.

The BIOS archive validator now checks bounded, user-supplied ZIP bytes against
the pinned driver's exact ROM sizes and SHA-1 values. It accepts the Magnavox
200 or Philips 220 F2 main ROM, requires both declared MCU dumps, rejects
duplicate case-insensitive member names, and chooses Magnavox when both main
ROMs are present. The source-marked nonbooting alternate BIOS is not accepted
as the only main ROM. The MCU hashes identify the source's `BAD_DUMP` entries;
matching them does not establish their hardware accuracy. No firmware has been
downloaded or validated at runtime.

Owned preparation now discovers `cdimono1.zip` beside the disc first, then in
`system/same_cdi/bios`. An invalid first archive fails rather than silently
falling back. It records the selected source (and absence of an earlier
candidate), validates its bytes, and copies them into the session's owned BIOS
directory. The command uses only that ROM search directory and explicitly pins
the selected `-bios`; the original disc remains the final argument. Source and
owned archive hashes are rechecked along with the configuration. This fixed
contract requires a ZIP archive; loose ROMs and 7z firmware packages are not yet
supported. Disc validation and retained launch integration remain required
before enabling profiles.

The launch-side preparation helper now resolves system and save directories
from explicit frontend settings, using the original disc's parent and the
source-reported library name `SAME_CDI` for save sorting. Its private append
configuration freezes those paths and disables a second sorting pass against
the owned command file. Effective original-disc core options are inspected
before preparation: an enabled native INI reader is rejected instead of being
silently disabled. Included configs, custom environments and non-native
launches require separate resolution. The helper is not yet invoked by the
launch path; command substitution must retain preparation through child exit.
No launch or tests were run.

State-path preparation now mirrors RetroArch's source ordering: start from
the explicit state directory (or original disc directory), append the original
content-directory name when enabled, then append `SAME_CDI` when core sorting
is enabled. The resolved directory must already exist; the private config
freezes it and disables further state sorting. The owned command filename now
uses the original disc stem with `.cmd`, preserving the basename RetroArch
uses for `.state` files and numbered slots. No existing states are moved,
loaded or rewritten by preparation. This is source-derived implementation,
not runtime verification; media guards and final launch wiring remain pending.

ISO preparation now has a native-geometry helper and retained file snapshot.
The pinned `chdcd_parse_iso` chooses 2048-byte sectors first, then 2336, then
2352, strictly by length divisibility; this ordering is preserved even when
multiple widths divide the length. Empty and over-4-GiB images are rejected.
The snapshot checks the original path, open file and current path metadata;
Unix identity checks include device, inode and change time. This detects
ordinary replacement/modification during preparation, not arbitrary concurrent
writes after launch. Geometry is not a CD-i identity or bootability check.
The helper remains unconnected while CUE/CHD handling is implemented.

CHD container preflight now checks the native CD unit size (2448 bytes),
aligned/bounded hunks and consistent logical geometry. Direct parent-dependent
CHDs are rejected: this core's image device opens supplied CHDs without a
parent resolver. The PlayStation adapter's sibling-parent search therefore
must not be reused for CD-i. Track metadata and decoded-sector checks remain
pending; a valid header alone is not counted as disc or controller coverage.

The CHD metadata reader now walks the linked metadata records with cycle,
file-boundary, entry-count and retained-byte limits. It preserves tag-local
ordering and payloads for CHTR, CHT2, legacy CD and GD records. This matters
because SAME CD-i prefers CHTR over CHT2 at each ordinal, whereas the existing
Beetle reader prefers the reverse. Track interpretation is still pending;
retaining a GD or legacy record does not claim it is supported CD-i content.

Text CD track interpretation now selects CHTR before CHT2 per ordinal and
parses track kinds, frame counts, subchannels and stored versus virtual gaps.
It preserves the native parser's first-match aliases: `CDI/2352` resolves to
Mode 1 raw, and `MODE2/2336` to Mode 2, despite duplicate later branches in
the source. Sequential numbering and bounded frame/gap values are required.
Whole-disc addressing, decoded sectors and binary legacy/GD interpretation
remain separate work; these helpers have not been run against media.

CHD text-track addressing now maintains the native physical, stored and
logical cursors independently. Stored pregaps offset the current logical
start; virtual pregaps advance the logical cursor, postgaps affect the next
track, and storage advances with four-frame padding. The map rejects native
counter overflow, pregap subtraction underflow and a mismatch between padded
track storage and the CHD logical image size. Decoding and launch integration
remain pending; this code has not been exercised against disc images.

The CHD decoder wrapper now composes header validation, text-track metadata
and addressing, retaining an open file and checking source identity before
and after reads. It decodes one bounded hunk at a time through the existing
CHD library and exposes 2448-byte storage frames, with cache invalidation on
decode failure. Storage frames are deliberately not labeled disc LBAs: native
track translation and data-sector interpretation remain required. No media
has been opened or decoded during this implementation step.

Native CHD sector reads now follow `logical_to_chd_lba` and
`read_partial_sector`: select by the next track's logical start, account for
stored pregaps, and return zero data for virtual pregaps. The reader copies
the native track's data width into the CDIC's zero-initialized 2560-byte
`RAW_DONTCARE` buffer. This is deliberately not PlayStation-style cooked ISO
data. The CDIC's subsequent stateful byte swapping, descrambling and sector
validity rules still need to be applied for media inspection.

The sector normalizer now implements the CDIC preprocessing order: latch
byte swapping from the native two-byte heuristic, swap the whole buffer,
check BCD MSF/mode/duplicate subheader fields, then try descrambling bytes
12 through 2351 and retain that result only if the same check succeeds.
The scrambling stream is generated with the standard 15-bit recurrence
instead of embedding the source table. This validity result is the native
heuristic, not EDC/ECC verification or proof of a playable disc. The recurrence
and complete pipeline remain untested as requested.

ISO snapshots now read native sectors into the same zero-initialized
2560-byte buffer as the CDIC, using exactly the sector width selected by the
native length rules. Reads check image bounds and source identity before and
after I/O. No synthetic sync header or cooked-to-raw conversion is added:
the core's `RAW_DONTCARE` path does not add one either. A cooked ISO therefore
must not be assumed compatible merely because its length is accepted.

CUE lexical and time parsing now follows the native conventions: single and
double quotes without backslash escapes; one numeric time field means frames,
two mean minutes/seconds, and three mean minutes/seconds/frames. Unterminated
quotes, overlong lines, invalid numeric fields and native-counter overflow
are rejected. FILE/TRACK/INDEX interpretation and backing-file preparation
remain pending; these primitives alone do not enable CUE launches.

CUE declarations now retain FILE type, native track kind, subchannel width,
byte-swap policy, indexes and gaps. INDEX 01 computes a stored pregap from
INDEX 00 only when the current pregap is zero, preserving directive ordering.
The parser requires sequential tracks, INDEX 01 and complete FILE/TRACK pairs,
and rejects ambiguous duplicates. BINARY, MOTOROLA and first-track WAVE
declarations are represented; backing-file lengths, WAVE payload inspection
and shared-WAVE offsets still need implementation. No referenced file has
been opened and no CUE launch profile has been enabled.

CUE span resolution now mirrors the native shared-file and final-track
arithmetic using declared filename equality, track widths and INDEX offsets.
It checks each resulting byte range against the backing-file length, including
the native carry-forward offset behavior between shared-file groups. WAVE
tracks accept separately validated payload regions; their parser and actual
file snapshots remain pending. No backing files were read during this step.

WAVE region inspection now locates the native-supported stereo 16-bit
44.1-kHz PCM payload with bounded RIFF chunk reads. Chunk stepping preserves
the pinned parser's lack of odd-byte padding; unsupported files fail rather
than being inspected under different playback assumptions. Audio payloads
are not decoded. The CUE parser also rejects the paired Redump GD-ROM density
markers that would select a different native loader. File snapshot integration
is still pending, and no WAVE files were inspected during implementation.

CUE preparation now snapshots the declaration and retains bounded regular
backing files, resolves WAVE regions, and checks the calculated spans. Original
relative names remain distinct even when they resolve to aliases, preserving
native filename comparisons. Ambiguous absolute/Windows-style references are
rejected. Declaration hashes and backing-file path/open-file metadata are
rechecked together. Native CUE sector reads and launch wiring remain pending;
this preparation has not been invoked against user media.

CUE sector reads now use native logical track starts, virtual/stored pregaps,
backing offsets and data-plus-subchannel strides. Data is read into the CDIC
buffer, with the native loose-file byte swapping through byte 2351 where
required. Out-of-file reads fail, and declaration/backing snapshots are checked
before and after reads. CUE, ISO and CHD now expose native sector buffers;
their common media inspection and retained launch integration remain pending.

The common disc snapshot now dispatches ISO, CUE and CHD preparation and
sector reads, with caller-owned normalization state. Owned CD-i configuration
preparation retains this disc snapshot and rechecks it with configuration and
BIOS inputs. Format/track failures therefore surface before session files are
prepared. Native validity observations are not promoted to a bootability or
controller-coverage verdict. The actual launcher still needs to retain and
attach this preparation; no media or tests were run for this step.

Launch integration now retains the complete preparation in `CalibratedLaunch`,
rechecks it immediately before spawning, and appends the frozen frontend paths.
Core options are resolved from the original disc before the final command-file
substitution. Substitution is staged on a cloned plan and accepted only after
configuration attachment and input rechecks; the resulting plan records the
actual command-file content identity. The state directory's canonical path is
also retained. Both profiles are now enabled as explicit one-player pointer
modes. This path has not been built, tested or launched.

The topology declares all six frontend ports polled by the native
input loop, while catalog validation limits any launch contract to one active
RetroPad. The native device-selection callback is empty, so unused ports need
the frontend's cleared bindings, not merely a different device ID. Both modes
require a fresh start, the exact `SAME_CDI` library identity and CHD/ISO/CUE
content. The platform alias is `Philips CD-i`, matching the emulator registry.
Source checks confirm the frontend clears unused button/axis/key bindings with
autodetection disabled, while the command loader appends explicit arguments
after default paths and derives NVRAM naming from the original final disc
argument. These are source-backed launch implementations, not runtime proof.
