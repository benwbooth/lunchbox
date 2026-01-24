#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "requests",
#     "beautifulsoup4",
# ]
# ///
"""
Scrape emulation.gametechwiki.com to build a list of recommended emulators per system.
"""

import requests
from bs4 import BeautifulSoup
import json
import re
import time

BASE_URL = "https://emulation.gametechwiki.com"
MAIN_PAGE = f"{BASE_URL}/index.php/Main_Page"

def get_soup(url):
    """Fetch page and return BeautifulSoup object."""
    time.sleep(0.5)  # Be nice to the server
    headers = {'User-Agent': 'Mozilla/5.0 (compatible; LunchboxBot/1.0)'}
    resp = requests.get(url, headers=headers)
    resp.raise_for_status()
    return BeautifulSoup(resp.text, 'html.parser')

def get_system_links():
    """Get all system emulator page links from main page."""
    soup = get_soup(MAIN_PAGE)
    systems = {}

    # Find all links ending in "_emulators"
    for link in soup.find_all('a', href=True):
        href = link.get('href', '')
        if '_emulators' in href and '/index.php/' in href:
            system_name = href.split('/index.php/')[-1].replace('_emulators', '').replace('_', ' ')
            full_url = BASE_URL + href if href.startswith('/') else href
            systems[system_name] = full_url

    return systems

def parse_emulator_table(table):
    """Parse an emulator comparison table and extract recommended ones."""
    emulators = []
    headers = []

    # Get headers
    header_row = table.find('tr')
    if header_row:
        for th in header_row.find_all(['th', 'td']):
            headers.append(th.get_text(strip=True).lower())

    # Find recommended column index
    rec_idx = None
    name_idx = 0
    platform_idx = None

    for i, h in enumerate(headers):
        if 'recommend' in h:
            rec_idx = i
        if 'name' in h or 'emulator' in h:
            name_idx = i
        if 'platform' in h or 'os' in h:
            platform_idx = i

    # Parse rows
    for row in table.find_all('tr')[1:]:
        cells = row.find_all(['td', 'th'])
        if len(cells) <= name_idx:
            continue

        # Get emulator name
        name_cell = cells[name_idx]
        name_link = name_cell.find('a')
        name = name_link.get_text(strip=True) if name_link else name_cell.get_text(strip=True)

        if not name:
            continue

        # Check if recommended (checkmark or checkmark)
        is_recommended = False
        if rec_idx and rec_idx < len(cells):
            rec_cell = cells[rec_idx]
            rec_text = rec_cell.get_text(strip=True)
            # Check for checkmark, "Yes", green background, etc.
            is_recommended = '\u2713' in rec_text or rec_text.lower() == 'yes'
            # Also check for green background class
            if 'table-yes' in rec_cell.get('class', []) or 'background:#9f9' in str(rec_cell.get('style', '')):
                is_recommended = True

        # Get platforms
        platforms = []
        if platform_idx and platform_idx < len(cells):
            plat_cell = cells[platform_idx]
            plat_text = plat_cell.get_text(strip=True)
            # Look for platform icons/text
            for img in plat_cell.find_all('img'):
                alt = img.get('alt', '').lower()
                if 'windows' in alt or 'win' in alt:
                    platforms.append('windows')
                if 'linux' in alt or 'tux' in alt:
                    platforms.append('linux')
                if 'mac' in alt or 'apple' in alt or 'osx' in alt:
                    platforms.append('macos')
            # Also check text
            if 'windows' in plat_text.lower():
                platforms.append('windows')
            if 'linux' in plat_text.lower():
                platforms.append('linux')
            if 'mac' in plat_text.lower() or 'osx' in plat_text.lower():
                platforms.append('macos')

        emulators.append({
            'name': name,
            'recommended': is_recommended,
            'platforms': list(set(platforms)) if platforms else ['windows', 'linux', 'macos']  # Default to all if unknown
        })

    return emulators

def get_emulators_for_system(url):
    """Get all emulators from a system's emulator page."""
    try:
        soup = get_soup(url)
    except Exception as e:
        print(f"  Error fetching {url}: {e}")
        return []

    all_emulators = []

    # Find all tables (wikitables)
    for table in soup.find_all('table', class_='wikitable'):
        emulators = parse_emulator_table(table)
        all_emulators.extend(emulators)

    # Deduplicate by name
    seen = set()
    unique = []
    for emu in all_emulators:
        if emu['name'] not in seen:
            seen.add(emu['name'])
            unique.append(emu)

    return unique

def main():
    print("Fetching system links from main page...")
    systems = get_system_links()
    print(f"Found {len(systems)} systems")

    results = {}

    for system, url in sorted(systems.items()):
        print(f"Processing: {system}")
        emulators = get_emulators_for_system(url)
        recommended = [e for e in emulators if e['recommended']]
        print(f"  Found {len(emulators)} emulators, {len(recommended)} recommended")

        if recommended:
            results[system] = {
                'wiki_url': url,
                'emulators': recommended
            }

    # Save to JSON for manual review
    with open('emulator_data.json', 'w') as f:
        json.dump(results, f, indent=2)

    print(f"\nSaved {len(results)} systems to emulator_data.json")

if __name__ == '__main__':
    main()
