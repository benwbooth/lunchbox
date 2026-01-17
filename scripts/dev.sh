#!/usr/bin/env bash
# Development script for Lunchbox
#
# Usage:
#   ./scripts/dev.sh          # Browser mode (default) - opens in your browser
#   ./scripts/dev.sh tauri    # Tauri mode - embedded webview

set -e

MODE="${1:-browser}"

# Kill any existing dev processes first
echo "Cleaning up old processes..."
pkill -f "trunk serve.*1420" 2>/dev/null || true
pkill -f "dev_server" 2>/dev/null || true
pkill -f "cargo watch.*dev_server" 2>/dev/null || true
sleep 1

cleanup() {
    echo ""
    echo "Shutting down..."
    jobs -p | xargs -r kill 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

if [ "$MODE" = "tauri" ]; then
    echo "Starting Tauri development mode..."
    cargo tauri dev

elif [ "$MODE" = "browser" ]; then
    echo "Starting browser development mode..."
    echo ""

    # Start trunk (frontend) with hot reload
    echo "Starting frontend (trunk) on http://127.0.0.1:1420..."
    trunk serve --port 1420 &

    # Give trunk a moment to start
    sleep 2

    # Start backend with cargo-watch for auto-restart on changes
    echo "Starting backend API server on http://127.0.0.1:3001..."
    echo "(Backend will auto-restart on changes to src-tauri/)"
    cargo watch -w src-tauri -x "run -p lunchbox --bin dev_server" &

    # Wait for backend to compile and start
    sleep 5

    # Open browser
    echo ""
    echo "Opening browser..."
    xdg-open http://127.0.0.1:1420 2>/dev/null || open http://127.0.0.1:1420 2>/dev/null || echo "Please open http://127.0.0.1:1420 in your browser"

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  Frontend:  http://127.0.0.1:1420 (auto-reloads)"
    echo "  API:       http://127.0.0.1:3001 (auto-restarts)"
    echo ""
    echo "  Press Ctrl+C to stop"
    echo "═══════════════════════════════════════════════════════"

    # Wait for background jobs
    wait

else
    echo "Unknown mode: $MODE"
    echo ""
    echo "Usage:"
    echo "  ./scripts/dev.sh          # Browser mode (default)"
    echo "  ./scripts/dev.sh tauri    # Tauri mode"
    exit 1
fi
