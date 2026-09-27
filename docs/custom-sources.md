# Add your own download sources

Use **Torrent Sources** to add a torrent collection that Lunchpail can search
when you open a game's download options. Registering a source does not start
downloading its games.

## Add a torrent or magnet

1. Open **Torrent Sources** from the library menu.
2. Choose a local `.torrent` file or paste a v1 magnet link.
3. Select the correct system.
4. Review the file list and register the source.

Magnets need the configured qBittorrent connection to retrieve their file list.
Lunchpail does not support every magnet format.

You can also attach a source while viewing an individual game. Review the
exact file before queuing it; a title match is a suggestion, not proof of
the correct game revision.

## Watch a folder

Downloads settings can watch a folder for new `.torrent` files. You still
review each source and choose its system before registering it.

The optional archive feature tidies successfully registered torrent files.
It moves the source `.torrent` file after verification, not the downloaded games.

## Import several sources together

For a larger collection, put a provider manifest beside your torrent files:

~~~text
my-library/
  provider.json
  torrents/
    homebrew.torrent
~~~

A minimal `provider.json` looks like this:

~~~json
{
  "format": "lunchpail-local-torrent-provider",
  "schema_version": 1,
  "provider": {
    "id": "my-homebrew",
    "name": "My homebrew collection",
    "catalog_version": "1",
    "authorization": "user-managed",
    "terms_url": "https://example.com/terms"
  },
  "offers": [
    {
      "id": "collection-1",
      "name": "Homebrew games",
      "platform": "OpenBOR",
      "torrent_path": "torrents/homebrew.torrent"
    }
  ]
}
~~~

Replace the example names, system, terms address, and torrent path with your
own. Use the system name shown by Lunchpail. Paths are relative to the manifest
and must stay inside its folder.

Open **Settings → Local provider catalogs → Import manifest…**.
A manifest can contain up to 256 sources. Increase `catalog_version` when
you change its contents.

Removing a registered catalog removes its Lunchpail source entries, not ROMs,
torrent files, or qBittorrent data.

## Pinball and OpenBOR

These collections often need user-added sources. For pinball, a table and
its required PinMAME ROM are different files; use the table author's instructions.
An OpenBOR game usually needs the matching engine and game data.

Registering a source does not install an emulator or make an unsupported game
compatible.

