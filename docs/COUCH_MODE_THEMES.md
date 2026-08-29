# Couch Mode theme packages

Couch Mode themes are deliberately declarative. A theme can change the living-room
palette, background image, hero contrast, and corner treatment, but it cannot ship
QML, scripts, commands, fonts, plugins, or executable behavior. This keeps one
responsive, controller-tested interface across Linux, macOS, and Windows.

Install and manage themes from **Settings > Couch Mode > Appearance Themes**. Theme
discovery, package validation, installation, replacement, removal, and preference
persistence run in native Rust workers without blocking the Qt thread. The selected
exact theme ID survives restart. If that theme is later missing or invalid, Lunchbox
falls back to its built-in default and reports the problem instead of loading partial
or untrusted content.

## Package layout

A `.lunchbox-theme` file is a ZIP archive containing a top-level `theme.json` and,
optionally, the one image named by `background_image`:

```text
theme.json
assets/background.webp
```

No other file is accepted. Paths must be portable relative paths with `/`
separators. Duplicate, case-ambiguous, absolute, parent-traversing, backslash, null,
and symbolic-link entries are rejected.

## Manifest schema 1

```json
{
  "schema_version": 1,
  "id": "ultraviolet-circuit",
  "name": "Ultraviolet Circuit",
  "author": "Theme Author",
  "description": "Deep violet surfaces with a cool mint secondary accent.",
  "palette": {
    "background": "#060714",
    "panel": "#11142b",
    "panel_raised": "#1b2142",
    "ink": "#f7f5ff",
    "muted": "#aaa7ca",
    "accent": "#8c7bff",
    "accent_cool": "#51dfca",
    "danger": "#ff6f91"
  },
  "background_image": "assets/background.webp",
  "hero_scrim_percent": 58,
  "card_radius": 24
}
```

Every property is closed-schema: unknown properties are rejected rather than
silently ignored.

| Property | Contract |
| --- | --- |
| `schema_version` | Must be `1`. |
| `id` | 3--64 lowercase ASCII letters, digits, or interior hyphens. It is the stable replacement and preference identity. |
| `name`, `author` | 1--80 printable characters each. |
| `description` | 1--240 printable characters. |
| palette colors | `#RRGGBB` or `#AARRGGBB`. Authors are responsible for readable contrast. |
| `background_image` | Optional PNG, JPEG, or WebP at a portable relative path no more than four components deep. The bytes must match the declared format. |
| `hero_scrim_percent` | Optional 20--90; defaults to 62. |
| `card_radius` | Optional 4--32 pixels; defaults to 16. |

Packages are limited to 16 MiB compressed, 32 MiB expanded, 16 ZIP entries, and a
64 KiB manifest. A background image is limited to 16 MiB. These are validation
ceilings, not recommended asset budgets; smaller appropriately sized images improve
startup and transition latency.

## Ownership and updates

Validated content is staged and atomically renamed below the platform-native
Lunchbox state directory. Lunchbox writes a receipt containing exact package,
manifest, and image SHA-256 digests. Reinstalling identical bytes is a no-op.
Installing a new package with the same ID atomically replaces the prior version only
when its current directory and receipt still verify. Removal has the same ownership
requirement and never recursively targets an unresolved or broader path. The three
built-in themes cannot be replaced or removed.

The format intentionally has no layout or behavior extension point. New visual
tokens should be added through a future schema version and implemented in the shared
Qt surface so keyboard, controller, accessibility, and cross-platform behavior remain
testable.
