#!/usr/bin/env python
"""Reject release archives that cannot boot because required files are absent."""

from __future__ import annotations

from pathlib import Path
import sys
import tarfile
import zipfile

WINDOWS_REQUIRED_FILES = (
    "Voxtera.exe",
    "VoxteraLauncher.exe",
    "assets/common/canary.canary",
    "assets/voxygen/logo.ico",
)

SERVER_REQUIRED_FILES = (
    "veloren-server-cli",
    "assets/common/canary.canary",
    "assets/server/manifests/kits.ron",
    "assets/world",
    "voxtera-server.service",
)

MACOS_REQUIRED_FILES = (
    "Voxtera.app/Contents/Info.plist",
    "Voxtera.app/Contents/MacOS/Voxtera",
    "Voxtera.app/Contents/MacOS/Voxtera-bin",
    "Voxtera.app/Contents/Resources/assets/common/canary.canary",
    "Voxtera.app/Contents/Resources/assets/voxygen/logo.ico",
)
GIT_LFS_POINTER_PREFIX = b"version https://git-lfs.github.com/spec/v1\n"


def required_files_for_archive(archive: Path) -> tuple[str, ...]:
    name = archive.name
    if "windows-x64" in name:
        return WINDOWS_REQUIRED_FILES
    if "server-linux-x86_64" in name:
        return SERVER_REQUIRED_FILES
    if "macos-universal" in name:
        return MACOS_REQUIRED_FILES
    raise ValueError(f"Unsupported release archive name: {name}")


def _archive_entries(archive: Path) -> tuple[set[str], object]:
    if archive.suffix == ".zip":
        bundle = zipfile.ZipFile(archive)
        return set(bundle.namelist()), bundle
    if archive.name.endswith(".tar.gz"):
        bundle = tarfile.open(archive, "r:gz")
        return {member.name.rstrip("/") for member in bundle.getmembers()}, bundle
    raise ValueError(f"Unsupported archive format: {archive.name}")


def _has_entry(entries: set[str], required: str) -> bool:
    if required in entries:
        return True
    return any(entry.startswith(required.rstrip("/") + "/") for entry in entries)


def validate_archive(archive: Path) -> list[str]:
    required_files = required_files_for_archive(archive)
    entries, bundle = _archive_entries(archive)
    try:
        errors = [path for path in required_files if not _has_entry(entries, path)]
        if isinstance(bundle, zipfile.ZipFile):
            for path in required_files:
                if path in entries and ("/assets/" in path or path.startswith("assets/")):
                    if bundle.read(path).startswith(GIT_LFS_POINTER_PREFIX):
                        errors.append(f"{path} is a Git LFS pointer")
        elif isinstance(bundle, tarfile.TarFile):
            for member in bundle.getmembers():
                if member.isfile() and member.name.startswith("assets/"):
                    payload = bundle.extractfile(member)
                    if payload is not None and payload.read(256).startswith(GIT_LFS_POINTER_PREFIX):
                        errors.append(f"{member.name} is a Git LFS pointer")
        return errors
    finally:
        bundle.close()


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"Usage: {Path(argv[0]).name} <release-archive>")
        return 2

    archive = Path(argv[1])
    if not archive.is_file():
        print(f"FAIL: release archive not found: {archive}")
        return 1

    try:
        missing = validate_archive(archive)
    except (OSError, tarfile.ReadError, zipfile.BadZipFile, ValueError) as error:
        print(f"FAIL: invalid release archive: {error}")
        return 1

    if missing:
        print("FAIL: release archive is missing required runtime files: " + ", ".join(missing))
        return 1

    print(f"PASS: {archive.name} contains the required platform files and assets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
