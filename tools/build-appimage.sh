#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

npm --prefix "$ROOT_DIR/electron" run dist:linux

mkdir -p "$ROOT_DIR/dist"
find "$ROOT_DIR/dist/electron" -maxdepth 1 -type f -name '*.AppImage' -exec cp '{}' "$ROOT_DIR/dist/" ';'
