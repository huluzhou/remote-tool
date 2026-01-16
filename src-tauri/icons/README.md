# 图标生成说明

## 图标设计

图标设计理念：
- **服务器图标**：表示远程服务器管理
- **连接线**：表示 SSH 连接
- **数据点**：表示数据传输和查询
- **网络符号**：表示远程连接
- **颜色方案**：
  - 蓝色渐变背景：技术感和专业性
  - 绿色指示灯：表示连接状态
  - 金色数据点：表示数据传输

## 生成图标文件

### 方法 1: 使用 Inkscape（推荐）

```bash
# 安装 Inkscape
sudo apt-get install inkscape

# 运行生成脚本
cd /home/zhouzhang/remote-tool
./generate_icons_from_svg.sh
```

### 方法 2: 使用 rsvg-convert

```bash
# 安装 librsvg2-bin
sudo apt-get install librsvg2-bin

# 运行生成脚本
./generate_icons_from_svg.sh
```

### 方法 3: 使用在线工具

1. 打开 SVG 文件：`src-tauri/icons/icon.svg`
2. 使用在线工具转换：
   - https://cloudconvert.com/svg-to-png
   - https://convertio.co/svg-png/
   - https://svgtopng.com/

3. 生成以下尺寸的 PNG：
   - 32x32.png
   - 128x128.png
   - 256x256.png (保存为 128x128@2x.png)

4. 生成 ICO 文件（Windows）：
   - 使用 https://convertio.co/png-ico/ 或类似工具
   - 需要包含多个尺寸：16x16, 32x32, 48x48, 64x64, 128x128, 256x256

5. 生成 ICNS 文件（macOS）：
   - 使用 https://cloudconvert.com/png-to-icns
   - 或使用 macOS 的 `iconutil` 命令

### 方法 4: 使用 Python + Pillow

```bash
# 安装 Pillow（如果系统允许）
sudo apt-get install python3-pil

# 或使用虚拟环境
python3 -m venv venv
source venv/bin/activate
pip install Pillow

# 运行 Python 脚本
python3 generate_icons.py
```

## 文件清单

生成后，`src-tauri/icons/` 目录应包含：

- `32x32.png` - 32x32 像素图标
- `128x128.png` - 128x128 像素图标
- `128x128@2x.png` - 256x256 像素图标（高分辨率）
- `icon.ico` - Windows 图标文件
- `icon.icns` - macOS 图标文件
- `icon.svg` - 源 SVG 文件

## 快速生成（如果已安装工具）

```bash
# 一键生成所有图标
./generate_icons_from_svg.sh
```
