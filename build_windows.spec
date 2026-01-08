# -*- mode: python ; coding: utf-8 -*-
"""
PyInstaller 配置文件 - Windows 64位打包
"""

import sys
from pathlib import Path

# 项目根目录
project_root = Path(SPECPATH).parent
query_tool_dir = project_root / "query_tool"

block_cipher = None

# 注意：配置文件不打包到 exe 内部
# 配置文件会在打包后自动复制到 dist 目录，作为外部文件
# 程序运行时从 exe 同目录读取配置文件
datas = []

# 隐藏导入（PyInstaller 可能无法自动检测的模块）
hiddenimports = [
    "paramiko",
    "toml",
    "tkinter",
    "tkinter.ttk",
    "tkinter.messagebox",
    "tkinter.filedialog",
    "tkinter.font",
    "cryptography",
    "cryptography.hazmat",
    "cryptography.hazmat.backends",
    "cryptography.hazmat.backends.openssl",
    "cryptography.hazmat.primitives",
    "cryptography.hazmat.primitives.ciphers",
    "cryptography.hazmat.primitives.serialization",
    "bcrypt",
    "nacl",  # pynacl 的实际导入名是 nacl
]

a = Analysis(
    ['run_ui.py'],
    pathex=[str(project_root)],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='query_tool',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,  # 不显示控制台窗口（GUI应用）
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch='x86_64',
    codesign_identity=None,
    entitlements_file=None,
    icon=None,  # 可以添加图标文件路径
)
