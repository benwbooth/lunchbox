#!/usr/bin/env bash
# Development script for Lunchbox
#
# Usage:
#   ./scripts/dev.sh          # Browser mode (default) - opens in your browser
#   ./scripts/dev.sh electron # Electron mode - Chromium desktop shell

set -e

MODE="${1:-browser}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
APPS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
LOCK_FILE="${XDG_RUNTIME_DIR:-/tmp}/lunchbox-dev.lock"

exec 9>"$LOCK_FILE"
if ! flock -n 9; then
    echo "Lunchbox dev session is already running."
    echo "Stop the existing ./scripts/dev.sh session before starting another one."
    exit 1
fi

# Install systemd user units
install_units() {
    mkdir -p "$UNIT_DIR"

    # Trunk (frontend) unit
    cat > "$UNIT_DIR/lunchbox-trunk.service" << EOF2
[Unit]
Description=Lunchbox Frontend (trunk)

[Service]
Type=simple
WorkingDirectory=$PROJECT_DIR
Environment=CARGO_TARGET_DIR=$PROJECT_DIR/target/dev-frontend
ExecStart=/nix/var/nix/profiles/system/sw/bin/nix develop --command trunk serve --port 1420
Restart=on-failure
RestartSec=2
EOF2

    # Backend unit - uses watchexec to auto-reload on code changes
    cat > "$UNIT_DIR/lunchbox-backend.service" << EOF2
[Unit]
Description=Lunchbox Backend (dev_server)

[Service]
Type=simple
WorkingDirectory=$PROJECT_DIR
Environment=CARGO_TARGET_DIR=$PROJECT_DIR/target/dev-backend
ExecStart=/nix/var/nix/profiles/system/sw/bin/nix develop --command watchexec -r -w backend/src -w backend/Cargo.toml -w Cargo.toml -- cargo run --profile dev-backend -p lunchbox --bin dev_server
Restart=on-failure
RestartSec=2
EOF2

    systemctl --user daemon-reload
}

install_desktop_entry() {
    mkdir -p "$APPS_DIR"

    cat > "$APPS_DIR/lunchbox.desktop" << EOF2
[Desktop Entry]
Type=Application
Name=Lunchbox
Comment=Lunchbox Electron Development Shell
Exec=/nix/var/nix/profiles/system/sw/bin/nix develop $PROJECT_DIR --command electron $PROJECT_DIR/electron
Icon=$PROJECT_DIR/backend/icons/icon.png
StartupWMClass=Lunchbox
Categories=Game;
Terminal=false
EOF2
}

start_units() {
    systemctl --user start lunchbox-trunk.service
    systemctl --user start lunchbox-backend.service
}

wait_for_url() {
    local name="$1"
    local url="$2"
    local timeout="${3:-120}"
    local elapsed=0

    printf "Waiting for %s" "$name"
    while [ "$elapsed" -lt "$timeout" ]; do
        if curl -fsS "$url" >/dev/null 2>&1; then
            echo " ready"
            return 0
        fi
        printf "."
        sleep 1
        elapsed=$((elapsed + 1))
    done

    echo " timeout"
    return 1
}

wait_for_services() {
    wait_for_url "frontend" "http://127.0.0.1:1420" 180
    wait_for_url "backend" "http://127.0.0.1:3001/api/health" 180
}

stop_units() {
    echo ""
    echo "Shutting down..."
    systemctl --user stop lunchbox-trunk.service 2>/dev/null || true
    systemctl --user stop lunchbox-backend.service 2>/dev/null || true
}

cleanup() {
    stop_units
    exit 0
}

trap cleanup SIGINT SIGTERM SIGPIPE EXIT

if [ "$MODE" = "electron" ]; then
    echo "Starting Electron development mode..."
    echo ""
    echo "Electron defaults to a safer WebGPU profile."
    echo "Use LUNCHBOX_AGGRESSIVE_GPU=1 for forced Vulkan/Wayland developer flags."
    echo ""

    install_units
    install_desktop_entry

    systemctl --user stop lunchbox-trunk.service 2>/dev/null || true
    systemctl --user stop lunchbox-backend.service 2>/dev/null || true

    echo "Starting frontend (trunk) on http://127.0.0.1:1420..."
    echo "Starting backend API server on http://127.0.0.1:3001..."
    start_units

    wait_for_services

    echo "Opening Electron shell..."
    env -u NO_COLOR nix develop "$PROJECT_DIR" --command electron "$PROJECT_DIR/electron" &

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  Electron:  native Chromium shell"
    echo "  Frontend:  http://127.0.0.1:1420 (auto-reloads)"
    echo "  API:       http://127.0.0.1:3001 (auto-restarts)"
    echo ""
    echo "  Press Ctrl+C to stop"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    journalctl --user -n 0 -f -u lunchbox-trunk.service -u lunchbox-backend.service

elif [ "$MODE" = "browser" ]; then
    echo "Starting browser development mode..."
    echo ""

    # Install/update systemd units
    install_units

    # Stop any existing instances
    systemctl --user stop lunchbox-trunk.service 2>/dev/null || true
    systemctl --user stop lunchbox-backend.service 2>/dev/null || true

    # Start the services
    echo "Starting frontend (trunk) on http://127.0.0.1:1420..."
    echo "Starting backend API server on http://127.0.0.1:3001..."
    start_units

    wait_for_services

    # Open browser with WebGPU/Vulkan enabled by default.
    # If Chromium becomes unstable on a specific system/session, use:
    #   LUNCHBOX_STABLE_CHROMIUM=1 ./scripts/dev.sh
    echo ""
    if [ "${LUNCHBOX_STABLE_CHROMIUM:-0}" = "1" ]; then
        echo "Opening Chromium with stable flags (WebGPU/Vulkan disabled via LUNCHBOX_STABLE_CHROMIUM=1)..."
        nix develop "$PROJECT_DIR" --command chromium \
            --ozone-platform-hint=auto \
            --disable-features=Vulkan \
            http://127.0.0.1:1420 "$@" &
    else
        echo "Opening Chromium with experimental WebGPU/Vulkan flags..."
        nix develop "$PROJECT_DIR" --command chromium \
            --ozone-platform=wayland \
            --enable-features=UseOzonePlatform,Vulkan \
            --use-vulkan \
            --enable-unsafe-webgpu \
            --enable-webgpu-developer-features \
            --disable-software-rasterizer \
            http://127.0.0.1:1420 "$@" &
    fi

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  Frontend:  http://127.0.0.1:1420 (auto-reloads)"
    echo "  API:       http://127.0.0.1:3001 (auto-restarts)"
    echo ""
    echo "  Press Ctrl+C to stop"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    # Follow the logs until interrupted
    journalctl --user -n 0 -f -u lunchbox-trunk.service -u lunchbox-backend.service

else
    echo "Unknown mode: $MODE"
    echo ""
    echo "Usage:"
    echo "  ./scripts/dev.sh          # Browser mode (default)"
    echo "  ./scripts/dev.sh electron # Electron mode"
    exit 1
fi
