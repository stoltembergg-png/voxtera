from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
ACTION_RE = re.compile(r"^\s*-?\s*uses:\s+([^\s#]+)")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")


class WorkflowSecurityContractTests(unittest.TestCase):
    def test_every_local_action_reference_is_immutable(self) -> None:
        floating: list[str] = []
        for path in sorted(WORKFLOWS.glob("*.y*ml")):
            for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                match = ACTION_RE.match(line)
                if not match:
                    continue
                reference = match.group(1)
                if "@" not in reference:
                    floating.append(f"{path.relative_to(ROOT)}:{line_number} {reference}")
                    continue
                _, revision = reference.rsplit("@", 1)
                if not SHA_RE.fullmatch(revision):
                    floating.append(f"{path.relative_to(ROOT)}:{line_number} {reference}")
        self.assertEqual(floating, [], "floating GitHub Action references: " + "; ".join(floating))

    def test_mirror_caller_declares_least_privilege_permissions(self) -> None:
        mirror = (WORKFLOWS / "mirror.yml").read_text(encoding="utf-8")
        self.assertRegex(mirror, r"(?m)^permissions:\s*$")
        self.assertRegex(mirror, r"(?m)^\s+contents:\s+read\s*$")

    def test_ui_writes_slider_value_as_text(self) -> None:
        ui = (ROOT / "server-cli" / "src" / "web" / "ui" / "ui.js").read_text(
            encoding="utf-8"
        )
        self.assertIn("sliderNo.textContent = slider.value;", ui)
        self.assertNotIn(".innerHTML", ui)


if __name__ == "__main__":
    unittest.main()
