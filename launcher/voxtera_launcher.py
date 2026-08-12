#!/usr/bin/env python3
"""
Voxtera Game Launcher
Downloads updates from GitHub releases and launches the game.
"""

import hashlib
import json
import logging
import os
import platform
import socket
import ssl
import subprocess
import sys
import queue
import threading
import time
from dataclasses import dataclass
from pathlib import Path
import tkinter as tk
from tkinter import ttk, messagebox, filedialog
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError
import zipfile

# ── Path helpers ───────────────────────────────────────────────────────────────

def get_base_dir():
    if getattr(sys, 'frozen', False):
        return os.path.dirname(sys.executable)
    return os.path.dirname(os.path.abspath(__file__))

BASE_DIR = get_base_dir()

# ── Constants ──────────────────────────────────────────────────────────────────
GITHUB_REPO = "stoltembergg-png/voxtera"
GITHUB_API = f"https://api.github.com/repos/{GITHUB_REPO}/releases"
APP_NAME = "Voxtera"
LAUNCHER_VERSION = "0.4.7"


def _user_home(home=None):
    """Return an injectable home directory for platform-path helpers."""
    return Path(home).expanduser() if home is not None else Path.home()


def user_config_path(system_name=None, *, home=None, environ=None):
    """Return the per-user launcher configuration path for an OS.

    This deliberately never resolves below ``BASE_DIR``: a frozen macOS app
    may run under Gatekeeper App Translocation, where its bundle is read-only.
    """
    system_name = system_name or platform.system()
    environment = os.environ if environ is None else environ
    user_home = _user_home(home)

    if system_name == "Windows":
        root = Path(environment.get("LOCALAPPDATA") or user_home / "AppData" / "Local")
    elif system_name == "Darwin":
        root = user_home / "Library" / "Application Support"
    else:
        root = Path(environment.get("XDG_CONFIG_HOME") or user_home / ".config")
    return root / APP_NAME / "voxtera_config.json"


def default_install_dir(system_name=None, *, home=None, environ=None):
    """Return a user-writable default game location, outside the launcher bundle."""
    system_name = system_name or platform.system()
    environment = os.environ if environ is None else environ
    user_home = _user_home(home)

    if system_name == "Windows":
        root = Path(environment.get("LOCALAPPDATA") or user_home / "AppData" / "Local")
        return root / APP_NAME
    if system_name == "Darwin":
        return user_home / "Applications" / APP_NAME
    return user_home / "Games" / APP_NAME
def normalize_install_dir(directory):
    """Canonicalize a directory selected by the user without requiring it to exist."""
    value = str(directory).strip()
    if not value:
        raise ValueError("A pasta de instalação não pode ficar vazia")
    return str(Path(value).expanduser().resolve())


def is_inside_launcher_bundle(directory, base_dir=None):
    """Return whether a chosen installation directory sits inside a .app bundle."""
    base = Path(base_dir or BASE_DIR).expanduser().resolve()
    bundle = next((candidate for candidate in (base, *base.parents)
                   if candidate.suffix.lower() == ".app"), None)
    if bundle is None:
        return False
    try:
        Path(directory).expanduser().resolve().relative_to(bundle)
        return True
    except ValueError:
        return False


def launcher_log_path(system_name=None, *, home=None, environ=None):
    """Store diagnostics beside the user configuration, never inside an app bundle."""
    return user_config_path(system_name, home=home, environ=environ).with_name("launcher.log")


def setup_launcher_logger(log_file=None):
    """Create a file logger, falling back to stderr if user storage is unavailable."""
    logger = logging.getLogger("voxtera_launcher")
    logger.setLevel(logging.INFO)
    logger.propagate = False
    target = Path(log_file or launcher_log_path()).resolve()
    for handler in logger.handlers:
        if isinstance(handler, logging.FileHandler) and Path(handler.baseFilename) == target:
            return logger
    try:
        target.parent.mkdir(parents=True, exist_ok=True)
        handler = logging.FileHandler(target, encoding="utf-8")
    except OSError:
        handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter(
        "%(asctime)s %(levelname)s %(threadName)s %(message)s"
    ))
    logger.addHandler(handler)
    return logger


CONFIG_FILE = user_config_path()
DEFAULT_INSTALL_DIR = default_install_dir()
# Legacy builds wrote beside the executable. Only frozen builds can sensibly
# migrate that old location; source checkouts must not import a repo-local config.
LEGACY_CONFIG_FILE = (
    Path(BASE_DIR) / "voxtera_config.json" if getattr(sys, "frozen", False) else None
)


@dataclass(frozen=True)
class PlatformSpec:
    key: str
    archive_prefix: str
    executable_path: tuple[str, ...]
    assets_path: tuple[str, ...]


PLATFORM_SPECS = {
    "Windows": PlatformSpec(
        key="windows-x64",
        archive_prefix="Voxtera-windows-x64-",
        executable_path=("Voxtera.exe",),
        assets_path=("assets",),
    ),
    "Darwin": PlatformSpec(
        key="macos-universal",
        archive_prefix="Voxtera-macos-universal-",
        executable_path=("Voxtera.app", "Contents", "MacOS", "Voxtera"),
        assets_path=("Voxtera.app", "Contents", "Resources", "assets"),
    ),
}


