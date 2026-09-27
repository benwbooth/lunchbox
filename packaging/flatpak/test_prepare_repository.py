import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("publisher", Path(__file__).with_name("prepare-repository.py"))
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class RepositoryPublicationTests(unittest.TestCase):
    def release(self):
        return {
            "tag_name": "v1.2.3", "draft": False, "prerelease": False,
            "html_url": "https://github.com/benwbooth/lunchbox/releases/tag/v1.2.3",
            "assets": [{"name": name, "size": 10, "digest": "sha256:" + "a" * 64}
                       for name in publisher.REQUIRED],
        }

    def test_requires_complete_stable_release(self):
        self.assertEqual(publisher.release_identity(self.release())["tag"], "v1.2.3")
        for field in ("draft", "prerelease"):
            release = self.release()
            release[field] = True
            with self.assertRaises(ValueError):
                publisher.release_identity(release)
        release = self.release()
        release["assets"].pop()
        with self.assertRaises(ValueError):
            publisher.release_identity(release)

    def test_requires_nonempty_assets_and_digests(self):
        for field, value in (("size", 0), ("digest", None), ("digest", "sha256:wrong")):
            release = self.release()
            release["assets"][0][field] = value
            with self.assertRaises(ValueError):
                publisher.release_identity(release)

    def test_rejects_modified_download(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "asset"
            path.write_bytes(b"bad")
            with self.assertRaises(ValueError):
                publisher.verify_download(path, {"size": 3, "digest": "sha256:" + "a" * 64})

    def archive(self, path, name, symlink=False):
        with tarfile.open(path, "w:gz") as archive:
            member = tarfile.TarInfo(name)
            if symlink:
                member.type = tarfile.SYMTYPE
                member.linkname = "/etc/passwd"
            else:
                member.size = 1
            archive.addfile(member, None if symlink else io.BytesIO(b"a"))

    def test_rejects_escaping_or_linked_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "repo.tar.gz"
            for name, symlink in (("repo/../../escape", False), ("/repo/file", False),
                                  ("unrelated/file", False), ("repo/link", True)):
                self.archive(archive, name, symlink)
                with self.assertRaises(ValueError):
                    publisher.extract_repository(archive, Path(directory) / "out")

    def test_extracts_repository(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "repo.tar.gz"
            self.archive(archive, "repo/config")
            publisher.extract_repository(archive, Path(directory) / "out")
            self.assertEqual((Path(directory) / "out/repo/config").read_bytes(), b"a")

    def test_installer_includes_trust_and_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            publisher.write_descriptors(Path(directory), "public-key")
            ref = (Path(directory) / "lunchbox.flatpakref").read_text()
            repo = (Path(directory) / "lunchbox.flatpakrepo").read_text()
            self.assertIn("RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo", ref)
            self.assertIn("Branch=master", ref)
            self.assertIn("SuggestRemoteName=lunchbox", ref)
            for descriptor in (ref, repo):
                self.assertIn("GPGKey=public-key", descriptor)
                self.assertIn("Url=https://benwbooth.github.io/lunchbox/flatpak/", descriptor)


if __name__ == "__main__":
    unittest.main()
