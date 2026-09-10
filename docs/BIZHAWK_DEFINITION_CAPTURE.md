# Native control-definition capture

Pinned BizHawk: `8c6b8958bbbe623eaaa36bc82af858b812893628`.

Source inspected: JoypadLuaLibrary, JoypadApi and IController.ToDictionary in
Emulation.Common/Extensions.cs. joypad.getimmediate() enumerates the active
controller's entire definition: boolean values for buttons and numbers for axes,
including unpressed controls. No player argument retains full native prefixes.
EmulationLuaLibrary.getsystemid and GameInfoLuaLibrary.getromhash provide loaded
system and game-info hash values; neither identifies the exact core implementation.

Step 378 adds a Rust-generated one-shot Lua script and bounded JSON parser.
The script sorts names, captures types rather than input activity, and writes
to a caller-supplied output path. Paths use fixed-width decimal byte escapes;
nonce is restricted to 64 lowercase hex characters. It does not advance frames,
inject controls, close the emulator or register persistent callbacks.

The future caller must allocate a private fresh directory, own the child process,
bind the expected content hash and runtime artifacts, and enforce a matching
control contract. A matching nonce is correlation, not authentication. Parsing a
response does not prove which core/process wrote it. The actual GPGX system/dev
array is not available through these APIs and is not inferred from the JSON.
No script or emulator has been run; capture lifecycle and launch gating remain
unfinished. ROM hash interpretation for discs/archives also needs explicit binding.

Step 379 adds GPGX exact-definition comparison. It requires GEN, an exact
caller-provided expected ROM hash, no axes, and precisely the requested normal-pad
or Activator controls plus Power/Reset and, for disc content, Previous/Next Disk.
Duplicate, missing and extra controls are rejected. This comparison does not
authenticate the response or infer raw native device-array placement. The caller
still needs trusted process ownership, runtime/core binding and launch gating.

Step 380 adds Linux CaptureFiles ownership: fresh private directory, random
256-bit nonce, create-new script with mode 0600, script-integrity checking and
bounded no-follow/nonblocking regular-file reads. Lua writes a sibling partial
file and renames it only after closing. Reader checks ownership, link count,
metadata stability and named-file identity. It does not spawn a process or
authenticate a same-user writer; callers still need child ownership and runtime/
content binding. No capture preparation or script was executed during implementation.

Step 387 adds argument preparation from pinned revision
`8c6b8958bbbe623eaaa36bc82af858b812893628`,
`src/BizHawk.Client.Common/ArgParser.cs`: `--lua` accepts a script or Console
session and implies `--luaconsole`. CaptureFiles inserts one literal
`--lua=<private path>` argv token after checking script integrity. Only application
arguments belong in this method, not a Mono executable/assembly prefix.
Competing script/tool, state/movie, dump and exit options are rejected in separate,
equals and colon forms. Response files and option terminators are conservatively
rejected. Input arguments remain untouched; process launch, installed-parser
compatibility, config-driven autoload behavior and capture gating remain unverified
and unfinished. No argument preparation or script was executed during implementation.

Step 388 prepares a capture-only private config through the existing source-
integrity/temporary-config owner. At the same pinned revision,
`LuaConsole.LuaConsole_Load` autoloads RecentLuaSession/RecentLua;
`MainForm` autoloads RecentRoms, RecentMovies and AutoLoadLastSaveSlot;
`MainForm.Events` checks RecentWatches and Cheats.Recent; `ToolManager.AutoLoad`
examines CommonToolSettings and typed CustomToolSettings entries. The private
copy disables those recent/state flags and empties both tool dictionaries.
Core settings, controller mappings and unrelated fields remain intact. Malformed
recent/cheat objects fail preparation. No original config is edited and no
preparation was executed. Selecting the private config in the owned launch,
preventing other runtime mutations and verifying installed-version behavior
remain unfinished.

