"""Tests for the Linux server runtime compatibility gate."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("verify_glibc_compatibility.py")
SPEC = importlib.util.spec_from_file_location("verify_glibc_compatibility", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class GlibcCompatibilityTests(unittest.TestCase):
    def test_accepts_glibc_234(self) -> None:
        completed = type("Completed", (), {"stdout": "Name: GLIBC_2.34\n"})()
        with patch.object(MODULE.subprocess, "run", return_value=completed):
            self.assertEqual(MODULE.max_required_version(Path("server")), (2, 34))

    def test_rejects_glibc_239(self) -> None:
        completed = type("Completed", (), {"stdout": "Name: GLIBC_2.34\nName: GLIBC_2.39\n"})()
        with patch.object(MODULE.subprocess, "run", return_value=completed):
            self.assertEqual(MODULE.max_required_version(Path("server")), (2, 39))

    def test_rejects_missing_symbol_metadata(self) -> None:
        completed = type("Completed", (), {"stdout": ""})()
        with patch.object(MODULE.subprocess, "run", return_value=completed):
            self.assertIsNone(MODULE.max_required_version(Path("server")))


if __name__ == "__main__":
    unittest.main()
