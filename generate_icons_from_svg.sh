#!/bin/bash
# 从 SVG 生成各种格式的图标
# 需要安装: inkscape 或 rsvg-convert

ICONS_DIR="src-tauri/icons"
SVG_FILE="$ICONS_DIR/icon.svg"

mkdir -p "$ICONS_DIR"

echo "正在生成图标..."
echo "=================================================="

# 检查是否有 inkscape
if command -v inkscape &> /dev/null; then
    echo "使用 Inkscape 生成图标..."
    
    # 生成 PNG 图标
    inkscape -w 32 -h 32 "$SVG_FILE" -o "$ICONS_DIR/32x32.png" 2>/dev/null
    inkscape -w 128 -h 128 "$SVG_FILE" -o "$ICONS_DIR/128x128.png" 2>/dev/null
    inkscape -w 256 -h 256 "$SVG_FILE" -o "$ICONS_DIR/128x128@2x.png" 2>/dev/null
    
    echo "✓ 已生成 PNG 图标"
    
    # 生成 ICO (需要多个尺寸)
    inkscape -w 16 -h 16 "$SVG_FILE" -o "$ICONS_DIR/icon_16.png" 2>/dev/null
    inkscape -w 32 -h 32 "$SVG_FILE" -o "$ICONS_DIR/icon_32.png" 2>/dev/null
    inkscape -w 48 -h 48 "$SVG_FILE" -o "$ICONS_DIR/icon_48.png" 2>/dev/null
    inkscape -w 64 -h 64 "$SVG_FILE" -o "$ICONS_DIR/icon_64.png" 2>/dev/null
    inkscape -w 128 -h 128 "$SVG_FILE" -o "$ICONS_DIR/icon_128.png" 2>/dev/null
    inkscape -w 256 -h 256 "$SVG_FILE" -o "$ICONS_DIR/icon_256.png" 2>/dev/null
    
    # 使用 ImageMagick 转换为 ICO (如果可用)
    if command -v convert &> /dev/null; then
        convert "$ICONS_DIR/icon_16.png" "$ICONS_DIR/icon_32.png" "$ICONS_DIR/icon_48.png" \
                "$ICONS_DIR/icon_64.png" "$ICONS_DIR/icon_128.png" "$ICONS_DIR/icon_256.png" \
                "$ICONS_DIR/icon.ico" 2>/dev/null
        echo "✓ 已生成 ICO 图标"
        rm "$ICONS_DIR/icon_"*.png
    else
        echo "⚠ ImageMagick 未安装，无法生成 ICO 文件"
        echo "  可以手动使用在线工具将 PNG 转换为 ICO"
    fi
    
# 检查是否有 rsvg-convert
elif command -v rsvg-convert &> /dev/null; then
    echo "使用 rsvg-convert 生成图标..."
    
    rsvg-convert -w 32 -h 32 "$SVG_FILE" -o "$ICONS_DIR/32x32.png"
    rsvg-convert -w 128 -h 128 "$SVG_FILE" -o "$ICONS_DIR/128x128.png"
    rsvg-convert -w 256 -h 256 "$SVG_FILE" -o "$ICONS_DIR/128x128@2x.png"
    
    echo "✓ 已生成 PNG 图标"
    
# 如果没有工具，提供安装说明
else
    echo "⚠ 未找到图标转换工具 (inkscape 或 rsvg-convert)"
    echo ""
    echo "请安装以下工具之一："
    echo "  Ubuntu/Debian: sudo apt-get install inkscape"
    echo "  或: sudo apt-get install librsvg2-bin"
    echo ""
    echo "或者使用在线工具将 SVG 转换为 PNG/ICO:"
    echo "  - https://cloudconvert.com/svg-to-png"
    echo "  - https://convertio.co/svg-png/"
    echo ""
    echo "SVG 文件已创建: $SVG_FILE"
    exit 1
fi

echo "=================================================="
echo "✓ 图标生成完成！"
echo ""
echo "注意: macOS ICNS 文件需要额外步骤："
echo "  1. 创建 icon.iconset 目录"
echo "  2. 将不同尺寸的 PNG 放入该目录"
echo "  3. 运行: iconutil -c icns icon.iconset"