Step 389 combines both private-file owners and their application arguments in
Linux PreparedCapture. It uses the shared config argument resolver, replacing
an explicit --config or selecting the executable directory's config.ini by
default. Working/executable bases must be absolute; direct-Mono prefixes remain
the caller's responsibility. Input argv is never modified. Source/generated
config and script checks run during preparation and around response reading.
This object must outlive the future owned child. It does not spawn a child,
prove which process wrote a response, or bind the response to a runtime/core/ROM.
No preparation routine was executed during implementation.

Step 390 adds a Linux capture process owner. It spawns the selected absolute
executable directly with an optional absolute Mono assembly, fingerprints both,
uses the prepared working directory/arguments, null standard streams and a new
process group. The child handle stays private. WNOWAIT keeps an exited leader's
PID reserved until group cleanup and reaping; ECHILD marks ownership lost and
prevents further signaling. Private files stay owned through normal cleanup.
This does not authenticate same-user response writers, establish transitive
runtime dependencies or prevent descendants escaping the group. Timeout,
cancellation, bounded response polling and launch gating remain unfinished.
No child was spawned during implementation.

Step 391 adds consuming worker-thread wait_for_definition: a nonzero timeout
up to 60 seconds, measured from spawn ownership; cancellation; 20 ms polling
of the atomically published response; and unreaped child-exit checks. Invalid
published responses fail immediately rather than waiting for a rewrite. The
owned group is stopped/reaped for both success and failure, with cleanup errors
reported. Successful return rechecks file integrity, cancellation and deadline.
Filesystem reads and kernel process cleanup are not hard-real-time interruptible;
the deadline prevents late success, not an absolute upper bound on OS calls.
Core/content authentication and launch integration remain pending. Nothing ran.

Step 392 makes top-level content selection explicit in PreparedCapture::prepare.
Input arguments contain only --config (separate/equals/colon), --gdi,
--fullscreen or --chromeless; unknown options and extra positional arguments
fail. Preparation appends one absolute UTF-8 content path last and fingerprints
its regular-file bytes with the existing artifact owner. Archive-member `|`
selectors are rejected. This does not bind archive extraction, playlist/disc
dependencies, database-derived GameInfo.Hash or the loaded core; it must not be
treated as complete ROM identity. No preparation or capture ran.

Step 393 connects the owned process waiter to GPGX's requested-definition
comparison. The caller supplies topology, expected GameInfo.Hash and disc mode;
these are not inferred from the response. Capture success is followed by exact
system/hash/button/axis validation and another cancellation check. This remains
a definition match, not an authenticated core or content receipt.

Source caution at the pinned revision: RomGame.cs strips recognized headers and
deinterleaves SMD before Database.GetGameInfo; Database/Database.cs looks up SHA1,
then MD5, then CRC32, returning the matching database entry, or SHA1 on a miss.
GameInfo.Hash therefore must not be presumed equal to the raw file SHA256 or even
always its raw SHA1. Patch options can further change loaded data after lookup.
Hash derivation, loaded-core provenance and launch integration remain pending.

Step 399 routes captured-definition validation through the same DigitalDeck enum
used by native digital-session preparation. DefinitionContent supplies independent
runtime system, expected hash and typed media metadata. SNES/SMS accept cartridge
expectations, NES requires explicit console metadata, and PCE/TurboNyma/GPGX
accept cartridge or disc expectations. PCEHawk discs require expected runtime
system PCECD. All six validators are reached through this dispatch, with an owned
capture waiter and cancellation recheck. Ordinary launch preparation does not yet
call it: expectation derivation, provenance and launch gating remain unfinished.
No tests or captures ran.

Step 400 sets StartPaused=true and AutosaveSaveRAM=false in the private capture
config. At the pinned revision MainForm applies StartPaused before Shown loads
the command-line Lua script; the main loop calls ResumeScripts(false) outside
gameplay frame advancement. The capture script does not call frameadvance.
This requests a paused inspection and disables periodic SaveRAM writes; it does
not isolate every persistence path, prevent user interaction, or prove that the
installed runtime advances zero frames. No capture or emulator was executed.

