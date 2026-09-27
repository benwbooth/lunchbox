# Add and organize games

**My Collection** shows games you have added. **All games** also includes
catalog entries you may not have installed. A catalog entry is information
about a game, not a copy of the game itself.

## Import a folder

1. Open **Library → Import ROMs**.
2. Choose a folder and, if appropriate, a system.
3. Optionally limit the scan to file extensions such as `.nes` or `.sfc`.
4. Scan, review the results, and import the selected rows.

You can cancel a long scan. Completed checksum work is cached so later scans
do not need to reread every unchanged file.

A recognized checksum links the file to a known release. Unmatched files can
remain local-only entries. Use **Match…** to search for the correct catalog
release; a similar title alone does not prove it is the same ROM.

## Scan the same folders again

Save a scan profile for a folder you use regularly. A profile remembers the
folder, system, extension filter, and checksum settings.

**Scan all** checks your saved profiles. Optional scheduled checks tell you
when something changed; they do not silently import or delete games.
Removing a scan profile does not remove its ROMs.

## Archives and multi-disc games

ZIP, 7z, and RAR files can be inspected during import. Encrypted and multi-volume
archives are not supported by this workflow.

Be careful with archives containing several files:

- A collection of independent ROMs can be imported as separate games after review.
- An arcade ROM set is one game made from several required files. Keep the set together.
- Disc images may need their CUE, GDI, CCD, or other companion files. Keep the original layout.

Do not treat every file inside an arcade ZIP as a separate game.
See [Arcade games](arcade.md) for ROM-set requirements.

## Find the right game

Use the platform list, search, and Filters together. Search text is remembered
per platform. If a game seems to disappear, clear the search and check filters
such as Installed, Minerva, or hidden non-retail releases.

Grid view emphasizes artwork. List view lets you choose columns, sort records,
and filter exact values.

Title sorting and the A–Z shortcuts ignore leading **The**, **A**, and **An**:
**The Simpsons** appears under **S**, without changing its displayed name.
Set a game's **Sort title** in its metadata if you want a different placement.

**Homebrew / pirate releases** also covers ROM hacks, bootlegs, and unlicensed
releases. Choose **Exclude** to hide them or **Only** to find them. Lunchpail
combines the recorded release type with ROM metadata, including homebrew tags
and individually identified HBMAME releases. An official game stays visible
when only one of its versions is a hack. Prototypes and demos are not treated
as homebrew just because they were never sold.

The **Releases** section in Game details lets you switch between regional and
versioned entries. Each release keeps its own identity; selecting a Japanese
release does not rename or replace an installed US release.

## Favorites and collections

Two automatic collections are always available under **Collections** in the sidebar:

- **Favorites** holds the games you star. Click the star on any grid cover to add
  or remove a game, or press **F** with a game selected. A filled star means it is
  a favorite. Your choices are remembered when you reopen Lunchpail.
- **Recently Played** shows games you have launched, with the most recently played
  first. It updates as you play, regardless of your sorting in the rest of the library.

Both are also available in Couch Mode. Search and platform filters can narrow
either collection.

A manual collection contains games you pick; a smart collection follows rules
such as system, tags, completion, or availability.

Manage collections from the library menu. Collections can be reordered,
imported, and exported. Exporting a collection shares its membership, not ROMs.

## Edit game information

Use the edit control in Game details to change the displayed title,
description, notes, tags, and other editable fields. These are your local
changes; they do not change the ROM or its download identity.

**Review online metadata** lets you compare another provider's information
before accepting it. Existing values are not a reason to accept every suggested
replacement.

## Missing, duplicate, or moved files

Use **Library Audit** to review missing paths, changed files, and duplicates.
An offline drive is different from a deleted game: reconnect it before cleaning
up the library.

Removing a library entry and uninstalling a downloaded game are different
actions. Lunchpail does not delete imported or leave-in-place game files when
removing their library association.

## Where catalog information comes from

Lunchpail combines game and platform metadata with checksum information from
Libretro and optional connected providers. Coverage varies by system and release.
Artwork and descriptions can be missing or wrong even when a game launches correctly.

For source and license information, see [Data licenses](../DATA_LICENSES.md).
