# 开发指南

## 环境要求

- Node.js 20+
- Rust 1.70+
- 系统依赖（根据平台）：

### Windows
- Microsoft Visual C++ Build Tools
- WebView2 (通常已预装)

### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libglib2.0-dev libgdk-pixbuf-2.0-dev libpango1.0-dev libgtk-3-dev libgirepository1.0-dev
```

### macOS
- Xcode Command Line Tools

## 项目设置

### 1. 克隆仓库
```bash
git clone https://github.com/huluzhou/remote-tool.git
cd remote-tool
```

### 2. 安装依赖
```bash
# 安装前端依赖
npm install

# Rust 依赖会在首次构建时自动安装
```

### 3. 开发模式运行
```bash
npm run tauri:dev
```

这将启动开发服务器，支持热重载。

## 项目结构

### 前端 (Vue 3 + TypeScript)
- `src/` - Vue 前端代码
  - `components/` - 可复用组件
  - `views/` - 页面视图
  - `stores/` - Pinia 状态管理
  - `App.vue` - 根组件
  - `main.ts` - 入口文件

### 后端 (Rust + Tauri)
- `src-tauri/src/` - Rust 后端代码
  - `main.rs` - Tauri 应用入口
  - `commands.rs` - Tauri 命令定义
  - `ssh/` - SSH 客户端模块
  - `query/` - 数据库查询模块
  - `export/` - CSV 导出模块
  - `deploy/` - 部署模块

## 开发工作流

### 添加新的 Tauri 命令

1. 在 `src-tauri/src/commands.rs` 中定义命令：
```rust
#[tauri::command]
pub async fn my_command(param: String) -> Result<String, String> {
    // 实现逻辑
    Ok("result".to_string())
}
```

2. 在 `src-tauri/src/main.rs` 中注册命令：
```rust
.invoke_handler(tauri::generate_handler![
    // ... 其他命令
    commands::my_command,
])
```

3. 在前端调用：
```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke<string>("my_command", { param: "value" });
```

### 添加新的 Vue 组件

1. 在 `src/components/` 中创建组件文件
2. 在需要的地方导入并使用

### 状态管理

使用 Pinia stores 管理应用状态：
- `stores/ssh.ts` - SSH 连接状态
- `stores/query.ts` - 查询状态
- `stores/deploy.ts` - 部署状态

## 构建

### 开发构建
```bash
npm run tauri:dev
```

### 生产构建
```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/` 目录。

### 构建特定平台
```bash
# Windows
npm run tauri:build -- --target x86_64-pc-windows-msvc

# Linux
npm run tauri:build -- --target x86_64-unknown-linux-gnu

# macOS (Intel)
npm run tauri:build -- --target x86_64-apple-darwin

# macOS (Apple Silicon)
npm run tauri:build -- --target aarch64-apple-darwin
```

## 测试

### 前端测试
```bash
npm run test  # 如果配置了测试框架
```

### Rust 测试
```bash
cd src-tauri
cargo test
```

## 调试

### 前端调试
- 使用浏览器开发者工具（开发模式下）
- Vue DevTools 扩展

### Rust 调试
- 使用 `println!` 宏输出日志
- 使用 Rust 调试器（如 VS Code 的 rust-analyzer）

## 发布

### 自动发布（推荐）

1. 创建 Git 标签：
```bash
git tag v0.1.0
git push origin v0.1.0
```

2. GitHub Actions 会自动：
   - 构建所有平台版本
   - 创建 GitHub Release
   - 上传构建产物
   - 配置自动更新

### 手动发布

1. 构建所有平台版本
2. 创建 GitHub Release
3. 上传构建产物
4. 更新 `src-tauri/tauri.conf.json` 中的版本号

## 自动更新配置

1. 生成签名密钥对：

   由于 Tauri CLI 是作为项目依赖安装的，需要使用以下方式之一运行命令：

   **方法 1：使用 npx（推荐）**
   ```bash
   npx tauri signer generate -w ~/.tauri/myapp.key
   ```

   **方法 2：全局安装 Tauri CLI（可选）**
   ```bash
   # 全局安装
   npm install -g @tauri-apps/cli
   
   # 然后可以直接使用
   tauri signer generate -w ~/.tauri/myapp.key
   ```

   **命令参数说明：**
   - `-w, --write-keys <PATH>`: 指定私钥保存路径
   - `-p, --password <PASSWORD>`: 设置私钥密码（可选，如果设置，需要在 GitHub Secrets 中配置）
   - `-f, --force`: 如果密钥文件已存在，强制覆盖

   **示例（带密码）：**
   ```bash
   npx tauri signer generate -w ~/.tauri/myapp.key -p "your-password"
   ```

   **注意：**
   - 如果 `~/.tauri/` 目录不存在，命令会自动创建
   - 生成密钥时如果没有使用 `-p` 参数，可能会提示输入密码，可以选择设置密码或留空
   - 密钥文件会保存在 `~/.tauri/myapp.key`
   - 同时会生成对应的公钥文件 `~/.tauri/myapp.key.pub`
   - 公钥内容需要添加到 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey` 字段