Step 401 adds shared request preflight and deck-aware capture preparation.
Topology, media/system compatibility, hash presence and FDS side bounds are
checked before private-file preparation, before waiting, and before comparison.
SMS preflight uses its complete declared deck rather than inferring selected
physical mappings. This only checks consistency of independently supplied
expectations; those expectations still need provenance and launch integration.
No preparation or captures ran.

Step 402 retains deck and content expectations in PreparedDeckCapture's private
fields alongside the prepared files. Its consuming worker method validates
cancellation and timeout before spawn, then uses those same owned expectations
for the shared comparison. The request cannot be replaced through this API.
The selected source config must still configure the intended deck; mismatch
is detected by comparison, not automatically corrected here. Runtime provenance,
expectation derivation and normal-launch integration remain unfinished. Nothing ran.

Step 403 adds GPGX generated-config preflight at deck-aware preparation and
immediately before spawn. Integrity-guarded config reading checks source and
generated artifacts around the read. The validator requires the encoder's exact
Genplus-gx preference, DontTryOtherCores=true, pad mode and left/right control
types. This includes adapter/Activator selections but does not prove native
game-driven device overrides, loaded-core identity or transitive runtime integrity.
Other core config preflights and launch integration remain pending. Nothing ran.

Step 404 extends config preflight to every DigitalDeck variant, using each
adapter encoder's exact selection fields. A shared bounded JSON reader requires
the expected core preference, DontTryOtherCores and a sync-settings object.
Individual checks cover SNES left/right devices, NES non-Famicom mode/empty
expansion/left/right ports, SMS standard ports with keyboard disabled, PCE fixed
port types, and TurboNyma gamepad/none devices plus multitap setting. The shared
dispatcher runs at preparation and before spawn. GPGX retains its existing check.
Missing or differently typed fields fail rather than assuming defaults. These
checks do not establish transitive runtime provenance, content-driven overrides
or normal-launch gating. No tests, preparation or captures ran.

Step 405 shares Waterbox payload resolution with normal digital sessions.
Capture retains the prepared installation/working directories, checks effective
BIZHAWK_HOME against that installation and fingerprints dll/snes9x.wbx,
dll/turbo.wbx or dll/gpgx.wbx for the selected deck. The artifact joins existing
config/script/program checks across capture. Managed cores have no Waterbox
payload in this mapping. This does not inventory transitive libraries or prove
which core actually loaded. No tests or captures ran.

Step 406 explicitly disables SingleInstanceMode, AcceptBackgroundInput and
AcceptBackgroundInputControllerOnly in capture copies. Pinned MainForm checks
SingleInstanceMode before assigning normal startup state and consults background
input flags when no owned form is focused. Single-instance implementation is
platform-specific; these flags are explicit capture policy, not proof of full
isolation. Focused-window input and user interaction remain possible. The user's
original config is unchanged, and no capture was executed.

Step 407 uses capture schema v2 with paused_before/paused_after and signed
32-bit frame_before/frame_after fields. Pinned ClientLuaLibrary.ispaused calls
EmuClient.IsPaused, and EmulationLuaLibrary.framecount calls Emulation.FrameCount.
The generated script samples before control enumeration and after identity
reads; both script and parser require paused endpoints and an unchanged,
nonnegative count. Missing evidence and schema v1 fail instead of defaulting.
This does not prove zero frames since startup, absence of counter resets, or
response authenticity. No scripts or tests ran.

Step 408 pipes stderr in nonblocking mode, reading at most sixteen 4 KiB chunks
per poll. Total diagnostic output above 1 MiB fails capture; only the final
8 KiB are retained. After cleanup another bounded drain collects pending output.
Failure context labels and escapes the child-provided text; stdout remains
discarded. This avoids unbounded logs and pipe backpressure while preserving
bounded cancellation checks. Lua-console-only messages may still be absent from
stderr. No tests, pipe reads or capture processes were executed.

