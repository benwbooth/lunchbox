# Lunchbox

Lunchbox is a cross-platform, acquisition-first game browser, collection manager, and emulator frontend. The application will use Rust with CXX-Qt and QML. This branch begins with the canonical data layer shared by the desktop and couch interfaces.

The previous Electron application is preserved in the `legacy/electron` branch.
The new native desktop shell is now runnable and is described in [the frontend
architecture](docs/FRONTEND_ARCHITECTURE.md).
The ongoing vertical-slice plan is tracked in [the native frontend parity
roadmap](docs/FRONTEND_ROADMAP.md).

## Native Qt frontend

Run the Rust/CXX-Qt application against the current development database:

```console
nix develop
cargo run -p lunchbox-app -- --database build/lunchbox.db
```

The window paints before catalog work begins. SQLite loading and collection
filtering run on worker threads, while virtualized Qt views consume a native
`QAbstractListModel`. Use `--startup-probe` to measure shell construction without
waiting for database loading.

The platform sidebar searches names and familiar aliases such as NES, SNES,
PS2, and MAME while showing the live match count. Drag its right edge to resize
it between 180 and 400 pixels, or double-click the edge to restore the default.
The query and width survive restart in the native state database; filtering is
in memory and persistence is coalesced off the Qt thread.

The header's Couch Mode control opens a fullscreen, keyboard-and-gamepad
living-room presentation over those same models. It preserves the exact
selected game, exposes All Games, My Collection, Minerva, Favorites, and Recent
views, plus an exact 191-platform picker with live game counts. The current
shelf and exact platform survive restart. Keyboard, controller, and pointer
input share the same picker behavior, and Play, favorite changes, and download
options route through the existing native services. Couch-native Details and
Game Menu overlays keep
browsing on the television while exposing full metadata, activity, launch and
download state, favorite changes, and an explicit handoff to the complete
desktop tools without losing the exact game. Attract Mode can be started from
Game Menu or automatically after a configurable idle delay; it rotates through
the current shelf rather than a second catalog, retains exact game identity,
supports immediate directional browsing, and returns to browsing with Back.
Its enable state, idle delay, and game interval are stored in the native state
database and edited from Settings. Settings also includes a native theme manager
with three built-in looks plus safe `.lunchbox-theme` packages. Themes can alter
the complete Couch Mode palette, fallback artwork, optional background image,
hero contrast, and card radius; the exact selection survives restart. Packages
are declarative and cannot contain QML, scripts, commands, fonts, plugins, or
other executable content. The complete authoring and safety contract is in
[Couch Mode theme packages](docs/COUCH_MODE_THEMES.md). A native Rust input worker handles
hotplugging, analog dead zones, deliberate hold-repeat, paging, and Xbox,
PlayStation, Nintendo, or generic button labels without blocking Qt. Escape or
the controller's east face button returns to desktop mode.

Settings can install or refresh the complete official RetroArch Slang and GLSL
shader collections. The Rust service discovers native RetroArch locations on
Linux, macOS, and Windows, verifies a bounded cache, validates every archive
entry, stages both packs, and publishes them transactionally. It replaces only
the official `shaders_slang` and `shaders_glsl` directories after explicit
confirmation when they were not previously managed by Lunchbox; custom sibling
folders are never touched.

To exercise the preserved acquisition-first catalog, layer the local legacy
game catalog, Minerva bundle index, and optional user state over the canonical
database:

```console
cargo run --release -p lunchbox-app -- \
  --database build/lunchbox.db \
  --games-database lunchbox-games.db \
  --minerva-database minerva.db \
  --user-database ~/.local/share/lunchbox/user.db
```

