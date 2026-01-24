#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Iterate over all platforms and call Claude to research emulators for each one.

Usage:
    ./scripts/scrape_all_emulators.py              # Run all remaining platforms
    ./scripts/scrape_all_emulators.py --parallel 3 # Run 3 in parallel
    ./scripts/scrape_all_emulators.py --dry-run    # Just print what would be done
"""

import subprocess
import sqlite3
import os
import sys
import argparse
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
import csv

DB_PATH = Path.home() / ".local/share/lunchbox/games.db"
OUTPUT_DIR = Path("emulator_details")
COMBINED_CSV = OUTPUT_DIR / "all_emulators.csv"
WIKI_BASE = "https://emulation.gametechwiki.com"

CSV_HEADER = "platform,emulator_name,supported_os,homepage,winget_id,homebrew_formula,flatpak_id,retroarch_core,save_directory,save_extensions,notes"

PROMPT_TEMPLATE = '''Research emulators for "{platform}" on the Emulation General Wiki.

1. First, use WebSearch or WebFetch to find the emulator page for this system on emulation.gametechwiki.com
   Try URLs like: {wiki_base}/index.php/{search_name}_emulators

2. Find the RECOMMENDED emulators (look for checkmarks, green cells, or "Recommended" in tables)

3. For EACH recommended emulator, research:
   - Emulator name
   - Supported OSes (Windows, Linux, macOS, Android, iOS)
   - Official homepage URL
   - Installation methods:
     * Windows: Check if available via `winget search <name>` - provide the winget package ID if found
     * macOS: Check if available via Homebrew - provide the formula/cask name if found
     * Linux: Check if available as Flatpak - provide the Flatpak app ID if found
   - Whether it's available as a libretro/RetroArch core (and the core name if so)
   - Save game information:
     * Default save directory path (e.g., ~/.config/emulator/saves or %APPDATA%/emulator/saves)
     * Save file extensions used (e.g., .sav, .srm, .state, .ss0)
     Search the emulator's documentation, wiki, or GitHub repo for this info.

4. Write a CSV file to {output_path}

CRITICAL: The CSV MUST have EXACTLY this header line and column order:
{csv_header}

Column definitions:
- platform: Always "{platform}" (the system being emulated)
- emulator_name: Name of the emulator (e.g., "Dolphin", "mGBA")
- supported_os: OS support, semicolon-separated (e.g., "Windows;Linux;macOS")
- homepage: Official website URL
- winget_id: Windows winget package ID (e.g., "Dolphin.Dolphin") or empty
- homebrew_formula: Homebrew formula/cask name (e.g., "dolphin" or "homebrew/cask/dolphin") or empty
- flatpak_id: Flatpak app ID (e.g., "org.DolphinEmu.dolphin-emu") or empty
- retroarch_core: RetroArch/libretro core name (e.g., "mgba") or empty if not available
- save_directory: Default save path using Linux convention (e.g., "~/.config/dolphin-emu/GC")
- save_extensions: Save file extensions, semicolon-separated (e.g., ".sav;.srm")
- notes: Any additional relevant info

Rules:
- Use semicolons (;) to separate multiple values within a field
- Use empty string for unknown/unavailable fields (not "N/A" or "unknown")
- Quote fields containing commas
- One row per emulator (if an emulator has both standalone and RetroArch core, that's one row)
- Always include the header row first
- If no recommended emulators found, write header + one row with platform and "No recommended emulators found" in notes

Be thorough - check multiple sources if needed to find package manager IDs and save file information.'''


def get_platforms() -> list[str]:
    """Get all platform names from the database."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.execute("SELECT name FROM platforms ORDER BY name")
    platforms = [row[0] for row in cursor.fetchall()]
    conn.close()
    return platforms


def platform_to_filename(platform: str) -> str:
    """Convert platform name to a safe filename."""
    import re
    name = platform.lower()
    name = re.sub(r'[^a-z0-9]+', '_', name)
    name = re.sub(r'_+', '_', name)
    name = name.strip('_')
    return f"{name}.csv"


def platform_to_search_name(platform: str) -> str:
    """Convert platform name to wiki search format."""
    # Common mappings for wiki URLs
    mappings = {
        "Nintendo Entertainment System": "Nintendo_Entertainment_System",
        "Super Nintendo Entertainment System": "Super_Nintendo_Entertainment_System",
        "Sony Playstation": "PlayStation",
        "Sony Playstation 2": "PlayStation_2",
        "Sony Playstation 3": "PlayStation_3",
        "Sony Playstation 4": "PlayStation_4",
        "Sony Playstation 5": "PlayStation_5",
        "Sony PSP": "PlayStation_Portable",
        "Sony Playstation Vita": "PlayStation_Vita",
        "Sega Genesis": "Sega_Genesis",
        "Sega Dreamcast": "Dreamcast",
        "Sega Saturn": "Sega_Saturn",
        "Sega Master System": "Master_System",
        "Sega Game Gear": "Master_System/Game_Gear",
        "Microsoft Xbox": "Xbox",
        "Microsoft Xbox 360": "Xbox_360",
        "Microsoft Xbox One": "Xbox_One",
        "NEC TurboGrafx-16": "PC_Engine_(TurboGrafx-16)",
        "NEC TurboGrafx-CD": "PC_Engine_(TurboGrafx-16)",
        "Commodore Amiga": "Amiga",
        "Commodore 64": "Commodore_64",
        "Atari 2600": "Atari_2600",
        "Atari 5200": "Atari_5200",
        "Atari 7800": "Atari_7800",
        "Atari Jaguar": "Atari_Jaguar",
        "Atari Lynx": "Atari_Lynx",
        "Atari ST": "Atari_ST",
        "SNK Neo Geo AES": "Neo_Geo",
        "SNK Neo Geo MVS": "Neo_Geo",
        "SNK Neo Geo CD": "Neo_Geo",
        "SNK Neo Geo Pocket": "Neo_Geo_Pocket",
        "SNK Neo Geo Pocket Color": "Neo_Geo_Pocket",
    }

    if platform in mappings:
        return mappings[platform]

    # Default: replace spaces with underscores
    return platform.replace(" ", "_").replace("-", "_")


