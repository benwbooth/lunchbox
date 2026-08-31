# Native packages and releases

The repository builds every package from the same locked Rust workspace and
checked-in compressed database. `flake.nix` remains the canonical Linux and
macOS development/build definition. GitHub Actions adds end-user packages on
pushes to `main`, version tags, and manual dispatches.

## Produced artifacts

- Windows x86-64: a self-contained portable ZIP and a per-machine MSI.
- macOS 13+: Apple Silicon and Intel DMGs. Tagged releases also contain a
  checksum-pinned Homebrew cask named `lunchbox.rb`.
- Linux x86-64: an AppImage, a Flatpak single-file bundle, and an OSTree
  Flatpak repository archive. The Nix job validates `.#lunchbox` directly.

The Homebrew cask can be installed directly after downloading it from a tagged
release:

```console
brew install --cask ./lunchbox.rb
```

The Flatpak bundle is convenient for a one-off install. The repository archive
is the update-capable format and can be served by any static file host after it
is extracted. The workflow retains both so local testing does not depend on a
published Flathub application.

## Local validation

```console
nix flake check
nix build .#lunchbox
nix develop -c cargo test --workspace --locked
```

Flatpak sources are generated from `Cargo.lock` by the official
`flatpak-cargo-generator`; regenerate `packaging/flatpak/cargo-sources.json`
whenever the lockfile changes. The Flatpak build itself is intentionally
offline, so a stale source manifest fails rather than downloading undeclared
dependencies.

## Signing and platform policy

Development artifacts are unsigned. A public macOS release should add an Apple
Developer ID certificate and notarization credentials; a public Windows
release should add an Authenticode certificate. Those credentials belong in
GitHub environment secrets, never in the repository.

Lunchbox does not publish a nominal “static binary.” Qt Quick depends on QML,
image, multimedia, graphics, and window-system plugins. AppImage is the
portable Linux deliverable and bundles that runtime closure without pretending
the application is a single statically linked executable.