def platform_spec(system_name=None):
    system_name = system_name or platform.system()
    spec = PLATFORM_SPECS.get(system_name)
    if spec is None:
        raise RuntimeError(f"Unsupported platform: {system_name}")
    return spec


def find_platform_archive(release, spec):
    matches = [
        asset
        for asset in release.get("assets", [])
        if asset.get("name", "").startswith(spec.archive_prefix)
        and asset.get("name", "").endswith(".zip")
    ]
    if len(matches) > 1:
        raise RuntimeError(f"Multiple {spec.key} archives found in this release")
    return matches[0] if matches else None


def find_latest_game_release(releases, spec):
    """Return the newest published release that contains this platform's game ZIP.

    Launcher-only releases are intentionally skipped: they must not make an
    otherwise working launcher conclude that no game package exists.
    """
    for release in releases:
        if release.get("draft") or release.get("prerelease"):
            continue
        archive = find_platform_archive(release, spec)
        if archive is not None:
            return release, archive
    return None, None


def manifest_sha256_for_platform(manifest, spec, archive_name):
    artifacts = manifest.get("artifacts")
    if isinstance(artifacts, dict):
        artifact = artifacts.get(spec.key)
        if isinstance(artifact, dict) and artifact.get("archive") == archive_name:
            sha256 = artifact.get("sha256")
            return sha256.lower().strip() if isinstance(sha256, str) else None

    # Releases created before platform-specific manifests remain valid on Windows.
    if spec.key == "windows-x64":
        sha256 = manifest.get("zip_sha256")
        return sha256.lower().strip() if isinstance(sha256, str) else None
    return None


def required_manifest_sha256(manifest, spec, archive_name):
    """Return a validated SHA-256 for the exact platform archive or fail closed."""
    sha256 = manifest_sha256_for_platform(manifest, spec, archive_name)
    is_sha256 = (
        isinstance(sha256, str)
        and len(sha256) == 64
        and all(character in "0123456789abcdef" for character in sha256.lower())
    )
    if not is_sha256:
        raise RuntimeError(f"Manifest sem SHA-256 válido para {archive_name}")
    assert isinstance(sha256, str)
    return sha256.lower()


def installed_game_path(install_dir, spec):
    return Path(install_dir).joinpath(*spec.executable_path)


def game_launch_environment(install_dir, spec, environ=None):
    environment = dict(os.environ if environ is None else environ)
    assets_dir = Path(install_dir).joinpath(*spec.assets_path)
    if assets_dir.is_dir():
        environment["VELOREN_ASSETS"] = str(assets_dir)
    return environment


def _fix_extracted_executable_permissions(install_dir, spec):
    """Set +x on game executables after extraction.

    GitHub Actions zips do not preserve Unix permission bits.
    Without this, subprocess.Popen raises PermissionError on macOS/Linux.
    """
    install_path = Path(install_dir)
    full = install_path.joinpath(*spec.executable_path)
    if full.is_file() and not os.access(full, os.X_OK):
        full.chmod(full.stat().st_mode | 0o111)
    # Also fix the sibling binary if present (macOS app bundles ship
    # a shell wrapper + a native Mach-O binary).
    if spec.key == "macos-universal":
        macos_dir = full.parent
        for sibling in macos_dir.iterdir() if macos_dir.is_dir() else ():
            if sibling.is_file() and not os.access(sibling, os.X_OK):
                sibling.chmod(sibling.stat().st_mode | 0o111)

# ── Theme ──────────────────────────────────────────────────────────────────────
BG_DARK = "#0d1117"
BG_MEDIUM = "#161b22"
BG_LIGHT = "#21262d"
BG_CARD = "#1c2128"
ACCENT = "#e94560"
ACCENT_HOVER = "#c81e45"
GREEN = "#3fb950"
GREEN_DARK = "#238636"
TEXT_PRIMARY = "#e6edf3"
TEXT_SECONDARY = "#8b949e"
TEXT_DIM = "#484f58"
BORDER = "#30363d"
UI_QUEUE_BATCH_SIZE = 64


def button_foreground(system_name=None):
    """Keep native macOS buttons legible when Aqua renders a light control face."""
    return "#1f2937" if (system_name or platform.system()) == "Darwin" else TEXT_PRIMARY

# ── Config ─────────────────────────────────────────────────────────────────────

def _default_config():
    return {
        "install_dir": str(DEFAULT_INSTALL_DIR),
        "installed_version": None,
    }


def _read_config(config_file):
    """Return a mapping from a config file, or None when it is unavailable/invalid."""
    try:
        with Path(config_file).open("r", encoding="utf-8") as config:
            parsed = json.load(config)
        return parsed if isinstance(parsed, dict) else None
    except (OSError, json.JSONDecodeError, ValueError):
        return None


def _merge_config(saved, defaults):
    merged = dict(defaults)
    if isinstance(saved, dict):
        for key in defaults:
            if key in saved:
                merged[key] = saved[key]
    # A malformed/non-string saved path must never flow into Path()/os.makedirs().
    if not isinstance(merged["install_dir"], str) or not merged["install_dir"].strip():
        merged["install_dir"] = defaults["install_dir"]
    if merged["installed_version"] is not None and not isinstance(merged["installed_version"], str):
        merged["installed_version"] = None
    return merged


