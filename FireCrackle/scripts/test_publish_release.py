import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import publish_release as release


class ReleaseSafetyTests(unittest.TestCase):
    def test_waits_for_draft_assets_and_published_state_with_a_bounded_retry(self):
        draft = {"draft": True, "assets": []}
        final = {"draft": False, "assets": [{"name": "app"}]}
        with patch.object(release, "release_info", side_effect=[None, draft, final]), patch.object(release.time, "sleep"):
            self.assertEqual(release.wait_release_info("owner/repo", "v0.1.0", published=True, asset_names=["app"]), final)
        with patch.object(release, "release_info", return_value=None) as lookup, patch.object(release.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "preserved for retry"):
                release.wait_release_info("owner/repo", "v0.1.0")
            self.assertEqual(lookup.call_count, 10)

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="firecrackle-release-test-")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)

    def source(self, cargo="0.1.0"):
        app = self.root / "FireCrackle"
        (app / "src-tauri").mkdir(parents=True)
        (app / "package.json").write_text('{"version":"0.1.0"}')
        (app / "src-tauri/tauri.conf.json").write_text('{"version":"0.1.0"}')
        (app / "src-tauri/Cargo.toml").write_text(f'[package]\nversion = "{cargo}"\n')

    def metadata(self, **changes):
        return {"version": "0.1.0", "commit": "a" * 40, "target": "windows-x64",
                "checksRun": True, "workingTreeDirty": False, **changes}

    def test_rejects_mismatched_versions_before_building(self):
        self.source(cargo="0.1.1")
        with self.assertRaisesRegex(ValueError, "versions must agree"):
            release.source_version(self.root)

    def test_tag_and_both_platforms_are_required_for_publication(self):
        self.source()
        with self.assertRaisesRegex(ValueError, "both platforms"):
            release.validate_plan("linux", True, "", self.root)
        with self.assertRaisesRegex(ValueError, "Tag must be"):
            release.validate_plan("all", True, "refs/tags/v9.9.9", self.root)
        self.assertEqual(release.validate_plan("all", True, "refs/tags/v0.1.0", self.root), "0.1.0")

    def test_rejects_dirty_skipped_or_different_commit_builds(self):
        for change in ({"workingTreeDirty": True}, {"checksRun": False}, {"commit": "b" * 40},
                       {"version": "0.1.1"}, {"target": "linux-x64"}):
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, "provenance"):
                release.verify_metadata(self.metadata(**change), "0.1.0", "a" * 40, "windows-x64")

    def test_rejects_checksum_tampering_and_path_escape(self):
        artifact = self.root / "app"
        artifact.write_bytes(b"verified")
        checksum = hashlib.sha256(artifact.read_bytes()).hexdigest()
        manifest = self.root / "SUMS"
        manifest.write_text(f"{checksum}  ./app\n")
        self.assertEqual(release.verify_checksums(self.root, "SUMS"), {"app"})
        artifact.write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "Checksum mismatch"):
            release.verify_checksums(self.root, "SUMS")
        for name in ("../app", "C:\\app", "/app"):
            manifest.write_text(f"{checksum}  {name}\n")
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "Unsafe"):
                release.verify_checksums(self.root, "SUMS")

    def test_rejects_zip_path_escape_before_reading_metadata(self):
        archive = self.root / "app.zip"
        with zipfile.ZipFile(archive, "w") as package:
            package.writestr("FireCrackle-0.1.0-windows-x64/../project.json", "private")
        with self.assertRaisesRegex(ValueError, "Unsafe Windows ZIP"):
            release.verify_windows_archive(archive, "0.1.0", "a" * 40)

    def test_rejects_changed_uploaded_bytes(self):
        artifact = self.root / "app"
        artifact.write_bytes(b"verified")
        info = {"assets": [{"name": "app", "digest": "sha256:" + "0" * 64}]}
        with self.assertRaisesRegex(ValueError, "Server checksum mismatch"):
            release.verify_uploaded("owner/repo", "v0.1.0", [artifact], info, self.root)

    def test_retained_build_directories_are_resolved_without_ambiguity(self):
        output = self.root / "v0.1.0" / "fresh-build"
        output.mkdir(parents=True)
        (output / "release-linux.json").write_text("{}")
        self.assertEqual(release.artifact_folder(self.root, "release-linux.json"), output)
        (self.root / "release-linux.json").write_text("{}")
        with self.assertRaisesRegex(ValueError, "Expected one"):
            release.artifact_folder(self.root, "release-linux.json")


if __name__ == "__main__":
    unittest.main()
