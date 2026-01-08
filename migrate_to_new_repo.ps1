# Query Tool 迁移脚本 (PowerShell)
# 用于将 query_tool 迁移到独立仓库

$ErrorActionPreference = "Stop"

$REPO_URL = "git@github.com:huluzhou/query-tool.git"
$REPO_NAME = "query-tool"
$CURRENT_DIR = Get-Location
$PARENT_DIR = Split-Path -Parent $CURRENT_DIR

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Query Tool 迁移脚本" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# 检查是否在正确的目录
$isQueryToolDir = Test-Path "requirements.txt"
$isMainDir = Test-Path "query_tool"

if (-not $isQueryToolDir -and -not $isMainDir) {
    Write-Host "错误: 请在 ems-analysis 项目根目录或 query_tool 目录下运行此脚本" -ForegroundColor Red
    exit 1
}

# 确定源目录
if ($isMainDir) {
    $SOURCE_DIR = "query_tool"
    $BASE_DIR = $CURRENT_DIR
} else {
    $SOURCE_DIR = "."
    $BASE_DIR = $PARENT_DIR
}

Write-Host "源目录: $BASE_DIR\$SOURCE_DIR" -ForegroundColor Yellow
Write-Host "目标仓库: $REPO_URL" -ForegroundColor Yellow
Write-Host ""

# 检查目标目录是否已存在
$targetPath = Join-Path $BASE_DIR "..\$REPO_NAME"
if (Test-Path $targetPath) {
    Write-Host "警告: 目标目录 $targetPath 已存在" -ForegroundColor Yellow
    $response = Read-Host "是否删除并重新克隆? (y/N)"
    if ($response -eq "y" -or $response -eq "Y") {
        Remove-Item -Recurse -Force $targetPath
    } else {
        Write-Host "取消迁移" -ForegroundColor Red
        exit 1
    }
}

# 克隆新仓库
Write-Host "步骤 1: 克隆新仓库..." -ForegroundColor Green
Set-Location $BASE_DIR
if (-not (Test-Path $REPO_NAME)) {
    git clone $REPO_URL $REPO_NAME
}
Set-Location $REPO_NAME

# 复制文件
Write-Host ""
Write-Host "步骤 2: 复制文件..." -ForegroundColor Green
if ($SOURCE_DIR -eq "query_tool") {
    # 从主仓库的 query_tool 目录复制
    $sourcePath = Join-Path $BASE_DIR "ems-analysis\$SOURCE_DIR"
    Copy-Item -Path "$sourcePath\*" -Destination . -Recurse -Force -Exclude ".git"
    Get-ChildItem -Path $sourcePath -Force -Filter ".*" | ForEach-Object {
        if ($_.Name -ne "." -and $_.Name -ne ".." -and $_.Name -ne ".git") {
            Copy-Item -Path $_.FullName -Destination . -Force -ErrorAction SilentlyContinue
        }
    }
} else {
    # 已经在 query_tool 目录中
    Copy-Item -Path "$SOURCE_DIR\*" -Destination . -Recurse -Force -Exclude ".git"
}

# 复制 GitHub Actions
$githubSource = Join-Path $BASE_DIR "ems-analysis\$SOURCE_DIR\.github"
if (Test-Path $githubSource) {
    New-Item -ItemType Directory -Force -Path ".github\workflows" | Out-Null
    $workflowFile = Join-Path $githubSource "workflows\build-windows.yml"
    if (Test-Path $workflowFile) {
        Copy-Item -Path $workflowFile -Destination ".github\workflows\" -Force
    }
}

# 重命名新文件
Write-Host ""
Write-Host "步骤 3: 重命名文件..." -ForegroundColor Green
if (Test-Path "README_NEW.md") {
    if (Test-Path "README.md") {
        Move-Item -Path "README.md" -Destination "README.md.old" -Force
    }
    Move-Item -Path "README_NEW.md" -Destination "README.md" -Force
    Write-Host "  ✓ README_NEW.md -> README.md" -ForegroundColor Green
}

if (Test-Path "USER_GUIDE_NEW.md") {
    if (Test-Path "USER_GUIDE.md") {
        Move-Item -Path "USER_GUIDE.md" -Destination "USER_GUIDE.md.old" -Force
    }
    Move-Item -Path "USER_GUIDE_NEW.md" -Destination "USER_GUIDE.md" -Force
    Write-Host "  ✓ USER_GUIDE_NEW.md -> USER_GUIDE.md" -ForegroundColor Green
}

if (Test-Path ".gitignore_ROOT") {
    if (-not (Test-Path ".gitignore")) {
        Copy-Item -Path ".gitignore_ROOT" -Destination ".gitignore" -Force
        Write-Host "  ✓ .gitignore_ROOT -> .gitignore" -ForegroundColor Green
    }
}

# 清理不需要的文件
Write-Host ""
Write-Host "步骤 4: 清理文件..." -ForegroundColor Green
Remove-Item -Path "README.md.old", "USER_GUIDE.md.old", ".gitignore_ROOT" -ErrorAction SilentlyContinue
Remove-Item -Path "build", "dist", "__pycache__" -Recurse -Force -ErrorAction SilentlyContinue
Get-ChildItem -Filter "*.pyc" -Recurse | Remove-Item -Force -ErrorAction SilentlyContinue

# 显示文件列表
Write-Host ""
Write-Host "步骤 5: 文件清单..." -ForegroundColor Green
Write-Host "已复制的文件:" -ForegroundColor Yellow
Get-ChildItem -File | Select-Object -First 20 | ForEach-Object {
    Write-Host "  - $($_.Name)" -ForegroundColor Gray
}

# 检查必要文件
Write-Host ""
Write-Host "步骤 6: 检查必要文件..." -ForegroundColor Green
$missingFiles = 0

function Check-File {
    param($fileName)
    if (-not (Test-Path $fileName)) {
        Write-Host "  ✗ 缺少: $fileName" -ForegroundColor Red
        $script:missingFiles++
    } else {
        Write-Host "  ✓ $fileName" -ForegroundColor Green
    }
}

Check-File "README.md"
Check-File "USER_GUIDE.md"
Check-File "requirements.txt"
Check-File "csv_export_config.toml"
Check-File ".gitignore"
Check-File ".github\workflows\build-windows.yml"
Check-File "query_tool\__init__.py"
Check-File "query_tool\cli.py"

if ($missingFiles -gt 0) {
    Write-Host ""
    Write-Host "警告: 缺少 $missingFiles 个必要文件" -ForegroundColor Yellow
} else {
    Write-Host ""
    Write-Host "✓ 所有必要文件都已就绪" -ForegroundColor Green
}

# 提示提交
$commitMsg = "初始提交：从 ems-analysis 迁移 query_tool"

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "迁移准备完成！" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "下一步操作:" -ForegroundColor Yellow
Write-Host "1. 检查文件是否正确: cd $REPO_NAME; ls" -ForegroundColor White
Write-Host "2. 添加文件到 Git: git add ." -ForegroundColor White
Write-Host "3. 提交: git commit -m `"$commitMsg`"" -ForegroundColor White
Write-Host "4. 推送: git push origin main" -ForegroundColor White
Write-Host ""
Write-Host "或者运行以下命令:" -ForegroundColor Yellow
Write-Host "  cd $REPO_NAME" -ForegroundColor White
Write-Host "  git add ." -ForegroundColor White
Write-Host "  git status  # 检查要提交的文件" -ForegroundColor White
Write-Host "  git commit -m `"$commitMsg`"" -ForegroundColor White
Write-Host "  git push origin main" -ForegroundColor White
Write-Host ""
