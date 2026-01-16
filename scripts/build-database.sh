#!/bin/bash
set -euo pipefail

# Build enriched Lunchbox game database
# Downloads source databases and runs enrichment pipeline

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DATA_DIR="${DATA_DIR:-$PROJECT_DIR/data}"
OUTPUT_DB="${OUTPUT_DB:-$PROJECT_DIR/lunchbox-games.db}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

header() {
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  $1${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
}

# Check dependencies
check_deps() {
    local missing=()
    for cmd in git curl unzip cargo; do
        if ! command -v "$cmd" &> /dev/null; then
            missing+=("$cmd")
        fi
    done

    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing[*]}"
        exit 1
    fi
}

# Create data directory
setup_dirs() {
    mkdir -p "$DATA_DIR"
    log_success "Data directory: $DATA_DIR"
}

# Download LibRetro database (git clone)
download_libretro() {
    header "LibRetro Database"

    local libretro_dir="$DATA_DIR/libretro-database"

    if [ -d "$libretro_dir/.git" ]; then
        log_info "LibRetro database exists, updating..."
        (cd "$libretro_dir" && git pull --quiet)
        log_success "LibRetro database updated"
    else
        log_info "Cloning LibRetro database (~200MB)..."
        rm -rf "$libretro_dir"
        git clone --depth 1 https://github.com/libretro/libretro-database "$libretro_dir"
        log_success "LibRetro database downloaded"
    fi
}

# Download OpenVGDB
download_openvgdb() {
    header "OpenVGDB"

    local openvgdb_zip="$DATA_DIR/openvgdb.zip"
    local openvgdb_db="$DATA_DIR/openvgdb.sqlite"

    if [ -f "$openvgdb_db" ]; then
        log_info "OpenVGDB already exists, skipping download"
    else
        log_info "Downloading OpenVGDB..."
        # OpenVGDB releases on GitHub
        local openvgdb_url="https://github.com/OpenVGDB/OpenVGDB/releases/latest/download/openvgdb.zip"
        curl -L -o "$openvgdb_zip" "$openvgdb_url"

        log_info "Extracting OpenVGDB..."
        unzip -o -q "$openvgdb_zip" -d "$DATA_DIR"
        rm "$openvgdb_zip"

        # Rename if needed (the zip contains openvgdb.sqlite)
        if [ -f "$DATA_DIR/OpenVGDB.sqlite" ]; then
            mv "$DATA_DIR/OpenVGDB.sqlite" "$openvgdb_db"
        fi

        log_success "OpenVGDB downloaded"
    fi
}

# Download LaunchBox metadata
download_launchbox() {
    header "LaunchBox Metadata"

    local launchbox_zip="$DATA_DIR/launchbox-metadata.zip"
    local launchbox_dir="$DATA_DIR/launchbox-metadata"
    local launchbox_xml="$launchbox_dir/Metadata.xml"

    if [ -f "$launchbox_xml" ]; then
        log_info "LaunchBox metadata already exists, skipping download"
    else
        log_info "Downloading LaunchBox metadata (~100MB)..."
        curl -L -o "$launchbox_zip" "https://gamesdb.launchbox-app.com/Metadata.zip"

        log_info "Extracting LaunchBox metadata..."
        mkdir -p "$launchbox_dir"
        unzip -o -q "$launchbox_zip" -d "$launchbox_dir"
        rm "$launchbox_zip"

        log_success "LaunchBox metadata downloaded"
    fi
}

# Build the CLI tool
build_cli() {
    header "Building lunchbox-cli"

    log_info "Compiling lunchbox-cli (release mode)..."
    (cd "$PROJECT_DIR" && cargo build --release -p lunchbox-cli)
    log_success "lunchbox-cli built"
}

# Build the game database from LibRetro
build_database() {
    header "Building Game Database"

    local libretro_dir="$DATA_DIR/libretro-database"
    local cli="$PROJECT_DIR/target/release/lunchbox-cli"

    log_info "Parsing LibRetro DAT files..."
    "$cli" build-db --libretro-path "$libretro_dir" --output "$OUTPUT_DB"
    log_success "Base database built"
}

