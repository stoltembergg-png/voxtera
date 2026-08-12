"""Structural release-workflow contract tests for Windows client and Linux server."""

from __future__ import annotations

from pathlib import Path
import unittest


WORKFLOW = Path(__file__).parents[1] / ".github" / "workflows" / "release.yml"


class ReleaseWorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_releases_are_tagged_or_explicitly_dispatched_not_pushed_on_main(self) -> None:
        self.assertIn("tags: ['v*']", self.workflow)
        self.assertIn("workflow_dispatch:", self.workflow)
        self.assertNotIn("branches: [main]", self.workflow)

    def test_builds_windows_client_and_launcher_with_pinned_launcher_dependencies(self) -> None:
        self.assertIn("build-windows-client:", self.workflow)
        self.assertIn("cargo build --locked --release --bin Voxtera", self.workflow)
        self.assertIn("pip install --requirement requirements-build.txt", self.workflow)
        self.assertIn("VoxteraLauncher.exe", self.workflow)
        self.assertIn("voxtera-windows-client", self.workflow)

    def test_builds_linux_server_and_requires_an_executable_binary(self) -> None:
        self.assertIn("build-linux-server:", self.workflow)
        self.assertIn("runs-on: ubuntu-latest", self.workflow)
        self.assertIn("container: rockylinux:9", self.workflow)
        self.assertIn("dnf install -y git git-lfs gcc gcc-c++ make perl openssl-devel pkg-config curl-minimal tar gzip binutils python3 nodejs", self.workflow)
        self.assertIn("toolchain: ${{ env.RUST_TOOLCHAIN }}-x86_64-unknown-linux-gnu", self.workflow)
        self.assertIn("cargo build --locked --release --bin veloren-server-cli", self.workflow)
        self.assertIn("ldd --version", self.workflow)
        self.assertIn("python3 scripts/verify_glibc_compatibility.py target/release/veloren-server-cli", self.workflow)
        self.assertIn('"glibc_max": os.environ["GLIBC_MAX_VERSION"]', self.workflow)
        self.assertIn('GLIBC_MAX_VERSION="2.34"', self.workflow)
        self.assertIn("test -x target/release/veloren-server-cli", self.workflow)
        self.assertIn("voxtera-server.service", self.workflow)
        self.assertNotIn("server-settings.ron.example", self.workflow)
        self.assertIn("package-and-release:", self.workflow)
        self.assertIn("needs: [build-windows-client, build-linux-server]", self.workflow)
        self.assertIn("windows_archive=", self.workflow)
        self.assertIn("server_archive=", self.workflow)
        self.assertIn("manifest=", self.workflow)
        self.assertIn("windows-x64", self.workflow)
        self.assertIn("linux-x86_64", self.workflow)
        self.assertIn("softprops/action-gh-release@v2", self.workflow)
        self.assertIn("cp artifacts/windows/VoxteraLauncher.exe .", self.workflow)
        self.assertIn("launcher_asset=VoxteraLauncher.exe", self.workflow)
        self.assertIn("steps.package.outputs.launcher_asset", self.workflow)
        self.assertIn("VoxteraLauncher.exe", self.workflow)
        self.assertIn("prerelease: false", self.workflow)

    def test_release_job_is_the_only_job_with_contents_write(self) -> None:
        self.assertIn("permissions:\n      contents: write", self.workflow)
        self.assertNotIn("pull_request_target", self.workflow)
        self.assertNotIn("SSH_PRIVATE_KEY", self.workflow)
        self.assertNotIn("VERCEL_TOKEN", self.workflow)


if __name__ == "__main__":
    unittest.main()