Step 409 wraps definition observation in Lua pcall and publishes either a normal
v2 definition or a v2/nonce/error envelope. Empty and oversized error messages
receive bounded fallback text; no partial definition is returned. Host parsing
checks the original envelope for duplicate/unknown fields, current nonce/version
and a nonempty error of at most 4096 bytes, then reports escaped diagnostic text.
The same private-file checks and atomic rename apply. Publication/I/O errors
outside pcall still need stderr or timeout handling. Nothing was executed.

## Step 441: independent cartridge identity prerequisite

Source inspected at BizHawk `8c6b8958bbbe623eaaa36bc82af858b812893628`:
`BizHawk.Client.Common/RomGame.cs`,
`BizHawk.Emulation.Common/Database/Database.cs`, and
`BizHawk.Client.Common/lua/CommonLibs/GameInfoLuaLibrary.cs` under `src/`.
This audit does not add a launch gate or an enabled profile.

`gameinfo.getromhash()` returns `GameInfo.Hash`, not a fresh file hash. The
cartridge path first derives RomData from FileData. Its header heuristic removes
a length remainder of 128 or 512 modulo 1024, with explicit tape/disk-extension
and two Intellivision SHA-1 exceptions. SMD processing then caps the output at
4 MiB and deinterleaves complete 16 KiB pages; any incomplete output tail remains
zero-filled. This behavior must not be replaced with an assumed generic ROM
header rule or a cleaner deinterleaver when matching the pinned loader.

Database.GetGameInfo hashes that RomData and tries SHA-1, then MD5, then CRC32.
A hit returns the database entry's stored digest and system metadata. A miss
uses SHA-1 and platform inference. Thus a valid captured hash can be a database
MD5 or CRC32, and an extension alone does not establish the runtime system.
RomGame applies database PatchBytes and optional explicit IPS/BPS patches after
GameInfo creation, without recomputing that identity there. The reported hash
alone therefore does not establish the final bytes delivered to a core.

Remaining implementation before automatic digital-deck capture launch gating:

1. Own the exact selected content and loader-normalized bytes independently of
   the response; preserve the original artifact fingerprint separately.
2. Resolve and retain the effective database inputs, precedence, system and
   patch metadata rather than accepting any matching digest candidate.
3. Account for the selected core's later content transformations and resolve
   disc/subsystem identities through their own loader paths.
4. Pair that independently derived expectation with PreparedDeckCapture and
   recheck retained content/database/runtime artifacts through launch handoff.

DefinitionContent currently accepts caller-provided expectations. Do not wire
it to raw-file SHA-1, copy the captured hash back as the expected value, or treat
the existing request/response equality check as independent content provenance.
No tests, builds, Lua scripts or emulator processes ran during this audit.

Step 442 adds `cartridge_identity::DatabaseInput` for the selected digital-deck
cartridge format families. It owns the pre-database normalized bytes, original
and normalized SHA-1 and transformation details. Input is bounded to 512 MiB;
the API requires a normalized supported extension and does not infer a core or
runtime system from it. The pinned header hash exceptions and SMD truncation /
zero-filled remainder are retained. Tape/disc, N64, 3DS and other loader paths
are not passed through this cartridge helper. It does not resolve database
precedence, apply later patches, or produce a final expected GameInfo.Hash.
Launch does not call it yet; independent database/core provenance and handoff
remain required. Only source inspection, formatting and whitespace checks ran.

Step 443 adds database-record parsing and RecordIndex lookup primitives. Records
retain name/system, sixth-field metadata, region and forced-core values; the
fifth field remains ignored, matching ParseCGIRecord rather than assuming the
SaveDatabaseEntry writer's shorter output changes the reader contract. Digest
prefix removal, ASCII uppercase keys, last-record-wins insertion, and ordered
SHA-1/MD5/CRC32 lookup follow the inspected Database.cs. Data rows and unique-key
counts are bounded; invalid digest keys/directives are rejected by these APIs.
The forthcoming include loader must decide how to handle rejected records and
establish complete ordered bundled/user inputs. No index result authenticates
those inputs, and a miss cannot yet authorize the fallback GameInfo identity.
Patch application and capture/launch handoff remain unfinished. No tests ran.

