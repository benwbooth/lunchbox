# Translations, mods, and cheats

A translation patch changes a game file. Live AI translation reads the screen
while you play. If a suitable community translation exists, start by checking
its requirements; it does not need a model running during gameplay.

Lunchpail creates patched copies and leaves your original ROM or disc image alone.

## Find a patch in Lunchpail

1. Open **Game details → Translations & mods**.
2. Search for a translation or mod. Try the original-language title if needed.
3. Read the result's description, required region, revision, and checksum.
4. Download the package and choose the intended patch variant.
5. Use **Apply & enable**, or **Import disabled** to keep it for later.

Available sources include:

- **RHDN community archive (2019, partial)** — a historical selection, not
  a complete or current Romhacking.net mirror.
- **Romhack Plaza** — requires an API key with read/download permissions.
- **GitHub inspected patch releases** — releases containing recognizable
  patch files, not arbitrary repositories or executable patchers.

A result containing a valid patch is not proof that it matches your ROM.
Romhack Plaza and historical RHDN archives are separate catalogs; neither
should be assumed to contain every patch.

## Import a patch you already have

Open **Settings & mappings → Patches & cheats** for the game and import the
patch file. For a manual ZIP or 7z download, extract the patch first.

Supported formats are IPS, IPS32, BPS, UPS, PPF, and xdelta/VCDIFF.
xdelta patches need `xdelta3`; the Nix package includes it, while other
installations may need it installed separately.

For disc patches, use the author's specified raw ISO or BIN. A CUE sheet,
playlist, or compressed disc container is not a substitute for the requested input.

## More than one patch

Enable only the patches you want and arrange them in the author's required
order. Two alternative translations usually should not be stacked.

Lunchpail checks available checksums and reports mismatches. It does not guess
whether to remove a ROM header or patch a different disc track. If validation
fails, check the base game rather than bypassing the warning.

Applied results are cached, so an unchanged patch setup does not need to be
rebuilt on every launch.

## What happens to saves?

Patched games use a separate derived filename. Existing saves or states may
not be compatible with the modification. Keep a backup before transferring any.

Removing a patch from the profile does not delete your original ROM, the
downloaded patch, or your saved games.

## Use cheats

In **Patches & cheats**, enter a code or import a RetroArch `.cht` file.
Imported codes start disabled. Enable both the game's cheat switch and the
individual codes you want, then relaunch.

Automatic cheat loading currently requires RetroArch. Standalone emulators
must use their own cheat interface. Code formats and compatibility depend on
the core and game revision.

Cheats can alter saves and affect achievement eligibility. Keep a normal save
backup before experimenting, and see [RetroAchievements](achievements.md)
if you use Hardcore mode.

