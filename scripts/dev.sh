#!/usr/bin/env bash
# Development script for Lunchbox
#
# Usage:
#   ./scripts/dev.sh          # Browser mode (default) - opens in your browser
#   ./scripts/dev.sh tauri    # Tauri mode - embedded webview

set -e

MODE="${1:-browser}"

cleanup() {
    echo ""
    echo "Shutting down..."
    kill $TRUNK_PID 2>/dev/null || true
    kill $SERVER_PID 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

if [ "$MODE" = "tauri" ]; then
    echo "Starting Tauri development mode..."
    cargo tauri dev

elif [ "$MODE" = "browser" ]; then
    echo "Starting browser development mode..."
    echo ""

    # Start trunk in background
    echo "Starting frontend (trunk) on http://127.0.0.1:1420..."
    trunk serve --port 1420 &
    TRUNK_PID=$!

    # Give trunk a moment to start
    sleep 2

    # Start the dev server in background
    echo "Starting backend API server on http://127.0.0.1:3001..."
    cargo run -p lunchbox --bin dev_server &
    SERVER_PID=$!

    # Wait for backend to be ready
    sleep 3

    # Open browser
    echo ""
    echo "Opening browser..."
    xdg-open http://127.0.0.1:1420 2>/dev/null || open http://127.0.0.1:1420 2>/dev/null || echo "Please open http://127.0.0.1:1420 in your browser"

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  Frontend:  http://127.0.0.1:1420"
    echo "  API:       http://127.0.0.1:3001"
    echo ""
    echo "  Press Ctrl+C to stop"
    echo "═══════════════════════════════════════════════════════"

    # Wait for either process to exit
    wait $TRUNK_PID $SERVER_PID

else
    echo "Unknown mode: $MODE"
    echo ""
    echo "Usage:"
    echo "  ./scripts/dev.sh          # Browser mode (default)"
    echo "  ./scripts/dev.sh tauri    # Tauri mode"
    exit 1
fi