The `Minerva` view contains only games whose platform has an exact Minerva
bundle mapping and which are not already present in the user database. Selecting
a platform keeps that availability filter active. The native Filters menu makes
the same distinction explicit alongside Installed, Hide Non-Retail, and Hide
Adult controls. Content classifications follow the bounded legacy rules, and
the content-filter preferences survive restart in the writable state database.
Select a game to open its
native details pane; Minerva sources can be inspected against the real torrent
contents, where title matches are presented as review candidates rather than
silently accepted identities. Exact alternate names tied to the same positive
LaunchBox database ID appear as regional metadata chips and expand member
matching; the review row discloses the alias that produced a match. They never
merge catalog records or replace the game's stable UUID. Regional, language,
prototype, and revision releases on the same exact platform appear as a native
horizontal carousel. It is ordered by the saved region/version preferences and
switches by the target row's exact UUID; duplicate full titles collapse only
for presentation and never transfer IDs or metadata. Installed,
Minerva-covered, catalog-only, and current states remain distinct. A persisted,
fully ordered region list plus the latest/original release-version policy ranks
otherwise equal candidates; the
picker still exposes the exact region, revision, filename, size, and match
quality before download. Region aliases and duplicate entries are normalized
before the ordering is saved. A reviewed file
can be sent to qBittorrent from that pane. Valid torrent metadata is cached in
the operating system cache directory for 24 hours, and the active torrent's
decoded file index is shared between source views. Multi-disc optical matches
collapse into one reviewable set that lists every required disc and CUE/CCD/MDS/GDI
companion before download, preserves the source layout, and creates an M3U only
after the full set imports safely. eXoDOS, eXoWin3x, and eXoWin9x matches likewise
collapse into one reviewed archive set containing the exact game, required
metadata, same-name game data, and collection utilities. DOS language variants
follow the legacy default/English/German/Polish/Spanish order. Shared dependencies
are staged once in a per-torrent hidden ROM cache, while only the exact primary
game archive becomes the installed collection record. The native Downloads drawer
persists progress and exposes pause, resume, and non-destructive cancel controls.
The drawer reports aggregate and per-job transfer rates. Failed jobs remain
visible with an exact-job Retry action and a newest-first durable event history.
Recovery verifies that the exact info hash is still owned by Lunchbox's isolated
qBittorrent category, requests a data recheck, and resumes the existing torrent
without changing reviewed file priorities, source provenance, or native/container
path mappings. If that torrent no longer exists, Lunchbox never guesses or
recreates metadata: the job stays failed and directs the user back to Game
Details to review and queue a source again. Finished records can be removed
individually or cleared in bulk after confirmation without deleting active work,
torrents, or downloaded files. Settings also provide a persisted
post-import seeding choice: follow qBittorrent's existing rules, or pause the
Lunchbox-owned torrent after every selected file in that bundle has imported.
The pause request is non-destructive, survives restart when deferred, and
retries automatically when qBittorrent is temporarily unavailable.

The source architecture deliberately separates catalog identity from
acquisition offers. Minerva remains the reviewed built-in provider. From an
exact game's details pane, **Add External Torrent Source** now accepts a lawful
user-supplied `.torrent`, parses at most 16 MiB off the GUI thread, rejects
unsafe or cross-platform-incompatible paths, and presents every file, its size,
the v1 info hash, and the current exact-file/whole-torrent policy before Queue
is enabled. The catalog UUID, title, and platform remain locked throughout the
review; torrent labels cannot establish or replace identity. Queueing reuses the
same qBittorrent, progress, import, and collection pipeline and durably records
the source-file name, torrent SHA-256, info hash, reviewed file, exact catalog
association, and resulting job. Magnet intake, watched folders, and authorized
provider adapters remain planned rather than implied by this manual path. See
[external torrent providers](docs/EXTERNAL_TORRENT_PROVIDERS.md).

The details header also opens a native metadata editor for display title,
description, release date, developer, publisher, genre, players, rating, age
rating, release type, personal notes, and per-game tags. Edits are durable per
stable game UUID in the writable profile database and immediately update desktop
search, sorting, collections, details, and Couch Mode. Tags appear as clickable
chips, compose with the other library filters, and can drive exact smart-
collection rules. Metadata and tag changes publish in one SQLite transaction;
tag spelling is normalized and case-insensitive duplicates are rejected. The
canonical title, platform, provider links, ROM identity, and file records remain
read-only and continue to drive Minerva matching, artwork lookup, downloads,
activity, and launching. Restoring catalog values removes only the metadata
overlay and keeps the user's tags.

The Minerva Laserdisc Collection is handled as machine media instead of a
single guessed file. Exact-title records from the pinned canonical LibRetro
MAME data provide reviewed set-name evidence. The MAME source view selects the
matching ROM ZIP and CHD together; Hypseus and Daphne source views select a ROM,
framefile, data, video, audio, and any compatible RAM files. Incomplete bundles
are not offered. The review dialog exposes every exact torrent path and role,
qBittorrent selects only those members, and ingestion materializes the complete
MAME or Hypseus-compatible launch layout. Leave-in-place mode keeps the payload
qBittorrent-owned and links that layout without copying multi-gigabyte media.