def load_config(config_file=None, legacy_config_file=None):
    """Load per-user settings, optionally migrating a frozen legacy bundle config.

    A bad or inaccessible config is never deleted and never prevents the launcher
    from starting. The next successful save repairs it in the user data directory.
    """
    target = Path(config_file or CONFIG_FILE)
    defaults = _default_config()
    saved = _read_config(target)

    if saved is None:
        legacy = LEGACY_CONFIG_FILE if legacy_config_file is None else legacy_config_file
        if legacy is not None and Path(legacy) != target:
            saved = _read_config(legacy)

    return _merge_config(saved, defaults)


def save_config(cfg, config_file=None):
    """Persist settings atomically in the user data directory."""
    target = Path(config_file or CONFIG_FILE)
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_name(f".{target.name}.{os.getpid()}.tmp")
    try:
        with temporary.open("w", encoding="utf-8") as config:
            json.dump(cfg, config, indent=2)
            config.write("\n")
        os.replace(temporary, target)
    finally:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass

# ── Network ────────────────────────────────────────────────────────────────────

TLS_CA_BUNDLE_RESOURCE = Path("certifi") / "cacert.pem"


def tls_ca_bundle_path():
    """Return the CA bundle that must accompany a frozen launcher.

    A Finder-launched macOS app does not inherit the shell's ``SSL_CERT_FILE``.
    Therefore a frozen build must carry its own CA bundle rather than relying on
    a machine-specific Python installation or environment variable.
    """
    if getattr(sys, "frozen", False):
        resource_root = Path(getattr(sys, "_MEIPASS", BASE_DIR))
        bundle = resource_root / TLS_CA_BUNDLE_RESOURCE
        if not bundle.is_file():
            raise RuntimeError("Pacote do launcher não contém certificados TLS")
        return bundle

    try:
        import certifi
    except ImportError as exc:
        raise RuntimeError("Dependência certifi ausente para validar conexões HTTPS") from exc

    bundle = Path(certifi.where())
    if not bundle.is_file():
        raise RuntimeError("Bundle CA do certifi não foi encontrado")
    return bundle


def create_https_context():
    """Create a certificate-verifying HTTPS context from the packaged CA bundle."""
    return ssl.create_default_context(cafile=str(tls_ca_bundle_path()))


def update_check_error_message(exc):
    """Turn common update failures into concise UI text; diagnostics stay in the log."""
    if isinstance(exc, URLError) and isinstance(exc.reason, ssl.SSLCertVerificationError):
        return "Não foi possível validar o certificado HTTPS. Verifique a data, a hora e a rede."
    if isinstance(exc, (URLError, socket.timeout, ConnectionError, TimeoutError)):
        return "Não foi possível acessar as atualizações. Verifique a rede e tente novamente."
    return f"Não foi possível verificar atualizações: {str(exc)[:80]}"


def api_get(url, timeout=30):
    req = Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "VoxteraLauncher"})
    with urlopen(req, timeout=timeout, context=create_https_context()) as resp:
        return json.loads(resp.read().decode())

def _download_file_once(url, dest, progress_cb=None):
    """Single download attempt. Raises on any network/IO error."""
    req = Request(url, headers={"User-Agent": "VoxteraLauncher"})
    with urlopen(req, timeout=120, context=create_https_context()) as resp:
        total = int(resp.headers.get("Content-Length", 0))
        downloaded = 0
        chunk_size = 65536
        with open(dest, "wb") as f:
            while True:
                chunk = resp.read(chunk_size)
                if not chunk:
                    break
                f.write(chunk)
                downloaded += len(chunk)
                if progress_cb:
                    progress_cb(downloaded, total)

def download_file(url, dest, progress_cb=None, status_cb=None):
    """
    Download with retry: up to 3 attempts, 5s pause between attempts.
    Catches URLError, socket.timeout and other transient network errors.
    status_cb(attempt, max_attempts) is called before each attempt so the
    UI can show "Tentativa X/3".
    """
    max_attempts = 3
    last_exc = None
    for attempt in range(1, max_attempts + 1):
        if status_cb:
            status_cb(attempt, max_attempts)
        try:
            _download_file_once(url, dest, progress_cb)
            return  # success
        except (URLError, HTTPError, socket.timeout, ConnectionError, TimeoutError) as e:
            last_exc = e
            # Clean up partial file before retrying
            try:
                if os.path.exists(dest):
                    os.remove(dest)
            except OSError:
                pass
            if attempt < max_attempts:
                time.sleep(5)
            else:
                raise
        except Exception as e:
            # Unexpected error — clean up and re-raise immediately
            try:
                if os.path.exists(dest):
                    os.remove(dest)
            except OSError:
                pass
            raise
    # Should not reach here, but just in case
    if last_exc:
        raise last_exc

def parse_version(v):
    if not v:
        return (0,)
    v = v.lstrip("v")
    try:
        return tuple(int(x) for x in v.split("."))
    except ValueError:
        return (v,)