Step 444 implements DatabaseSnapshot over explicitly selected absolute roots.
It expands gamedb.txt in order, resolves includes relative to the configured
bundled/user root (not each including file), retains the user-root fallback,
and records optional absent files so their later appearance invalidates lookup.
Repeated non-cyclic includes are replayed; cycles, more than 1024 visits, depth
32 and expanded bytes above 128 MiB fail. Consumed files retain their requested
paths, canonical targets and SHA-256 fingerprints. Parsed bytes must match the
fingerprint, and lookup rechecks roots, absence observations and all artifacts.

This is deliberately not a partial-success reader: malformed records, missing
required bundled files, unsupported encodings, absolute/parent-traversing include
paths and non-directory roots fail even where a particular upstream build might
log and continue. UTF-8 with an optional BOM is implemented. The caller still
must derive the actual runtime roots and establish the selected build's database
policy (including ALWAYS_MISS variants). No current launch path invokes this
snapshot; post-database patching and capture handoff remain unfinished. No
database traversal, emulator run, build or test was executed in this step.

Step 445 adds DatabaseSnapshot::prepare_for_environment for Linux direct-Mono
invocations. Source: GameDBHelper.BackgroundInitAll appends gamedb to
PathUtils.ExeDirectoryPath and DataDirectoryPath; PathExtensions.cs initializes
those from existing BIZHAWK_HOME and BIZHAWK_DATA_HOME directories on Unix.
The existing Lunchbox direct-Mono environment builder supplies both explicitly.
The new entry point requires unique explicit UTF-8 absolute existing directories
and checks BIZHAWK_HOME against the selected installation. It retains requested
paths for symlink-change checks, then delegates to the ordered snapshot loader.
It does not guess inherited values, accept relative-path/fallback ambiguity or
claim wrapper equivalence. The entry point is not yet invoked by capture or
launch. Build database policy, patched content identity and handoff still need
implementation; no tests or runtime reads were performed.

Step 446 connects CaptureIdentity to PreparedDeckCapture for Cartridge and Nes
media. It rereads the retained content path under a bound, compares those bytes
to a SHA-256 artifact fingerprint, normalizes the cartridge, resolves the selected
database and compares independently derived system/hash with the request before
spawn. The original PreparedCapture content artifact is still checked as well.
The new identity/database owner remains alive through capture and is rechecked
after the child finishes, including on a failed definition result. Cancellation
is checked again before spawn after identity preparation.

Database misses currently implement fixed NES/SMS/GG/SG/GEN/PCE/SGX extension
branches and non-1-MiB SNES files. Unknown extensions and the 1 MiB Satellaview
heuristic remain explicit errors until implemented. Database hits use the actual
entry's digest/system, not a list of acceptable response candidates. This proves
neither post-GameInfo patch bytes nor loaded core/build policy; disc media still
require their separate identity contract. Ordinary launch does not yet call the
prepared capture API. No cartridge/database reads, captures, tests or builds ran
during implementation; source, formatting and whitespace checks only.

Step 447 implements the pinned SatellaviewFileTypeDetector header score for
1 MiB SNES database misses, checking LoROM first and HiROM second. The title is
decoded as Shift-JIS with encoding_rs 0.8.35, trimmed, and tested against the
source's ASCII/kana/fullwidth character ranges. Malformed byte sequences fail
explicitly until the runtime's .NET decoder replacement behavior is reproduced.
Self-destruct values, speed/mapping bits, content type and DRM penalties retain
the source threshold. Upstream's unconditional VerifyChecksum contributes no
penalty; this is heuristic system selection, not checksum validation. A positive
match produces BSX and is rejected by the ordinary SNES deck's system comparison,
not silently treated as SNES. Dependency lock resolution ran offline and added
only encoding_rs; no builds, tests, content reads or captures were run.

