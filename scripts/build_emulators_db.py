#!/usr/bin/env python3
"""Build emulators.db from scraped CSV data.

This script reads emulator_details/all_emulators.csv and creates a SQLite database
with emulator information and platform mappings.
"""

import csv
import sqlite3
import subprocess
from pathlib import Path


def create_database(db_path: Path) -> sqlite3.Connection:
    """Create the emulators database with schema."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Create emulators table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS emulators (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            homepage TEXT,
            supported_os TEXT,
            winget_id TEXT,
            homebrew_formula TEXT,
            flatpak_id TEXT,
            retroarch_core TEXT,
            save_directory TEXT,
            save_extensions TEXT,
            notes TEXT
        )
    """)

    # Create platform_emulators junction table
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS platform_emulators (
            platform_name TEXT NOT NULL,
            emulator_id INTEGER NOT NULL REFERENCES emulators(id),
            is_recommended INTEGER DEFAULT 1,
            PRIMARY KEY (platform_name, emulator_id)
        )
    """)

    # Create indexes
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_platform_emulators_platform
        ON platform_emulators(platform_name)
    """)
    cursor.execute("""
        CREATE INDEX IF NOT EXISTS idx_emulators_retroarch
        ON emulators(retroarch_core)
    """)

    conn.commit()
    return conn


def import_csv(conn: sqlite3.Connection, csv_path: Path) -> tuple[int, int]:
    """Import emulator data from CSV.

    Returns tuple of (emulator_count, platform_mapping_count).
    """
    cursor = conn.cursor()

    # Track emulators we've already inserted (by name)
    emulator_ids: dict[str, int] = {}

    # Track platform mappings
    platform_mappings: list[tuple[str, int]] = []

    with open(csv_path, "r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            platform = row["platform"].strip()
            name = row["emulator_name"].strip()

            if not name:
                continue

            # Insert or get emulator ID
            if name not in emulator_ids:
                # Insert new emulator
                cursor.execute(
                    """
                    INSERT INTO emulators (
                        name, homepage, supported_os, winget_id, homebrew_formula,
                        flatpak_id, retroarch_core, save_directory, save_extensions, notes
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        name,
                        row.get("homepage", "").strip() or None,
                        row.get("supported_os", "").strip() or None,
                        row.get("winget_id", "").strip() or None,
                        row.get("homebrew_formula", "").strip() or None,
                        row.get("flatpak_id", "").strip() or None,
                        row.get("retroarch_core", "").strip() or None,
                        row.get("save_directory", "").strip() or None,
                        row.get("save_extensions", "").strip() or None,
                        row.get("notes", "").strip() or None,
                    ),
                )
                emulator_ids[name] = cursor.lastrowid

            # Record platform mapping
            emulator_id = emulator_ids[name]
            platform_mappings.append((platform, emulator_id))

    # Insert platform mappings
    for platform, emulator_id in platform_mappings:
        cursor.execute(
            """
            INSERT OR IGNORE INTO platform_emulators (platform_name, emulator_id, is_recommended)
            VALUES (?, ?, 1)
            """,
            (platform, emulator_id),
        )

    conn.commit()

    # Get counts
    cursor.execute("SELECT COUNT(*) FROM emulators")
    emulator_count = cursor.fetchone()[0]

    cursor.execute("SELECT COUNT(*) FROM platform_emulators")
    mapping_count = cursor.fetchone()[0]

    return emulator_count, mapping_count


def compress_database(db_path: Path) -> Path:
    """Compress database with zstd."""
    zst_path = db_path.with_suffix(".db.zst")
    subprocess.run(
        ["zstd", "-f", "-19", str(db_path), "-o", str(zst_path)],
        check=True,
    )
    return zst_path


def main():
    # Paths
    project_root = Path(__file__).parent.parent
    csv_path = project_root / "emulator_details" / "all_emulators.csv"
    db_dir = project_root / "db"
    db_path = db_dir / "emulators.db"

    # Ensure db directory exists
    db_dir.mkdir(parents=True, exist_ok=True)

    # Remove existing database
    if db_path.exists():
        db_path.unlink()

    print(f"Reading CSV from: {csv_path}")
    print(f"Creating database at: {db_path}")

    # Create database and import data
    conn = create_database(db_path)
    emulator_count, mapping_count = import_csv(conn, csv_path)
    conn.close()

    print(f"Imported {emulator_count} emulators")
    print(f"Created {mapping_count} platform mappings")

    # Compress database
    print("Compressing database...")
    zst_path = compress_database(db_path)
    print(f"Compressed database at: {zst_path}")

    # Print some stats
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Top emulators by platform count
    cursor.execute("""
        SELECT e.name, COUNT(pe.platform_name) as platform_count
        FROM emulators e
        JOIN platform_emulators pe ON e.id = pe.emulator_id
        GROUP BY e.id
        ORDER BY platform_count DESC
        LIMIT 10
    """)
    print("\nTop emulators by platform count:")
    for name, count in cursor.fetchall():
        print(f"  {name}: {count} platforms")

    # Platforms with most emulators
    cursor.execute("""
        SELECT platform_name, COUNT(emulator_id) as emulator_count
        FROM platform_emulators
        GROUP BY platform_name
        ORDER BY emulator_count DESC
        LIMIT 10
    """)
    print("\nPlatforms with most emulators:")
    for name, count in cursor.fetchall():
        print(f"  {name}: {count} emulators")

    conn.close()


if __name__ == "__main__":
    main()