# Enrich with OpenVGDB
enrich_openvgdb() {
    header "Enriching with OpenVGDB"

    local openvgdb_db="$DATA_DIR/openvgdb.sqlite"
    local cli="$PROJECT_DIR/target/release/lunchbox-cli"

    if [ ! -f "$openvgdb_db" ]; then
        log_warn "OpenVGDB not found, skipping"
        return
    fi

    log_info "Running OpenVGDB enrichment (CRC + fuzzy matching)..."
    "$cli" enrich-db --database "$OUTPUT_DB" --openvgdb "$openvgdb_db"
    log_success "OpenVGDB enrichment complete"
}

# Enrich with LaunchBox
enrich_launchbox() {
    header "Enriching with LaunchBox"

    local launchbox_xml="$DATA_DIR/launchbox-metadata/Metadata.xml"
    local cli="$PROJECT_DIR/target/release/lunchbox-cli"

    if [ ! -f "$launchbox_xml" ]; then
        log_warn "LaunchBox metadata not found, skipping"
        return
    fi

    log_info "Running LaunchBox enrichment (title matching)..."
    "$cli" enrich-launchbox --database "$OUTPUT_DB" --metadata-xml "$launchbox_xml"
    log_success "LaunchBox enrichment complete"
}

# Print final stats
print_stats() {
    header "Final Database Statistics"

    if [ -f "$OUTPUT_DB" ]; then
        local size=$(du -h "$OUTPUT_DB" | cut -f1)
        echo "Output: $OUTPUT_DB ($size)"
        echo ""

        # Query stats using sqlite3 if available
        if command -v sqlite3 &> /dev/null; then
            local total=$(sqlite3 "$OUTPUT_DB" "SELECT COUNT(*) FROM games")
            local platforms=$(sqlite3 "$OUTPUT_DB" "SELECT COUNT(*) FROM platforms")
            local with_desc=$(sqlite3 "$OUTPUT_DB" "SELECT COUNT(*) FROM games WHERE description IS NOT NULL AND description != ''")
            local with_dev=$(sqlite3 "$OUTPUT_DB" "SELECT COUNT(*) FROM games WHERE developer IS NOT NULL AND developer != ''")
            local with_genre=$(sqlite3 "$OUTPUT_DB" "SELECT COUNT(*) FROM games WHERE genre IS NOT NULL AND genre != ''")

            echo "Platforms:        $platforms"
            echo "Total games:      $total"
            echo "With description: $with_desc ($(echo "scale=1; $with_desc * 100 / $total" | bc)%)"
            echo "With developer:   $with_dev ($(echo "scale=1; $with_dev * 100 / $total" | bc)%)"
            echo "With genre:       $with_genre ($(echo "scale=1; $with_genre * 100 / $total" | bc)%)"
        fi

        echo ""
        log_success "Database build complete!"
    else
        log_error "Database not found at $OUTPUT_DB"
        exit 1
    fi
}

# Main
main() {
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║  Lunchbox Database Builder             ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""

    check_deps
    setup_dirs

    # Download sources
    download_libretro
    download_openvgdb
    download_launchbox

    # Build
    build_cli
    build_database

    # Enrich
    enrich_openvgdb
    enrich_launchbox

    # Stats
    print_stats
}

# Handle arguments
case "${1:-}" in
    --help|-h)
        echo "Usage: $0 [options]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help"
        echo "  --clean        Remove data directory and start fresh"
        echo "  --skip-download  Skip downloading sources (use existing)"
        echo ""
        echo "Environment variables:"
        echo "  DATA_DIR       Directory for downloaded sources (default: ./data)"
        echo "  OUTPUT_DB      Output database path (default: ./lunchbox-games.db)"
        exit 0
        ;;
    --clean)
        log_warn "Removing data directory..."
        rm -rf "$DATA_DIR"
        rm -f "$OUTPUT_DB"
        main
        ;;
    --skip-download)
        check_deps
        setup_dirs
        build_cli
        build_database
        enrich_openvgdb
        enrich_launchbox
        print_stats
        ;;
    *)
        main
        ;;
esac
