# Remote Tool

通过SSH连接远程服务器，进行数据查询和应用部署的工具。

## 功能特性

- ✅ **SSH连接**：支持密码和密钥文件认证，连接状态在数据查询和应用部署模块间共享
- ✅ **数据查询**：支持按时间范围、设备序列号查询SQLite数据库
  - 设备数据查询
  - 指令数据查询
  - 宽表查询
- ✅ **CSV导出**：将查询结果导出为CSV格式（Excel兼容）
- ✅ **应用部署**：通过SSH部署应用程序到远程服务器
  - 文件上传（可执行文件、配置文件、拓扑文件）
  - 服务管理（systemd服务创建、启动、停止）
  - 部署状态检查
- ✅ **现代化UI**：基于Vue 3 + TypeScript的现代化界面
- ✅ **自动更新**：支持从GitHub Releases自动更新
- ✅ **跨平台**：支持Windows、Linux、macOS

## 技术栈

- **后端**: Rust + Tauri
- **前端**: Vue 3 + TypeScript + Vite
- **状态管理**: Pinia
- **自动更新**: Tauri Updater

## 快速开始

### 下载安装

1. 从 [Releases](https://github.com/huluzhou/remote-tool/releases) 下载最新版本的安装包
2. 根据您的操作系统选择对应的安装包：
   - Windows: `remote-tool_*_x64-setup.exe`
   - Linux: `remote-tool_*_amd64.deb` 或 `remote-tool_*_amd64.AppImage`
   - macOS: `remote-tool_*_x64.dmg` 或 `remote-tool_*_aarch64.dmg`
3. 安装并运行

### 开发环境运行

#### 前置要求

- Node.js 20+
- Rust 1.70+
- 系统依赖（Linux）:
  ```bash
  # Ubuntu/Debian
  sudo apt-get update
  sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libglib2.0-dev libgdk-pixbuf-2.0-dev libpango1.0-dev libgtk-3-dev libgirepository1.0-dev
  ```

#### 安装和运行

```bash
# 克隆仓库
git clone https://github.com/huluzhou/remote-tool.git
cd remote-tool

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 使用说明

### 数据查询

1. **连接SSH服务器**
   - 在"SSH连接配置"区域输入SSH连接指令（格式：`ssh user@host -p port`）
   - 输入密码
   - 点击"连接"按钮

2. **配置查询参数**
   - 选择查询类型（设备数据/指令数据/宽表）
   - 设置数据库路径
   - 设置时间范围（可使用快捷按钮：今天/昨天/最近7天）
   - 如需要，输入设备序列号
   - 选择是否包含扩展表数据

3. **执行查询**
   - 点击"执行查询"按钮
   - 查看查询结果（支持分页浏览）

4. **导出数据**
   - 点击"导出为CSV"按钮
   - 选择保存位置
   - 数据将导出为CSV格式

### 应用部署

1. **连接SSH服务器**（与数据查询共用连接）

2. **配置部署选项**
   - 选择可执行文件路径
   - 选择是否上传配置文件（config.toml）
   - 选择是否上传拓扑文件（topo.json）
   - 选择运行用户（普通用户/root用户）
   - 选择是否部署后启动服务

3. **执行部署**
   - 点击"开始部署"按钮
   - 查看部署日志
   - 部署完成后检查服务状态

4. **检查部署状态**
   - 点击"检查状态"按钮
   - 查看服务安装、运行状态

## 项目结构

```
remote-tool/
├── src-tauri/              # Tauri 后端（Rust）
│   ├── src/
│   │   ├── main.rs         # Tauri 应用入口
│   │   ├── commands.rs     # Tauri 命令定义
│   │   ├── ssh/            # SSH 客户端模块
│   │   ├── query/          # 数据库查询模块
│   │   ├── export/         # CSV 导出模块
│   │   └── deploy/         # 部署模块
│   ├── Cargo.toml          # Rust 依赖
│   └── tauri.conf.json     # Tauri 配置（含自动更新）
├── src/                     # Vue 3 前端
│   ├── components/         # Vue 组件
│   │   ├── SshConnection.vue    # SSH 连接组件（公共）
│   │   ├── DataQuery/           # 数据查询模块
│   │   └── AppDeploy/           # 应用部署模块
│   ├── views/              # 页面视图
│   │   ├── QueryView.vue   # 数据查询页面
│   │   └── DeployView.vue  # 应用部署页面
│   ├── stores/             # Pinia 状态管理
│   ├── App.vue
│   └── main.ts
├── .github/workflows/      # GitHub Actions
│   └── build-release.yml   # 构建和发布工作流
└── README.md
```

## 开发说明

### 本地开发

```bash
# 安装依赖
npm install

# 开发模式（热重载）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

### 构建发布

项目使用 GitHub Actions 自动构建和发布。当创建新的 Git 标签（格式：`v*`）时，会自动：

1. 构建 Windows、Linux、macOS 多平台版本
2. 创建 GitHub Release
3. 上传构建产物
4. 配置自动更新

手动构建：

```bash
# 构建当前平台
npm run tauri build

# 构建指定平台（需要交叉编译工具链）
npm run tauri build -- --target x86_64-pc-windows-msvc
npm run tauri build -- --target x86_64-unknown-linux-gnu
npm run tauri build -- --target x86_64-apple-darwin
```

## 自动更新

应用支持从 GitHub Releases 自动更新：

1. 配置 `src-tauri/tauri.conf.json` 中的 `updater` 设置
2. 设置 GitHub Secrets：
   - `TAURI_SIGNING_PRIVATE_KEY`: 签名私钥
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 私钥密码
3. 应用启动时会自动检查更新
4. 发现新版本时会提示用户下载和安装

## 故障排查

### SSH连接失败
- 检查服务器地址、端口、用户名、密码是否正确
- 检查防火墙设置
- 确认SSH服务正在运行

### 数据库查询失败
- 检查数据库路径是否正确
- 确认数据库文件存在且有读取权限
- 确认远程服务器已安装Python 3

### 应用部署失败
- 检查可执行文件路径是否正确
- 确认SSH用户有sudo权限
- 检查远程服务器系统服务配置

### 自动更新失败
- 检查网络连接
- 确认GitHub Releases中有新版本
- 检查签名密钥配置

## 许可证

本项目遵循与主项目相同的许可证。

## 贡献

欢迎提交 Issue 和 Pull Request！
