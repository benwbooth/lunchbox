#!/usr/bin/env python3
"""
Download platform icons from IGDB.

Usage:
    export TWITCH_CLIENT_ID=your_client_id
    export TWITCH_CLIENT_SECRET=your_client_secret
    python scripts/download_igdb_platform_icons.py

Get credentials at: https://dev.twitch.tv/console/apps
"""

import os
import sys
import json
import time
import sqlite3
import requests
from pathlib import Path

# IGDB platform ID to our canonical platform name mapping
IGDB_TO_CANONICAL = {
    3: "Linux",
    4: "Nintendo 64",
    5: "Nintendo Wii",
    6: "Windows",
    7: "Sony Playstation",
    8: "Sony Playstation 2",
    9: "Sony Playstation 3",
    11: "Microsoft Xbox",
    12: "Microsoft Xbox 360",
    13: "MS-DOS",
    14: "Apple Mac OS",
    15: "Commodore 64",
    16: "Commodore Amiga",
    18: "Nintendo Entertainment System",
    19: "Super Nintendo Entertainment System",
    20: "Nintendo DS",
    21: "Nintendo GameCube",
    22: "Nintendo Game Boy Color",
    23: "Sega Dreamcast",
    24: "Nintendo Game Boy Advance",
    25: "Amstrad CPC",
    26: "Sinclair ZX Spectrum",
    27: "Microsoft MSX",
    29: "Sega Genesis",
    30: "Sega 32X",
    32: "Sega Saturn",
    33: "Nintendo Game Boy",
    34: "Android",
    35: "Sega Game Gear",
    37: "Nintendo 3DS",
    38: "Sony PSP",
    39: "Apple iOS",
    41: "Nintendo Wii U",
    42: "Nokia N-Gage",
    44: "Tapwave Zodiac",
    46: "Sony Playstation Vita",
    48: "Sony Playstation 4",
    49: "Microsoft Xbox One",
    50: "3DO Interactive Multiplayer",
    51: "Nintendo Famicom Disk System",
    52: "Arcade",
    53: "Microsoft MSX2",
    57: "WonderSwan",
    59: "Atari 2600",
    60: "Atari 7800",
    61: "Atari Lynx",
    62: "Atari Jaguar",
    63: "Atari ST",
    64: "Sega Master System",
    65: "Atari 800",
    66: "Atari 5200",
    67: "Mattel Intellivision",
    68: "ColecoVision",
    69: "BBC Microcomputer System",
    70: "GCE Vectrex",
    71: "Commodore VIC-20",
    72: "Ouya",
    75: "Apple II",
    77: "Sharp X1",
    78: "Sega CD",
    79: "SNK Neo Geo MVS",
    80: "SNK Neo Geo AES",
    82: "Web Browser",
    84: "Sega SG-1000",
    86: "NEC TurboGrafx-16",
    87: "Nintendo Virtual Boy",
    88: "Magnavox Odyssey",
    90: "Commodore PET",
    91: "Bally Astrocade",
    94: "Commodore Plus 4",
    99: "Nintendo Entertainment System",  # Famicom
    114: "Commodore Amiga CD32",
    115: "Apple IIGS",
    116: "Acorn Archimedes",
    117: "Philips CD-i",
    118: "Fujitsu FM Towns Marty",
    119: "SNK Neo Geo Pocket",
    120: "SNK Neo Geo Pocket Color",
    121: "Sharp X68000",
    122: "Nuon",
    123: "WonderSwan Color",
    125: "NEC PC-8801",
    126: "Tandy TRS-80",
    127: "Fairchild Channel F",
    128: "PC Engine SuperGrafx",
    129: "Texas Instruments TI 99/4A",
    130: "Nintendo Switch",
    132: "Amazon Fire TV",
    133: "Philips Videopac+",
    134: "Acorn Electron",
    136: "SNK Neo Geo CD",
    137: "Nintendo 3DS",  # New 3DS
    138: "Interton VC 4000",
    149: "NEC PC-9801",
    150: "NEC TurboGrafx-CD",
    151: "TRS-80 Color Computer",
    152: "Fujitsu FM-7",
    153: "Dragon 32/64",
    156: "Matra and Hachette Alice",  # Thomson MO5
    158: "Commodore CDTV",
    159: "Nintendo DSi",
    166: "Nintendo Pokemon Mini",
    167: "Sony Playstation 5",
    169: "Microsoft Xbox Series X/S",
    386: "Sega Pico",
    471: "PICO-8",
    508: "Nintendo Switch 2",
}

