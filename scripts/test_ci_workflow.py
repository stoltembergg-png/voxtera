from __future__ import annotations

from pathlib import Path
import unittest


WORKFLOW = Path(__file__).parents[1] / ".github" / "workflows" / "ci.yml"


class CiWorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_declares_all_required_producers(self) -> None:
        for job in (
            "rust-format:",
            "rust-server-test:",
            "launcher-tests:",
            "site-tests:",
            "release-contract:",
        ):
            self.assertIn(job, self.workflow)

    def test_quality_gate_is_always_run_and_depends_on_all_producers(self) -> None:
        self.assertIn("name: CI / Quality Gate", self.workflow)
        self.assertIn("if: ${{ always() }}", self.workflow)
        for job in (
            "- rust-format",
            "- rust-server-test",
            "- launcher-tests",
            "- site-tests",
            "- release-contract",
        ):
            self.assertIn(job, self.workflow)

    def test_pr_jobs_do_not_receive_write_permissions_or_deploy_secrets(self) -> None:
        self.assertIn("permissions:\n  contents: read", self.workflow)
        self.assertNotIn("pull_request_target", self.workflow)
        self.assertNotIn("VERCEL_TOKEN", self.workflow)
        self.assertNotIn("SSH_PRIVATE_KEY", self.workflow)

    def test_quality_gate_invokes_the_fail_closed_evaluator(self) -> None:
        self.assertIn("python scripts/quality_gate.py quality-gate-results.json", self.workflow)
        self.assertIn('"repository": os.environ["GITHUB_REPOSITORY"]', self.workflow)
        self.assertIn('"sha": os.environ["GITHUB_SHA"]', self.workflow)

    def test_rust_format_gate_has_a_push_fallback_when_the_base_commit_is_unavailable(self) -> None:
        self.assertIn("git cat-file -e", self.workflow)
        self.assertIn("Base commit unavailable; checking all tracked Rust files fail-closed.", self.workflow)

    def test_rust_format_gate_resolves_pull_request_base_from_fetch_head(self) -> None:
        self.assertIn("BASE_COMMIT=$(git rev-parse FETCH_HEAD)", self.workflow)
        self.assertIn("git diff --diff-filter=ACMR --name-only \"$BASE_COMMIT\" \"$GITHUB_SHA\"", self.workflow)

    def test_rust_format_gate_fails_closed_when_pull_request_base_is_unavailable(self) -> None:
        self.assertIn('echo "Pull request base commit unavailable; refusing to format the full repository."', self.workflow)
        self.assertIn("exit 1", self.workflow)

    def test_rust_format_gate_does_not_reformat_unchanged_child_modules(self) -> None:
        self.assertIn("rustfmt --edition 2024 --config skip_children=true --check", self.workflow)


if __name__ == "__main__":
    unittest.main()