# ── Manifest / Integrity ───────────────────────────────────────────────────────

def compute_sha256(path, progress_cb=None, chunk_size=65536):
    """Compute SHA-256 of a file, optionally reporting bytes processed."""
    h = hashlib.sha256()
    processed = 0
    with open(path, "rb") as f:
        while True:
            chunk = f.read(chunk_size)
            if not chunk:
                break
            h.update(chunk)
            processed += len(chunk)
            if progress_cb:
                progress_cb(processed)
    return h.hexdigest()

def fetch_manifest(manifest_url):
    """Download and parse the manifest JSON for a release."""
    data = api_get(manifest_url)
    return data


def validate_game_archive(
    archive_path,
    destination,
    *,
    max_entries=100_000,
    max_uncompressed_bytes=75 * 1024 * 1024 * 1024,
    max_compression_ratio=1_000,
):
    """Reject unsafe or implausibly large game ZIPs before extraction."""
    target = Path(destination).expanduser().resolve()
    with zipfile.ZipFile(archive_path, "r") as archive:
        entries = archive.infolist()
        if len(entries) > max_entries:
            raise RuntimeError("Arquivo ZIP contém arquivos demais")

        total_uncompressed = 0
        for entry in entries:
            if "\\" in entry.filename:
                raise RuntimeError("Arquivo ZIP contém caminho inválido")
            member = (target / entry.filename).resolve()
            try:
                member.relative_to(target)
            except ValueError as exc:
                raise RuntimeError("Arquivo ZIP contém caminho fora da pasta de instalação") from exc

            if entry.is_dir():
                continue
            total_uncompressed += entry.file_size
            if total_uncompressed > max_uncompressed_bytes:
                raise RuntimeError("Arquivo ZIP excede o tamanho máximo descompactado")
            if entry.file_size > 0:
                if entry.compress_size == 0 or entry.file_size / entry.compress_size > max_compression_ratio:
                    raise RuntimeError("Arquivo ZIP excede a razão de compressão permitida")


def find_manifest_url(release):
    """
    Locate the manifest asset URL in a GitHub release.
    Looks for an asset named manifest-v{version}.json, falls back to any
    asset starting with 'manifest-' and ending with '.json'.
    """
    tag = release.get("tag_name", "").lstrip("v")
    expected = f"manifest-v{tag}.json"
    for asset in release.get("assets", []):
        if asset["name"] == expected:
            return asset["browser_download_url"]
    # Fallback: any manifest-*.json asset
    for asset in release.get("assets", []):
        name = asset["name"]
        if name.startswith("manifest-") and name.endswith(".json"):
            return asset["browser_download_url"]
    return None

# ── Main Application ───────────────────────────────────────────────────────────

