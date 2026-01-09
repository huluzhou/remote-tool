# 图标生成快速指南

## 🎨 图标设计

我已经为你设计了一个专业的图标，体现远程工具的核心功能：
- **服务器图标**：表示远程服务器
- **连接线和数据点**：表示 SSH 连接和数据传输
- **网络符号**：表示远程连接
- **蓝色渐变背景**：现代、专业的外观

## ⚡ 最快方法：使用在线工具

### 步骤 1: 打开 SVG 文件
SVG 源文件位于：`src-tauri/icons/icon.svg`

### 步骤 2: 转换为 PNG
访问以下任一在线工具：
- https://cloudconvert.com/svg-to-png
- https://convertio.co/svg-png/
- https://svgtopng.com/

生成以下尺寸：
1. **32x32.png** - 上传 SVG，选择 32x32 尺寸
2. **128x128.png** - 上传 SVG，选择 128x128 尺寸  
3. **256x256.png** - 上传 SVG，选择 256x256 尺寸（保存为 `128x128@2x.png`）

### 步骤 3: 生成 ICO 文件（Windows）
- 访问：https://convertio.co/png-ico/
- 上传 256x256 的 PNG 文件
- 下载为 `icon.ico`

### 步骤 4: 生成 ICNS 文件（macOS）
- 访问：https://cloudconvert.com/png-to-icns
- 上传 512x512 的 PNG（如果需要，先用 SVG 生成）
- 下载为 `icon.icns`

### 步骤 5: 放置文件
将所有生成的文件放到 `src-tauri/icons/` 目录：
```
src-tauri/icons/
├── 32x32.png
├── 128x128.png
├── 128x128@2x.png
├── icon.ico
├── icon.icns
└── icon.svg (已存在)
```

## 🛠️ 本地生成（如果已安装工具）

### 使用 Inkscape
```bash
sudo apt-get install inkscape
./generate_icons_from_svg.sh
```

### 使用 rsvg-convert
```bash
sudo apt-get install librsvg2-bin
./generate_icons_from_svg.sh
```

## 📝 文件清单

确保以下文件存在于 `src-tauri/icons/` 目录：

- ✅ `32x32.png`
- ✅ `128x128.png`
- ✅ `128x128@2x.png` (256x256)
- ✅ `icon.ico` (Windows)
- ✅ `icon.icns` (macOS)
- ✅ `icon.svg` (源文件，已创建)

## 🎯 图标预览

图标特点：
- **尺寸**: 512x512 (SVG 可缩放)
- **背景**: 蓝色到青色渐变圆形
- **主元素**: 白色服务器机架，带绿色指示灯
- **连接**: 绿色连接线和金色数据点
- **风格**: 现代、扁平化设计

## 💡 提示

如果暂时无法生成所有图标文件，Tauri 会使用默认图标。你可以：
1. 先生成 PNG 文件（最简单）
2. ICO 和 ICNS 可以稍后添加
3. 或者暂时移除 `tauri.conf.json` 中的图标配置
