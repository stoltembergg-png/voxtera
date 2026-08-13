# -*- mode: python ; coding: utf-8 -*-

import os
import sys

from PyInstaller.utils.hooks import collect_data_files

try:
    import certifi
except ImportError as exc:
    raise SystemExit("certifi é obrigatório para empacotar conexões HTTPS verificadas") from exc

CERTIFI_DATAS = collect_data_files("certifi", includes=["cacert.pem"])
if not CERTIFI_DATAS:
    raise SystemExit("certifi/cacert.pem não foi encontrado para o bundle do launcher")

a = Analysis(
    ['voxtera_launcher.py'],
    pathex=[],
    binaries=[],
    datas=[('voxtera_logo.png', '.')] + CERTIFI_DATAS,
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)
is_macos = sys.platform == "darwin"
LAUNCHER_VERSION = os.environ.get("VOXTERA_LAUNCHER_VERSION", "0.4.11")

exe = EXE(
    pyz,
    a.scripts,
    [] if is_macos else a.binaries,
    [] if is_macos else a.datas,
    [],
    name='VoxteraLauncher',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=os.environ.get("VOXTERA_PYINSTALLER_TARGET_ARCH"),
    codesign_identity=None,
    entitlements_file=None,
    exclude_binaries=sys.platform == "darwin",
)

if is_macos:
    from PyInstaller.building.osx import BUNDLE

    coll = COLLECT(
        exe,
        a.binaries,
        a.zipfiles,
        a.datas,
        strip=False,
        upx=True,
        upx_exclude=[],
        name="VoxteraLauncher",
    )
    app = BUNDLE(
        coll,
        name="VoxteraLauncher.app",
        icon=None,
        bundle_identifier="app.voxtera.launcher",
        info_plist={
            "CFBundleDisplayName": "Voxtera Launcher",
            "CFBundleShortVersionString": LAUNCHER_VERSION,
            "CFBundleVersion": LAUNCHER_VERSION,
            "NSHighResolutionCapable": "True",
        },
    )
