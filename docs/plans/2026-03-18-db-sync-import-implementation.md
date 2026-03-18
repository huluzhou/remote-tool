# 数据库同步路径与本地导入能力 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在保留默认远程同步的前提下，支持配置数据库本地落盘路径与本地数据库导入，并让重启后可直接复用已存在本地库完成 CSV 导出。

**Architecture:** 后端扩展同步与本地库校验命令，前端新增“数据库来源”状态与持久化配置，导出流程统一读取当前有效本地数据库路径。通过“远程同步/本地导入”双模式避免重启后重复下载，同时保持原有查询与导出业务语义不变。

**Tech Stack:** Rust + Tauri 2、Vue 3 + TypeScript、Pinia、`@tauri-apps/plugin-dialog`、`rusqlite`

---

## 执行约束

- 采用 @superpowers:test-driven-development 思路：先写失败测试/校验，再补最小实现。
- 每个任务结束执行验证命令；优先 `cargo test`、`cargo check`、`npm run build`。
- 保持 DRY/YAGNI：只引入“来源切换 + 路径持久化 + 可读性校验”所需最小字段。
- 频繁提交（建议每个任务 1 次提交）。

### Task 1: 后端为同步命令增加可选落盘路径

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/query/mod.rs`
- Test: `src-tauri/src/query/mod.rs`（新增 `#[cfg(test)]` 单元测试）

**Step 1: 写失败测试（路径解析）**

在 `src-tauri/src/query/mod.rs` 新增测试：

```rust
#[test]
fn should_prefer_target_path_when_provided() {
    let resolved = resolve_local_db_path(Some("/tmp/custom.db".to_string()), "abc123");
    assert_eq!(resolved, "/tmp/custom.db");
}
```

**Step 2: 运行测试并确认失败**

Run: `cargo test should_prefer_target_path_when_provided`  
Expected: FAIL（`resolve_local_db_path` 未定义）

**Step 3: 实现最小功能**

- 在 `query/mod.rs` 增加纯函数 `resolve_local_db_path(target_path, uuid)`：
  - `target_path` 非空时返回指定路径；
  - 否则回退到当前临时目录逻辑。
- 修改 `sync_database` 签名，接收 `target_path: Option<String>`。
- 在 `commands.rs` 的 `sync_database` 命令参数中透传 `target_path`。

**Step 4: 运行测试确认通过**

Run: `cargo test should_prefer_target_path_when_provided`  
Expected: PASS

**Step 5: 运行后端编译检查**

Run: `cargo check`  
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/query/mod.rs src-tauri/src/commands.rs
git commit -m "feat(查询): 支持同步数据库自定义落盘路径"
```

### Task 2: 后端新增本地数据库校验命令

**Files:**
- Modify: `src-tauri/src/query/mod.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/query/mod.rs`（新增校验函数单元测试）

**Step 1: 写失败测试（关键表校验）**

```rust
#[test]
fn should_fail_when_required_tables_missing() {
    let err = validate_database_schema(vec!["sqlite_sequence".to_string()]).unwrap_err();
    assert!(err.contains("data_wide") || err.contains("demand_results"));
}
```

**Step 2: 运行测试并确认失败**

Run: `cargo test should_fail_when_required_tables_missing`  
Expected: FAIL（`validate_database_schema` 未定义）

**Step 3: 实现最小功能**

- 在 `query/mod.rs` 新增 `validate_local_database(path: String) -> Result<(), String>`：
  - 文件存在且可读；
  - 能打开 SQLite；
  - 至少存在 `data_wide` 或 `demand_results`。
- 新增纯函数 `validate_database_schema(tables: Vec<String>)` 便于单测。
- 在 `commands.rs` 注册 `validate_local_database` 命令。
- 在 `main.rs` 的 `generate_handler![]` 中添加该命令。

**Step 4: 运行测试确认通过**

Run: `cargo test should_fail_when_required_tables_missing`  
Expected: PASS

**Step 5: 回归后端检查**

Run: `cargo test && cargo check`  
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/query/mod.rs src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(查询): 新增本地数据库文件校验命令"
```

### Task 3: 前端 store 增加数据源状态与本地持久化

**Files:**
- Modify: `src/stores/query.ts`

**Step 1: 写失败校验（类型约束）**

在 `query.ts` 增加类型后，先在代码里临时引用新字段（如 `sourceMode`）但不实现初始化，触发 TS 报错。