def is_done(platform: str) -> bool:
    """Check if platform has already been processed."""
    filename = platform_to_filename(platform)
    return (OUTPUT_DIR / filename).exists()


def combine_all_csvs() -> int:
    """
    Combine all individual platform CSVs into a single all_emulators.csv.
    Returns the number of emulator rows written.
    """
    all_rows = []
    csv_files = sorted(OUTPUT_DIR.glob("*.csv"))

    for csv_file in csv_files:
        # Skip the combined file itself
        if csv_file.name == "all_emulators.csv":
            continue

        try:
            with open(csv_file, newline='', encoding='utf-8') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    all_rows.append(row)
        except Exception as e:
            print(f"Warning: Failed to read {csv_file.name}: {e}")

    if not all_rows:
        print("No emulator data found to combine.")
        return 0

    # Write combined CSV
    fieldnames = CSV_HEADER.split(',')
    with open(COMBINED_CSV, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in all_rows:
            # Ensure all fields exist (fill missing with empty string)
            clean_row = {k: row.get(k, '') for k in fieldnames}
            writer.writerow(clean_row)

    return len(all_rows)


def process_platform(platform: str, dry_run: bool = False) -> tuple[str, bool, str]:
    """
    Call Claude to research emulators for a platform.
    Returns (platform, success, message).
    """
    filename = platform_to_filename(platform)
    output_path = OUTPUT_DIR / filename
    search_name = platform_to_search_name(platform)

    prompt = PROMPT_TEMPLATE.format(
        platform=platform,
        wiki_base=WIKI_BASE,
        search_name=search_name,
        output_path=output_path,
        csv_header=CSV_HEADER,
    )

    if dry_run:
        return (platform, True, f"Would process -> {filename}")

    try:
        # Call Claude CLI
        result = subprocess.run(
            ["claude", "-p", prompt, "--allowedTools", "WebFetch,WebSearch,Write,Read"],
            capture_output=True,
            text=True,
            timeout=300,  # 5 minute timeout per platform
        )

        if result.returncode == 0 and output_path.exists():
            return (platform, True, f"Created {filename}")
        else:
            error = result.stderr[:200] if result.stderr else "Unknown error"
            return (platform, False, f"Failed: {error}")

    except subprocess.TimeoutExpired:
        return (platform, False, "Timeout after 5 minutes")
    except Exception as e:
        return (platform, False, f"Error: {e}")


def main():
    parser = argparse.ArgumentParser(description="Scrape emulator info for all platforms")
    parser.add_argument("--parallel", type=int, default=1, help="Number of parallel workers")
    parser.add_argument("--dry-run", action="store_true", help="Just print what would be done")
    parser.add_argument("--platform", type=str, help="Process only this specific platform")
    parser.add_argument("--combine-only", action="store_true", help="Only combine existing CSVs, don't scrape")
    args = parser.parse_args()

    OUTPUT_DIR.mkdir(exist_ok=True)

    if args.combine_only:
        print("Combining all CSV files...")
        total_rows = combine_all_csvs()
        print(f"Combined {total_rows} emulator entries into {COMBINED_CSV}")
        return

    platforms = get_platforms()

    if args.platform:
        # Process single platform
        if args.platform not in platforms:
            print(f"Platform not found: {args.platform}")
            sys.exit(1)
        remaining = [args.platform]
    else:
        # Filter to remaining platforms
        remaining = [p for p in platforms if not is_done(p)]

    total = len(platforms)
    done = total - len(remaining)

    print(f"Platforms: {total} total, {done} done, {len(remaining)} remaining")

    if not remaining:
        print("All platforms processed!")
        return

    if args.dry_run:
        print("\nDry run - would process:")
        for p in remaining[:20]:
            filename = platform_to_filename(p)
            print(f"  {p} -> {filename}")
        if len(remaining) > 20:
            print(f"  ... and {len(remaining) - 20} more")
        return

    print(f"\nProcessing {len(remaining)} platforms with {args.parallel} worker(s)...\n")

    success_count = 0
    fail_count = 0

    if args.parallel == 1:
        # Sequential processing
        for i, platform in enumerate(remaining, 1):
            print(f"[{i}/{len(remaining)}] {platform}...", end=" ", flush=True)
            _, success, msg = process_platform(platform)
            print(msg)
            if success:
                success_count += 1
            else:
                fail_count += 1
            # Small delay between requests
            time.sleep(1)
    else:
        # Parallel processing
        with ThreadPoolExecutor(max_workers=args.parallel) as executor:
            futures = {executor.submit(process_platform, p): p for p in remaining}
            for i, future in enumerate(as_completed(futures), 1):
                platform = futures[future]
                _, success, msg = future.result()
                print(f"[{i}/{len(remaining)}] {platform}: {msg}")
                if success:
                    success_count += 1
                else:
                    fail_count += 1

    print(f"\nDone! Success: {success_count}, Failed: {fail_count}")

    # Combine all CSVs into a single file
    print("\nCombining all CSV files...")
    total_rows = combine_all_csvs()
    print(f"Combined {total_rows} emulator entries into {COMBINED_CSV}")


if __name__ == "__main__":
    main()
