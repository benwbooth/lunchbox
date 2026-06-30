#!/usr/bin/env bash
# Desktop launcher for Lunchbox.
#
# Unlike clicking Electron directly, this first brings up the frontend (trunk)
# and backend (dev_server) systemd user services that the Electron shell needs,
# then opens Electron. Starting an already-running service is a no-op, and
# Electron's splash shows compile progress until they are serving — so the app
# no longer sits forever at "18% compiling" when launched from the desktop.
#
# The service units are installed by scripts/dev.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
NIX=/nix/var/nix/profiles/system/sw/bin/nix

# Bring up the dev services (idempotent).
if ! systemctl --user start lunchbox-trunk.service lunchbox-backend.service; then
    echo "Could not start Lunchbox services — run ./scripts/dev.sh once to install them." >&2
fi

exec env -u NO_COLOR "$NIX" develop "$PROJECT_DIR" --command electron "$PROJECT_DIR/electron"