Installed eXoDOS, eXoWin3x, and eXoWin9x archives expose a native `PC Install`
card in game details. Preparation runs off the GUI thread, resolves the exact
matching metadata/game-data/utility archives, safely extracts title files and
shared MT-32/DOSBox/86Box/PCBox assets, and atomically publishes a versioned
cache keyed by the stable game ID and source signature. It can be cancelled
without leaving a published partial tree, and unchanged sources are reused on
retry or restart. Prepared installs now detect compatible standalone emulators
off the GUI thread from the canonical host/package catalog and expose a real
`Play` action only when one is installed. Native executable, macOS app-bundle,
Windows install-directory, and Linux Flatpak discovery are path-aware and do
not invoke a shell. eXo exception metadata selects DOSBox or ScummVM, while
Win9x launchers route to DOSBox-X, 86Box, or a PCBox-compatible profile. VM
parent disks produce persistent writable children without hard-linking them.
Linux Flatpak detection and process launch have been exercised end to end;
native Windows and macOS launch branches are covered structurally but still
require their platform release gates.

Imported local ROMs expose a native `Play Locally` card. Emulator discovery
runs off the GUI thread and combines exact canonical platform/alias mappings
with installed standalone executables and RetroArch cores. The user can choose
among every detected option, persist an exact stable-ID/core default for one
game or an entire platform, reset either override, and choose among multiple
local files without losing native path bytes. Launches use direct argument
vectors; Flatpak access is limited to the ROM directory. The unattended Linux
gate has loaded a real NES ROM through the installed RetroArch Flatpak and the
exact `fceumm` core. Arcade, laserdisc, and pinball files deliberately require
dedicated profiles instead of being passed to a generic emulator command.
Standalone MAME ZIP/7z sets now retain their archive and launch by exact set
name with the collection and runtime ROM directories; Hypseus framefiles
validate their `vldp`/`singe` bundle, ROM directory, and support assets before
building its native laserdisc argument vector. Known archive-oriented CLI
emulators (FinalBurn Neo, Flycast, and Supermodel) retain the legacy direct-file
contract. The MAME Flatpak path has been runtime-verified with MAME's ROM-free
Pong driver. A dedicated Daphne runtime profile and TeknoParrot remain explicit
follow-on work; Daphne-source media already normalizes to the validated
Hypseus-compatible layout.

Each detected runtime also exposes a native launch-command editor. Exact game,
platform, and all-platform profiles inherit independently for extra arguments
and replacement templates, and apply to ordinary ROMs as well as prepared eXo
PC games. Built-in templates remain visible, saves are validated, and
placeholders become direct native argument values with the existing Flatpak or
Wine path mapping. Quotes only group arguments: Lunchbox never passes a profile
through a shell or evaluates substitutions, pipes, globs, or redirection.
Settings also opens a native Launch Commands manager over the complete catalog.
Its virtualized, searchable matrix keeps standalone and exact RetroArch-core
runtimes separate, filters all-platform/per-platform and built-in/customized
rows, previews inherited behavior, and edits one stable-emulator-ID profile in
a focused side panel without blocking the GUI.

The native Emulator Manager restores the legacy host-aware lifecycle instead
of delegating it to Electron. It discovers installations off the GUI thread,
chooses one deterministic source per emulator, and installs, updates, or
removes only exact package identities. Linux supports user Flatpaks, a
Lunchbox-only Nix profile, self-updating AppImages with reviewed GitHub-release
fallbacks, GitHub archives, and reviewed official downloads such as Altirra
under an isolated Wine prefix. Windows uses winget and macOS uses Homebrew;
Snap is deliberately unsupported. Lunchbox records ownership receipts and
never removes an externally managed runtime. Exact RetroArch core slugs from
the canonical emulator/platform catalog are installed from the host-specific
Libretro buildbot ZIP into the appropriate native or Flatpak core directory,
with atomic replacement and ownership-safe removal. Managed native,
AppImage/GitHub, Nix-profile, Flatpak, and Wine installations feed the same
launch discovery used by local ROMs and prepared PC games.

