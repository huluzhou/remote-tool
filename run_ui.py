#!/usr/bin/env python3
"""
启动图形界面
"""

import sys
import os
from pathlib import Path

# 处理打包后的路径
if getattr(sys, 'frozen', False):
    # 打包后的可执行文件
    base_path = Path(sys.executable).parent
    project_root = base_path
else:
    # 开发环境
    project_root = Path(__file__).parent.parent
    sys.path.insert(0, str(project_root))

# 检查 tkinter 是否可用
try:
    import tkinter
except ImportError:
    print("=" * 60)
    print("错误: tkinter 未安装")
    print("=" * 60)
    print()
    print("图形界面需要安装 tkinter 系统包。")
    print()
    print("安装方法:")
    print("  Ubuntu/Debian: sudo apt-get install python3-tk")
    print("  CentOS/RHEL:   sudo yum install python3-tkinter")
    print()
    print("如果只使用命令行工具，可以跳过此步骤。")
    print("命令行工具: python -m query_tool.cli")
    print()
    sys.exit(1)

import logging

from query_tool.ui.tkinter_ui import main

if __name__ == "__main__":
    # 配置日志输出到控制台
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S'
    )
    main()
