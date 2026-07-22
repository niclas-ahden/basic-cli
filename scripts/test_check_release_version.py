#!/usr/bin/env python3
"""Tests for check_release_version.py."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check_release_version.py")


class CheckReleaseVersionTests(unittest.TestCase):
    def run_check(self, package_version: str, release_version: str) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            manifest_path = Path(directory) / "Cargo.toml"
            source_path = Path(directory) / "src"
            source_path.mkdir()
            (source_path / "lib.rs").write_text("", encoding="utf-8")
            manifest_path.write_text(
                "[package]\n"
                'name = "version-check-fixture"\n'
                f'version = "{package_version}"\n'
                'edition = "2021"\n',
                encoding="utf-8",
            )
            return subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    release_version,
                    "--manifest-path",
                    str(manifest_path),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

    def test_matching_prerelease_succeeds(self) -> None:
        result = self.run_check("0.21.0-rc4", "0.21.0-rc4")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "Cargo package version 0.21.0-rc4 matches the release version",
            result.stdout,
        )

    def test_mismatched_version_has_clear_diagnostic(self) -> None:
        result = self.run_check("0.21.0-rc4", "0.21.0")

        self.assertEqual(result.returncode, 1)
        self.assertIn(
            "error: release version mismatch: Cargo package version is 0.21.0-rc4, "
            "but the validated release version is 0.21.0",
            result.stderr,
        )


if __name__ == "__main__":
    unittest.main()