Emulator updates are a separate explicit review step, matching the useful part
of the legacy manager without automatic background mutation. Lunchbox checks
every installed backend instead of only the manager's preferred source, lists
only proven available updates with current and available versions, selects
rows for review, and applies the chosen updates sequentially with durable
per-row progress and errors. Linux checks user/system Flatpaks, the isolated
Nix profile, and managed GitHub/AppImage releases; Windows queries exact winget
IDs; macOS consumes Homebrew's structured outdated inventory. A failed source
check is reported as a warning and never converted into a false update.

Firmware is resolved for the selected runtime and core, rather than from one
global folder per platform. The canonical catalog carries 118 reviewed rules
and 17 exact sources recovered from the legacy Lunchbox design, including
required and optional packages, HLE fallbacks, launch-scoped MAME files, and
manual-only dumps. The game-details card can download exact Minerva packages,
official HTTPS files, and reviewed GitHub source archives; it can also import an
exact local package. Imports are traversal-safe and bounded, retain archive and
per-file SHA-256 provenance in the local state database, and activate through a
staged package store. Sync and repair verify the complete manifest before
copying to native RetroArch, DuckStation, PCSX2, Dolphin, Flycast, openMSX,
MAME, 86Box, PCem, and other reviewed runtime layouts on Linux, Windows, and
macOS. Manual-only firmware opens a dedicated native folder with instructions;
Lunchbox never bundles firmware or silently substitutes a different package.

Configure qBittorrent and both sides of its filesystem mapping in the native
Settings dialog. Native paths are chosen with the operating system folder
picker; qBittorrent/container paths are deliberately retained as verbatim
client strings. Passwords use the operating system credential store and never
the SQLite state database. Empty credentials remain valid for qBittorrent
instances whose Web UI authentication bypass is intentionally enabled.

On completion, Lunchbox verifies and materializes the reviewed file using the
selected symbolic-link, hard-link, reflink, copy, or leave-in-place policy. It
never overwrites different existing content, and only records the exact game as
installed after the destination exists. That installed state immediately keeps
the game out of the active Minerva/not-installed view; the catalog refreshes
and reapplies the current search and platform filters after ingestion.

`Import ROMs` opens the native local-collection workflow. It recursively scans
supported files off the GUI thread, streams progress, can be cancelled during
large-file hashing, and presents a searchable, sortable review table. Unique
SHA-1/MD5 matches are selected by default; filenames never establish identity.
Named scan profiles make repeat imports one action: each durably restores its
lossless native root, exact platform hint, normalized extension scope, and
checksum policy. Editing a loaded scope clearly switches to a one-time scan;
removing the profile never removes ROMs or library records. Extension-scoped
rescans affect missing status only inside that scope, so scanning `.nes` cannot
make an existing `.sfc` collection disappear.
**Scan all** prepares the checksum catalog once, visits every saved profile
sequentially, supports cancellation between or during collections, and keeps a
compact durable history instead of retaining every review row in memory. The
history records the profile snapshot, exact scope, batch position, outcome,
counts, errors, and cached/read work for up to 200 runs. A historical profile
can be loaded for an ordinary detailed rescan and review; deleting the profile
does not erase its audit trail.
Completed regular-file and archive-member hashes are retained in per-user state,
including results completed before cancellation. A later scan reuses them only
when the lossless native path, stable filesystem identity, size, and operating-
system change metadata still agree; new, replaced, or changed files are read
again. The review surface reports cached versus read containers, and a complete
scan prunes records for paths no longer present without writing unchanged hits.
For ZIP, 7z, or RAR archives containing exactly one recognized ROM, the scanner
streams that member without extracting it and hashes the emulated bytes, while
retaining the original archive as the local launch path. The review row shows the
container and member separately. Archives with multiple recognized ROMs expose one
checksum-identified row per safe member but select none automatically, because an
arcade set or another multi-file game must not be split by guesswork. Explicitly
selected members are revalidated, materialized atomically into a content-addressed
owned cache, and recorded with the exact source archive and member provenance.
Users can also include ambiguous or unmatched files, which remain visible as
honest local-only entries in `My Collection` without suppressing an unrelated
Minerva game. Every readable review row also offers `Match…`: it searches
canonical and alternate catalog titles, optionally constrained to one platform,
but never accepts a suggestion until the user explicitly chooses `Link ROM`.
That reviewed identity is restored on later scans only when the lossless source
path, archive member, byte size, MD5, and SHA-1 all still agree. Imports and
rescans are idempotent, retain missing-path history, and store lossless OS-native
path bytes alongside display paths.
RAR support is pure Rust; encrypted and multi-volume archives are rejected with
an actionable review error rather than invoking a host utility or guessing.

