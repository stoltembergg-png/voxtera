#!/usr/bin/env python3
"""Fail if an ELF binary requires GLIBC symbols newer than the target runtime."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

GLIBC_SYMBOL = re.compile(r"GLIBC_(\d+)\.(\d+)")


def max_required_version(binary: Path) -> tuple[int, int] | None:
    result = subprocess.run(
        ["readelf", "--version-info", str(binary)],
        check=True,
        capture_output=True,
        text=True,
    )
    versions = [
        (int(match.group(1)), int(match.group(2)))
        for match in GLIBC_SYMBOL.finditer(result.stdout)
    ]
    return max(versions) if versions else None


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"Usage: {Path(argv[0]).name} <elf-binary>")
        return 2

    binary = Path(argv[1])
    if not binary.is_file():
        print(f"FAIL: binary not found: {binary}")
        return 1

    try:
        required = max_required_version(binary)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"FAIL: could not inspect {binary}: {error}")
        return 1

    if required is None:
        print(f"FAIL: no GLIBC symbol information found in {binary}")
        return 1

    target = (2, 34)
    print(f"Detected maximum required GLIBC version: {required[0]}.{required[1]}")
    if required > target:
        print("FAIL: binary requires GLIBC newer than 2.34")
        return 1

    print("PASS: binary is compatible with GLIBC 2.34")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
