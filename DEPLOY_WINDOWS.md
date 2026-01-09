# Windows 部署指南

## 构建状态

✅ **构建成功！** 可执行文件已包含前端资源，可以直接使用。

## 文件位置

```
src-tauri/target/x86_64-pc-windows-gnu/release/remote-tool.exe
```

文件大小：约 36MB（已包含前端资源）

## 部署到 Windows

### 方式 1：直接使用（推荐）

1. **复制文件到 Windows：**
   ```bash
   # 在 Linux 上
   cp src-tauri/target/x86_64-pc-windows-gnu/release/remote-tool.exe /path/to/windows/share/
   ```

2. **在 Windows 上：**
   - 将 `remote-tool.exe` 复制到任意目录
   - 双击运行即可
   - **不需要其他文件**（前端资源已打包在 exe 中）

### 方式 2：如果需要安装包

如果需要生成安装包（MSI/NSIS），需要安装 NSIS：

```bash
# Ubuntu/Debian
sudo apt-get install nsis

# 然后重新构建
npm run tauri build -- --target x86_64-pc-windows-gnu
```

安装包会生成在：
```
src-tauri/target/x86_64-pc-windows-gnu/release/bundle/
```

## 验证

### 检查文件是否包含前端资源

```bash
# 文件大小应该 > 30MB
ls -lh src-tauri/target/x86_64-pc-windows-gnu/release/remote-tool.exe
```

如果文件大小正确（约 36MB），说明前端资源已打包。

### 在 Windows 上测试

1. 复制 `remote-tool.exe` 到 Windows
2. 双击运行
3. 应该正常显示界面，不会出现 "localhost 拒绝连接" 错误

## 常见问题

### Q: 为什么不需要安装包？

A: 可执行文件已经包含了所有必要的资源（前端、依赖等），可以直接运行。安装包只是提供了安装/卸载功能。

### Q: 如何生成安装包？

A: 安装 NSIS 后，修改 `tauri.conf.json`：
```json
"bundle": {
  "active": true,
  "targets": "all"
}
```

然后重新构建。

### Q: 文件太大怎么办？

A: 36MB 是正常的，包含了：
- Rust 运行时
- Tauri 框架
- 前端资源（Vue 应用）
- 所有依赖库

### Q: 可以压缩吗？

A: 可以使用 UPX 压缩（可选）：
```bash
upx --best remote-tool.exe
```

但可能影响启动速度和杀毒软件检测。

## 当前配置

为了跳过安装包生成（避免需要 NSIS），已设置：
```json
"bundle": {
  "active": false
}
```

如果需要安装包，可以：
1. 安装 NSIS：`sudo apt-get install nsis`
2. 修改配置：`"active": true, "targets": "all"`
3. 重新构建
