# 本地 SQLite 查询方案实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将远程 Python 脚本查询改为 SFTP 下载整个 SQLite 数据库到本地，用 Rust `rusqlite` 本地查询和导出 CSV，彻底消除嵌入式设备 CPU 负载。

**Architecture:** 远程只做一次 `sqlite3 .backup` 安全拷贝 + SFTP 下载，本地用 `rusqlite` 打开 db 文件做所有查询/导出操作。引入缓存机制：同一 `db_path` 的 db 文件缓存到本地临时目录，多次查询/导出复用已下载的文件，用户可手动刷新。

**Tech Stack:** Rust (`rusqlite` + `chrono`)、Tauri 2.0 Commands、SFTP（现有 `SshClient`）

---

## 改造范围

### 当前架构
```
远程: Python 查询 SQLite → 写 CSV → （之前还有 gzip）
      ↓ SFTP
本地: 下载 CSV → 复制到目标
```

### 目标架构
```
远程: sqlite3 .backup → /tmp/export.db  （CPU 极低，纯 I/O）
      ↓ SFTP
本地: rusqlite 打开 db → 查询 → 格式化 → 写 CSV / 返回 JSON
```

### 涉及文件
- **修改:** `src-tauri/Cargo.toml` — 添加 `rusqlite` 依赖
- **重写:** `src-tauri/src/query/mod.rs` — 三个函数 + 新增缓存管理
- **修改:** `src-tauri/src/commands.rs` — 新增 `sync_database` 和 `clear_db_cache` 命令
- **修改:** `src-tauri/src/main.rs` — 注册新命令
- **修改:** `src/stores/query.ts` — 新增下载/刷新数据库操作
- **修改:** `src/components/DataQuery/QueryForm.vue`（或对应 UI）— 添加"同步数据库"按钮

---

## Task 1: 添加 `rusqlite` 依赖

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: 添加依赖**

在 `[dependencies]` 中添加：
```toml
rusqlite = { version = "0.31", features = ["bundled"] }
```

`bundled` feature 会自动编译 SQLite C 库，不依赖系统安装。

**Step 2: 编译验证**

Run: `cd src-tauri && cargo check`
Expected: 编译通过

**Step 3: Commit**
```
feat(查询): 添加 rusqlite 依赖，为本地 SQLite 查询做准备
```

---

## Task 2: 实现数据库缓存管理模块

**Files:**
- Modify: `src-tauri/src/query/mod.rs` — 在文件顶部新增缓存管理代码

**核心设计：**
- 使用 `OnceLock<Mutex<HashMap<String, CachedDb>>>` 存储缓存状态
- `CachedDb` 记录：本地文件路径、下载时间、远程 db_path
- `sync_database()` 函数：远程 backup → SFTP 下载 → 更新缓存
- `get_cached_db_path()` 函数：返回已缓存的本地 db 路径，未缓存则报错提示先同步
- `clear_db_cache()` 函数：清理缓存文件

**Step 1: 实现缓存结构和 sync_database**

```rust
use std::sync::{Mutex, OnceLock};
use rusqlite::Connection;

struct CachedDb {
    local_path: String,
    remote_path: String,
    synced_at: chrono::DateTime<chrono::Utc>,
}

static DB_CACHE: OnceLock<Mutex<HashMap<String, CachedDb>>> = OnceLock::new();

fn db_cache() -> &'static Mutex<HashMap<String, CachedDb>> {
    DB_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 从远程同步数据库到本地缓存
pub async fn sync_database(
    db_path: String,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<String, String> {
    // 1. 远程执行 sqlite3 backup 到 /tmp
    // 2. SFTP 下载到本地临时目录
    // 3. 更新缓存 HashMap
    // 4. 返回本地路径
}

/// 获取已缓存的数据库路径
fn get_cached_db_path(remote_db_path: &str) -> Result<String, String> {
    let cache = db_cache().lock().map_err(|e| format!("缓存锁定失败: {}", e))?;
    cache.get(remote_db_path)
        .map(|c| c.local_path.clone())
        .ok_or_else(|| "数据库尚未同步，请先点击「同步数据库」".to_string())
}

/// 清理缓存
pub async fn clear_db_cache() -> Result<(), String> {
    let mut cache = db_cache().lock().map_err(|e| format!("缓存锁定失败: {}", e))?;
    for (_, cached) in cache.drain() {
        let _ = std::fs::remove_file(&cached.local_path);
    }
    Ok(())
}
```

**Step 2: 编译验证**

**Step 3: Commit**

---

## Task 3: 重写 `export_wide_table_direct` — 用本地 rusqlite 导出

