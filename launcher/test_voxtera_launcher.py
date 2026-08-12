"""Contract and regression tests for the Voxtera launcher."""

from __future__ import annotations

import os
from pathlib import Path
import queue
import shutil
import ssl
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch
from urllib.error import URLError
import zipfile

import certifi

import voxtera_launcher as launcher
from voxtera_launcher import (
    find_platform_archive,
    game_launch_environment,
    installed_game_path,
    manifest_sha256_for_platform,
    platform_spec,
)


class UserConfigurationTests(unittest.TestCase):
    def test_macos_uses_user_writable_config_and_install_directories(self) -> None:
        """A distributed .app must never write configuration or game files inside
        its own bundle, because Gatekeeper/App Translocation can make it read-only."""
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"

            config_file = launcher.user_config_path("Darwin", home=home)
            install_dir = launcher.default_install_dir("Darwin", home=home)

        self.assertEqual(
            config_file,
            home / "Library" / "Application Support" / "Voxtera" / "voxtera_config.json",
        )
        self.assertEqual(install_dir, home / "Applications" / "Voxtera")
        self.assertNotIn("VoxteraLauncher.app", str(config_file))
        self.assertNotIn("VoxteraLauncher.app", str(install_dir))

    def test_windows_uses_localappdata_for_config_and_installation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            environment = {"LOCALAPPDATA": str(root / "LocalAppData")}

            config_file = launcher.user_config_path("Windows", home=root / "home", environ=environment)
            install_dir = launcher.default_install_dir("Windows", home=root / "home", environ=environment)

        self.assertEqual(config_file, root / "LocalAppData" / "Voxtera" / "voxtera_config.json")
        self.assertEqual(install_dir, root / "LocalAppData" / "Voxtera")

    def test_selected_install_directory_is_normalized_and_persisted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config_file = root / "Application Support" / "Voxtera" / "voxtera_config.json"
            selected = root / "Games" / "alpha" / ".." / "Voxtera"

            install_dir = launcher.normalize_install_dir(selected)
            launcher.save_config(
                {"install_dir": install_dir, "installed_version": "v0.4.0"},
                config_file=config_file,
            )
            reloaded = launcher.load_config(config_file=config_file, legacy_config_file=None)

            self.assertEqual(install_dir, str((root / "Games" / "Voxtera").resolve()))
            self.assertEqual(reloaded["install_dir"], install_dir)
            self.assertEqual(reloaded["installed_version"], "v0.4.0")
            self.assertTrue(config_file.is_file())

    def test_launcher_bundle_is_not_an_install_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = root / "VoxteraLauncher.app"
            bundle_game_path = bundle / "Contents" / "MacOS" / "game"

            self.assertTrue(
                launcher.is_inside_launcher_bundle(bundle_game_path, base_dir=bundle / "Contents" / "MacOS")
            )
            self.assertFalse(
                launcher.is_inside_launcher_bundle(root / "Games" / "Voxtera", base_dir=bundle / "Contents" / "MacOS")
            )


class TlsCertificateTests(unittest.TestCase):
    def test_frozen_launcher_uses_bundled_ca_and_keeps_tls_verification_enabled(self) -> None:
        """The distributed macOS app must not rely on the launcher's inherited shell CA path."""
        with tempfile.TemporaryDirectory() as directory:
            resource_root = Path(directory)
            certificate = resource_root / "certifi" / "cacert.pem"
            certificate.parent.mkdir()
            shutil.copyfile(certifi.where(), certificate)

            with patch.object(launcher.sys, "frozen", True, create=True), patch.object(
                launcher.sys,
                "_MEIPASS",
                str(resource_root),
                create=True,
            ):
                context = launcher.create_https_context()

        self.assertEqual(context.verify_mode, ssl.CERT_REQUIRED)
        self.assertTrue(context.check_hostname)
        self.assertGreater(len(context.get_ca_certs()), 0)

    def test_tls_validation_failure_has_a_concise_user_facing_message(self) -> None:
        error = URLError(ssl.SSLCertVerificationError(1, "certificate verify failed"))

        self.assertEqual(
            launcher.update_check_error_message(error),
            "Não foi possível validar o certificado HTTPS. Verifique a data, a hora e a rede.",
        )

    def test_api_get_passes_the_verifying_context_to_urlopen(self) -> None:
        context = object()

        class Response:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, traceback):
                return False

            def read(self):
                return b"[]"

        with patch.object(launcher, "create_https_context", return_value=context), patch.object(
            launcher,
            "urlopen",
            return_value=Response(),
        ) as mocked_urlopen:
            self.assertEqual(launcher.api_get("https://example.invalid/releases"), [])

        self.assertIs(mocked_urlopen.call_args.kwargs["context"], context)

    def test_download_passes_the_verifying_context_to_urlopen(self) -> None:
        context = object()

        class Response:
            headers = {"Content-Length": "2"}

            def __init__(self):
                self._chunks = iter((b"ok", b""))

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, traceback):
                return False

            def read(self, chunk_size):
                return next(self._chunks)

        with tempfile.TemporaryDirectory() as directory, patch.object(
            launcher,
            "create_https_context",
            return_value=context,
        ), patch.object(launcher, "urlopen", return_value=Response()) as mocked_urlopen:
            destination = Path(directory) / "archive.zip"
            launcher._download_file_once("https://example.invalid/archive.zip", destination)
            self.assertEqual(destination.read_bytes(), b"ok")

        self.assertIs(mocked_urlopen.call_args.kwargs["context"], context)

    def test_macos_buttons_use_dark_text_on_the_native_light_control_surface(self) -> None:
        self.assertEqual(launcher.button_foreground("Darwin"), "#1f2937")
        self.assertEqual(launcher.button_foreground("Windows"), launcher.TEXT_PRIMARY)

