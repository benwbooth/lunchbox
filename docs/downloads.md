# Download games

Lunchpail uses qBittorrent for torrent transfers. A catalog match or a download
badge does not guarantee that a particular file is available or has seeders.

Only download content you have permission to use.

## Connect qBittorrent

1. Start qBittorrent and enable its Web UI.
2. In Lunchpail Settings, enter the Web UI address and credentials.
3. Set the download folder as qBittorrent sees it and as Lunchpail sees it.
4. Use the connection test before queuing a game.

For example, a container may call a folder `/downloads` while your computer
sees the same files at `/mnt/games/downloads`. Lunchpail needs that relationship
to find completed downloads. If both apps see the same path, use that path.

Do not expose an unauthenticated Web UI to the internet.

## Review a download

Select a game and open its download options. Choose the correct region and
revision, then review the selected files and their size before queuing.

Minerva groups files into larger collections. Lunchpail normally selects the
reviewed game and any required companion files, not every game in the collection.
Check the whole-torrent setting if the proposed download is larger than expected.

Multi-disc games, arcade sets, and prepared PC collections can require several
files. Do not deselect required companions simply because only one is the main ROM.

## Progress and recovery

Open **Downloads** for transfer progress, pause/resume, cancel, and retry.
Lunchpail automatically force-starts its own downloads so they do not wait
behind qBittorrent's ordinary queue limits.

Force-start cannot create seeders or fix an unreachable tracker. If a download
has no peers, inspect it in qBittorrent as well.

If a job fails, read its error before retrying. A missing torrent or changed
source may require returning to Game details and reviewing the source again.

Cancelling a job or clearing completed history is not the same as deleting
the downloaded files.

## Where completed games go

The import setting controls whether Lunchpail copies, links, or leaves completed
files in place. A leave-in-place game depends on the original download folder
remaining available.

After import, the game appears in **My Collection**. It may disappear from a
Minerva/not-installed view because it is now installed; that is not a failed
download.

Choose whether to keep seeding under qBittorrent's rules or pause Lunchpail-owned
torrents after their selected games have imported.

## No matching file?

Try the correct release or an alternate title, and check the source's system
and ROM-set version. For arcade games, use a complete compatible set rather
than an arbitrary similarly named file.

You can also [register your own source](custom-sources.md) or import a local
game. Repeating the same search will not repair an incomplete source index.

