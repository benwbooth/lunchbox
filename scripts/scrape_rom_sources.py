#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Iterate over all platforms and call Claude to research ROM/BIOS/firmware sources.

Finds the best and most up-to-date sources for complete ROM sets, BIOS files,
firmware, and encryption keys for each platform. Sources include archive.org,
myrient, pleasuredome, retro-exo, cdromance, vimm's lair, etc.

Usage:
    ./scripts/scrape_rom_sources.py                  # Run all remaining platforms
    ./scripts/scrape_rom_sources.py --parallel 3     # Run 3 in parallel
    ./scripts/scrape_rom_sources.py --dry-run        # Just print what would be done
    ./scripts/scrape_rom_sources.py --platform "Sega Genesis"  # Single platform
    ./scripts/scrape_rom_sources.py --combine-only   # Only combine existing CSVs
    ./scripts/scrape_rom_sources.py --min-games 50   # Skip platforms with <50 games
    ./scripts/scrape_rom_sources.py --limit 15       # Only process first 15 remaining
"""

import subprocess
import sqlite3
import os
import sys
import argparse
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED
import csv

DB_PATH = Path.home() / ".local/share/lunchbox/games.db"
OUTPUT_DIR = Path("rom_sources")
COMBINED_CSV = OUTPUT_DIR / "all_rom_sources.csv"

CSV_HEADER = "platform,source_name,url,content_type,set_format,download_method,size,requires_login,requires_bios,bios_included,bios_source,recommended,notes"

# Platforms to skip - modern/non-emulation platforms
SKIP_PLATFORMS = {
    "Windows",
    "Linux",
    "Sony Playstation 5",
    "Microsoft Xbox Series X/S",
    "Android",
    "Apple iOS",
    "Apple Mac OS",
    "Web Browser",
}

PROMPT_TEMPLATE = '''Research ROM/BIOS/firmware download sources for the "{platform}" platform.

Search for the best, most complete, and most up-to-date sources for:
1. Complete ROM sets (full collections for the platform)
2. BIOS files, firmware, and encryption keys needed for emulation
3. Individual game downloads if full sets aren't available

Check these sources (and any others you find):
- archive.org (Internet Archive) - search for "{platform}" ROM collections
- myrient.erista.me - check for No-Intro, Redump, or TOSEC sets
- pleasuredome (pleasuredome.github.io) - torrents for MAME/other sets
- retro-exo (exo.lc or exodos.the-eye.us) - ExoDOS/ExoWin3x/etc collections
- cdromance.org - disc-based game downloads
- vimm.net (Vimm's Lair) - individual ROM downloads
- edgeemulation.net
- r-roms (r/roms megathread links)
- nointro.org / redump.org / tosec.org - verification/dat files
- Any other well-known ROM preservation sites

For EACH source you find, determine:
- The exact URL to the collection/page
- What type of content it has (full_set, individual, bios_only, mixed)
- What set format it uses if applicable (No-Intro, Redump, TOSEC, GoodTools, or empty)
- Download method (direct, torrent, usenet, or empty)
- Approximate total size of the collection if available
- Whether the platform requires BIOS files for emulation
- Whether BIOS files are included in the set
- Where to get BIOS files separately if needed
- Whether this is the recommended/best source for this platform

Write a CSV file to {output_path}

CRITICAL: The CSV MUST have EXACTLY this header line and column order:
{csv_header}

Column definitions:
- platform: Always "{platform}"
- source_name: Name of the source (e.g., "Internet Archive - No-Intro Collection", "Myrient", "Vimm's Lair")
- url: Direct URL to the collection or page for this platform
- content_type: One of: full_set, individual, bios_only, mixed
- set_format: One of: No-Intro, Redump, TOSEC, GoodTools, or empty if not applicable
- download_method: One of: direct, torrent, usenet, or empty
- size: Approximate size (e.g., "2.1 GB", "450 MB", "1.2 TB") or empty if unknown
- requires_login: yes or no - whether the source requires creating an account or logging in to download
- requires_bios: yes or no - whether this platform needs BIOS/firmware files for emulation
- bios_included: yes, no, partial, or empty if not applicable
- bios_source: URL or description of where to get BIOS files, or empty
- recommended: yes or no - whether this is a recommended primary source
- notes: Any additional relevant info (e.g., "Updated monthly", "Requires free account", "Best for disc-based games")

Rules:
- Use semicolons (;) to separate multiple values within a field
- Use empty string for unknown/unavailable fields (not "N/A" or "unknown")
- Quote fields containing commas
- One row per source (multiple rows for the same platform is expected)
- Always include the header row first
- Mark the BEST source for full ROM sets as recommended=yes
- Mark the BEST source for BIOS files as recommended=yes (can have multiple recommended for different purposes)
- If no sources found, write header + one row with platform and "No sources found" in notes

Be thorough - actually visit the URLs to verify they work and check what's available for this specific platform.'''


def get_platforms(min_games: int = 10) -> list[tuple[str, int]]:
    """Get platform names and game counts from the database."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.execute("""
        SELECT p.name, COUNT(g.id) as game_count
        FROM platforms p
        LEFT JOIN games g ON g.platform_id = p.id
        GROUP BY p.name
        ORDER BY p.name ASC
    """)
    platforms = [(row[0], row[1]) for row in cursor.fetchall()]
    conn.close()
    return [(name, count) for name, count in platforms if count >= min_games]


def platform_to_filename(platform: str) -> str:
    """Convert platform name to a safe filename."""
    import re
    name = platform.lower()
    name = re.sub(r'[^a-z0-9]+', '_', name)
    name = re.sub(r'_+', '_', name)
    name = name.strip('_')
    return f"{name}.csv"


def is_done(platform: str) -> bool:
    """Check if platform has already been processed."""
    filename = platform_to_filename(platform)
    return (OUTPUT_DIR / filename).exists()


def combine_all_csvs() -> int:
    """
    Combine all individual platform CSVs into a single all_rom_sources.csv.
    Returns the number of source rows written.
    """
    all_rows = []
    csv_files = sorted(OUTPUT_DIR.glob("*.csv"))

    for csv_file in csv_files:
        if csv_file.name == "all_rom_sources.csv":
            continue

        try:
            with open(csv_file, newline='', encoding='utf-8') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    all_rows.append(row)
        except Exception as e:
            print(f"Warning: Failed to read {csv_file.name}: {e}")

    if not all_rows:
        print("No ROM source data found to combine.")
        return 0

    fieldnames = CSV_HEADER.split(',')
    with open(COMBINED_CSV, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in all_rows:
            clean_row = {k: row.get(k, '') for k in fieldnames}
            writer.writerow(clean_row)

    return len(all_rows)


def process_platform(platform: str, dry_run: bool = False) -> tuple[str, bool, str]:
    """
    Call Claude to research ROM sources for a platform.
    Returns (platform, success, message).
    """
    filename = platform_to_filename(platform)
    output_path = OUTPUT_DIR / filename
    tmp_path = OUTPUT_DIR / f".{filename}.tmp"

    prompt = PROMPT_TEMPLATE.format(
        platform=platform,
        output_path=tmp_path,
        csv_header=CSV_HEADER,
    )

    if dry_run:
        return (platform, True, f"Would process -> {filename}")

    # Clean up any leftover temp file from a previous failed run
    tmp_path.unlink(missing_ok=True)

    try:
        proc = subprocess.Popen(
            ["claude", "-p", prompt, "--allowedTools", "WebFetch,WebSearch,Write,Read"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        stdout, stderr = proc.communicate(timeout=600)

        if not tmp_path.exists():
            tmp_path.unlink(missing_ok=True)
            error = stderr.decode()[:200] if stderr else "No output file created"
            return (platform, False, f"Failed: {error}")

        # Validate CSV content
        try:
            with open(tmp_path, newline='', encoding='utf-8') as f:
                reader = csv.DictReader(f)
                expected = set(CSV_HEADER.split(','))
                actual = set(reader.fieldnames or [])
                if not expected.issubset(actual):
                    missing = expected - actual
                    tmp_path.unlink(missing_ok=True)
                    return (platform, False, f"Failed: CSV missing columns: {missing}")
                rows = list(reader)
                if not rows:
                    tmp_path.unlink(missing_ok=True)
                    return (platform, False, "Failed: CSV has header but no data rows")
        except Exception as e:
            tmp_path.unlink(missing_ok=True)
            return (platform, False, f"Failed: Invalid CSV: {e}")

        tmp_path.rename(output_path)
        return (platform, True, f"Created {filename} ({len(rows)} sources)")

    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
        tmp_path.unlink(missing_ok=True)
        return (platform, False, "Timeout after 10 minutes")
    except KeyboardInterrupt:
        proc.kill()
        proc.wait()
        tmp_path.unlink(missing_ok=True)
        raise
    except Exception as e:
        tmp_path.unlink(missing_ok=True)
        return (platform, False, f"Error: {e}")


def cleanup_temp_files():
    """Remove any leftover temp files."""
    for tmp in OUTPUT_DIR.glob(".*.tmp"):
        tmp.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description="Scrape ROM/BIOS/firmware sources for all platforms")
    parser.add_argument("--parallel", type=int, default=1, help="Number of parallel workers")
    parser.add_argument("--dry-run", action="store_true", help="Just print what would be done")
    parser.add_argument("--platform", type=str, help="Process only this specific platform")
    parser.add_argument("--combine-only", action="store_true", help="Only combine existing CSVs, don't scrape")
    parser.add_argument("--min-games", type=int, default=10, help="Skip platforms with fewer than N games (default: 10)")
    parser.add_argument("--limit", type=int, default=0, help="Only process first N remaining platforms")
    args = parser.parse_args()

    OUTPUT_DIR.mkdir(exist_ok=True)

    if args.combine_only:
        print("Combining all CSV files...")
        total_rows = combine_all_csvs()
        print(f"Combined {total_rows} ROM source entries into {COMBINED_CSV}")
        return

    all_platforms = get_platforms(min_games=args.min_games)

    # Filter out skip platforms
    platforms = [(name, count) for name, count in all_platforms if name not in SKIP_PLATFORMS]

    if args.platform:
        match = [(name, count) for name, count in platforms if name == args.platform]
        if not match:
            # Also check in skipped platforms in case user explicitly wants one
            all_match = [(name, count) for name, count in all_platforms if name == args.platform]
            if all_match:
                match = all_match
                print(f"Note: {args.platform} is normally skipped but processing since explicitly requested")
            else:
                print(f"Platform not found: {args.platform}")
                print(f"Available platforms with >= {args.min_games} games:")
                for name, count in platforms[:20]:
                    print(f"  {name} ({count} games)")
                sys.exit(1)
        remaining_names = [match[0][0]]
    else:
        remaining_names = [name for name, _ in platforms if not is_done(name)]

    if args.limit > 0:
        remaining_names = remaining_names[:args.limit]

    total = len(platforms)
    done = total - len([name for name, _ in platforms if not is_done(name)])

    print(f"Platforms: {total} total (skipping {len(all_platforms) - len(platforms)} non-ROM platforms), {done} done, {len(remaining_names)} remaining")

    if not remaining_names:
        print("All platforms processed!")
        total_rows = combine_all_csvs()
        print(f"Combined {total_rows} ROM source entries into {COMBINED_CSV}")
        return

    if args.dry_run:
        print("\nDry run - would process:")
        for p in remaining_names[:30]:
            filename = platform_to_filename(p)
            count = next((c for n, c in platforms if n == p), 0)
            print(f"  {p} ({count} games) -> {filename}")
        if len(remaining_names) > 30:
            print(f"  ... and {len(remaining_names) - 30} more")
        return

    print(f"\nProcessing {len(remaining_names)} platforms with {args.parallel} worker(s)...\n")

    success_count = 0
    fail_count = 0

    try:
        if args.parallel == 1:
            for i, platform in enumerate(remaining_names, 1):
                print(f"[{i}/{len(remaining_names)}] {platform}...", end=" ", flush=True)
                _, success, msg = process_platform(platform)
                print(msg)
                if success:
                    success_count += 1
                else:
                    fail_count += 1
                time.sleep(1)
        else:
            with ThreadPoolExecutor(max_workers=args.parallel) as executor:
                pending = {}
                remaining_iter = iter(remaining_names)
                completed = 0

                # Submit initial batch
                for _ in range(args.parallel):
                    p = next(remaining_iter, None)
                    if p:
                        pending[executor.submit(process_platform, p)] = p

                while pending:
                    done_futures, _ = wait(pending, return_when=FIRST_COMPLETED)
                    for future in done_futures:
                        platform = pending.pop(future)
                        completed += 1
                        _, success, msg = future.result()
                        print(f"[{completed}/{len(remaining_names)}] {platform}: {msg}")
                        if success:
                            success_count += 1
                        else:
                            fail_count += 1

                        # Submit next platform as each one completes
                        p = next(remaining_iter, None)
                        if p:
                            pending[executor.submit(process_platform, p)] = p

    except KeyboardInterrupt:
        print(f"\n\nCancelled!")
        cleanup_temp_files()

    print(f"\nDone! Success: {success_count}, Failed: {fail_count}")

    print("\nCombining all CSV files...")
    total_rows = combine_all_csvs()
    print(f"Combined {total_rows} ROM source entries into {COMBINED_CSV}")


if __name__ == "__main__":
    main()