2. 将公钥添加到 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey` 字段

   生成密钥后，需要将公钥内容添加到配置文件中：

   ```bash
   # 查看公钥内容
   cat ~/.tauri/myapp.key.pub
   ```

   **重要：** Tauri signer 生成的公钥是 base64 编码格式，**不需要**添加 `-----BEGIN PUBLIC KEY-----` 和 `-----END PUBLIC KEY-----` 标记。直接复制整个 base64 字符串即可。

   更新 `src-tauri/tauri.conf.json`：

   ```json
   {
     "plugins": {
       "updater": {
         "active": true,
         "endpoints": [
           "https://github.com/huluzhou/remote-tool/releases/latest/download/{{target}}/{{current_version}}"
         ],
         "dialog": true,
         "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEU3NEFERTIyNUY5Nzk1MTUKUldRVmxaZGZJdDVLNTBmaGszYjArMkxLZFdBbTBDT0NocVJzcjNGYXhTMEQ5cWowcE1IbytlZnUK"
       }
     }
   }
   ```

   **注意：**
   - 直接复制整个 base64 字符串（通常是一行很长的字符串）
   - 不需要添加任何标记或换行符
   - 确保公钥字符串在 JSON 中用双引号包裹

3. 将私钥添加到 GitHub Secrets：

   **重要：选择 "Actions" 选项**
   
   由于签名密钥用于 GitHub Actions 工作流构建和发布应用，必须选择 **"Actions"** 而不是 Codespaces 或 Dependabot。

   **详细步骤：**
   
   a. 访问你的 GitHub 仓库页面
   
   b. 点击仓库顶部的 **"Settings"**（设置）标签
   
   c. 在左侧边栏中，找到并点击 **"Secrets and variables"**（密钥和变量）
   
   d. 点击 **"Actions"**（不是 Codespaces 或 Dependabot）
   
   e. 点击右上角的 **"New repository secret"**（新建仓库密钥）按钮
   
   f. 添加第一个密钥：
      - **Name（名称）**: `TAURI_SIGNING_PRIVATE_KEY`
      - **Secret（密钥内容）**: 粘贴私钥文件的完整内容（通常是 `~/.tauri/myapp.key` 文件的内容）
      - 点击 **"Add secret"**（添加密钥）
   
   g. 再次点击 **"New repository secret"**，添加第二个密钥：
      - **Name（名称）**: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
      - **Secret（密钥内容）**: 输入生成密钥时设置的密码（如果设置了密码）
      - 点击 **"Add secret"**（添加密钥）
   
   **注意事项：**
   - 私钥内容应该包含完整的文件内容，包括 `-----BEGIN PRIVATE KEY-----` 和 `-----END PRIVATE KEY-----` 标记
   - 如果生成密钥时没有设置密码，`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 可以留空或设置为空字符串
   - 密钥一旦添加后，GitHub 不会显示密钥内容，只能更新或删除
   - 确保密钥安全，不要将私钥提交到代码仓库中

## 常见问题

### 构建失败

**问题**: `error: linker 'cc' not found`
**解决**: 安装系统构建工具（Linux: `build-essential`, macOS: Xcode Command Line Tools）

**问题**: `error: failed to run custom build command for 'openssl-sys'`
**解决**: 安装 OpenSSL 开发库（Linux: `libssl-dev`, macOS: `brew install openssl`）

### 运行时错误

**问题**: SSH 连接失败
**解决**: 检查网络连接和服务器配置

**问题**: 数据库查询失败
**解决**: 确认远程服务器已安装 Python 3，数据库路径正确

## 贡献

1. Fork 仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目遵循与主项目相同的许可证。
