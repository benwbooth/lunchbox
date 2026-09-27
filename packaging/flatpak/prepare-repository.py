#!/usr/bin/env python3
"""Stage a published release for the signed Flatpak update channel (no rebuild)."""

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tempfile
import urllib.error
import urllib.request


PROJECT = "benwbooth/lunchbox"
SITE = "https://benwbooth.github.io/lunchbox"
APP = "io.github.benwbooth.Lunchbox"
ARCHIVE = "Lunchbox-flatpak-repo.tar.gz"
REQUIRED = {
    ARCHIVE, "Lunchbox-windows-x86_64.msi", "Lunchbox-windows-x86_64.zip",
    "Lunchbox-macos-arm64.dmg", "Lunchbox-linux-x86_64.AppImage",
    "Lunchbox-linux-x86_64.flatpak", "lunchbox.rb", "SHA256SUMS",
}


def api(endpoint):
    return json.loads(subprocess.check_output(
        ["gh", "api", f"repos/{PROJECT}/{endpoint}"], text=True))


def release_identity(release):
    tag = release["tag_name"]
    if release["draft"] or release["prerelease"] or not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        raise ValueError("Only published stable releases can reach the update channel")
    assets = {asset["name"]: asset for asset in release["assets"]}
    if not REQUIRED <= assets.keys():
        raise ValueError("Release upload is incomplete")
    for name in REQUIRED:
        asset = assets[name]
        if asset["size"] <= 0 or not re.fullmatch(r"sha256:[0-9a-f]{64}", asset.get("digest") or ""):
            raise ValueError(f"Missing content or SHA256 digest for {name}")
    return {
        "schema": 1,
        "tag": tag,
        "release_url": release["html_url"],
        "archive_sha256": assets[ARCHIVE]["digest"].removeprefix("sha256:"),
    }


def verify_download(path, asset):
    with path.open("rb") as stream:
        checksum = hashlib.file_digest(stream, "sha256").hexdigest()
    if path.stat().st_size != asset["size"] or "sha256:" + checksum != asset["digest"]:
        raise ValueError(f"Downloaded {path.name} does not match the published asset")


def extract_repository(archive, output):
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        for member in members:
            path = PurePosixPath(member.name)
            if (path.is_absolute() or ".." in path.parts or not path.parts
                    or path.parts[0] != "repo" or not (member.isfile() or member.isdir())):
                raise ValueError(f"Unsafe repository archive member: {member.name}")
        bundle.extractall(output, members=members, filter="data")


def write_descriptors(output, key):
    common = f"Url={SITE}/flatpak/\nHomepage=https://github.com/{PROJECT}\nGPGKey={key}\n"
    (output / "lunchbox.flatpakrepo").write_text(
        "[Flatpak Repo]\nTitle=Lunchbox\nComment=Official Lunchbox releases\n"
        "Description=Retro game library and emulator frontend\nDefaultBranch=master\n" + common,
        encoding="utf-8")
    (output / "lunchbox.flatpakref").write_text(
        f"[Flatpak Ref]\nName={APP}\nTitle=Lunchbox\nBranch=master\n"
        "IsRuntime=false\nSuggestRemoteName=lunchbox\n"
        "RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo\n" + common,
        encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()
    release = api("releases/latest")
    identity = release_identity(release)
    if not args.force:
        try:
            with urllib.request.urlopen(f"{SITE}/release.json", timeout=20) as response:
                published = json.load(response)
            if all(published.get(key) == value for key, value in identity.items()):
                print(f"{identity['tag']} is already published")
                return
        except (urllib.error.URLError, ValueError):
            pass  # A missing or unavailable site is safe to republish.
    args.output.mkdir(parents=True, exist_ok=False)
    assets = {asset["name"]: asset for asset in release["assets"]}
    with tempfile.TemporaryDirectory(prefix="lunchbox-flatpak-") as directory:
        subprocess.run([
            "gh", "release", "download", identity["tag"], "--repo", PROJECT,
            "--pattern", ARCHIVE, "--pattern", "SHA256SUMS", "--dir", directory,
        ], check=True)
        archive = Path(directory) / ARCHIVE
        checksums = Path(directory) / "SHA256SUMS"
        verify_download(archive, assets[ARCHIVE])
        verify_download(checksums, assets["SHA256SUMS"])
        matches = [line.split()[0] for line in checksums.read_text().splitlines()
                   if len(line.split()) == 2 and Path(line.split()[1]).name == ARCHIVE]
        if matches != [identity["archive_sha256"]]:
            raise ValueError("Repository digest disagrees with SHA256SUMS")
        extract_repository(archive, args.output)
    (args.output / "repo").rename(args.output / "flatpak")
    ref = f"app/{APP}/x86_64/master"
    available = subprocess.check_output([
        "flatpak", "remote-ls", "--user", "--columns=ref", args.output.resolve().as_uri() + "/flatpak",
    ], text=True).splitlines()
    if ref not in available:
        raise ValueError(f"Published repository is missing {ref}")
    source = Path(__file__).resolve().parent
    key = subprocess.check_output(["gpg", "--dearmor", "--output", "-", str(source / "signing-key.asc")])
    write_descriptors(args.output, base64.b64encode(key).decode("ascii"))
    identity["source_commit"] = api(f"commits/{identity['tag']}")["sha"]
    (args.output / "release.json").write_text(json.dumps(identity, indent=2) + "\n")
    (args.output / "index.html").write_text((source / "download.html").read_text())
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write("publish=true\n")
    print(f"Verified {identity['tag']} ({identity['source_commit']}) for publication")


if __name__ == "__main__":
    main()
