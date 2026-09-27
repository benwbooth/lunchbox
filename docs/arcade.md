# Arcade games

Arcade game files must match the emulator's expected ROM set. A correct title
is not enough: revisions, parent sets, BIOS files, and CHDs may all matter.

## Choose the right set

For MAME or FinalBurn Neo, use a set compatible with the selected emulator.
Keep the archive's standard set name and contents intact.

A self-contained set is convenient because it includes the required game ROMs.
Lunchbox prefers non-merged MAME sets and hides merged sets from its download
choices. This does not remove any arcade games you have already installed.
Split sets may need a parent archive nearby. Disc-based arcade games may also
need CHDs and the expected folder layout.

Lunchbox's download review tries to include the files needed for the chosen
game. Check the review rather than downloading a single matching filename
from an unrelated collection.

Lunchbox checks imported arcade ROM names against the MAME catalog to avoid
mixing up video games and identically named pinball or gambling machines.
For an installed game, **Other download options** lets you review another
set without deleting the current files. Arcade artwork uses the exact MAME
release names too, so player-count and revision labels do not hide matching art.

## Choose the right hardware

“Arcade” covers several different machines. Neo Geo, Naomi, Atomiswave, and
laserdisc games do not all use the same emulator setup.

Metal Slug 6 uses Atomiswave hardware, not Neo Geo. Use a compatible
Atomiswave setup and its required firmware; changing Neo Geo settings will
not fix an Atomiswave launch.

Laserdisc and pinball games need their companion media. A ROM alone is often
not enough.

## Six-button and N64-style pads

For a conventional six-button fighting-game layout, buttons 1–3 form the top
row and 4–6 form the bottom row, left to right. The selected emulator's game
mapping determines what those numbered buttons do.

Lunchbox's N64-style arcade layout uses:

| Arcade row | Left | Middle | Right |
| --- | --- | --- | --- |
| 1–3 | B | C-left | C-up |
| 4–6 | A | C-down | C-right |

Review the preview for the selected game. For supported single-stick targets,
both a source D-pad and left analog stick can control the arcade stick.

## Game-specific bezels

Choose a game bezel in **Settings & mappings → Display** when one is available.
Artwork keeps its original proportions on ultrawide displays; black areas
are preferable to stretched artwork.

See [Display and bezels](display.md) if the game is cropped or does not fit
the opening.

## Adult-content filtering

The **Adult** filter also recognizes arcade games whose names and genres do
not mention adult content, such as *Excelsior*. Lunchbox uses
[progetto-SNAPS' Mature list](https://www.progettosnaps.net/catver/) and matches
its ROM-set names to game titles and revisions. It does not treat every
mahjong or puzzle game as adult, or change a game's official age rating.

The small classification list downloads automatically and is cached for
offline use. A first run without internet access has only the catalog's
existing ratings and keyword checks. Like any community list, it can have
gaps; the filter is not a parental-control guarantee.

## Metal Slug blood settings

Supported Neo Geo releases of Metal Slug, 2, X, 3, 4, and 5 expose
**Settings & mappings → Arcade settings → Blood**.

- **Use game setting** leaves the game's current choice alone.
- **Red blood** turns the supported native option on.
- **Censored blood** turns it off.

The setting is saved per game and applies on the next launch. The current
automatic implementation uses RetroArch MAME with an MVS arcade BIOS.
It also reapplies the choice after a supported state restore.

AES console mode, other cores, console ports, and Metal Slug 6 are not covered
by that Neo Geo setting. Choosing “Use game setting” does not reset an old
save state's options to factory defaults.

## Resume problems

If the game briefly restores and then resets, note the exact emulator/core and
whether it was loading an automatic state. Try a fresh start without deleting
the old state. In-game saves and NVRAM are separate from emulator states.

Use [Save locations](saves.md) to keep a recovery copy before changing cores
or clearing any arcade data.
