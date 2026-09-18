#!/usr/bin/env bash
# Rebuild and restart Lunchbox on every code change.
#
# Thin wrapper around the `lunchbox-dev` command provided by the Nix dev shell
# (see README). Run it in the background to keep a live app that reflects the
# working tree:
#
#     ./dev.sh &
#
# It runs the debug binary; the first link takes a few minutes and later
# one-file Rust rebuilds take about twenty-five seconds.
set -euo pipefail
cd "$(dirname "$0")"
# Keep Qt/QML diagnostics on the terminal so a broken binding is visible.
export QT_LOGGING_TO_CONSOLE=1
exec nix develop --command bash -c 'lunchbox-dev'
