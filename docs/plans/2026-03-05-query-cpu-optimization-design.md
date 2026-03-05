# 查询导出 CPU 优化 & 需量导出错误修复 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 降低嵌入式设备上宽表导出的 CPU 负载，并修复需量导出在表不存在时的错误提示。

**Architecture:** 仅修改 `src-tauri/src/query/mod.rs` 中嵌入的 Python 脚本参数和写入方式，以及 Rust 端的错误处理逻辑。不涉及前端变更。

**Tech Stack:** Rust (Tauri 后端)、内嵌 Python 脚本、gzip、csv 模块

---

### Task 1: 优化宽表导出 Python 脚本 — 降低 CPU 负载

**Files:**
- Modify: `src-tauri/src/query/mod.rs:118-232`（`export_wide_table_direct` 函数中的 Python 脚本）

**Step 1: 将 gzip 压缩级别从 9 降为 1**

在 `export_wide_table_direct` 的 Python 脚本中，搜索所有 `compresslevel=9`，替换为 `compresslevel=1`。
涉及两处：
- 约第 177 行：空列时的 `gzip.open(..., compresslevel=9)`
- 约第 188 行：正式写入时的 `gzip.open(..., compresslevel=9)`

同时更新第 119 行的注释，将"最高压缩级别"改为"最快压缩级别"。

**Step 2: 将 DictWriter 替换为 csv.writer**

将 `csv.DictWriter` 改为 `csv.writer`，避免每行构建字典的开销。

改前：
```python
writer = csv.DictWriter(gz_file, fieldnames=columns, extrasaction='ignore', quoting=csv.QUOTE_NONNUMERIC)
writer.writeheader()

while True:
    rows = cursor.fetchmany(batch_size)
    if not rows:
        break
    for row in rows:
        row_dict = {}
        for i, col in enumerate(columns):
            value = row[i]
            if value is None:
                row_dict[col] = ''
            elif col == 'local_timestamp':
                row_dict[col] = format_timestamp_ms(value)
            else:
                if isinstance(value, (int, float)):
                    row_dict[col] = value
                else:
                    row_dict[col] = str(value)
        writer.writerow(row_dict)
        row_count += 1
```

改后：
```python
writer = csv.writer(gz_file, quoting=csv.QUOTE_NONNUMERIC)
writer.writerow(columns)

while True:
    rows = cursor.fetchmany(batch_size)
    if not rows:
        break
    for row in rows:
        row_data = []
        for i, col in enumerate(columns):
            value = row[i]
            if value is None:
                row_data.append('')
            elif col == 'local_timestamp':
                row_data.append(format_timestamp_ms(value))
            else:
                if isinstance(value, (int, float)):
                    row_data.append(value)
                else:
                    row_data.append(str(value))
        writer.writerow(row_data)
        row_count += 1
```

**Step 3: 增大 batch_size**

将 `batch_size = 1000` 改为 `batch_size = 5000`。

**Step 4: 添加 data_wide 表存在性检查**

在 `cursor.execute(sql, ...)` 前增加：
```python
cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='data_wide'")
if not cursor.fetchone():
    error_msg = json.dumps({"error": "数据库中不存在 data_wide 表"}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
```

**Step 5: 验证并提交**

```bash
cd /home/zhouzhang/remote-tool && cargo check -p remote-tool
git add src-tauri/src/query/mod.rs
git commit -m "perf(查询): 优化宽表导出性能，降低嵌入式设备CPU负载"
```

---

### Task 2: 优化需量导出 Python 脚本 — 同样优化 + 修复错误提示

**Files:**
- Modify: `src-tauri/src/query/mod.rs:398-491`（`export_demand_results_direct` 函数中的 Python 脚本）

**Step 1: 将 gzip 压缩级别从 9 降为 1**

搜索 `export_demand_results_direct` 函数中 Python 脚本的所有 `compresslevel=9`，替换为 `compresslevel=1`。同步更新相关注释。

**Step 2: 将 DictWriter 替换为 csv.writer**

与 Task 1 相同的模式，将 `csv.DictWriter` 改为 `csv.writer`。

改后：
```python
writer = csv.writer(gz_file, quoting=csv.QUOTE_NONNUMERIC)
writer.writerow(columns)

while True:
    rows = cursor.fetchmany(batch_size)
    if not rows:
        break
    for row in rows:
        row_data = []
        row_data.append(row[0] if row[0] is not None else '')
        row_data.append(format_timestamp(row[1]))
        row_data.append(row[2] if row[2] is not None else '')
        row_data.append(row[3] if row[3] is not None else 0.0)
        writer.writerow(row_data)
        row_count += 1
```

**Step 3: 增大 batch_size**

将 `batch_size = 1000` 改为 `batch_size = 5000`。

**Step 4: 添加 demand_results 表存在性检查**

在 `cursor.execute(sql, ...)` 前增加：
```python
cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='demand_results'")
if not cursor.fetchone():
    error_msg = json.dumps({"error": "数据库中不存在 demand_results 表"}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
```

**Step 5: 验证并提交**

```bash
cd /home/zhouzhang/remote-tool && cargo check -p remote-tool
git add src-tauri/src/query/mod.rs
git commit -m "perf(查询): 优化需量导出性能并修复表不存在时的错误提示"
```

---

### Task 3: 增强 Rust 端错误处理

**Files:**
- Modify: `src-tauri/src/query/mod.rs:268-277`（`export_wide_table_direct` 的 stdout 解析）
- Modify: `src-tauri/src/query/mod.rs:527-536`（`export_demand_results_direct` 的 stdout 解析）

**Step 1: 改进 stdout JSON 解析失败时的错误信息**

对两个导出函数中的 stdout 解析部分，将：
```rust
let result: HashMap<String, serde_json::Value> = serde_json::from_str(&stdout.trim())
    .map_err(|e| format!("解析查询结果失败: {}", e))?;
```
改为：
```rust
let result: HashMap<String, serde_json::Value> = serde_json::from_str(&stdout.trim())
    .map_err(|e| format!("解析查询结果失败: {}。原始输出: {}, 错误输出: {}", e, stdout.trim(), stderr.trim()))?;
```

**Step 2: 改进"未找到临时文件路径"的错误信息**

将：
```rust
let remote_temp_file = result.get("file")
    .and_then(|v| v.as_str())
    .ok_or_else(|| "未找到临时文件路径".to_string())?;
```
改为：
```rust
let remote_temp_file = result.get("file")
    .and_then(|v| v.as_str())
    .ok_or_else(|| format!("远程脚本未返回临时文件路径，返回内容: {}", stdout.trim()))?;
```

**Step 3: 验证并提交**

```bash
cd /home/zhouzhang/remote-tool && cargo check -p remote-tool
git add src-tauri/src/query/mod.rs
git commit -m "fix(查询): 增强导出错误处理，提供更明确的错误信息"
```

---

### Task 4: 同步优化 execute_sql_query 中的 Python 脚本（被 execute_wide_table_query 调用，保持一致）

**Files:**
- Modify: `src-tauri/src/query/mod.rs:645-705`（`execute_sql_query` 函数中的 Python 脚本）

**Step 1: 将 compresslevel=9 改为 compresslevel=1**

**Step 2: 将 fetchall() 改为 fetchmany(5000) 分批写入**

**Step 3: 验证并提交**

```bash
cd /home/zhouzhang/remote-tool && cargo check -p remote-tool
git add src-tauri/src/query/mod.rs
git commit -m "perf(查询): 同步优化 execute_sql_query 的压缩级别和查询方式"
```