**Files:**
- Modify: `src-tauri/src/query/mod.rs`

**核心逻辑改动：**
删除所有远程 Python 脚本代码，改为：
1. `get_cached_db_path()` 获取本地 db 路径
2. `rusqlite::Connection::open()` 打开本地 db
3. 执行 SQL 查询 `data_wide` 表
4. Rust 端做时间戳格式化（复用 `chrono`）
5. 用 `csv::Writer` 写入目标 CSV（带 UTF-8 BOM）

**关键点：**
- `local_timestamp` 是毫秒时间戳，需格式化为 `'YYYY-MM-DD HH:MM:SS.mmm`（前面加单引号保护 Excel）
- 空值处理：NULL → 空字符串
- 数值保持原始精度

**Step 1: 重写函数**

**Step 2: 编译验证**

**Step 3: Commit**

---

## Task 4: 重写 `export_demand_results_direct` — 用本地 rusqlite 导出

**Files:**
- Modify: `src-tauri/src/query/mod.rs`

与 Task 3 结构相同，区别：
- 查询 `demand_results` 表
- `timestamp` 是秒级时间戳（不是毫秒）
- 列固定：`id, timestamp, meter_sn, calculated_demand`

**Step 1: 重写函数**

**Step 2: 编译验证**

**Step 3: Commit**

---

## Task 5: 重写 `execute_sql_query` 和 `execute_wide_table_query` — 用本地 rusqlite 查询

**Files:**
- Modify: `src-tauri/src/query/mod.rs`

改为：
1. `get_cached_db_path()` 获取本地 db
2. `rusqlite::Connection::open()` 打开
3. 执行 SQL
4. 将结果转为 `Vec<serde_json::Value>` 返回前端

不再需要 CSV 中间格式，直接 rusqlite → JSON。

**Step 1: 重写函数**

**Step 2: 编译验证**

**Step 3: Commit**

---

## Task 6: 新增 Tauri 命令 `sync_database` 和 `clear_db_cache`

**Files:**
- Modify: `src-tauri/src/commands.rs` — 新增两个 command
- Modify: `src-tauri/src/main.rs` — 注册命令

```rust
// commands.rs
#[tauri::command]
pub async fn sync_database(
    app: tauri::AppHandle,
    db_path: String,
) -> Result<String, String> {
    crate::query::sync_database(db_path, Some(&app)).await
}

#[tauri::command]
pub async fn clear_db_cache() -> Result<(), String> {
    crate::query::clear_db_cache().await
}
```

**Step 1: 实现命令**

**Step 2: 编译验证**

**Step 3: Commit**

---

## Task 7: 前端集成 — 同步数据库按钮和流程调整

**Files:**
- Modify: `src/stores/query.ts` — 新增 `syncDatabase` action
- Modify: `src/components/DataQuery/QueryForm.vue` — 添加同步按钮

**核心变更：**
- 新增 `syncDatabase(dbPath)` action：调用 `sync_database` 命令
- 在查询/导出前检查数据库是否已同步
- UI 上添加"同步数据库"按钮，带同步状态显示

**Step 1: 修改 store**

**Step 2: 修改 UI**

**Step 3: 编译验证**

**Step 4: Commit**

---

## Task 8: 清理不再需要的代码和依赖

**Files:**
- Modify: `src-tauri/Cargo.toml` — 移除 `csv` crate（如果不再需要）
- Modify: `src-tauri/src/query/mod.rs` — 删除 `base64`、`uuid`、`tempfile` 相关 import（如果不再需要）

**注意：** `csv` crate 可能仍被导出函数使用（本地写 CSV），需要保留。检查 `uuid`、`tempfile` 是否还有其他用途。

**Step 1: 检查和清理**

**Step 2: 编译验证**

**Step 3: Commit**

---

## 关键技术细节

### 远程 backup 命令
```bash
sqlite3 /path/to/db ".backup '/tmp/remote_tool_backup.db'"
```
- `.backup` 是 SQLite 的原子操作，即使数据库正在被写入也安全
- CPU 开销极低，纯 I/O 操作
- 如果远程没有 `sqlite3` CLI，回退到 `cp`（WAL 模式下可能不完全一致）

### 时间戳格式化（Rust 端）
```rust
fn format_timestamp_ms_for_excel(timestamp_ms: i64) -> String {
    let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let secs = timestamp_ms / 1000;
    let ms = (timestamp_ms % 1000) as u32;
    let dt = chrono::DateTime::from_timestamp(secs, ms * 1_000_000)
        .unwrap()
        .with_timezone(&beijing_tz);
    format!("'{}", dt.format("%Y-%m-%d %H:%M:%S%.3f"))
}
```
