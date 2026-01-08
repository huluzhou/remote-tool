#!/bin/bash
# 设置Python虚拟环境的脚本

set -e

echo "=== 设置Python虚拟环境 ==="

# 检查是否已安装python3-venv
if ! python3 -m venv --help > /dev/null 2>&1; then
    echo "错误: python3-venv 未安装"
    echo ""
    echo "请先安装 python3-venv:"
    echo "  Ubuntu/Debian: sudo apt-get install python3-venv"
    echo "  或者针对Python 3.12: sudo apt-get install python3.12-venv"
    echo "  CentOS/RHEL:   sudo yum install python3-venv"
    exit 1
fi

# 测试虚拟环境创建（检查ensurepip是否可用）
echo "检查虚拟环境支持..."
if ! python3 -m venv --help > /dev/null 2>&1; then
    echo "警告: python3-venv 可能未正确安装"
fi

# 进入项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "项目目录: $PROJECT_ROOT"

# 检查虚拟环境是否完整
if [ -d "venv" ]; then
    if [ -f "venv/bin/activate" ]; then
        echo "虚拟环境已存在且完整，跳过创建"
    else
        echo "检测到不完整的虚拟环境，删除并重新创建..."
        rm -rf venv
        echo "创建虚拟环境..."
        python3 -m venv venv
        if [ ! -f "venv/bin/activate" ]; then
            echo "错误: 虚拟环境创建失败"
            echo "请确保已安装 python3-venv:"
            echo "  sudo apt-get install python3.12-venv"
            exit 1
        fi
        echo "虚拟环境创建成功"
    fi
else
    echo "创建虚拟环境..."
    python3 -m venv venv
    if [ ! -f "venv/bin/activate" ]; then
        echo "错误: 虚拟环境创建失败"
        echo "请确保已安装 python3-venv:"
        echo "  sudo apt-get install python3.12-venv"
        exit 1
    fi
    echo "虚拟环境创建成功"
fi

# 激活虚拟环境并安装依赖
echo ""
echo "激活虚拟环境并安装依赖..."
if [ ! -f "venv/bin/activate" ]; then
    echo "错误: 无法找到 venv/bin/activate"
    exit 1
fi
source venv/bin/activate

echo "升级pip..."
pip install --upgrade pip --quiet

echo "安装项目依赖..."
pip install -r query_tool/requirements.txt

echo ""
echo "=== 安装完成 ==="
echo ""
echo "要使用虚拟环境，请运行:"
echo "  source venv/bin/activate"
echo ""
echo "然后可以使用工具:"
echo "  python -m query_tool.ui.tkinter_ui"
echo "  python -m query_tool.cli -H <host> -u <user> -p <password> -s today -e now"
echo ""