`Library Audit` rescans every configured root on a worker with one shared
catalog match index, then separates exact healthy files, changed content,
duplicate SHA-1 content, new untracked ROMs, missing records, unreadable files,
and offline roots. A disconnected root is never converted into missing records.
The review is searchable, sortable, and virtualized; Lunchbox never merges,
relinks, or deletes content automatically. The only cleanup action requires an
explicit selection and confirmation, and removes only already-missing SQLite
records—not ROMs, archives, saves, downloads, or emulator data.

The equivalent catalog environment variables are
`LUNCHBOX_GAMES_DATABASE`, `LUNCHBOX_MINERVA_DATABASE`, and
`LUNCHBOX_USER_DATABASE`. `--state-database` or
`LUNCHBOX_STATE_DATABASE` can select an alternate writable settings, queue, and
native collection database for testing or portable deployments.

Favorites and collections live in that writable state database and refer to
games by Lunchbox's stable UUIDs. Manual shelves preserve explicit order and
support per-game or confirmed bulk visible-result membership changes. Smart
shelves compose title, exact platform, availability, favorite, completion, and
content rules and stay current as source state changes. The Collections sidebar
and two-pane manager provide navigation, editing, reorder controls, and bounded
portable JSON import/export. Import uses only exact Lunchbox UUIDs or unique
positive LaunchBox database IDs, never display-title matching. Collection views
remain composable with search and platform navigation, and writes run off the
GUI thread with visible failure rollback.

Play activity is stored by the same stable game UUID. Lunchbox starts a durable
session only after an emulator process exists, records play count and last-played
immediately, then adds measured elapsed time exactly once when that process
exits. The details pane shows plays, time, recency, and editable completion state;
its bounded session-history dialog shows newest-first launch timestamps, exact
emulator, duration, and completed/interrupted/failed/in-progress outcome filters
without loading an unbounded model on the Qt thread. Recently Played stays
composable with search and platform navigation. Unfinished rows remain auditable
after interruption instead of guessing play time.

Cached gameplay videos and manuals now appear in a native `Game Media` card.
Rust accepts only canonicalized, non-empty files inside the configured media
root, chooses the preferred provider deterministically, and gives each video a
bounded SHA-256 content identity from its length plus head/tail samples so a
replacement does not normally inherit stale progress without hashing an entire
large video. Qt Multimedia auto-plays the clip muted and looped with native
seek, sound, restart, and fullscreen controls; playback pauses before a game
takes over or when details close. Five-second checkpoints are written off the
GUI thread by stable game UUID and resume after restart. Manuals open with Qt's
operating-system URL handler, without shell commands or platform-specific path
strings.

Artwork indexing keeps every valid cached candidate instead of discarding all
but the preferred provider. The details hero exposes previous/next controls and
the current source while preserving the legacy provider and media-type fallback
order. The complete provider order is editable and durable in Settings; saving
it immediately rebuilds the in-memory artwork index off the GUI thread and also
governs cached videos and manuals. Its refresh action bypasses the seven-day
miss cache, validates a fresh LibRetro PNG, and replaces only Lunchbox's owned
LibRetro file through a synced temporary file. Other providers remain available,
and a miss or transfer error restores the existing image.

The sidebar's native **Media Audit** opens a virtualized, cancellable coverage
review for either My Collection or the complete catalog. It checks one exact
artwork category at a time, clearly separates exact LibRetro repairs from games
that need per-provider review, and never treats a fallback image as completion.
Users select reviewed rows before a confirmed, two-worker background repair;
each response is size/signature validated and atomically published into only
LibRetro's owned cache path. Provider misses and transfer failures stay visible
for retry or per-game Find Artwork review.

The unified native **Find Artwork** flow supports SteamGridDB and IGDB. Provider
credentials live only in the operating-system credential store. Search results
are always review candidates: the user chooses the exact provider game before
Lunchbox persists its stable provider ID, then chooses one individual artwork
file. SteamGridDB supplies backgrounds, covers, and logos; IGDB supplies covers,
artwork, and screenshots through Twitch client-credentials authentication. Both
use the same bounded PNG/JPEG/WebP signature validation and atomic provider-owned
cache publisher, so replacing one source and media kind cannot delete another
provider's media. IGDB access is suitable for non-commercial use unless the
distributor has arranged the required commercial partnership.

