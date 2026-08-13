from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class TransitiveDependencySecurityContractTests(unittest.TestCase):
    def test_rusttype_is_patched_to_a_local_auditable_copy(self) -> None:
        manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('rusttype = { path = "vendor/rusttype-0.8.3" }', manifest)

    def test_local_rusttype_uses_patched_crossbeam_line(self) -> None:
        manifest = (ROOT / "vendor" / "rusttype-0.8.3" / "Cargo.toml").read_text(
            encoding="utf-8"
        )
        self.assertIn('version = "0.8"', manifest)
        self.assertNotIn('version = "0.7"', manifest)

    def test_lockfile_contains_no_vulnerable_transitive_versions(self) -> None:
        lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        for package, minimum in (("crossbeam-utils", (0, 8, 7)), ("memoffset", (0, 6, 2))):
            versions = re.findall(
                rf'(?ms)^name = "{re.escape(package)}"\nversion = "([^"]+)"',
                lock,
            )
            self.assertTrue(versions, f"missing {package} entries")
            for version in versions:
                parsed = tuple(int(part) for part in version.split(".")[:3])
                self.assertGreaterEqual(
                    parsed,
                    minimum,
                    f"{package} {version} is below patched version {minimum}",
                )


if __name__ == "__main__":
    unittest.main()
