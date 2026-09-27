# Lunchpail

Your retro game collection, ready to play.

Lunchpail is a free, open-source game library and emulator frontend for **Windows,
macOS, and Linux**. Browse your collection, discover artwork and translations,
set up controllers, and launch games from one place—at your desk or on the couch.

[Download Lunchpail](https://github.com/benwbooth/lunchpail/releases) ·
[Report an issue](https://github.com/benwbooth/lunchpail/issues) ·
[User guide](docs/README.md)

![Lunchpail desktop library with box art from multiple systems](docs/images/library.png)

## What you can do

- **Bring your games together.** Import ROMs and disc images, or manage
  selected-file downloads with optional Minerva and qBittorrent integration.
- **Browse and organize.** Box-art grids, sortable lists, search, favorites,
  collections, regional releases, and play history.
- **Explore each game.** Descriptions, artwork, video previews, manuals, music,
  and interactive 3D boxes, with optional media-provider integrations.
- **Play your way.** Launch RetroArch or standalone emulators, choose defaults
  per system or game, and configure controller mappings for supported emulators.
- **Pick up where you left off.** Automatic save-state resume and save backups
  for supported emulators, including a local folder managed by your sync app.
- **Make it look right.** CRT shaders and system or game-specific bezels,
  with aspect-ratio-preserving artwork on ultrawide displays.
- **Try translations and mods.** Find community patches inside Lunchpail or
  import your own; patched copies leave the original ROM or disc image untouched.
- **Add extras.** RetroArch cheats, RetroAchievements account setup, and
  per-game options such as supported arcade blood settings.
- **Move to the couch.** Cover wall, 3D album, animated logo wheel and classic
  shelf views, with themes, attract mode and access to all game and library tools.

Experimental local AI translation is also available for supported RetroArch
setups. It is opt-in per game, uses OCR and Ollama on a supported GPU, and has a
guided setup wizard. GPU OCR is not yet included in the Linux AppImage or
Flatpak packages. See [translation requirements](docs/translation.md).

## Install

Lunchpail was previously named Lunchbox. The source, Nix package, and Homebrew
tap use the new name. The latest packaged release, **v0.1.2**, still has its
original filenames and app ID; the commands below match those published files.
The next release will use Lunchpail throughout. Existing settings and saves
are preserved when upgrading to the renamed app.

| Your computer | Installation options |
| --- | --- |
| Windows x86-64 | [MSI installer or portable ZIP](#windows) |
| Apple Silicon Mac, macOS 13+ | [Homebrew or DMG](#macos) |
| Linux x86-64 | [Flatpak or AppImage](#linux) |
| Linux x86-64 / ARM64, Apple Silicon Mac | [Nix](#nix-and-nixos) |

Intel Mac packages are not available. Normal packages do not require Nix or Docker.

### Windows

- **Installer:** [Download the MSI](https://github.com/benwbooth/lunchpail/releases/download/v0.1.2/Lunchbox-windows-x86_64.msi)
  and run it. Open the installed app from the Start menu.
- **Portable:** [Download the ZIP](https://github.com/benwbooth/lunchpail/releases/download/v0.1.2/Lunchbox-windows-x86_64.zip),
  extract the whole folder, and open `lunchbox.exe`. Keep its accompanying files
  together. The ZIP is portable; your settings and saves still use your user folders.

To update, close Lunchpail and install the newer MSI, or extract the newer ZIP into
a fresh folder. Your library settings are kept separately.

### macOS

With [Homebrew](https://brew.sh/) installed:

```sh
brew install --cask benwbooth/lunchpail/lunchpail
```

Update with `brew update` followed by `brew upgrade --cask lunchpail`.
The [Lunchpail tap](https://github.com/benwbooth/homebrew-lunchpail) tracks stable releases.

Prefer a regular download? [Open the DMG](https://github.com/benwbooth/lunchpail/releases/download/v0.1.2/Lunchbox-macos-arm64.dmg)
and drag the app into **Applications**. To update, quit it and replace
the application with the newer copy. Both options require Apple Silicon and macOS 13+.

### Linux

**Flatpak — recommended for managed updates.** First [install Flatpak for your distribution](https://flatpak.org/setup/).
Then [open the Lunchpail installer](https://benwbooth.github.io/lunchpail/lunchpail.flatpakref)
in Discover / your software manager, or run:

```sh
flatpak install --user https://benwbooth.github.io/lunchpail/lunchpail.flatpakref
flatpak run io.github.benwbooth.Lunchbox
```

This adds the signed Lunchpail repository and offers to add Flathub for the KDE
runtime. Future updates appear in your software manager, or run `flatpak update --user`.
Lunchpail is hosted in its own repository, not on Flathub.

To add the repository separately:

```sh
flatpak remote-add --user --if-not-exists lunchpail https://benwbooth.github.io/lunchpail/lunchpail.flatpakrepo
```

**AppImage — a single downloadable app.**
[Download the AppImage](https://github.com/benwbooth/lunchpail/releases/download/v0.1.2/Lunchbox-linux-x86_64.AppImage),
then run these commands from your download folder:

```sh
chmod +x Lunchbox-linux-x86_64.AppImage
./Lunchbox-linux-x86_64.AppImage
```

To update, replace the AppImage with the latest download. If your distribution
reports a missing FUSE library, use Flatpak or try
`./Lunchbox-linux-x86_64.AppImage --appimage-extract-and-run`.

**Standalone Flatpak bundle.** For a manual install, download the
[`.flatpak` file](https://github.com/benwbooth/lunchpail/releases/download/v0.1.2/Lunchbox-linux-x86_64.flatpak):

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user ./Lunchbox-linux-x86_64.flatpak
```

Prefer the installer above for repository updates. The repository archive
release asset is for self-hosting; you do not need to unpack it to install Lunchpail.

### Nix and NixOS

With [Nix](https://nixos.org/download/) installed and flakes enabled, run the
renamed development version without adding it to your profile:

```sh
nix run github:benwbooth/lunchpail
```

Or install it persistently:

```sh
nix profile install github:benwbooth/lunchpail#lunchpail
```

For NixOS, add Lunchpail to your system flake:

```nix
inputs.lunchpail.url = "github:benwbooth/lunchpail";
```

Then add its package in a module that receives your flake inputs:

```nix
environment.systemPackages = [
  inputs.lunchpail.packages.${pkgs.stdenv.hostPlatform.system}.lunchpail
];
```

Rebuild NixOS normally. Update your lock file to get newer changes. You can append
a release tag to the input URL once a Lunchpail-named release is published.
Nix may compile the app and dependencies; it is not the quickest first install.

### Build from source

The easiest source build uses the included Nix environment:

```sh
git clone https://github.com/benwbooth/lunchpail.git
cd lunchpail
nix build .#lunchpail
./result/bin/lunchpail
```

See [Building from source](docs/building.md) for development details. Published
packages and `SHA256SUMS` are on the [release page](https://github.com/benwbooth/lunchpail/releases/latest).
The Windows installer is unsigned, and the macOS app is not notarized yet. Follow
your OS's per-app security prompt after verifying the download. Do not disable system-wide
security protections. The Flatpak repository is signed separately.

## Get started

1. Install Lunchpail using one of the options above.
2. Open Lunchpail, choose your storage folders, and use **Library → Import ROMs**
   to add your games. Media accounts and qBittorrent downloads are optional.
3. Select a game, choose an emulator, and press **Play**. Open
   **Settings & mappings** in Game details when you want to customize it.

Lunchpail is under active development. These screenshots and feature notes reflect
the current source; check release notes for what's in each downloadable version.
Games and BIOS files are not included. Use content you have the right to use.

## On the couch

Browse the same collection with a gamepad, keyboard, or mouse.

![Lunchpail Couch Mode showing Zelda artwork and a collection of installed games](docs/images/couch-mode.png)

Screenshots show a configured library. Game artwork belongs to its respective
rights holders; available media depends on your collection and connected providers.

## Learn more

- [Play your first game](docs/getting-started.md).
- [Controllers](docs/controllers.md) · [Display and bezels](docs/display.md) ·
  [Saves and backups](docs/saves.md).
- [Translations, mods, and cheats](docs/patches.md) · [Couch Mode](docs/couch-mode.md).
- [Troubleshooting](docs/troubleshooting.md) · [All guides](docs/README.md).

## Contribute

Built with Rust, Qt, and QML. For incremental development with Nix installed:

```sh
./dev.sh
```

The script enters the development environment when needed, rebuilds changes,
and relaunches Lunchpail. See [Building from source](docs/building.md)
for more options.

Licensed under the [MIT License](LICENSE).
