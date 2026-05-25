#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.benwbooth.lunchbox"
VERSION="$(node -p "require('$ROOT_DIR/electron/package.json').version")"
BUILD_DIR="$ROOT_DIR/build-flatpak"
MANIFEST="$ROOT_DIR/packaging/flatpak/$APP_ID.yml"
ARCH="$(flatpak --default-arch)"

if ! flatpak --user remotes --columns=name | grep -qx flathub; then
  flatpak --user remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
fi

flatpak --user install -y --noninteractive flathub "org.freedesktop.Platform//$ARCH/24.08" "org.freedesktop.Sdk//$ARCH/24.08"

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/prebuilt" "$ROOT_DIR/dist"

npm --prefix "$ROOT_DIR/electron" run pack:linux-dir
cp -a "$ROOT_DIR/dist/electron/linux-unpacked" "$BUILD_DIR/prebuilt/linux-unpacked"

flatpak-builder \
  --force-clean \
  --default-branch=stable \
  --repo="$BUILD_DIR/repo" \
  "$BUILD_DIR/build" \
  "$MANIFEST"

flatpak build-bundle \
  "$BUILD_DIR/repo" \
  "$ROOT_DIR/dist/Lunchbox-$VERSION-$ARCH.flatpak" \
  "$APP_ID" \
  stable

tar -C "$BUILD_DIR" -czf "$ROOT_DIR/dist/lunchbox-flatpak-repo-$ARCH.tar.gz" repo
