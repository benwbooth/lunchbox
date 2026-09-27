# Build Lunchbox from source

Most users should use a [release package](installing.md). Building from source
is useful if you want to try a change before the next release or contribute a fix.

## Build with Nix

Clone the repository and run:

~~~sh
git clone https://github.com/benwbooth/lunchbox.git
cd lunchbox
nix build .#lunchbox
./result/bin/lunchbox
~~~

The build uses the versions pinned by the repository. The first build may
download and compile substantial dependencies.

## Work on the app

From the repository root, run:

~~~sh
./dev.sh
~~~

The script enters the development environment when needed, watches source
changes, rebuilds, and relaunches Lunchbox. Leave one watcher running rather
than starting another after every edit.

Do not use this workflow against an irreplaceable profile without a backup.
For a separate writable profile, the app supports
`--state-database /path/to/test-state.db`. That isolates Lunchbox's profile,
not every external emulator or service you might launch.

## Other build environments

The repository's [package workflow](../.github/workflows/native-packages.yml)
contains the current Windows, macOS, AppImage, and Flatpak build steps.
The [flake](../flake.nix) defines the Nix environment.

An app that builds successfully may still need platform-specific packaging,
multimedia plugins, and GPU support. In particular, do not assume a custom
build includes live-translation acceleration just because the main UI opens.

## Contribute

Keep changes focused and describe how you tested the behavior. For a visual
change, a screenshot helps reviewers understand it.

For normal use, return to the [user guide](README.md).

