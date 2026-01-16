# 生成 ICO 和 ICNS 文件（用于发布）

## 当前状态

✅ PNG 图标已生成：
- `32x32.png`
- `128x128.png`
- `128x128@2x.png` (256x256)
- `icon_512.png` (用于生成 ICO/ICNS)

⚠️ 需要生成（用于打包发布）：
- `icon.ico` (Windows)
- `icon.icns` (macOS)

## 快速生成方法

### 方法 1: 在线工具（推荐，最简单）

#### 生成 ICO (Windows)
1. 访问：https://convertio.co/png-ico/
2. 上传：`icon_512.png` 或 `128x128@2x.png`
3. 下载为：`icon.ico`
4. 保存到：`src-tauri/icons/`

#### 生成 ICNS (macOS)
1. 访问：https://cloudconvert.com/png-to-icns
2. 上传：`icon_512.png`
3. 下载为：`icon.icns`
4. 保存到：`src-tauri/icons/`

### 方法 2: 安装 ImageMagick（本地生成 ICO）

```bash
# 安装 ImageMagick
sudo apt-get install imagemagick

# 生成多个尺寸的 PNG（用于 ICO）
cd src-tauri/icons
rsvg-convert -w 16 -h 16 icon.svg -o icon_16.png
rsvg-convert -w 32 -h 32 icon.svg -o icon_32.png
rsvg-convert -w 48 -h 48 icon.svg -o icon_48.png
rsvg-convert -w 64 -h 64 icon.svg -o icon_64.png
rsvg-convert -w 128 -h 128 icon.svg -o icon_128.png
rsvg-convert -w 256 -h 256 icon.svg -o icon_256.png

# 转换为 ICO
convert icon_16.png icon_32.png icon_48.png icon_64.png icon_128.png icon_256.png icon.ico

# 清理临时文件
rm icon_*.png
```

### 方法 3: macOS 上生成 ICNS

如果你有 macOS 系统：

```bash
# 创建 iconset 目录
mkdir -p icon.iconset

# 生成不同尺寸
rsvg-convert -w 16 -h 16 icon.svg -o icon.iconset/icon_16x16.png
rsvg-convert -w 32 -h 32 icon.svg -o icon.iconset/icon_16x16@2x.png
rsvg-convert -w 32 -h 32 icon.svg -o icon.iconset/icon_32x32.png
rsvg-convert -w 64 -h 64 icon.svg -o icon.iconset/icon_32x32@2x.png
rsvg-convert -w 128 -h 128 icon.svg -o icon.iconset/icon_128x128.png
rsvg-convert -w 256 -h 256 icon.svg -o icon.iconset/icon_128x128@2x.png
rsvg-convert -w 256 -h 256 icon.svg -o icon.iconset/icon_256x256.png
rsvg-convert -w 512 -h 512 icon.svg -o icon.iconset/icon_256x256@2x.png
rsvg-convert -w 512 -h 512 icon.svg -o icon.iconset/icon_512x512.png
rsvg-convert -w 1024 -h 1024 icon.svg -o icon.iconset/icon_512x512@2x.png

# 生成 ICNS
iconutil -c icns icon.iconset

# 移动并清理
mv icon.icns ../icon.icns
rm -rf icon.iconset
```

## 注意

- **开发环境**：当前配置只使用 PNG 文件，可以正常工作
- **发布构建**：在构建发布版本前，建议生成 ICO 和 ICNS 文件
- **Tauri 2.0**：会自动处理图标，即使缺少某些格式也能工作

## 验证

生成后，检查文件：

```bash
cd src-tauri/icons
ls -lh icon.ico icon.icns
file icon.ico icon.icns
```
