#!/usr/bin/env bash
# Development script for Lunchbox
#
# Usage:
#   ./scripts/dev.sh          # Browser mode (default) - opens in your browser
#   ./scripts/dev.sh tauri    # Tauri mode - embedded webview

set -e

MODE="${1:-browser}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

# Install systemd user units
install_units() {
    mkdir -p "$UNIT_DIR"

    # Trunk (frontend) unit
    cat > "$UNIT_DIR/lunchbox-trunk.service" << EOF
[Unit]
Description=Lunchbox Frontend (trunk)

[Service]
Type=simple
WorkingDirectory=$PROJECT_DIR
ExecStart=/nix/var/nix/profiles/system/sw/bin/nix develop --command trunk serve --port 1420
Restart=on-failure
RestartSec=2
EOF

    # Backend unit - uses watchexec to auto-reload on code changes
    cat > "$UNIT_DIR/lunchbox-backend.service" << EOF
[Unit]
Description=Lunchbox Backend (dev_server)

[Service]
Type=simple
WorkingDirectory=$PROJECT_DIR
ExecStart=/nix/var/nix/profiles/system/sw/bin/nix develop --command watchexec -r -w src-tauri/src -w src-tauri/Cargo.toml -- cargo run -p lunchbox --bin dev_server
Restart=on-failure
RestartSec=2
EOF

    systemctl --user daemon-reload
}

start_units() {
    systemctl --user start lunchbox-trunk.service
    systemctl --user start lunchbox-backend.service
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

if [ "$MODE" = "tauri" ]; then
    echo "Starting Tauri development mode..."
    cargo tauri dev

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

    # Wait a moment for services to start
    sleep 3

    # Open browser with hardware-accelerated WebGPU
    echo ""
    echo "Opening Chromium with WebGPU enabled..."
    nix develop "$PROJECT_DIR" --command chromium \
        --ozone-platform=wayland \
        --enable-features=UseOzonePlatform,Vulkan \
        --use-vulkan \
        --enable-unsafe-webgpu \
        --enable-webgpu-developer-features \
        --disable-software-rasterizer \
        http://127.0.0.1:1420 "$@" &

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  Frontend:  http://127.0.0.1:1420 (auto-reloads)"
    echo "  API:       http://127.0.0.1:3001 (auto-restarts)"
    echo ""
    echo "  Press Ctrl+C to stop"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    # Follow the logs until interrupted
    journalctl --user -f -u lunchbox-trunk.service -u lunchbox-backend.service

else
    echo "Unknown mode: $MODE"
    echo ""
    echo "Usage:"
    echo "  ./scripts/dev.sh          # Browser mode (default)"
    echo "  ./scripts/dev.sh tauri    # Tauri mode"
    exit 1
fi
