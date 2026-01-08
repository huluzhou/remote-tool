# Windows 64位打包说明

本文档说明如何将 query_tool 打包为 Windows 64位可执行文件。

## 前置要求

1. **Python 3.8+** (64位版本)
2. **Windows 10/11** (64位)
3. **网络连接** (用于下载依赖)

## 打包步骤

### 方法一：使用批处理脚本（推荐）

1. 打开命令提示符（CMD）或 PowerShell
2. 进入 `query_tool` 目录
3. 运行打包脚本：

```batch
build_windows.bat
```

脚本会自动：
- 检查 Python 环境
- 安装依赖（包括 PyInstaller）
- 清理旧的构建文件
- 执行打包
- 生成可执行文件到 `dist` 目录

### 方法二：手动打包

1. **安装依赖**：

```batch
pip install -r requirements.txt
pip install pyinstaller
```

2. **执行打包**：

```batch
pyinstaller build_windows.spec
```

3. **打包命令行版本**（可选）：

```batch
pyinstaller build_windows_cli.spec
```

## 打包结果

打包完成后，可执行文件位于：

- **GUI版本**: `dist\query_tool.exe`
- **CLI版本**: `dist\query_tool_cli.exe`

## 文件说明

- `build_windows.spec` - GUI版本打包配置
- `build_windows_cli.spec` - CLI版本打包配置
- `build_windows.bat` - 自动化打包脚本

## 使用打包后的程序

### GUI版本

直接双击 `query_tool.exe` 运行图形界面。

### CLI版本

在命令提示符中运行：

```batch
query_tool_cli.exe --help
```

## 注意事项

1. **首次运行**：首次运行可能需要几秒钟启动时间
2. **杀毒软件**：某些杀毒软件可能会误报，需要添加信任
3. **依赖库**：所有依赖已打包到 exe 中，无需额外安装
4. **配置文件**：`csv_export_config.toml` 已包含在打包文件中
5. **文件大小**：打包后的 exe 文件可能较大（约 50-100MB），这是正常的

## 故障排除

### 问题：打包失败

- 确保使用 64位 Python
- 确保所有依赖已正确安装
- 检查是否有杀毒软件阻止

### 问题：运行时缺少模块

- 检查 `hiddenimports` 配置
- 重新打包并确保所有依赖已安装

### 问题：无法找到配置文件

- 确保 `csv_export_config.toml` 在 `datas` 列表中
- 检查程序运行时的路径处理

## 自定义打包选项

可以编辑 `.spec` 文件来自定义打包选项：

- **图标**：在 `exe` 部分添加 `icon='path/to/icon.ico'`
- **版本信息**：添加 `version='version_info.txt'`
- **UPX压缩**：设置 `upx=True`（需要安装 UPX）

## 分发

打包完成后，可以：

1. 直接分发 `query_tool.exe` 文件
2. 或创建安装包（使用 Inno Setup、NSIS 等工具）
