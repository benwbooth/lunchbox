# Install Lunchbox

The [README has installation commands for every supported package and OS](../README.md#install).
Use a published release rather than an unfinished GitHub Actions build.

| Your computer | Package |
| --- | --- |
| Windows, 64-bit | MSI installer or portable ZIP |
| Mac with Apple Silicon | Homebrew tap or arm64 DMG |
| Linux, x86-64 | Flatpak repository or AppImage |
| Linux, x86-64 / ARM64; Apple Silicon Mac | Nix |

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

With Homebrew installed:

~~~sh
brew install --cask benwbooth/lunchbox/lunchbox
~~~

Update with `brew update` and `brew upgrade --cask lunchbox`. You no longer
need to download a local cask file.

## Linux

For the AppImage, make the downloaded file executable in your file manager,
then open it.

For Flatpak, [open the installer](https://benwbooth.github.io/lunchbox/lunchbox.flatpakref)
in your software manager, or use:

~~~sh
flatpak install --user https://benwbooth.github.io/lunchbox/lunchbox.flatpakref
flatpak run io.github.benwbooth.Lunchbox
~~~

This adds Lunchbox's signed update repository and offers the Flathub runtime
source. Update through your software manager or `flatpak update --user`.
Lunchbox itself is hosted here, not on Flathub.

If you previously installed the standalone bundle, switch its update source with:

~~~sh
flatpak remote-add --user --if-not-exists lunchbox https://benwbooth.github.io/lunchbox/lunchbox.flatpakrepo
flatpak install --user --reinstall lunchbox io.github.benwbooth.Lunchbox
~~~

This keeps the app's data. For a system-wide installation, use `--system` in place
of `--user`. The release also includes a standalone bundle and repository archive
for manual installation or self-hosting; see the README for those options.

Flatpak permissions can affect access to game folders, external drives, and
controllers. If a folder works outside the sandbox but not inside Lunchbox,
check the app's permissions before moving your files.

For Nix/NixOS commands, see the [README](../README.md#nix-and-nixos).
For a local build, see [Building from source](building.md).

## Security warnings

Windows/macOS packages are not yet signed/notarized. The Flatpak repository is
signed. Download only from the project's release page or official package channels,
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
