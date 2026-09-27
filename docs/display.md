# Display, CRT effects, and bezels

Open a game's **Settings & mappings → Display** to change its presentation.
Use broader defaults for your usual setup and a game override for exceptions.
Changes apply on the next launch.

## Choose the look

A shader changes the game image: for example, scanlines and CRT curvature.
A bezel is artwork around that image. You can use either one without the other.

RetroArch display profiles can use CRT presets and artwork from The Bezel
Project, Orionsangel, and Duimon. Available choices depend on the system,
game, and installed artwork.

**RetroTube TV** also lights the bezel with the colors from the game. This
works with external artwork as well as Koko's built-in TV frame. The artwork
keeps its shape, unused sidebars stay black, and the game retains its curved
screen opening. The effect applies on the next game launch.

If a shader pack is missing, use the shader controls in Settings to install
or refresh it. Installing a shader collection does not select it for every game.

## Which bezel should I use?

Prefer a game-specific bezel for arcade games when one is available. A system
bezel is a useful fallback. Use the artwork selector when the game or system
has several choices.

Duimon's 21:9 options are intended for ultrawide displays. They are optional
artwork, not a requirement to install the entire Mega Bezel shader setup.

## Ultrawide monitors

Bezel images keep their original aspect ratio. A 16:9 bezel on a wider display
should leave black areas rather than stretch. Native ultrawide artwork can
fill more of the screen.

The game should fit the opening in the bezel. Some ultrawide profiles need
fullscreen; Lunchpail reports when a selected profile cannot be used in the
current window mode.

## Cut-off text or a squashed picture

Try one change at a time:

1. Turn off the bezel and check whether the whole game image is visible.
2. Turn off the shader and compare again.
3. Check for a per-game display override or a custom RetroArch viewport.
4. Relaunch using the default aspect ratio for the system.

If only the shader causes cropping, changing bezel artwork is unlikely to fix
it. If the image is correct without the overlay, check the bezel selection and
viewport instead.

When reporting a problem, include the game, core, shader, bezel, screen
resolution, and whether fullscreen is enabled. A screenshot of the full window
is much more useful than a cropped game image.

## Artwork credits

Bezel artwork comes from its respective creators, including
[The Bezel Project](https://github.com/thebezelproject) and
[Duimon's 21:9 collection](https://github.com/Duimon/Duimon-Mega-Bezel-Potato-21x9).
Duimon's collection is licensed CC BY-NC-ND 4.0. Lunchpail fetches those images
on demand rather than bundling or modifying them. Check each pack's license
before sharing its artwork outside Lunchpail.