class VoxteraLauncher(tk.Tk):
    def __init__(self):
        super().__init__()
        self._logger = setup_launcher_logger()
        self._logger.info("Launcher %s started; executable=%s", LAUNCHER_VERSION, sys.executable)
        self.title("Voxtera")
        self.geometry("520x740")
        self.resizable(False, False)
        self.configure(bg=BG_DARK)

        self.cfg = load_config()
        self.platform = platform_spec()
        self.latest_version = None
        self.download_url = None
        self.download_asset = None
        self.manifest_url = None
        self._downloading = False
        self._ui_queue = queue.Queue()
        self._progress_lock = threading.Lock()
        self._pending_progress = None
        self._progress_callback_posted = False
        self._tk_alive = True

        self._build_ui()
        self._start_ui_queue_pump()
        self._check_updates_thread()

    def _post_ui(self, callback):
        """Send a callback from any worker thread to Tk's owning thread.

        Worker threads must not call *any* Tk method, including ``after``. The
        main thread polls this queue from its Tk event loop in
        ``_drain_ui_queue`` and performs the widget mutation there.
        """
        if self._tk_alive:
            self._ui_queue.put(callback)

    def _post_progress_update(self, value, label_text=None):
        """Coalesce high-frequency worker progress notifications into one UI callback."""
        with self._progress_lock:
            self._pending_progress = (value, label_text)
            if self._progress_callback_posted:
                return
            self._progress_callback_posted = True
        self._post_ui(self._apply_pending_progress)

    def _apply_pending_progress(self):
        """Apply the newest progress state on the Tk-owning thread."""
        with self._progress_lock:
            pending = self._pending_progress
            self._pending_progress = None
            self._progress_callback_posted = False
        if pending is None or not self._tk_alive:
            return
        value, label_text = pending
        self.progress.config(value=value)
        if label_text is not None:
            self.progress_label.config(text=label_text)

    def _drain_ui_queue(self):
        """Run pending UI callbacks on the Tk-owning main thread."""
        if not self._tk_alive:
            return
        processed = 0
        while processed < UI_QUEUE_BATCH_SIZE:
            try:
                callback = self._ui_queue.get_nowait()
            except queue.Empty:
                break
            processed += 1
            try:
                callback()
            except (tk.TclError, RuntimeError):
                # The window may be closing while a network worker completes.
                if not self._tk_alive:
                    return
            except Exception as exc:
                self._logger.exception("UI callback failed: %s", exc)
        if self._tk_alive:
            delay_ms = 1 if not self._ui_queue.empty() else 50
            self.after(delay_ms, self._drain_ui_queue)

    def _start_ui_queue_pump(self):
        """Schedule the queue pump from the same thread that created Tk."""
        self.after(50, self._drain_ui_queue)

    # ── UI ─────────────────────────────────────────────────────────────────────

    def _build_ui(self):
        # Main container
        main = tk.Frame(self, bg=BG_DARK)
        main.pack(fill="both", expand=True, padx=30, pady=20)

        # ── Logo ───────────────────────────────────────────────────────────────
        # When frozen (PyInstaller), files are extracted to sys._MEIPASS temp dir
        if getattr(sys, 'frozen', False):
            logo_path = os.path.join(sys._MEIPASS, "voxtera_logo.png")
        else:
            logo_path = os.path.join(BASE_DIR, "voxtera_logo.png")

        if os.path.exists(logo_path):
            try:
                img = tk.PhotoImage(file=logo_path)
                scale = max(1, (max(img.width(), img.height()) + 249) // 250)
                self._logo_img = img.subsample(scale, scale)
                tk.Label(main, image=self._logo_img, bg=BG_DARK).pack(pady=(10, 5))
            except tk.TclError:
                tk.Label(main, text="VOXTERA", font=("Consolas", 42, "bold"),
                         bg=BG_DARK, fg=ACCENT).pack(pady=(20, 5))
        else:
            tk.Label(main, text="VOXTERA", font=("Consolas", 42, "bold"),
                     bg=BG_DARK, fg=ACCENT).pack(pady=(20, 5))

        tk.Label(main, text="Voxel RPG", font=("Consolas", 12),
                 bg=BG_DARK, fg=TEXT_SECONDARY).pack(pady=(0, 20))

        # ── Status ─────────────────────────────────────────────────────────────
        status_frame = tk.Frame(main, bg=BG_MEDIUM, highlightbackground=BORDER,
                                highlightthickness=1)
        status_frame.pack(fill="x", pady=(0, 10), ipady=10)

        self.version_label = tk.Label(
            status_frame,
            text="Verificando atualizações...",
            font=("Consolas", 10),
            bg=BG_MEDIUM,
            fg=TEXT_SECONDARY,
            justify="center",
            wraplength=430,
        )
        self.version_label.pack(pady=(5, 2))

        self.local_ver_label = tk.Label(status_frame, text="", font=("Consolas", 9),
                                         bg=BG_MEDIUM, fg=TEXT_DIM)
        self.local_ver_label.pack(pady=(0, 5))

        # ── Progress ───────────────────────────────────────────────────────────
        style = ttk.Style()
        style.theme_use("default")
        style.configure("Voxtera.Horizontal.TProgressbar",
                        troughcolor=BG_LIGHT, background=ACCENT,
                        darkcolor=ACCENT, lightcolor=ACCENT,
                        bordercolor=BG_DARK, relief="flat")

        self.progress = ttk.Progressbar(main, mode="determinate",
                                         style="Voxtera.Horizontal.TProgressbar")
        self.progress.pack(fill="x", pady=(0, 3))
        self.progress["value"] = 0

        self.progress_label = tk.Label(main, text="", font=("Consolas", 8),
                                        bg=BG_DARK, fg=TEXT_DIM)
        self.progress_label.pack(anchor="w", pady=(0, 15))

        # ── Buttons ────────────────────────────────────────────────────────────
        button_text = button_foreground()
        btn_style = {"font": ("Consolas", 14, "bold"), "width": 22, "height": 2,
                     "bd": 0, "cursor": "hand2", "relief": "flat",
                     "fg": button_text, "activeforeground": button_text}

        self.play_btn = tk.Button(main, text="▶  JOGAR", bg=GREEN,
                                   activebackground=GREEN_DARK,
                                   command=self._play, **btn_style)
        self.play_btn.pack(pady=(0, 8))

        self.update_btn = tk.Button(main, text="⟳  ATUALIZAR", bg=ACCENT,
                                     activebackground=ACCENT_HOVER,
                                     command=self._update, **btn_style)
        self.update_btn.pack(pady=(0, 8))
        self.update_btn.config(state="disabled")

        self.repair_btn = tk.Button(main, text="✦  REPARAR", bg=ACCENT,
                                     activebackground=ACCENT_HOVER,
                                     command=self._repair, **btn_style)
        self.repair_btn.pack(pady=(0, 15))
        self.repair_btn.config(state="disabled")

        # ── Install settings ────────────────────────────────────────────────────
        dir_frame = tk.Frame(main, bg=BG_CARD, highlightbackground=BORDER,
                             highlightthickness=1)
        dir_frame.pack(fill="x", pady=(0, 10), ipady=7)

        tk.Label(dir_frame, text="CONFIGURAÇÕES DE INSTALAÇÃO", font=("Consolas", 9, "bold"),
                 bg=BG_CARD, fg=TEXT_PRIMARY).pack(anchor="w", padx=10, pady=(3, 0))
        tk.Label(dir_frame, text="Escolha onde o jogo será baixado e atualizado.",
                 font=("Consolas", 8), bg=BG_CARD, fg=TEXT_DIM).pack(
                     anchor="w", padx=10, pady=(0, 4)
                 )

        dir_row = tk.Frame(dir_frame, bg=BG_CARD)
        dir_row.pack(fill="x", padx=10, pady=(0, 3))
        self.dir_label = tk.Label(
            dir_row,
            text=self.cfg["install_dir"],
            font=("Consolas", 8),
            bg=BG_CARD,
            fg=TEXT_SECONDARY,
            anchor="w",
            justify="left",
        )
        self.dir_label.pack(side="left", expand=True, fill="x", padx=(0, 7))

        tk.Button(
            dir_row,
            text="ESCOLHER PASTA…",
            bg=BG_LIGHT,
            fg=button_text,
            activeforeground=button_text,
            font=("Consolas", 8, "bold"),
            bd=0,
            cursor="hand2",
            command=self._change_install_dir,
        ).pack(side="right")

        # ── Footer ─────────────────────────────────────────────────────────────
        tk.Label(main, text=f"Launcher v{LAUNCHER_VERSION}", font=("Consolas", 8),
                 bg=BG_DARK, fg=TEXT_DIM).pack(side="bottom")

    # ── Install Check ─────────────────────────────────────────────────────────

    def _is_installed(self):
        """Check if the platform-specific game executable exists."""
        return installed_game_path(self.cfg["install_dir"], self.platform).is_file()

    # ── Update Check ───────────────────────────────────────────────────────────

    def _check_updates_thread(self):
        threading.Thread(target=self._do_check_updates, daemon=True).start()

    def _do_check_updates(self):
        try:
            self._logger.info(
                "Checking updates: platform=%s install_dir=%s",
                self.platform.key,
                self.cfg["install_dir"],
            )
            # First check if game is actually installed
            if not self._is_installed():
                self.cfg["installed_version"] = None
                save_config(self.cfg)
                self._post_ui(lambda: self._set_status("Jogo não instalado", ACCENT))
                self._post_ui(lambda: self.local_ver_label.config(text=""))
                self._post_ui(lambda: self.play_btn.config(state="disabled"))
            else:
                local_ver = self.cfg.get("installed_version")
                if local_ver:
                    self._post_ui(lambda: self.local_ver_label.config(
                        text=f"Instalado: {local_ver}"))
                    self._post_ui(lambda: self.play_btn.config(state="normal"))

            releases = api_get(GITHUB_API)
            self._logger.info("GitHub releases response contains %d entries", len(releases or []))
            if not releases:
                if self._is_installed():
                    self._post_ui(lambda: self._set_status(
                        "✓ Instalado (sem verificação de atualização)", GREEN))
                else:
                    self._post_ui(lambda: self._set_status(
                        "Nenhum release encontrado", ACCENT))
                return

            release, self.download_asset = find_latest_game_release(releases, self.platform)
            if release is not None:
                assert self.download_asset is not None
                self.latest_version = release["tag_name"]
                self.download_url = self.download_asset["browser_download_url"]
                self.manifest_url = find_manifest_url(release)
                self._logger.info(
                    "Selected game release=%s asset=%s manifest=%s",
                    self.latest_version,
                    self.download_asset["name"],
                    self.manifest_url,
                )
            else:
                self.latest_version = None
                self.download_url = None
                self.manifest_url = None
                self._logger.warning("No compatible game archive found for %s", self.platform.key)

            local_ver = self.cfg.get("installed_version")
            if self.download_url:
                if self._is_installed() and local_ver and parse_version(local_ver) >= parse_version(self.latest_version):
                    self._post_ui(lambda: self._set_status(
                        f"✓ Atualizado ({self.latest_version})", GREEN))
                    self._post_ui(lambda: self.play_btn.config(state="normal"))
                    self._post_ui(lambda: self.repair_btn.config(state="normal"))
                else:
                    self._post_ui(lambda: self._set_status(
                        f"Nova versão: {self.latest_version}", ACCENT))
                    self._post_ui(lambda: self.update_btn.config(state="normal"))
                    if self._is_installed():
                        self._post_ui(lambda: self.play_btn.config(state="normal"))
                        self._post_ui(lambda: self.repair_btn.config(state="normal"))
            else:
                self._post_ui(lambda: self._set_status(
                    f"Sem pacote para {self.platform.key}", ACCENT))

        except Exception as exc:
            error_message = update_check_error_message(exc)
            self._logger.exception("Update check failed")
            self._post_ui(lambda message=error_message: self._set_status(f"Erro: {message}", ACCENT))

    def _set_status(self, text, color=TEXT_SECONDARY):
        self._logger.info("UI status: %s", text)
        self.version_label.config(text=text, fg=color)

    # ── Download ───────────────────────────────────────────────────────────────

    def _update(self):
        if self._downloading or not self.download_url:
            return
        self._start_download(force=False)

    def _repair(self):
        """Force reinstall of the current/latest version (same version)."""
        if self._downloading or not self.download_url:
            return
        if not messagebox.askyesno(
                "Reparar instalação",
                "Isto vai reinstalar a versão atual, sobrescrevendo os arquivos do jogo.\n\nContinuar?"):
            return
        self._start_download(force=True)

    def _start_download(self, force=False):
        if self._downloading or not self.download_url:
            return
        self._downloading = True
        verb = "REINSTALANDO" if force else "BAIXANDO"
        self.update_btn.config(state="disabled", text=verb if not force else "⟳  ATUALIZAR")
        self.repair_btn.config(state="disabled", text="REINSTALANDO...")
        self.play_btn.config(state="disabled")
        threading.Thread(target=self._do_install, args=(force,), daemon=True).start()

    def _do_install(self, force=False):
        """Shared install/update logic. If force=True, reinstall same version."""
        try:
            install_dir = self.cfg["install_dir"]
            os.makedirs(install_dir, exist_ok=True)
            zip_path = os.path.join(install_dir, "voxtera_update.zip")

            target_version = self.latest_version

            def progress(downloaded, total):
                if total > 0:
                    pct = (downloaded / total) * 100
                    mb = downloaded / (1024 * 1024)
                    total_mb = total / (1024 * 1024)
                    self._post_progress_update(
                        pct,
                        f"{mb:.1f} / {total_mb:.1f} MB ({pct:.0f}%)",
                    )

            def download_status(attempt, max_attempts):
                self._post_ui(lambda current=attempt, total_attempts=max_attempts: self._set_status(
                    f"Baixando... (Tentativa {current}/{total_attempts})", TEXT_SECONDARY))
                self._post_ui(lambda current=attempt, total_attempts=max_attempts: self.progress_label.config(
                    text=f"Tentativa {current}/{total_attempts}"))

            self._post_ui(lambda: self.progress.config(mode="determinate", value=0))
            self._post_ui(lambda: self.progress_label.config(text=""))
            self._post_ui(lambda: self._set_status("Baixando...", TEXT_SECONDARY))
            download_file(self.download_url, zip_path, progress, status_cb=download_status)

            # ── SHA-256 verification via manifest ────────────────────────────────
            expected_sha = None
            if self.manifest_url:
                self._post_ui(lambda: self._set_status("Verificando integridade...", TEXT_SECONDARY))
                self._post_ui(lambda: self.progress.config(mode="indeterminate"))
                self._post_ui(lambda: self.progress.start(15))
                try:
                    manifest = fetch_manifest(self.manifest_url)
                    assert self.download_asset is not None
                    expected_sha = required_manifest_sha256(
                        manifest,
                        self.platform,
                        self.download_asset["name"],
                    )
                except Exception as me:
                    self._post_ui(lambda: self.progress.stop())
                    self._post_ui(lambda: self.progress.config(mode="determinate", value=0))
                    raise RuntimeError(f"Falha ao obter manifest: {str(me)[:80]}")

                if not expected_sha:
                    self._post_ui(lambda: self.progress.stop())
                    self._post_ui(lambda: self.progress.config(mode="determinate", value=0))
                    raise RuntimeError(
                        f"Manifest sem SHA-256 para {self.platform.key}")

                self._post_ui(lambda: self.progress.stop())
                self._post_ui(lambda: self.progress.config(mode="determinate"))
                self._post_ui(lambda: self._set_status(
                    "Calculando SHA-256...", TEXT_SECONDARY))

                zip_size = os.path.getsize(zip_path)
                def hash_progress(processed):
                    if zip_size > 0:
                        pct = (processed / zip_size) * 100
                        self._post_progress_update(pct)

                actual_sha = compute_sha256(zip_path, hash_progress)

                if actual_sha.lower() != expected_sha:
                    try:
                        os.remove(zip_path)
                    except OSError:
                        pass
                    self._post_ui(lambda: self.progress.config(value=0))
                    raise RuntimeError(
                        f"SHA-256 não confere!\n"
                        f"  Esperado: {expected_sha[:16]}...\n"
                        f"  Obtido:   {actual_sha[:16]}...\n"
                        f"O arquivo pode estar corrompido.")
                self._post_ui(lambda: self._set_status(
                    "✓ Integridade verificada", GREEN))
            else:
                raise RuntimeError(
                    f"Release sem manifesto de integridade para {self.platform.key}")

            # ── Extract ──────────────────────────────────────────────────────────
            self._post_ui(lambda: self._set_status("Extraindo...", TEXT_SECONDARY))
            self._post_ui(lambda: self.progress.config(mode="indeterminate"))
            self._post_ui(lambda: self.progress.start(15))

            validate_game_archive(zip_path, install_dir)
            with zipfile.ZipFile(zip_path, "r") as zf:
                zf.extractall(install_dir)
            os.remove(zip_path)

            _fix_extracted_executable_permissions(install_dir, self.platform)

            self.cfg["installed_version"] = target_version
            save_config(self.cfg)

            self._post_ui(lambda: self.progress.stop())
            self._post_ui(lambda: self.progress.config(mode="determinate", value=100))
            self._post_ui(lambda: self.progress_label.config(text=""))
            self._post_ui(lambda: self._set_status(
                f"✓ Instalado ({target_version})", GREEN))
            self._post_ui(lambda: self.local_ver_label.config(
                text=f"Instalado: {target_version}"))
            self._post_ui(lambda: self.play_btn.config(state="normal"))
            self._post_ui(lambda: self.update_btn.config(text="⟳  ATUALIZAR", state="disabled"))
            self._post_ui(lambda: self.repair_btn.config(text="✦  REPARAR", state="normal"))

        except Exception as exc:
            error_detail = str(exc)
            err_msg = error_detail[:120]
            self._logger.exception("Install/update failed")
            self._post_ui(lambda message=err_msg: self._set_status(f"Erro: {message}", ACCENT))
            self._post_ui(lambda message=error_detail: messagebox.showerror("Erro", message))
            self._post_ui(lambda: self.update_btn.config(text="⟳  ATUALIZAR", state="normal"))
            self._post_ui(lambda: self.repair_btn.config(text="✦  REPARAR", state="normal"))
            try:
                self._post_ui(lambda: self.progress.stop())
                self._post_ui(lambda: self.progress.config(mode="determinate", value=0))
                self._post_ui(lambda: self.progress_label.config(text=""))
            except Exception:
                pass
        finally:
            self._downloading = False

    # ── Actions ────────────────────────────────────────────────────────────────

    def _play(self):
        game_path = installed_game_path(self.cfg["install_dir"], self.platform)
        if game_path.is_file():
            try:
                subprocess.Popen(
                    [str(game_path)],
                    cwd=self.cfg["install_dir"],
                    env=game_launch_environment(self.cfg["install_dir"], self.platform),
                )
                self.destroy()
            except PermissionError:
                # Self-heal: GitHub Actions zips don't preserve Unix +x bits.
                _fix_extracted_executable_permissions(
                    self.cfg["install_dir"], self.platform
                )
                try:
                    subprocess.Popen(
                        [str(game_path)],
                        cwd=self.cfg["install_dir"],
                        env=game_launch_environment(
                            self.cfg["install_dir"], self.platform
                        ),
                    )
                    self.destroy()
                except Exception as exc:
                    messagebox.showerror(
                        "Erro ao iniciar o jogo",
                        f"Não foi possível executar {game_path.name}.\n\n{exc}",
                    )
            except Exception as exc:
                messagebox.showerror(
                    "Erro ao iniciar o jogo",
                    f"Não foi possível executar {game_path.name}.\n\n{exc}",
                )
        else:
            messagebox.showerror(
                "Jogo não encontrado",
                f"{game_path.name} não foi encontrado na pasta escolhida.\n\n"
                "Escolha a pasta onde o Voxtera já está instalado ou baixe o jogo.",
            )
            self._change_install_dir()

    def _change_install_dir(self):
        """Let the user choose and persist the directory used for game files."""
        if self._downloading:
            messagebox.showinfo(
                "Download em andamento",
                "Aguarde o término do download antes de trocar a pasta de instalação.",
            )
            return

        current = Path(self.cfg["install_dir"]).expanduser()
        initial_dir = current if current.is_dir() else _user_home()
        selected = filedialog.askdirectory(
            title="Escolha a pasta de instalação do Voxtera",
            initialdir=str(initial_dir),
        )
        if not selected:
            return

        try:
            install_dir = normalize_install_dir(selected)
        except ValueError as exc:
            messagebox.showerror("Pasta inválida", str(exc))
            return
        if is_inside_launcher_bundle(install_dir):
            messagebox.showerror(
                "Pasta inválida",
                "O jogo não pode ser instalado dentro do VoxteraLauncher.app.\n\n"
                "Escolha uma pasta fora do aplicativo, por exemplo ~/Applications/Voxtera.",
            )
            return

        previous = dict(self.cfg)
        self.cfg["install_dir"] = install_dir
        if self._is_installed():
            self.cfg["installed_version"] = previous.get("installed_version") or "unknown"
        else:
            self.cfg["installed_version"] = None

        try:
            save_config(self.cfg)
        except OSError as exc:
            self.cfg.clear()
            self.cfg.update(previous)
            self._logger.exception("Could not save selected install directory")
            messagebox.showerror("Não foi possível salvar a configuração", str(exc))
            return

        self._logger.info("Installation directory changed to %s", install_dir)
        self.dir_label.config(text=install_dir)
        if self._is_installed():
            version = self.cfg["installed_version"]
            self.local_ver_label.config(text=f"Instalado: {version}")
            self.play_btn.config(state="normal")
            self.repair_btn.config(state="normal" if self.download_url else "disabled")
            self._set_status("✓ Jogo encontrado na pasta escolhida", GREEN)
        else:
            self.play_btn.config(state="disabled")
            self.repair_btn.config(state="disabled")
            self.local_ver_label.config(text="")
            if self.download_url:
                self.update_btn.config(state="normal")
                self._set_status("Pasta escolhida — pronto para baixar", ACCENT)
            else:
                self._set_status("Pasta escolhida — verificando release", TEXT_SECONDARY)


    def destroy(self):
        # Workers may still finish, but _post_ui drops their callbacks after close.
        self._tk_alive = False
        try:
            super().destroy()
        except Exception:
            pass

# ── Entry Point ────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    app = VoxteraLauncher()
    app.mainloop()
