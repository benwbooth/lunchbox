# Lunchbox

Lunchbox is a cross-platform emulator frontend.

## Installation

Release builds are published on the GitHub releases page:

https://github.com/benwbooth/lunchbox/releases

The current release is `v0.1.0`:

https://github.com/benwbooth/lunchbox/releases/tag/v0.1.0

### Linux AppImage

Download `Lunchbox-0.1.0-linux-x86_64.AppImage`, make it executable, and run it:

```sh
chmod +x Lunchbox-0.1.0-linux-x86_64.AppImage
./Lunchbox-0.1.0-linux-x86_64.AppImage
```

### Linux Flatpak

To install from the Lunchbox Flatpak repository:

```sh
flatpak remote-add --if-not-exists --user flathub \
  https://flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --if-not-exists --user lunchbox \
  https://benwbooth.github.io/lunchbox/lunchbox.flatpakrepo
flatpak install --user lunchbox io.github.benwbooth.lunchbox
flatpak run io.github.benwbooth.lunchbox
```

You can also install the release bundle directly:

```sh
flatpak install --user ./Lunchbox-0.1.0-x86_64.flatpak
flatpak run io.github.benwbooth.lunchbox
```

### Nix and NixOS

Lunchbox provides a Linux flake package.

Run it directly:

```sh
nix run github:benwbooth/lunchbox/v0.1.0
```

Install it into your profile:

```sh
nix profile install github:benwbooth/lunchbox/v0.1.0
```

Use it from a NixOS configuration:

```nix
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.lunchbox.url = "github:benwbooth/lunchbox/v0.1.0";

  outputs = { nixpkgs, lunchbox, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        {
          environment.systemPackages = [
            lunchbox.packages.x86_64-linux.default
          ];
        }
      ];
    };
  };
}
```

### macOS

Download `Lunchbox-0.1.0-mac-arm64.dmg`, open it, and drag Lunchbox into
`Applications`.

The macOS build is not currently notarized, so macOS may block the first launch.
If that happens, right-click Lunchbox in Finder and choose `Open`.

### Windows

Download and run `Lunchbox-0.1.0-win-x64.msi`.

The Windows installer is not currently code-signed, so Windows SmartScreen may
show a warning on first install.

## Build From Source

With Nix:

```sh
nix build .#packages.x86_64-linux.default
./result/bin/lunchbox
```

For development:

```sh
nix develop
```
