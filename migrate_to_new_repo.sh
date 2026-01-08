#!/bin/bash
# Query Tool 迁移脚本
# 用于将 query_tool 迁移到独立仓库

set -e

REPO_URL="git@github.com:huluzhou/query-tool.git"
REPO_NAME="query-tool"
CURRENT_DIR=$(pwd)
PARENT_DIR=$(dirname "$CURRENT_DIR")

echo "=========================================="
echo "Query Tool 迁移脚本"
echo "=========================================="
echo ""

# 检查是否在正确的目录
if [ ! -d "query_tool" ] && [ ! -f "requirements.txt" ]; then
    echo "错误: 请在 ems-analysis 项目根目录或 query_tool 目录下运行此脚本"
    exit 1
fi

# 确定源目录
if [ -d "query_tool" ]; then
    SOURCE_DIR="query_tool"
    BASE_DIR="$CURRENT_DIR"
else
    SOURCE_DIR="."
    BASE_DIR="$PARENT_DIR"
fi

echo "源目录: $BASE_DIR/$SOURCE_DIR"
echo "目标仓库: $REPO_URL"
echo ""

# 检查目标目录是否已存在
if [ -d "../$REPO_NAME" ]; then
    echo "警告: 目标目录 ../$REPO_NAME 已存在"
    read -p "是否删除并重新克隆? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "../$REPO_NAME"
    else
        echo "取消迁移"
        exit 1
    fi
fi

# 克隆新仓库
echo "步骤 1: 克隆新仓库..."
cd "$BASE_DIR"
if [ ! -d "$REPO_NAME" ]; then
    git clone "$REPO_URL" "$REPO_NAME"
fi
cd "$REPO_NAME"

# 复制文件
echo ""
echo "步骤 2: 复制文件..."
if [ "$SOURCE_DIR" = "query_tool" ]; then
    # 从主仓库的 query_tool 目录复制
    cp -r "../ems-analysis/$SOURCE_DIR"/* . 2>/dev/null || true
    cp -r "../ems-analysis/$SOURCE_DIR"/.* . 2>/dev/null || true
else
    # 已经在 query_tool 目录中
    cp -r "$SOURCE_DIR"/* . 2>/dev/null || true
    cp -r "$SOURCE_DIR"/.* . 2>/dev/null || true
fi

# 复制 GitHub Actions
if [ -d "../ems-analysis/$SOURCE_DIR/.github" ]; then
    mkdir -p .github/workflows
    cp "../ems-analysis/$SOURCE_DIR/.github/workflows/build-windows.yml" .github/workflows/ 2>/dev/null || true
fi

# 重命名新文件
echo ""
echo "步骤 3: 重命名文件..."
if [ -f "README_NEW.md" ]; then
    if [ -f "README.md" ]; then
        mv README.md README.md.old
    fi
    mv README_NEW.md README.md
    echo "✓ README_NEW.md -> README.md"
fi

if [ -f "USER_GUIDE_NEW.md" ]; then
    if [ -f "USER_GUIDE.md" ]; then
        mv USER_GUIDE.md USER_GUIDE.md.old
    fi
    mv USER_GUIDE_NEW.md USER_GUIDE.md
    echo "✓ USER_GUIDE_NEW.md -> USER_GUIDE.md"
fi

if [ -f ".gitignore_ROOT" ]; then
    if [ ! -f ".gitignore" ]; then
        cp .gitignore_ROOT .gitignore
        echo "✓ .gitignore_ROOT -> .gitignore"
    fi
fi

# 清理不需要的文件
echo ""
echo "步骤 4: 清理文件..."
rm -f README.md.old USER_GUIDE.md.old .gitignore_ROOT 2>/dev/null || true
rm -rf build dist __pycache__ *.pyc 2>/dev/null || true

# 显示文件列表
echo ""
echo "步骤 5: 文件清单..."
echo "已复制的文件:"
ls -la | grep -E "^-" | awk '{print "  - " $9}'
echo ""
echo "目录结构:"
find . -type d -not -path '*/\.*' | head -20

# 检查必要文件
echo ""
echo "步骤 6: 检查必要文件..."
MISSING_FILES=0

check_file() {
    if [ ! -f "$1" ]; then
        echo "  ✗ 缺少: $1"
        MISSING_FILES=$((MISSING_FILES + 1))
    else
        echo "  ✓ $1"
    fi
}

check_file "README.md"
check_file "USER_GUIDE.md"
check_file "requirements.txt"
check_file "csv_export_config.toml"
check_file ".gitignore"
check_file ".github/workflows/build-windows.yml"
check_file "query_tool/__init__.py"
check_file "query_tool/cli.py"

if [ $MISSING_FILES -gt 0 ]; then
    echo ""
    echo "警告: 缺少 $MISSING_FILES 个必要文件"
else
    echo ""
    echo "✓ 所有必要文件都已就绪"
fi

# 提示提交
echo ""
echo "=========================================="
echo "迁移准备完成！"
echo "=========================================="
echo ""
echo "下一步操作:"
echo "1. 检查文件是否正确: cd $REPO_NAME && ls -la"
echo "2. 添加文件到 Git: git add ."
echo "3. 提交: git commit -m '初始提交：从 ems-analysis 迁移 query_tool'"
echo "4. 推送: git push origin main"
echo ""
echo "或者运行以下命令:"
echo "  cd $REPO_NAME"
echo "  git add ."
echo "  git status  # 检查要提交的文件"
echo "  git commit -m '初始提交：从 ems-analysis 迁移 query_tool'"
echo "  git push origin main"
echo ""
