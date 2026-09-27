# Install Lunchbox

Download a package from [Lunchbox Releases](https://github.com/benwbooth/lunchbox/releases).
Use a published release rather than an unfinished GitHub Actions build.

| Your computer | Package |
| --- | --- |
| Windows, 64-bit | MSI installer or portable ZIP |
| Mac with Apple Silicon | arm64 DMG |
| Linux, x86-64 | AppImage or Flatpak |

Intel Mac packages are not provided. You do not need Nix or Docker to use the
regular application.

## Windows

Run the MSI installer, or extract the portable ZIP into a folder and run
Lunchbox from there. Keep the ZIP's accompanying files together.

“Portable ZIP” describes how the app is distributed; it does not mean your
library settings and saves are automatically stored beside the executable.

## macOS

Open the DMG and copy Lunchbox to Applications. The package targets Apple
Silicon Macs running macOS 13 or later.

Releases also provide a Homebrew cask file. After downloading `lunchbox.rb`,
you can install it with:

~~~sh
brew install --cask ./lunchbox.rb
~~~

## Linux

For the AppImage, make the downloaded file executable in your file manager,
then open it.

For the Flatpak, open the downloaded `.flatpak` bundle with your software
installer, or use:

~~~sh
flatpak install --user ./Lunchbox.flatpak
~~~

Replace the filename with the one you downloaded. The release also includes
a Flatpak repository archive for people hosting their own update source; it
is not needed for a normal bundle installation.

Flatpak permissions can affect access to game folders, external drives, and
controllers. If a folder works outside the sandbox but not inside Lunchbox,
check the app's permissions before moving your files.

For a Nix installation or a local build, see [Building from source](building.md).

## Security warnings

Packages may be unsigned. Download only from the project's release page,
check the published checksums if you are unsure, and do not disable your
computer's security protections globally to install the app.

## What else do I need?

An emulator and games. Lunchbox can help manage supported emulators, but it
does not include ROMs or BIOS dumps.

qBittorrent is optional and only needed for torrent downloads. Local AI
translation has separate GPU and model requirements; see
[Live translation](translation.md). In particular, GPU OCR is not currently
available in the Linux AppImage or Flatpak.

Next: [Add and play your first game](getting-started.md).

