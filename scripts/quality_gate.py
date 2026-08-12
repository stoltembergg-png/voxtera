#!/usr/bin/env python3
"""Fail-closed evaluator for the repository's finite CI producer contract."""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from typing import Any

REQUIRED_PRODUCERS = (
    "rust-format",
    "rust-server-test",
    "launcher-tests",
    "site-tests",
    "release-contract",
)
SUCCESS = "success"


@dataclass(frozen=True)
class ProducerResult:
    name: str
    status: str
    conclusion: str | None


def parse_results(payload: Any) -> tuple[ProducerResult, ...]:
    if not isinstance(payload, dict):
        raise ValueError("quality gate input must be an object")
    raw_results = payload.get("producers")
    if not isinstance(raw_results, list):
        raise ValueError("quality gate input must contain a producers list")

    results: list[ProducerResult] = []
    seen: set[str] = set()
    for raw in raw_results:
        if not isinstance(raw, dict):
            raise ValueError("producer result must be an object")
        name = raw.get("name")
        status = raw.get("status")
        conclusion = raw.get("conclusion")
        if not isinstance(name, str) or not name:
            raise ValueError("producer name must be a non-empty string")
        if name in seen:
            raise ValueError(f"duplicate producer: {name}")
        if not isinstance(status, str) or not isinstance(conclusion, (str, type(None))):
            raise ValueError(f"malformed producer: {name}")
        seen.add(name)
        results.append(ProducerResult(name, status, conclusion))
    return tuple(results)


def evaluate(payload: Any) -> tuple[bool, list[str]]:
    try:
        results = parse_results(payload)
    except ValueError as error:
        return False, [f"INVALID_INPUT: {error}"]

    by_name = {result.name: result for result in results}
    reasons: list[str] = []
    for name in REQUIRED_PRODUCERS:
        result = by_name.get(name)
        if result is None:
            reasons.append(f"MISSING: {name}")
            continue
        if result.status != "completed":
            reasons.append(f"NOT_COMPLETED: {name}={result.status}")
        if result.conclusion != SUCCESS:
            reasons.append(f"NOT_SUCCESS: {name}={result.conclusion}")
    return not reasons, reasons


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"Usage: {argv[0]} <results.json>")
        return 2
    try:
        payload = json.loads(open(argv[1], encoding="utf-8").read())
    except (OSError, json.JSONDecodeError) as error:
        print(f"INVALID_INPUT: {error}")
        return 1
    passed, reasons = evaluate(payload)
    if passed:
        print("CI / Quality Gate: PASS")
        return 0
    print("CI / Quality Gate: FAIL")
    print("\n".join(reasons))
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