# Reverse mapping for lookup
CANONICAL_TO_IGDB = {v: k for k, v in IGDB_TO_CANONICAL.items()}


def get_oauth_token(client_id: str, client_secret: str) -> str:
    """Get OAuth token from Twitch."""
    response = requests.post(
        "https://id.twitch.tv/oauth2/token",
        data={
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "client_credentials",
        },
    )
    response.raise_for_status()
    return response.json()["access_token"]


def get_platforms_with_logos(client_id: str, token: str) -> list:
    """Fetch all platforms with their logos from IGDB."""
    response = requests.post(
        "https://api.igdb.com/v4/platforms",
        headers={
            "Client-ID": client_id,
            "Authorization": f"Bearer {token}",
            "Accept": "application/json",
        },
        data="fields id,name,platform_logo.image_id; limit 500;",
    )
    response.raise_for_status()
    return response.json()


def download_logo(image_id: str, output_path: Path) -> bool:
    """Download a platform logo from IGDB."""
    # Use logo_med size (284x160)
    url = f"https://images.igdb.com/igdb/image/upload/t_logo_med/{image_id}.png"
    try:
        response = requests.get(url, timeout=30)
        if response.status_code == 200 and len(response.content) > 100:
            output_path.write_bytes(response.content)
            return True
    except Exception as e:
        print(f"  Error downloading {image_id}: {e}")
    return False


def sanitize_filename(name: str) -> str:
    """Sanitize platform name for use as filename."""
    return name.replace("/", "-").replace(":", "-").replace("&", "and")


def main():
    # Get credentials from environment
    client_id = os.environ.get("TWITCH_CLIENT_ID")
    client_secret = os.environ.get("TWITCH_CLIENT_SECRET")

    if not client_id or not client_secret:
        print("Error: TWITCH_CLIENT_ID and TWITCH_CLIENT_SECRET environment variables required")
        print("Get credentials at: https://dev.twitch.tv/console/apps")
        sys.exit(1)

    assets_dir = Path(__file__).parent.parent / "assets" / "platforms"
    assets_dir.mkdir(parents=True, exist_ok=True)

    # Get our platform list from database
    db_path = Path.home() / ".local/share/lunchbox/games.db"
    if db_path.exists():
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute("SELECT name FROM platforms")
        our_platforms = {row[0] for row in cursor.fetchall()}
        conn.close()
        print(f"Found {len(our_platforms)} platforms in database")
    else:
        our_platforms = set(CANONICAL_TO_IGDB.keys())
        print(f"No database found, using {len(our_platforms)} known platforms")

    # Get OAuth token
    print("Getting OAuth token...")
    token = get_oauth_token(client_id, client_secret)

    # Fetch platforms from IGDB
    print("Fetching platforms from IGDB...")
    igdb_platforms = get_platforms_with_logos(client_id, token)
    print(f"Found {len(igdb_platforms)} platforms on IGDB")

    # Build IGDB ID to logo mapping
    igdb_logos = {}
    for p in igdb_platforms:
        if p.get("platform_logo") and p["platform_logo"].get("image_id"):
            igdb_logos[p["id"]] = {
                "name": p["name"],
                "image_id": p["platform_logo"]["image_id"],
            }

    print(f"Found {len(igdb_logos)} platforms with logos")

    # Download logos for our platforms
    downloaded = 0
    skipped = 0
    missing = []

    for platform_name in sorted(our_platforms):
        filename = sanitize_filename(platform_name) + ".png"
        output_path = assets_dir / filename

        # Check if we already have a good icon (not the placeholder)
        if output_path.exists():
            size = output_path.stat().st_size
            # Skip if file exists and is larger than placeholder (1682 bytes)
            if size > 2000:
                skipped += 1
                continue

        # Find IGDB ID for this platform
        igdb_id = CANONICAL_TO_IGDB.get(platform_name)
        if igdb_id and igdb_id in igdb_logos:
            logo_info = igdb_logos[igdb_id]
            print(f"Downloading: {platform_name} <- IGDB:{logo_info['name']}")
            if download_logo(logo_info["image_id"], output_path):
                downloaded += 1
                time.sleep(0.25)  # Rate limit: 4 req/sec
            else:
                missing.append(platform_name)
        else:
            missing.append(platform_name)

    print(f"\nResults:")
    print(f"  Downloaded: {downloaded}")
    print(f"  Skipped (already have): {skipped}")
    print(f"  Missing: {len(missing)}")

    if missing:
        print(f"\nPlatforms still missing icons ({len(missing)}):")
        for name in sorted(missing):
            print(f"  - {name}")


if __name__ == "__main__":
    main()
