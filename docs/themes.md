# Couch Mode themes

Themes change Couch Mode's colors, background, contrast, and rounded corners.
They do not change your games, controller mappings, or library layout.

## Install a theme

Open **Settings → Couch Mode → Appearance Themes** and choose a
`.lunchpail-theme` package.

Select the installed theme to use it. Installing an updated package with the
same theme ID replaces that theme. Built-in themes cannot be removed.

If an installed theme is missing or damaged, Lunchpail returns to its built-in
default. You can reinstall the package without rebuilding your library.

## Make a simple theme

A theme package is a ZIP file renamed to end in `.lunchpail-theme`.
Put `theme.json` at the archive's top level, not inside an extra folder.

This example needs no image:

~~~json
{
  "schema_version": 1,
  "id": "quiet-blue",
  "name": "Quiet Blue",
  "author": "Your name",
  "description": "Dark blue panels with warm highlights.",
  "palette": {
    "background": "#08111c",
    "panel": "#152334",
    "panel_raised": "#22364b",
    "ink": "#f4f7fb",
    "muted": "#a3b2c2",
    "accent": "#ffb454",
    "accent_cool": "#62d9d0",
    "danger": "#ff6f91"
  },
  "hero_scrim_percent": 62,
  "card_radius": 16
}
~~~

Use a unique lowercase ID containing letters, digits, and interior hyphens.
The ID must be 3–64 characters. Keep the name and author within 80 characters,
and the description within 240.

The palette uses `#RRGGBB` colors; `#AARRGGBB` also allows transparency.
Keep text clearly readable against both the panels and background artwork.

`hero_scrim_percent` controls the dark layer behind game information:
higher values make the background less distracting. It accepts 20–90.
`card_radius` accepts 4–32.

## Add a background

Include one PNG, JPEG, or WebP and add its relative path:

~~~json
"background_image": "assets/background.webp"
~~~

The archive should then contain just `theme.json` and the named image.
Use forward slashes and keep the image inside the archive. Keep the package
below 16 MiB; smaller images make themes easier to share and load.

Themes cannot contain scripts, QML, plugins, fonts, or extra files. Unknown
settings are rejected, so check spelling if a package will not install.

Only share artwork you have permission to redistribute.

