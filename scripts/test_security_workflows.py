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

    def test_mirror_clones_upstream_master_and_pushes_fork_main(self) -> None:
        mirror = (WORKFLOWS / "mirror.yml").read_text(encoding="utf-8")
        steps = {
            block.splitlines()[0]: block
            for block in re.split(r"(?m)^      - name: ", mirror)[1:]
        }
        download = steps["Download and move Git LFS"]
        clone = steps["Clone upstream master"]
        push = steps["Push mirror to fork main"]
        non_push = "\n".join(
            block for name, block in steps.items() if name != "Push mirror to fork main"
        )

        self.assertEqual(
            "git clone --branch master https://gitlab.com/veloren/veloren.git source",
            next(
                line.strip()
                for line in clone.splitlines()
                if line.strip().startswith("run: ")
            ).removeprefix("run: "),
        )
        archive = "git-lfs-linux-amd64-v${GIT_LFS_VERSION}.tar.gz"
        self.assertIn(
            f'echo "${{GIT_LFS_SHA256}}  {archive}" | sha256sum --check --strict',
            download,
        )
        self.assertLess(download.index("sha256sum --check --strict"), download.index("tar xzf"))
        self.assertIn("MIRROR_TOKEN: ${{ secrets.MIRROR_TOKEN_GITHUB }}", push)
        self.assertNotIn("MIRROR_TOKEN", non_push)
        self.assertIn(
            'GIT_CONFIG_VALUE_0="AUTHORIZATION: bearer ${MIRROR_TOKEN}"',
            push,
        )
        self.assertIn(
            "GIT_TERMINAL_PROMPT=0 git push --force --tags origin HEAD:refs/heads/main",
            push,
        )
        self.assertNotIn("github.ref_name", mirror)
        self.assertNotIn("uses: veloren/.github/.github/workflows/mirror.yml", mirror)

    def test_ui_writes_slider_value_as_text(self) -> None:
        ui = (ROOT / "server-cli" / "src" / "web" / "ui" / "ui.js").read_text(
            encoding="utf-8"
        )
        self.assertIn("sliderNo.textContent = slider.value;", ui)
        self.assertNotIn(".innerHTML", ui)


if __name__ == "__main__":
    unittest.main()
