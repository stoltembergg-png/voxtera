"""Regression tests for platform-specific release archive validation."""

from __future__ import annotations

from pathlib import Path
import tempfile
import tarfile
import unittest
import zipfile

from release_asset_gate import required_files_for_archive, validate_archive


class ReleaseAssetGateTests(unittest.TestCase):
    def make_archive(self, name: str, entries: tuple[str, ...]) -> Path:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / name
        with zipfile.ZipFile(archive, "w") as bundle:
            for entry in entries:
                bundle.writestr(entry, "fixture")
        return archive

    def test_windows_archive_requires_the_windows_executable_and_assets(self) -> None:
        required = required_files_for_archive(Path("Voxtera-windows-x64-v0.4.0.zip"))

        self.assertIn("Voxtera.exe", required)
        self.assertIn("assets/common/canary.canary", required)
        self.assertNotIn("Voxtera.app/Contents/MacOS/Voxtera", required)

    def test_macos_archive_requires_the_app_launcher_binary_and_resources(self) -> None:
        required = required_files_for_archive(Path("Voxtera-macos-universal-v0.4.0.zip"))

        self.assertIn("Voxtera.app/Contents/MacOS/Voxtera", required)
        self.assertIn("Voxtera.app/Contents/MacOS/Voxtera-bin", required)
        self.assertIn("Voxtera.app/Contents/Info.plist", required)
        self.assertIn("Voxtera.app/Contents/Resources/assets/common/canary.canary", required)
        self.assertNotIn("Voxtera.exe", required)

    def test_macos_archive_is_rejected_when_resources_are_missing(self) -> None:
        archive = self.make_archive(
            "Voxtera-macos-universal-v0.4.0.zip",
            (
                "Voxtera.app/Contents/MacOS/Voxtera",
                "Voxtera.app/Contents/MacOS/Voxtera-bin",
                "Voxtera.app/Contents/Info.plist",
                "Voxtera.app/Contents/Resources/assets/voxygen/logo.ico",
            ),
        )

        self.assertEqual(
            validate_archive(archive),
            ["Voxtera.app/Contents/Resources/assets/common/canary.canary"],
        )

    def test_macos_archive_with_launcher_binary_and_assets_is_accepted(self) -> None:
        archive = self.make_archive(
            "Voxtera-macos-universal-v0.4.0.zip",
            (
                "Voxtera.app/Contents/MacOS/Voxtera",
                "Voxtera.app/Contents/MacOS/Voxtera-bin",
                "Voxtera.app/Contents/Info.plist",
                "Voxtera.app/Contents/Resources/assets/common/canary.canary",
                "Voxtera.app/Contents/Resources/assets/voxygen/logo.ico",
            ),
        )

        self.assertEqual(validate_archive(archive), [])

    def test_archive_with_a_git_lfs_pointer_is_rejected(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / "Voxtera-windows-x64-v0.4.0.zip"
        required = required_files_for_archive(archive)

        with zipfile.ZipFile(archive, "w") as bundle:
            for entry in required:
                content = (
                    "version https://git-lfs.github.com/spec/v1\n"
                    "oid sha256:deadbeef\n"
                    "size 123\n"
                    if entry == "assets/common/canary.canary"
                    else "fixture"
                )
                bundle.writestr(entry, content)

        self.assertEqual(
            validate_archive(archive),
            ["assets/common/canary.canary is a Git LFS pointer"],
        )

    def test_linux_server_archive_requires_binary_assets_and_service_contract(self) -> None:
        required = required_files_for_archive(Path("voxtera-server-linux-x86_64-v1.0.0.tar.gz"))

        self.assertIn("veloren-server-cli", required)
        self.assertIn("assets/common/canary.canary", required)
        self.assertIn("voxtera-server.service", required)

    def test_linux_server_archive_is_rejected_when_service_is_missing(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / "voxtera-server-linux-x86_64-v1.0.0.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            for entry in (
                "veloren-server-cli",
                "assets/common/canary.canary",
                "assets/server/manifests/kits.ron",
                "assets/world/manifest.ron",
            ):
                path = Path(directory.name) / entry
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture", encoding="utf-8")
                if entry == "veloren-server-cli":
                    info = tarfile.TarInfo(entry)
                    info.mode = 0o755
                    info.size = path.stat().st_size
                    with path.open("rb") as payload:
                        bundle.addfile(info, payload)
                else:
                    bundle.add(path, arcname=entry)

        self.assertEqual(validate_archive(archive), ["voxtera-server.service"])

    def test_linux_server_archive_is_rejected_when_binary_is_not_executable(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / "voxtera-server-linux-x86_64-v1.0.0.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            for entry in (
                "veloren-server-cli",
                "assets/common/canary.canary",
                "assets/server/manifests/kits.ron",
                "assets/world/manifest.ron",
                "voxtera-server.service",
            ):
                path = Path(directory.name) / entry
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture", encoding="utf-8")
                bundle.add(path, arcname=entry)

        self.assertEqual(validate_archive(archive), ["veloren-server-cli is not executable"])

    def test_linux_server_archive_with_runtime_contract_is_accepted(self) -> None:
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        archive = Path(directory.name) / "voxtera-server-linux-x86_64-v1.0.0.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            for entry in (
                "veloren-server-cli",
                "assets/common/canary.canary",
                "assets/server/manifests/kits.ron",
                "assets/world/manifest.ron",
                "voxtera-server.service",
            ):
                path = Path(directory.name) / entry
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture", encoding="utf-8")
                if entry == "veloren-server-cli":
                    info = tarfile.TarInfo(entry)
                    info.mode = 0o755
                    info.size = path.stat().st_size
                    with path.open("rb") as payload:
                        bundle.addfile(info, payload)
                else:
                    bundle.add(path, arcname=entry)

        self.assertEqual(validate_archive(archive), [])


if __name__ == "__main__":
    unittest.main()