**Step 2: 运行前端构建确认失败**

Run: `npm run build`  
Expected: FAIL（缺少字段或类型不匹配）

**Step 3: 实现最小功能**

- 新增状态字段：
  - `sourceMode: "remote_sync" | "local_import"`
  - `remoteDbPath`
  - `syncTargetPath`
  - `importedDbPath`
  - `activeDbPath`
  - `lastReadyAt`
- 新增方法：
  - `loadSourceConfig()`
  - `saveSourceConfig()`
  - `setSourceMode()`
  - `setImportedDbPath()`
  - `setSyncTargetPath()`
- 在 `syncDatabase()` 成功后更新 `activeDbPath` 并持久化。
- 使用 `localStorage` 做最小持久化实现（键名如 `query-source-config`）。

**Step 4: 运行前端构建确认通过**

Run: `npm run build`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/stores/query.ts
git commit -m "feat(前端): 持久化数据库来源配置与活动数据库路径"
```

### Task 4: 查询表单新增“数据库来源”交互区

**Files:**
- Modify: `src/components/DataQuery/QueryForm.vue`

**Step 1: 写失败校验（交互字段）**

先在模板中引用 `queryStore.sourceMode`、`queryStore.syncTargetPath`、`queryStore.importedDbPath`，暂不补全方法绑定。

**Step 2: 运行前端构建确认失败**

Run: `npm run build`  
Expected: FAIL（事件/字段未定义）

**Step 3: 实现最小功能**

- 新增“数据库来源”区域：
  - 单选 `远程同步（默认）` / `本地导入`。
- 远程同步模式：
  - 保留远程数据库路径输入；
  - 新增同步落盘路径输入与“选择路径”按钮（可用文件保存对话框选择 `.db` 文件路径）。
- 本地导入模式：
  - 新增“导入数据库文件”按钮（文件选择对话框）；
  - 选择后调用 `validate_local_database`，成功后更新 `activeDbPath`。
- 提交导出前校验当前 `activeDbPath`（无效则提示并阻止）。

**Step 4: 运行前端构建确认通过**

Run: `npm run build`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/components/DataQuery/QueryForm.vue
git commit -m "feat(前端): 新增数据库来源切换与本地导入交互"
```

### Task 5: 统一导出入口读取活动数据库路径并补齐联调

**Files:**
- Modify: `src/views/QueryView.vue`
- Modify: `src/stores/query.ts`
- Modify: `src-tauri/src/commands.rs`（如参数名调整）

**Step 1: 写失败校验（导出参数来源）**

在 `QueryView.vue` 中先改为读取 `queryStore.activeDbPath`，不做空值处理，触发构建失败。

**Step 2: 运行前端构建确认失败**

Run: `npm run build`  
Expected: FAIL（空值或类型问题）

**Step 3: 实现最小功能**

- 导出参数中的 `dbPath` 统一改为“活动数据库路径”。
- 当 `activeDbPath` 不存在或不可用时，统一提示：
  - “请先同步数据库或导入本地数据库文件”。
- 确保远程同步成功后无需再次同步即可直接导出。

**Step 4: 运行全量验证**

Run: `npm run build && cargo check`  
Expected: PASS

**Step 5: 手工验收（Tauri Dev）**

Run: `npm run tauri:dev`  
Expected:
- 场景 A：远程同步一次后重启，直接导出成功；
- 场景 B：导入本地库后直接导出成功；
- 场景 C：删除活动库文件后导出被拦截并提示中文错误。

**Step 6: Commit**

```bash
git add src/views/QueryView.vue src/stores/query.ts src-tauri/src/commands.rs
git commit -m "feat(查询): 统一活动数据库路径并完成导出联调"
```

### Task 6: 文档与回归清单

**Files:**
- Modify: `USER_GUIDE.md`
- Modify: `README.md`（如需补充）
- Create: `docs/plans/2026-03-18-db-sync-import-design.md`（已存在时仅补引用）

**Step 1: 更新用户文档**

- 新增“数据库来源模式”说明；
- 新增“同步落盘路径设置”说明；
- 新增“导入本地数据库并导出 CSV”说明。

**Step 2: 回归执行**

Run: `npm run build && cargo check`  
Expected: PASS

**Step 3: Commit**

```bash
git add USER_GUIDE.md README.md docs/plans/2026-03-18-db-sync-import-design.md
git commit -m "docs(查询): 补充数据库来源与导出新流程说明"
```