Step 448 adds .rom to the cartridge format family and resolves empty loader
system IDs using the owned capture config's PreferredPlatformsForExtensions.
ConfigExtensions.TryGetChosenSystemForFileExt uses the exact lowercase extension
key and a nonempty value; RomLoader consults it only when GameInfo.System is
empty. The new resolver preserves that precedence, including empty-system
database hits, and never substitutes the requested deck as platform evidence.
Unselected/invalid preferences fail rather than opening an interactive chooser.
PreparedDeckCapture obtains and validates one configuration string before identity
preparation; the existing generated-config fingerprint is still rechecked at
spawn. Ordinary launch integration and the previously listed runtime provenance
gaps remain. No tests, builds, captures or data-loading routines ran.

Step 449 changes PreparedDeckCapture::capture to return CapturedDeckDefinition,
which owns the observation and optional cartridge/database snapshot. Definition
access rechecks retained cartridge inputs, and verify_cartridge_identity fails
explicitly when the observation has no such snapshot (currently disc media).
The result exposes neither mutable definition access nor a consuming extraction
that silently discards its retained snapshot. The eventual launch owner must keep
this object and request the appropriate evidence check at handoff. Other runtime
artifacts, build/core provenance and post-identity patches remain separate gaps;
this is not a complete launch receipt. No callers of this prepared capture API
exist in ordinary launch yet. No tests, builds or captures ran.

Step 450 retains selected program, optional managed assembly and Waterbox artifact
snapshots in CapturedDeckDefinition. Child preparation receives clones of those
same baselines, and completion and definition access recheck them; there is also
an explicit verify_runtime_files handoff method. No post-capture recapture can
silently bless changed files. This covers only selected top-level files, not
transitive dependencies, generated configuration lifetime, actual loaded-core
identity or build policy. Ordinary launch is still not connected to the capture
API. No tests, builds, file-fingerprint routines or emulator captures ran.

Step 451 binds CapturedDeckDefinition to its original DigitalDeck and
DefinitionMedium (including FDS side count/VS mode), with exact structural
equality. verify_request rejects changed topology/media, then rechecks retained
files and the full definition against the proposed content request. The result
also stores a SHA-256 of the exact owned capture configuration and exposes a
bounded exact-byte comparison. That method checks a capture config, not a normal
launch config whose startup policies differ. Semantic capture-to-launch config
comparison and normal launch integration are still pending. No tests, builds or
captures ran; only source, formatting and whitespace checks.

Step 452 adds verify_capture_projection for the future handoff path. It rechecks
the proposed deck/content request and retained artifacts, applies the same
encode_capture_config function used during preparation to candidate config text,
compares that canonical output to the retained capture fingerprint, and repeats
deck config validation. It does not maintain a separate list of ignored keys
which could drift from capture preparation. This checks configuration after
capture policy is applied, not the safety/equivalence of startup actions that
policy suppresses. Autoloads, tool settings and single-instance/background-input
behavior still require independent normal-launch validation. No launch path is
enabled by this helper, and no tests, builds or captures ran.

Step 453 adds a read-only startup check and combined cartridge handoff config
gate. Autoload booleans must be false or absent with their source-default false
value; malformed shapes are not treated as defaults. It checks state loading,
single-instance forwarding, recent Lua/session/ROM/movie/watch and cheat loading,
and common tool AutoLoad flags. Nonempty custom tool settings remain an explicit
unsupported case because ToolManager can instantiate typed nested settings.
The combined gate requires cartridge evidence and the existing projected-config
comparison. It does not change saved settings, validate launch CLI/environment,
or establish loaded-core/build provenance. Ordinary launch integration remains
pending. No tests, builds or runtime checks ran.

