import json
from pathlib import Path
import tempfile
import unittest

import prepare_release as release


class ReleaseCutTests(unittest.TestCase):
    def test_updates_all_version_files_without_touching_dependency_versions(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            app = root / "FireCrackle"
            (app / "src-tauri").mkdir(parents=True)
            for name in release.FILES[:3]:
                (app / name).write_text(json.dumps({"version": "0.1.0", "packages": {"": {"version": "0.1.0"}}}))
            (app / "src-tauri/Cargo.toml").write_text('[package]\nversion = "0.1.0"\n')
            lock = app / "src-tauri/Cargo.lock"
            lock.write_text('[[package]]\nname = "dependency"\nversion = "1.2.3"\n\n'
                            '[[package]]\nname = "firecrackle"\nversion = "0.1.0"\n')
            release.bump_version(root, "0.2.0")
            self.assertIn('name = "dependency"\nversion = "1.2.3"', lock.read_text())
            self.assertIn('name = "firecrackle"\nversion = "0.2.0"', lock.read_text())
            for name in release.FILES[:3]:
                self.assertEqual(json.loads((app / name).read_text())["version"], "0.2.0")
            self.assertEqual(json.loads((app / "package-lock.json").read_text())["packages"][""]["version"], "0.2.0")
            before = {name: (app / name).read_bytes() for name in release.FILES}
            lock.write_text("missing package")
            with self.assertRaisesRegex(ValueError, "lockfile"):
                release.bump_version(root, "0.3.0")
            for name in release.FILES[:-1]:
                self.assertEqual((app / name).read_bytes(), before[name])

    def test_invalid_version_cannot_create_a_workspace_path(self):
        with self.assertRaises(ValueError):
            release.bump_version(Path("unused"), "../../bad")
