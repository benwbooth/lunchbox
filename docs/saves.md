# Saves and backups

Lunchbox can resume supported games and back up their saves. These are related,
but different, features.

| File | What it contains |
| --- | --- |
| In-game save, saved RAM, or memory card | Progress saved through the game's own save system. |
| Save state | A snapshot of the emulator at a particular moment. |
| Backup | A copy of those files kept in your chosen sync location. |

Keep using the game's own save system when possible. Save states can become
incompatible after an emulator, core, ROM, or patch changes.

## Find your files

Scroll to **Save locations** at the bottom of Game details. It shows the live
save/state locations and the configured backup location for the selected emulator.

The live files may be under Lunchbox's local application-data folder even if
you chose an Insync or other cloud-synced folder. That is expected: the emulator
writes locally, then Lunchbox synchronizes a backup.

Changing the backup folder does not make RetroArch write directly into it.

## Set up a local-folder backup

1. In Settings, open **Save & state synchronization**.
2. Choose **Local folder**.
3. Select an existing folder. It can be managed by Insync, Syncthing, Nextcloud,
   or another sync application.
4. Verify the connection and enable **Synchronize automatically around game sessions**.

Lunchbox checks for changes before launch and backs up supported save locations
after the emulator exits. Your sync application then transfers those files to
its cloud or other computers. A successful Lunchbox backup does not by itself
prove that external transfer has finished.

## What is inside the backup folder?

Readable save files are under:

~~~text
your chosen folder/
  lunchbox/saves/v1/
    emulator/runtime/
      current/saves/...
      current/states/...
      versions/...
~~~

The actual files retain their names, such as `game.srm` and
`game.state.auto`. Version-history folders may have long identifiers, and
JSON files keep track of synchronization history. You do not need to decode
JSON to copy a save out of `current/`.

Lunchbox manages these folders. Copy a backup elsewhere before editing it;
do not edit the managed backup in place.

Older backups may still contain hash-named blobs. They are a legacy format,
not the intended local-folder layout. Do not delete them until a migration has
verified the readable copies and your external sync service has received them.

## Resume a game

Choose the save/resume behavior in the game's play settings. Support depends
on the emulator and core. A game that has never created a state has nothing
to resume.

To resume, use the same game version and compatible emulator/core. Patched
games normally have their own save names. Do not copy states between unrelated
patches just because the titles match.

## A conflict needs attention

A conflict means both copies changed, or one was removed while the other was
edited. Lunchbox asks which copy you want; it does not assume the newest
timestamp is the correct progress.

Review **Local** and **Remote**, keep a copy of anything you may need, then
choose deliberately. **Play without sync** skips synchronization for that
session; it does not resolve the conflict.

An unavailable save route is different from a conflict. It can mean Lunchbox
does not know the selected emulator's save folder, even for an unplayed game.

## Check the result

After exiting a game, look for the save notification or open notification
history. Check whether it reports a completed backup, no changes, or an error.

If files seem missing, first confirm the emulator/core and its **Save locations**.
Native and Flatpak installations can use separate folders.

## Direct cloud connections

Google Drive, Dropbox, and OneDrive connections are also exposed in Settings,
but currently require OAuth credentials rather than a simple browser sign-in.
Their storage format differs from the readable local-folder backup.

For an existing desktop sync app, choose **Local folder** instead. No cloud
token is needed for that route.

