from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class DependencySecurityContractTests(unittest.TestCase):
    def test_jsonwebtoken_requirement_is_patched(self) -> None:
        manifest = (ROOT / "server" / "Cargo.toml").read_text(encoding="utf-8")
        self.assertRegex(
            manifest,
            r'(?m)^jsonwebtoken\s*=\s*\{\s*version\s*=\s*"10\.3\.0"\s*,\s*features\s*=\s*\["rust_crypto"\]\s*\}',
        )

    def test_lockfile_has_no_vulnerable_jsonwebtoken(self) -> None:
        lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        entry = re.search(r'(?ms)^name = "jsonwebtoken"\nversion = "([^"]+)"', lock)
        self.assertIsNotNone(entry)
        assert entry is not None
        major, minor, patch = (int(part) for part in entry.group(1).split(".")[:3])
        self.assertTrue((major, minor, patch) >= (10, 3, 0))

    def test_lockfile_enables_one_jsonwebtoken_crypto_provider(self) -> None:
        lock = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        match = re.search(r'(?ms)^name = "jsonwebtoken"\n.*?^dependencies = \[\n(.*?)^\]', lock)
        self.assertIsNotNone(match)
        assert match is not None
        self.assertIn('"p256', match.group(1))
        self.assertIn('"signature', match.group(1))


if __name__ == "__main__":
    unittest.main()
