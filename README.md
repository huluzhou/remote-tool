# Query Tool

通过SSH连接远程服务器，查询SQLite数据库并导出为CSV的工具。

## 功能特性

- ✅ **SSH连接**：支持密码和密钥文件认证
- ✅ **SQL查询**：支持按时间范围、设备序列号查询
- ✅ **CSV导出**：将查询结果导出为CSV格式（Excel兼容）
- ✅ **图形界面**：提供Tkinter GUI界面
- ✅ **命令行接口**：提供CLI工具，方便脚本化使用
- ✅ **跨平台**：支持Windows和Linux

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
# 基本用法
python -m query_tool.cli -H 192.168.1.100 -u root -p password -s today -e now -o result.csv

# 查询指定设备
python -m query_tool.cli -H 192.168.1.100 -u root -p password \
    -s "2024-01-01" -e "2024-01-31" --device-sn METER001 -o meter_data.csv
```

支持的时间格式：
- 时间戳：`1704067200`
- 日期：`2024-01-01`
- 日期时间：`2024-01-01 12:00:00`
- 关键字：`now`, `today`, `yesterday`, `7days`

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

## 配置

CSV导出字段可通过 `csv_export_config.toml` 配置文件自定义。详细配置说明请参考 [用户手册](USER_GUIDE.md)。

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
