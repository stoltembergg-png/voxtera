from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from quality_gate import REQUIRED_PRODUCERS, evaluate


class QualityGateTests(unittest.TestCase):
    def complete_payload(self) -> dict[str, object]:
        return {
            "producers": [
                {"name": name, "status": "completed", "conclusion": "success"}
                for name in REQUIRED_PRODUCERS
            ]
        }

    def test_all_allowlisted_producers_are_required(self) -> None:
        passed, reasons = evaluate(self.complete_payload())
        self.assertTrue(passed)
        self.assertEqual(reasons, [])

    def test_missing_producer_fails_closed(self) -> None:
        payload = self.complete_payload()
        payload["producers"] = payload["producers"][:-1]  # type: ignore[index]
        passed, reasons = evaluate(payload)
        self.assertFalse(passed)
        self.assertIn("MISSING: release-contract", reasons)

    def test_skipped_producer_fails_even_with_success_conclusion(self) -> None:
        payload = self.complete_payload()
        payload["producers"][0]["status"] = "skipped"  # type: ignore[index]
        passed, reasons = evaluate(payload)
        self.assertFalse(passed)
        self.assertIn("NOT_COMPLETED: rust-format=skipped", reasons)

    def test_failed_producer_fails_closed(self) -> None:
        payload = self.complete_payload()
        payload["producers"][1]["conclusion"] = "failure"  # type: ignore[index]
        passed, reasons = evaluate(payload)
        self.assertFalse(passed)
        self.assertIn("NOT_SUCCESS: rust-server-test=failure", reasons)

    def test_duplicate_and_malformed_inputs_fail_closed(self) -> None:
        payload = self.complete_payload()
        payload["producers"].append(payload["producers"][0])  # type: ignore[index]
        passed, reasons = evaluate(payload)
        self.assertFalse(passed)
        self.assertTrue(any(reason.startswith("INVALID_INPUT") for reason in reasons))

        passed, reasons = evaluate({"producers": "not-a-list"})
        self.assertFalse(passed)
        self.assertIn("INVALID_INPUT", reasons[0])


if __name__ == "__main__":
    unittest.main()