class PlayActionTests(unittest.TestCase):
    """Reproduce the silent-failure bug when game binary lacks +x."""

    @unittest.skipIf(os.name == "nt", "Windows does not expose POSIX execute-bit semantics")
    def test_popen_raises_permission_error_when_game_binary_is_not_executable(self) -> None:
        """Demonstrates that the current _play code path fails silently on macOS
        because the extracted game binary is -rw-r--r-- (no execute bit)."""
        with tempfile.TemporaryDirectory() as tmp:
            spec = platform_spec("Darwin")
            game_path = Path(tmp).joinpath(*spec.executable_path)
            game_path.parent.mkdir(parents=True, exist_ok=True)
            game_path.write_text("#!/bin/sh\nexit 0\n")
            # Confirm it IS a file (so _play enters the Popen branch)
            self.assertTrue(game_path.is_file())
            # But without +x, Popen raises PermissionError
            self.assertEqual(oct(game_path.stat().st_mode)[-3:], "644")
            with self.assertRaises(PermissionError):
                subprocess.Popen([str(game_path)], cwd=tmp)

    def test_extracted_binaries_receive_execute_permissions(self) -> None:
        """After extraction, platform-specific executables must be +x."""
        with tempfile.TemporaryDirectory() as tmp:
            zip_path = Path(tmp) / "test.zip"
            with zipfile.ZipFile(zip_path, "w") as zf:
                zf.writestr("Voxtera.app/Contents/MacOS/Voxtera", "#!/bin/sh\nexit 0\n")
                zf.writestr("Voxtera.app/Contents/MacOS/Voxtera-bin", "binary-contents")
                zf.writestr("Voxtera.app/Contents/Resources/assets/placeholder", "data")
            install_dir = Path(tmp) / "install"
            with zipfile.ZipFile(zip_path, "r") as zf:
                zf.extractall(install_dir)
            spec = platform_spec("Darwin")
            exe = install_dir / "Voxtera.app" / "Contents" / "MacOS" / "Voxtera"
            bin_ = install_dir / "Voxtera.app" / "Contents" / "MacOS" / "Voxtera-bin"
            if os.name != "nt":
                self.assertFalse(os.access(exe, os.X_OK))
                self.assertFalse(os.access(bin_, os.X_OK))
            # Apply the fix
            launcher._fix_extracted_executable_permissions(str(install_dir), spec)
            if os.name != "nt":
                self.assertTrue(os.access(exe, os.X_OK))
                self.assertTrue(os.access(bin_, os.X_OK))
            else:
                self.assertTrue(exe.is_file())
                self.assertTrue(bin_.is_file())


