# Display shaders and bezels

Open a game's **Settings & mappings → Display** to change its presentation.
Use broader defaults for your usual setup and a game override for exceptions.
Changes apply on the next launch.

## Choose the look

A shader changes the game image: for example, a handheld LCD pixel grid or
scanlines and CRT curvature.
A bezel is artwork around that image. You can use either one without the other.

RetroArch display profiles can use CRT or LCD presets and artwork from The Bezel
Project, Orionsangel, and Duimon. Available choices depend on the system,
game, and installed artwork. Choose a look under **Display shader** in game
details, Couch Mode, or a launch profile. Existing choices stay unchanged.

## Handheld LCD screens

Choose the LCD preset named for your handheld: **Game Boy (green)**,
**Game Boy Pocket (gray)**, **Game Boy Color**, **Game Boy Advance**,
**Nintendo DS**, **Nintendo 3DS**, or **PSP**. For other color handhelds,
start with **LCD grid · general handheld**.

These use [Libretro's handheld shaders](https://github.com/libretro/slang-shaders/tree/master/handheld),
not a curved TV screen. The GBC, GBA, DS and PSP choices include system-specific
color correction; use the general grid if your emulator core already corrects
colors and you don't want to apply that effect twice. Game Boy and 3DS presets
also simulate LCD response; use the general grid if you prefer no ghosting.

Shaders do not choose your DS/3DS screen layout or add a handheld border.
Set the screen layout in RetroArch's core options, and choose bezel artwork
separately. Use **Whole system** to keep your LCD choice for that platform,
or **This game** for an individual override. These presets require RetroArch;
standalone emulators keep their own display settings.

## CRT screens and reflections

**RetroTube TV** reflects the picture onto the dark inner lip beside the screen,
not over the printed outer artwork. Both the dark bezel beside a full-screen
picture and narrow black padding can receive this light without cropping or
enlarging the game. This works with
external artwork as well as Koko's built-in TV frame. The artwork
keeps its shape, unused sidebars stay black, and the game retains its curved
screen opening. The effect applies on the next game launch.

The inner surface uses a subdued, softly blurred reflection with darker corners
and fine surface roughness, following Koko's TV treatment. RetroArch's shader
parameters let you adjust reflection strength, sharpness, and roughness.

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