Step 454 retains explicit program/assembly paths, working directory and its
canonical target, and the supplied environment vector in the capture result.
verify_invocation compares those exact requested values before rechecking runtime
files. Directory identity is also checked after observation and on subsequent
runtime-file checks. This is intentionally not equivalence of reordered duplicate
environment keys or path aliases. Inherited process environment, CLI options,
transitive dependencies and normal launch wiring still require their own checks.
No tests, builds, emulator captures or runtime-directory checks were executed.

Step 455 adds captured-result application-argument validation, excluding the
Mono/assembly prefix. It requires the original absolute content path as the last
argument and exactly one absolute --config path, accepting the existing separated,
equals and colon forms. It shares the capture option allowlist, so state/movie/
Lua options, extra positionals and unsupported options fail. Argument count/text
are bounded and control characters rejected. The returned configuration path is
not evidence of its contents; the eventual handoff must fingerprint/read/check
that file and retain ownership through spawn. No ordinary launch call site is
enabled, and no tests, builds or captures ran.

Step 456 adds CartridgeHandoffRequest and PreparedCartridgeHandoff. Preparation
consumes the capture result, checks the requested invocation/CLI, fingerprints
the selected configuration file, compares its read bytes with that fingerprint,
and runs cartridge/deck/startup/projected-config validation. The returned owner
retains capture evidence, configuration fingerprint and exact application args.
Its final-request check rejects changed args/invocation/deck/content and rechecks
retained files; arguments are exposed read-only after verification. The owner
does not keep a caller-created temporary config alive by itself, prove loaded
core/build policy or launch an emulator. Ordinary launch wiring remains pending.
No tests, builds, configuration-read routines or captures were executed.

Step 457 makes PreparedCartridgeHandoff consume and retain the launch
PreparedConfig itself. The owner path must exactly match the selected config
argument before its contents are accepted. Rechecking the handoff invokes the
owner's source/config-runtime verification as well as the retained generated
file fingerprint. This closes the temporary-file lifetime gap noted in Step 456;
dropping the returned owner performs the existing private-config cleanup, without
editing the original source config. Normal launch wiring and loaded-core/build
policy remain pending. No tests, builds, reads through this API or captures ran.

Step 458 adds captured-handoff ownership to CalibratedLaunch using an internal
enum alongside the existing PreparedConfig variant. The same launch lifetime
now has a constructor for either owner, and check_launch_inputs dispatches to
the appropriate config/handoff verification while retaining topology monitoring.
The captured constructor checks launch inputs before returning. No native adapter
calls it yet: final-plan comparison, actual capture invocation and supported
core/build policy must be connected before enabling the mode. Existing native
constructors remain on the configuration-only path. No tests, builds, checks
through these runtime APIs or captures ran.

Step 459 splits native digital session preparation from final argument commit.
PreparedDigitalSession retains the generated config, topology, description,
arguments and deck. Existing prepare_session delegates to its config-only finish;
finish_with_capture uses the prepared config and final application arguments to
build a checked cartridge handoff and the captured CalibratedLaunch owner. The
Mono assembly prefix must match before being excluded from application parsing.
Both paths commit plan arguments only after successful session checks. The new
path consumes an already captured observation; no automatic capture invocation
or runtime/build-policy acceptance is enabled. No tests, builds or runtime
preparation functions were executed.

Step 460 adds PreparedDigitalSession::capture_and_finish with an explicit
SessionCaptureRequest. It preflights topology/source, the generated config's CLI
selection and startup policy, then prepares/runs the owned definition capture
using that config. Topology/source/cancellation are checked again before the
existing captured finish path. Shared assembly-prefix slicing avoids separate
capture and handoff interpretations. Disc handoff is rejected before spawn until
its independent provenance exists. This operation is not selected by default
launch or previews; supported runtime/build policy still needs caller integration.
No code path added here was executed, and no tests/builds/captures ran.
