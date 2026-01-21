#!/usr/bin/env python3
"""
Download platform icons from LaunchBox.

LaunchBox stores platform images at:
https://images.launchbox-app.com/Platforms/{Platform Name}/Clear%20Logo.png

Usage:
    python scripts/download_launchbox_platform_icons.py
"""

import os
import sqlite3
import time
import urllib.parse
import requests
from pathlib import Path

# Mapping from our canonical names to LaunchBox platform names
CANONICAL_TO_LAUNCHBOX = {
    "Aamber Pegasus": "Aamber Pegasus",
    "Apogee BK-01": "Apogee BK-01",
    "Camputers Lynx": "Camputers Lynx",
    "Casio PV-1000": "Casio PV-1000",
    "Coleco ADAM": "Coleco ADAM",
    "Commodore Amiga": "Commodore Amiga",
    "Commodore MAX Machine": "Commodore MAX Machine",
    "EACA EG2000 Colour Genie": "EACA Colour Genie",
    "Elektronika BK": "Elektronika BK",
    "Enterprise": "Enterprise",
    "Epoch Game Pocket Computer": "Epoch Game Pocket Computer",
    "Exelvision EXL 100": "Exelvision EXL 100",
    "Exidy Sorcerer": "Exidy Sorcerer",
    "Fujitsu FM Towns Marty": "Fujitsu FM Towns Marty",
    "Hector HRX": "Hector HRX",
    "Jupiter Ace": "Jupiter Ace",
    "MUGEN": "MUGEN",
    "Magnavox Odyssey": "Magnavox Odyssey",
    "Mattel Aquarius": "Mattel Aquarius",
    "Mattel HyperScan": "Mattel HyperScan",
    "Memotech MTX512": "Memotech MTX512",
    "Microsoft MSX2+": "Microsoft MSX2+",
    "Nintendo - Nintendo DSi": "Nintendo DSi",
    "Nintendo - Wii U (Digital)": "Nintendo Wii U",
    "OpenBOR": "OpenBOR",
    "Oric Atmos": "Oric Atmos",
    "Othello Multivision": "Othello Multivision",
    "Philips VG 5000": "Philips VG 5000",
    "Pinball": "Pinball",
    "RCA Studio II": "RCA Studio II",
    "SAM Coupé": "SAM Coupe",  # No accent in URL
    "Sega Dreamcast VMU": "Sega Dreamcast VMU",
    "Sega Triforce": "Sega Triforce",
    "Sharp MZ-2500": "Sharp MZ-2500",
    "Sord M5": "Sord M5",
    "Spectravideo": "Spectravideo",
    "Tomy Tutor": "Tomy Tutor",
    "VTech Socrates": "VTech Socrates",
    "Vector-06C": "Vector-06C",
    "WASM-4": "WASM-4",
    "WoW Action Max": "WoW Action Max",
    "XaviXPORT": "XaviXPORT",
    "VTech CreatiVision": "VTech CreatiVision",
    "Watara Supervision": "Watara Supervision",
}


def sanitize_filename(name: str) -> str:
    """Sanitize platform name for use as filename."""
    return name.replace("/", "-").replace(":", "-").replace("&", "and")


def download_launchbox_logo(platform_name: str, output_path: Path) -> bool:
    """Download a platform logo from LaunchBox."""
    # Try different image types
    image_types = ["Clear Logo", "Banner", "Device"]

    for img_type in image_types:
        url = f"https://images.launchbox-app.com/Platforms/{urllib.parse.quote(platform_name)}/{urllib.parse.quote(img_type)}.png"
        try:
            response = requests.get(url, timeout=30)
            if response.status_code == 200 and len(response.content) > 500:
                output_path.write_bytes(response.content)
                print(f"  Downloaded {img_type} for {platform_name}")
                return True
        except Exception as e:
            pass

    return False


def main():
    assets_dir = Path(__file__).parent.parent / "assets" / "platforms"
    placeholder_size = 1682  # Our placeholder icon size

    # Get platforms needing icons
    db_path = Path.home() / ".local/share/lunchbox/games.db"
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("SELECT name FROM platforms ORDER BY name")
    all_platforms = [row[0] for row in cursor.fetchall()]
    conn.close()

    need_icons = []
    for name in all_platforms:
        filename = sanitize_filename(name) + ".png"
        path = assets_dir / filename
        if not path.exists() or path.stat().st_size <= placeholder_size:
            need_icons.append(name)

    print(f"Platforms needing icons: {len(need_icons)}")

    downloaded = 0
    failed = []

    for platform_name in need_icons:
        filename = sanitize_filename(platform_name) + ".png"
        output_path = assets_dir / filename

        # Get LaunchBox name
        lb_name = CANONICAL_TO_LAUNCHBOX.get(platform_name, platform_name)

        print(f"Trying: {platform_name} -> {lb_name}")
        if download_launchbox_logo(lb_name, output_path):
            downloaded += 1
            time.sleep(0.5)  # Be nice to their server
        else:
            failed.append(platform_name)

    print(f"\nResults:")
    print(f"  Downloaded: {downloaded}")
    print(f"  Failed: {len(failed)}")

    if failed:
        print(f"\nStill missing ({len(failed)}):")
        for name in failed:
            print(f"  - {name}")


if __name__ == "__main__":
    main()