The details hero can switch from flat artwork to an interactive native 3D box
whenever an exact cached front cover exists. Qt Quick 3D renders separate front,
back, spine, and edge faces using the real cached files; a missing back cover
uses the front without inventing another asset. The box rotates automatically,
supports pointer drag and wheel zoom, and exposes pause and reset controls.

The Minerva manual action searches the dedicated manual-scan torrent for a
high-confidence exact title, asks qBittorrent for only that reviewed ZIP member,
persists byte progress across restarts, and supports cancellation without
disrupting other manuals selected from the same torrent. A completed non-empty
ZIP is validated and copied through a same-directory temporary file before an
atomic rename into the owned media cache. Availability is intentionally limited
to exact entries in Minerva's current archive; a broad fuzzy match is never
silently downloaded. EmuMovies fallback remains gated on documented developer
API access rather than embedding the legacy application's retired credentials
or endpoints.

The preserved game catalog is a local runtime input and is not added to the
published artifact because redistribution permission for its LaunchBox-derived
records has not been established.

## Database principles

- A game, a platform release, an exact ROM/disc artifact, and a downloadable offer are different entities.
- Provider evidence packs are distinct from the canonical catalog; importing a DAT does not declare
  every dump to be a separate canonical game.
- Lunchbox owns stable internal IDs; provider IDs are versioned links, never primary keys.
- Exact hashes and explicitly reviewed provider links may establish identity. Fuzzy title matches may only create review candidates.
- Raw provider assertions retain their provenance and do not overwrite user choices.
- Ambiguous legacy records remain quarantined until they can be resolved safely.

See [the metadata-backbone decision](docs/METADATA_BACKBONE.md), [the database
architecture](docs/DATABASE_ARCHITECTURE.md), [the pinned Libretro source
policy](docs/LIBRETRO_SOURCE.md), [Minerva and local collection
import](docs/MINERVA_AND_LOCAL_IMPORT.md), and [the legacy
audit](docs/LEGACY_DATABASE_AUDIT.md).

## Reproducible workflow

Enter the development environment:

```console
nix develop
```

Prepare the exact pinned Libretro snapshot. This downloads into an ignored local cache, verifies
the archive size and SHA-256, extracts only `rdb/*.rdb`, and verifies the complete extracted tree:

```console
cargo run -p lunchbox-db -- prepare-libretro \
  --manifest sources/libretro.json \
  --cache source-cache
```

Build the canonical database from the maintained emulator catalog and verified Libretro RDBs:

```console
cargo run -p lunchbox-db -- build \
  --database build/lunchbox.db \
  --emulators emulator_details/all_emulators.csv \
  --libretro-manifest sources/libretro.json \
  --libretro-rdb-dir source-cache/libretro-database-4de37d2c8042d9d0f608c43006c9b35438c536ce/rdb
```

Audit it strictly:

```console
cargo run -p lunchbox-db -- audit --database build/lunchbox.db --strict
```

Create and verify a maximal-compression 7z artifact. The command fails if the archive is 100,000,000 bytes or larger:

```console
cargo run -p lunchbox-db -- compress \
  --database build/lunchbox.db \
  --output artifacts/lunchbox.db.7z
```

Audit preserved local databases without modifying them:

```console
cargo run -p lunchbox-db -- inspect-existing \
  --legacy-catalog lunchbox-games.db \
  --openvgdb openvgdb.sqlite \
  --emulators db/emulators.db \
  --minerva db/minerva.db \
  --output reports/existing-databases.json
```

Import Minerva bundle offers into a writable user database:

```console
cargo run -p lunchbox-db -- import-minerva \
  --database /path/to/user/lunchbox.db \
  --catalog /path/to/minerva.db
```

Hash and import a local collection. `--platform` may be omitted when strong Libretro evidence
resolves each file to exactly one platform:

```console
cargo run -p lunchbox-db -- scan-local \
  --database /path/to/user/lunchbox.db \
  --root /path/to/roms/game-boy \
  --platform "Nintendo Game Boy" \
  --extensions gb,gbc
```

All commands are implemented directly in Rust and are safe to rerun.
