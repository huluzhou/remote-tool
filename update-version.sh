#!/bin/bash

VERSION=$1

if [ -z "$VERSION" ]; then
  echo "用法: ./update-version.sh <版本号>"
  echo "示例: ./update-version.sh 0.1.1"
  exit 1
fi

# 修改 tauri.conf.json
sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json

# 修改 Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml

# 修改 package.json
sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json

echo "版本号已更新为: $VERSION"
echo ""
echo "请检查以下文件："
echo "  - src-tauri/tauri.conf.json"
echo "  - src-tauri/Cargo.toml"
echo "  - package.json"