class ArchiveValidationTests(unittest.TestCase):
    def test_rejects_members_that_escape_install_dir_or_expand_excessively(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            malicious = root / "malicious.zip"
            with zipfile.ZipFile(malicious, "w") as archive:
                archive.writestr("../outside.txt", "nope")

            with self.assertRaisesRegex(RuntimeError, "fora"):
                launcher.validate_game_archive(malicious, root / "install")

            compressed = root / "compressed.zip"
            with zipfile.ZipFile(compressed, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                archive.writestr("large-zeroes.bin", b"\0" * 16_384)

            with self.assertRaisesRegex(RuntimeError, "compress"):
                launcher.validate_game_archive(
                    compressed,
                    root / "install",
                    max_compression_ratio=2,
                )


class PlatformAwareLauncherTests(unittest.TestCase):
    def setUp(self) -> None:
        self.release = {
            "assets": [
                {
                    "name": "Voxtera-windows-x64-v0.4.0.zip",
                    "browser_download_url": "https://example.invalid/windows.zip",
                },
                {
                    "name": "Voxtera-macos-universal-v0.4.0.zip",
                    "browser_download_url": "https://example.invalid/macos.zip",
                },
            ],
        }
        self.manifest = {
            "artifacts": {
                "windows-x64": {
                    "archive": "Voxtera-windows-x64-v0.4.0.zip",
                    "sha256": "windows-sha256",
                },
                "macos-universal": {
                    "archive": "Voxtera-macos-universal-v0.4.0.zip",
                    "sha256": "macos-sha256",
                },
            },
        }

    def test_windows_selects_only_the_windows_archive(self) -> None:
        spec = platform_spec("Windows")

        archive = find_platform_archive(self.release, spec)

        self.assertEqual(archive["name"], "Voxtera-windows-x64-v0.4.0.zip")
        self.assertEqual(
            installed_game_path(Path("C:/Games/Voxtera"), spec),
            Path("C:/Games/Voxtera/Voxtera.exe"),
        )

    def test_macos_selects_the_universal_archive_and_app_executable(self) -> None:
        spec = platform_spec("Darwin")

        archive = find_platform_archive(self.release, spec)

        self.assertEqual(archive["name"], "Voxtera-macos-universal-v0.4.0.zip")
        self.assertEqual(
            installed_game_path(Path("/Applications/Voxtera"), spec),
            Path("/Applications/Voxtera/Voxtera.app/Contents/MacOS/Voxtera"),
        )

    def test_manifest_hash_is_bound_to_the_selected_platform_archive(self) -> None:
        self.assertEqual(
            manifest_sha256_for_platform(
                self.manifest,
                platform_spec("Windows"),
                "Voxtera-windows-x64-v0.4.0.zip",
            ),
            "windows-sha256",
        )
        self.assertEqual(
            manifest_sha256_for_platform(
                self.manifest,
                platform_spec("Darwin"),
                "Voxtera-macos-universal-v0.4.0.zip",
            ),
            "macos-sha256",
        )

    def test_missing_or_invalid_platform_manifest_hash_is_rejected(self) -> None:
        spec = platform_spec("Darwin")
        archive = "Voxtera-macos-universal-v0.4.0.zip"
        sha256 = "a" * 64
        self.assertEqual(
            launcher.required_manifest_sha256(
                {"artifacts": {spec.key: {"archive": archive, "sha256": sha256}}},
                spec,
                archive,
            ),
            sha256,
        )

        with self.assertRaisesRegex(RuntimeError, "SHA-256"):
            launcher.required_manifest_sha256({}, spec, archive)
        with self.assertRaisesRegex(RuntimeError, "SHA-256"):
            launcher.required_manifest_sha256(
                {"artifacts": {spec.key: {"archive": archive, "sha256": "not-a-sha"}}},
                spec,
                archive,
            )

    def test_legacy_single_hash_manifest_remains_compatible_with_windows(self) -> None:
        self.assertEqual(
            manifest_sha256_for_platform(
                {"zip_sha256": "legacy-windows-sha256"},
                platform_spec("Windows"),
                "Voxtera-windows-x64-v0.3.8.5.zip",
            ),
            "legacy-windows-sha256",
        )
        self.assertIsNone(
            manifest_sha256_for_platform(
                {"zip_sha256": "legacy-windows-sha256"},
                platform_spec("Darwin"),
                "Voxtera-macos-universal-v0.3.8.5.zip",
            ),
        )

    def test_macos_launch_environment_points_to_packaged_resources(self) -> None:
        spec = platform_spec("Darwin")
        with tempfile.TemporaryDirectory() as directory:
            install_dir = Path(directory)
            assets_dir = install_dir / "Voxtera.app" / "Contents" / "Resources" / "assets"
            assets_dir.mkdir(parents=True)

            environment = game_launch_environment(install_dir, spec, {"PATH": os.environ["PATH"]})

        self.assertEqual(environment["VELOREN_ASSETS"], str(assets_dir))

    def test_unsupported_platform_has_no_release_contract(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "Unsupported platform"):
            platform_spec("Linux")


class ReleaseSelectionTests(unittest.TestCase):
    def test_launcher_only_release_is_skipped_for_game_updates(self) -> None:
        """A new launcher release must not hide the latest platform game archive."""
        launcher_release = {
            "tag_name": "launcher-v0.4.1",
            "assets": [{"name": "VoxteraLauncher.app.zip"}],
        }
        game_release = {
            "tag_name": "v0.4.0",
            "assets": [
                {
                    "name": "Voxtera-macos-universal-v0.4.0.zip",
                    "browser_download_url": "https://example.invalid/macos-game.zip",
                },
            ],
        }

        release, asset = launcher.find_latest_game_release(
            [launcher_release, game_release],
            platform_spec("Darwin"),
        )

        self.assertIs(release, game_release)
        self.assertEqual(asset["name"], "Voxtera-macos-universal-v0.4.0.zip")

    def test_prerelease_is_skipped_in_favor_of_a_stable_game_release(self) -> None:
        prerelease = {
            "tag_name": "v0.5.0-rc.1",
            "prerelease": True,
            "assets": [{"name": "Voxtera-macos-universal-v0.5.0-rc.1.zip"}],
        }
        stable = {
            "tag_name": "v0.4.0",
            "assets": [{"name": "Voxtera-macos-universal-v0.4.0.zip"}],
        }

        release, asset = launcher.find_latest_game_release(
            [prerelease, stable],
            platform_spec("Darwin"),
        )

        self.assertIs(release, stable)
        self.assertEqual(asset["name"], "Voxtera-macos-universal-v0.4.0.zip")


class MainThreadUiQueueTests(unittest.TestCase):
    """Tk must only be called from the thread that created its interpreter."""

    class _ThreadOwnedLauncher:
        def __init__(self) -> None:
            self._ui_queue = queue.Queue()
            self._tk_alive = True
            self._tk_thread_id = threading.get_ident()
            self.after_calls = []
            self.status = "Verificando atualizações..."

        def after(self, milliseconds, callback):
            if threading.get_ident() != self._tk_thread_id:
                raise AssertionError("worker thread invoked Tk.after")
            self.after_calls.append((milliseconds, callback))

        def _set_status(self, text, color=None) -> None:
            self.status = text

    def _bind(self, fake) -> None:
        fake._post_ui = launcher.VoxteraLauncher._post_ui.__get__(fake)
        fake._drain_ui_queue = launcher.VoxteraLauncher._drain_ui_queue.__get__(fake)

    def test_worker_posts_without_touching_tk(self) -> None:
        fake = self._ThreadOwnedLauncher()
        self._bind(fake)

        worker = threading.Thread(
            target=lambda: fake._post_ui(lambda: fake._set_status("Nova versão: v0.4.0")),
        )
        worker.start()
        worker.join(timeout=2)

        self.assertFalse(worker.is_alive())
        self.assertEqual(fake.after_calls, [])
        self.assertEqual(fake._ui_queue.qsize(), 1)

    def test_main_thread_drain_updates_widgets_and_reschedules(self) -> None:
        fake = self._ThreadOwnedLauncher()
        self._bind(fake)
        fake._post_ui(lambda: fake._set_status("Nova versão: v0.4.0"))

        fake._drain_ui_queue()

        self.assertEqual(fake.status, "Nova versão: v0.4.0")
        self.assertEqual(len(fake.after_calls), 1)
        self.assertEqual(fake.after_calls[0][0], 50)

    def test_drain_uses_a_bounded_batch_when_work_keeps_arriving(self) -> None:
        fake = self._ThreadOwnedLauncher()
        fake.completed_callbacks = 0
        self._bind(fake)
        for _ in range(launcher.UI_QUEUE_BATCH_SIZE + 1):
            fake._post_ui(lambda: setattr(fake, "completed_callbacks", fake.completed_callbacks + 1))

        fake._drain_ui_queue()

        self.assertEqual(fake.completed_callbacks, launcher.UI_QUEUE_BATCH_SIZE)
        self.assertEqual(fake._ui_queue.qsize(), 1)
        self.assertEqual(fake.after_calls[0][0], 1)

    def test_progress_updates_are_coalesced_before_the_main_thread_drains_them(self) -> None:
        class ProgressBar:
            def __init__(self) -> None:
                self.values = []

            def config(self, **kwargs) -> None:
                self.values.append(kwargs)

        class ProgressLabel:
            def __init__(self) -> None:
                self.texts = []

            def config(self, **kwargs) -> None:
                self.texts.append(kwargs.get("text"))

        fake = self._ThreadOwnedLauncher()
        fake.progress = ProgressBar()
        fake.progress_label = ProgressLabel()
        fake._progress_lock = threading.Lock()
        fake._pending_progress = None
        fake._progress_callback_posted = False
        self._bind(fake)
        fake._post_progress_update = launcher.VoxteraLauncher._post_progress_update.__get__(fake)
        fake._apply_pending_progress = launcher.VoxteraLauncher._apply_pending_progress.__get__(fake)

        fake._post_progress_update(10, "1.0 / 10.0 MB (10%)")
        fake._post_progress_update(20, "2.0 / 10.0 MB (20%)")
        self.assertEqual(fake._ui_queue.qsize(), 1)

        fake._drain_ui_queue()

        self.assertEqual(fake.progress.values, [{"value": 20}])
        self.assertEqual(fake.progress_label.texts, ["2.0 / 10.0 MB (20%)"])


if __name__ == "__main__":
    unittest.main()
