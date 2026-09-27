# Troubleshooting

Start with the message shown for the game or failed action. Changing several
settings at once makes it harder to find the cause.

## A game disappeared

Clear search and check the active platform, collection, and filters.
An imported game leaves a Minerva/not-installed view because it is now in
My Collection. Search text is remembered per platform.

## Nothing launches, or the emulator shows a black screen

Check the selected emulator/core, BIOS requirements, and exact game files.
An arcade title may need a different hardware profile or ROM-set version.

Try the default launch command if you customized it. Keep CUE/GDI companions
and arcade archives together. See [Launching games](playing.md).

## Preparation takes a long time

A first launch can involve extraction, patching, firmware checks, or save
synchronization. Look at the current stage and any error rather than repeatedly
pressing Play. Large disc patches and first-time PC collection preparation can
take longer than a small ROM launch.

If every unchanged launch repeats the same expensive preparation, report the
stage name and game format.

## The wrong controller works, or input happens twice

Check Player 1's dropdown and the saved mapping scope. If using Steam Input,
decide whether you want the physical pad or its virtual controller, not both.

Use the input test to identify generic device names and desktop-mode keyboard
events. See [Controllers](controllers.md).

## Game text is cut off, or a bezel looks stretched

Compare once without the bezel, then without the shader. This separates
artwork/viewport issues from shader cropping.

Include the full window in a screenshot and note the selected core, shader,
bezel, display resolution, and fullscreen setting.
See [Display and bezels](display.md).

## My saves are still in a local folder

That is normally the emulator's live save location. The folder you chose for
sync is the backup destination. Scroll to **Save locations** in Game details
to see both, and check notification history for the backup result.

See [Saves and backups](saves.md) before moving or deleting anything.

## A download is queued or has no progress

Test Lunchpail's qBittorrent connection, then check the torrent in qBittorrent.
Lunchpail force-starts its managed downloads, but unavailable peers and
incorrect folder mappings need separate fixes.

A download badge describes a possible source, not guaranteed availability.
See [Downloads](downloads.md).

## Translation is missing or slow

Confirm **Translate this game** is enabled, then use F10 to start automatic
translation. F8 is the screenshot key.

Check both GPU tests in the setup wizard. The Linux AppImage and Flatpak do
not currently include GPU OCR. Even a working GPU setup can miss text or
compete with a running emulator. See [Live translation](translation.md).

## A patch won't apply

Check the author's required region, revision, and header format. For disc
patches, use the specified raw image rather than its playlist or compressed
container. Do not disable checksum checks just to make a mismatch proceed.

## Settings or an account seem to vanish

Most settings save automatically; mapping and other review editors may still
have explicit confirmation buttons. Credentials use your operating system's
credential store and are not included in profile exports.

Check for a credential-store error before entering the same secret repeatedly.

## Report a problem

Open an issue at [Lunchpail on GitHub](https://github.com/benwbooth/lunchpail/issues)
and include:

- Lunchpail version, operating system, and package type.
- Game title/release and the emulator/core.
- The exact steps and error message.
- A screenshot when the problem is visual.

Do not attach ROMs, BIOS dumps, account tokens, or passwords. Review logs and
screenshots for private paths and account details before sharing them.

