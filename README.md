# Query Tool

通过SSH连接远程服务器，查询SQLite数据库并导出为CSV的工具。

## 功能特性

- ✅ **SSH连接**：支持密码和密钥文件认证
- ✅ **SQL查询**：支持按时间范围、设备序列号查询
- ✅ **CSV导出**：将查询结果导出为CSV格式（Excel兼容）
- ✅ **图形界面**：提供Tkinter GUI界面，操作简单直观
- ✅ **命令行接口**：提供CLI工具，方便脚本化使用和自动化
- ✅ **跨平台**：支持Windows和Linux
- ✅ **模块化设计**：可作为Python库集成到其他项目

## 快速开始

### Windows 用户

1. 从 [Releases](https://github.com/huluzhou/query-tool/releases) 下载最新版本的压缩包
2. 解压到任意目录
3. 双击 `query_tool.exe` 运行

### Linux 用户

#### 安装依赖

```bash
# Ubuntu/Debian
sudo apt-get install python3-tk python3-venv

# CentOS/RHEL
sudo yum install python3-tkinter python3-venv
```

#### 安装和运行

```bash
# 克隆仓库
git clone https://github.com/huluzhou/query-tool.git
cd query-tool

# 创建虚拟环境
python3 -m venv venv
source venv/bin/activate

# 安装依赖
pip install -r requirements.txt

# 运行GUI
python -m query_tool.ui.tkinter_ui

# 或运行CLI
python -m query_tool.cli -H 192.168.1.100 -u root -p password -s today -e now
```

## 使用说明

### 图形界面

1. 启动程序：双击 `query_tool.exe`（Windows）或运行 `python -m query_tool.ui.tkinter_ui`（Linux）
2. 配置SSH连接：输入服务器地址、端口、用户名、密码
3. 配置查询：设置数据库路径、时间范围、设备序列号（可选）
4. 执行查询：点击"执行查询"按钮
5. 导出数据：点击"导出为CSV"按钮

详细使用说明请参考 [用户手册](USER_GUIDE.md)。

### 命令行

```bash
# 基本用法 - 查询今天的数据
python -m query_tool.cli -H 192.168.1.100 -u root -p password -s today -e now -o result.csv

# 查询指定设备最近7天的数据
python -m query_tool.cli -H 192.168.1.100 -u root -p password \
    -s 7days -e now --device-sn METER001 -o meter_data.csv

# 查询指定时间范围的数据
python -m query_tool.cli -H 192.168.1.100 -u root -p password \
    -s "2024-01-01" -e "2024-01-31" --device-sn METER001 -o meter_data.csv

# 使用SSH密钥文件认证
python -m query_tool.cli -H 192.168.1.100 -u root -k ~/.ssh/id_rsa \
    -s today -e now -o result.csv

# 指定数据库路径
python -m query_tool.cli -H 192.168.1.100 -u root -p password \
    -d /opt/analysis/data/device_data.db -s today -e now -o result.csv
```

**支持的时间格式：**
- 时间戳：`1704067200`
- 日期：`2024-01-01`
- 日期时间：`2024-01-01 12:00:00`
- 关键字：`now`, `today`, `yesterday`, `7days`（表示最近7天）

**CLI参数说明：**
- `-H, --host`：SSH服务器地址（必需）
- `-P, --port`：SSH端口（默认：22）
- `-u, --username`：SSH用户名（默认：root）
- `-p, --password`：SSH密码
- `-k, --key-file`：SSH私钥文件路径
- `-d, --db-path`：远程数据库文件路径（默认：/opt/analysis/data/device_data.db）
- `-s, --start-time`：开始时间（必需）
- `-e, --end-time`：结束时间（必需）
- `--device-sn`：设备序列号（可选）
- `--include-ext`：包含扩展表数据（默认：True）
- `--no-ext`：不包含扩展表数据
- `-o, --output`：输出CSV文件路径
- `--format`：输出格式（csv/json，默认：csv）
- `-v, --verbose`：详细输出

## 作为Python库使用

```python
from query_tool import SSHClient, DBQuery, CSVExporter

with SSHClient(host="192.168.1.100", username="root", password="password") as ssh:
    db_query = DBQuery(ssh)
    results = db_query.query_by_time_range(
        db_path="/opt/analysis/data/device_data.db",
        start_time=1704067200,
        end_time=1704153600,
        device_sn="METER001",
        include_ext=True
    )
    formatted_results = CSVExporter.prepare_for_export(results)
    CSVExporter.export_to_csv(formatted_results, "output.csv")
```

## 项目结构

```
query-tool/
├── core/                  # 核心功能模块
│   ├── ssh_client.py      # SSH连接管理
│   ├── db_query.py        # 数据库查询
│   ├── csv_export.py      # CSV导出
│   └── deploy.py          # 部署相关
├── ui/                    # 用户界面
│   └── tkinter_ui.py      # Tkinter GUI界面
├── cli.py                 # 命令行接口
├── run_ui.py             # GUI启动脚本
├── requirements.txt       # Python依赖
├── csv_export_config.toml # CSV导出配置
├── build_windows.spec     # Windows打包配置
├── build_windows_cli.spec # Windows CLI打包配置
├── README.md             # 项目说明
└── USER_GUIDE.md         # 用户手册
```

## 配置

CSV导出字段可通过 `csv_export_config.toml` 配置文件自定义。详细配置说明请参考 [用户手册](USER_GUIDE.md)。

## 开发说明

### 本地开发

```bash
# 克隆仓库
git clone https://github.com/huluzhou/query-tool.git
cd query-tool

# 创建虚拟环境
python3 -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# 安装依赖
pip install -r requirements.txt

# 运行GUI
python run_ui.py
# 或
python -m query_tool.ui.tkinter_ui

# 运行CLI
python -m query_tool.cli -H 192.168.1.100 -u root -p password -s today -e now
```

### 构建Windows可执行文件

项目使用PyInstaller打包，通过GitHub Actions自动构建。手动构建方法：

```bash
# 安装PyInstaller
pip install pyinstaller

# 构建GUI版本
python -m PyInstaller build_windows.spec --clean --noconfirm

# 构建CLI版本
python -m PyInstaller build_windows_cli.spec --clean --noconfirm
```

构建产物位于 `dist/` 目录。

## 故障排查

### SSH连接失败
- 检查服务器地址、端口、用户名、密码是否正确
- 检查防火墙设置

### 数据库查询失败
- 检查数据库路径是否正确
- 确认数据库文件存在且有读取权限

### CSV导出失败
- 检查输出目录是否有写入权限
- 确认磁盘空间充足

更多问题请参考 [用户手册](USER_GUIDE.md) 的"常见问题"部分。

## 许可证

本项目遵循与主项目相同的许可证。
