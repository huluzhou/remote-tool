use crate::ssh::SshClient;
use serde::{Deserialize, Serialize};
use chrono::{Utc, FixedOffset, TimeZone};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use rusqlite::OpenFlags;
use uuid::Uuid;

// ============================================================
// 数据库缓存管理
// ============================================================

/// 缓存的数据库信息
struct CachedDb {
    local_path: String,
    #[allow(dead_code)]
    remote_path: String,
    #[allow(dead_code)]
    synced_at: chrono::DateTime<chrono::Utc>,
}

/// 全局数据库缓存：remote_path -> CachedDb
fn db_cache() -> &'static Mutex<HashMap<String, CachedDb>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedDb>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 同步远程数据库到本地缓存，返回本地文件路径
pub async fn sync_database(
    db_path: String,
    app_handle: Option<tauri::AppHandle>,
) -> Result<String, String> {
    let app_handle_ref = app_handle.as_ref();
    add_query_log(app_handle_ref, &format!("开始同步数据库: {}", db_path));

    // 生成远程临时文件路径
    let mut uuid_buf = [0u8; 32];
    let uuid_str = Uuid::new_v4().simple().encode_lower(&mut uuid_buf).to_string();
    let remote_tmp = format!("/tmp/remote_tool_backup_{}.db", uuid_str);

    // 三级备份策略：sqlite3 .backup → python sqlite3.backup → cp + WAL
    // 以验证结果为准，不信任 exit_code（JumpServer 可能返回假成功）
    let verify_cmd = format!("ls -l \"{}\" 2>&1", remote_tmp);
    let mut used_cp_fallback = false;
    let mut file_info: String;

    // 第 1 级：sqlite3 .backup
    add_query_log(app_handle_ref, "尝试使用 sqlite3 .backup 创建数据库快照...");
    let backup_cmd = format!("sqlite3 \"{}\" \".backup '{}'\"", db_path, remote_tmp);
    let _ = SshClient::execute_command(&backup_cmd)
        .await
        .map_err(|e| format!("执行远程备份命令失败: {}", e))?;

    let (verify_exit, file_info_temp, _) = SshClient::execute_command(&verify_cmd)
        .await
        .map_err(|e| format!("验证远程文件失败: {}", e))?;
    file_info = file_info_temp;
    let mut need_next = verify_exit != 0 || file_info.contains("No such file");

    if need_next {
        // 第 2 级：Python sqlite3.Connection.backup()
        add_query_log(app_handle_ref, "备份文件未生成，尝试 Python backup...");
        let py_backup_cmd = format!(
            "python3 -c \"import sqlite3; s=sqlite3.connect('{}'); d=sqlite3.connect('{}'); s.backup(d); d.close(); s.close(); print('ok')\"",
            db_path, remote_tmp
        );
        let (py_exit, py_stdout, py_stderr) = SshClient::execute_command(&py_backup_cmd)
            .await
            .map_err(|e| format!("执行 Python 备份命令失败: {}", e))?;

        if py_exit != 0 || !py_stdout.trim().contains("ok") {
            let py_not_found = py_stderr.to_lowercase().contains("command not found")
                || py_stderr.to_lowercase().contains("not found")
                || py_stderr.to_lowercase().contains("no module");
            if py_not_found {
                add_query_log(app_handle_ref, "Python 不可用，使用 cp 复制...");
            } else {
                add_query_log(app_handle_ref, &format!("Python 备份失败({}), 使用 cp 复制...", py_stderr.trim()));
            }
        } else {
            add_query_log(app_handle_ref, "Python sqlite3.backup 快照创建成功");
        }

        let (v_exit, v_stdout, _) = SshClient::execute_command(&verify_cmd)
            .await
            .map_err(|e| format!("验证远程文件失败: {}", e))?;
        file_info = v_stdout;
        need_next = v_exit != 0 || file_info.contains("No such file");

        if need_next {
            // 第 3 级：cp + WAL/SHM
            add_query_log(app_handle_ref, "备份文件未生成，使用 cp 复制...");
            used_cp_fallback = true;
            let cp_cmd = format!(
                "cp \"{}\" \"{}\" && cp \"{}-wal\" \"{}-wal\" 2>/dev/null; cp \"{}-shm\" \"{}-shm\" 2>/dev/null; true",
                db_path, remote_tmp,
                db_path, remote_tmp,
                db_path, remote_tmp
            );
            let (cp_exit, _, cp_stderr) = SshClient::execute_command(&cp_cmd)
                .await
                .map_err(|e| format!("执行远程复制命令失败: {}", e))?;
            if cp_exit != 0 {
                return Err(format!("远程复制数据库文件失败: {}", cp_stderr.trim()));
            }
            let (v2_exit, v2_stdout, _) = SshClient::execute_command(&verify_cmd)
                .await
                .map_err(|e| format!("验证远程文件失败: {}", e))?;
            if v2_exit != 0 || v2_stdout.contains("No such file") {
                return Err(format!("远程数据库文件无法创建。ls 输出: {}", v2_stdout.trim()));
            }
            file_info = v2_stdout;
        }
    }

    add_query_log(app_handle_ref, &format!("远程文件就绪: {}", file_info.trim()));

    // 本地临时目录
    let local_dir = std::env::temp_dir();
    let local_filename = format!("remote_tool_cache_{}.db", uuid_str);
    let local_path = local_dir.join(&local_filename);
    let local_path_str = local_path.to_string_lossy().to_string();

    // SFTP 流式下载（分块读写，不加载整个文件到内存）
    add_query_log(app_handle_ref, "通过 SFTP 下载数据库文件...");
    SshClient::download_file(&remote_tmp, &local_path_str)
        .await
        .map_err(|e| format!("下载数据库文件失败: {}", e))?;

    // cp 路径下需要额外下载 WAL/SHM 文件；sqlite3 .backup 和 Python backup 生成的是完整独立 .db
    if used_cp_fallback {
        let remote_wal = format!("{}-wal", remote_tmp);
        let local_wal = format!("{}-wal", local_path_str);
        if SshClient::download_file(&remote_wal, &local_wal).await.is_ok() {
            add_query_log(app_handle_ref, "已下载 WAL 文件");
        }
        let remote_shm = format!("{}-shm", remote_tmp);
        let local_shm = format!("{}-shm", local_path_str);
        if SshClient::download_file(&remote_shm, &local_shm).await.is_ok() {
            add_query_log(app_handle_ref, "已下载 SHM 文件");
        }
    }

    // 验证下载的文件
    let file_size = std::fs::metadata(&local_path)
        .map_err(|e| format!("获取下载文件信息失败: {}", e))?
        .len();
    add_query_log(app_handle_ref, &format!("数据库下载完成，文件大小: {:.2}MB", file_size as f64 / 1024.0 / 1024.0));

    // 清理远程临时文件
    if used_cp_fallback {
        let _ = SshClient::execute_command(&format!(
            "rm -f \"{}\" \"{}-wal\" \"{}-shm\"", remote_tmp, remote_tmp, remote_tmp
        )).await;
    } else {
        let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_tmp)).await;
    }

    // 更新缓存
    {
        let mut cache = db_cache().lock().unwrap();
        // 删除旧的缓存文件
        if let Some(old) = cache.remove(&db_path) {
            let _ = std::fs::remove_file(&old.local_path);
        }
        cache.insert(db_path.clone(), CachedDb {
            local_path: local_path_str.clone(),
            remote_path: db_path.clone(),
            synced_at: Utc::now(),
        });
    }

    add_query_log(app_handle_ref, "数据库同步完成");
    Ok(local_path_str)
}

/// 获取已缓存的数据库本地路径
fn get_cached_db_path(remote_db_path: &str) -> Result<String, String> {
    let cache = db_cache().lock().unwrap();
    cache
        .get(remote_db_path)
        .map(|c| c.local_path.clone())
        .ok_or_else(|| "数据库尚未同步，请先点击「同步数据库」".to_string())
}

/// 清除所有数据库缓存
pub fn clear_db_cache() {
    let mut cache = db_cache().lock().unwrap();
    for (_, cached) in cache.drain() {
        let _ = std::fs::remove_file(&cached.local_path);
    }
}

// ============================================================
// 数据结构定义与工具函数
// ============================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    pub db_path: String,
    pub start_time: i64,
    pub end_time: i64,
    pub query_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub total_rows: usize,
}

// 格式化时间戳为GMT+8时区字符串
fn format_gmt8_time(timestamp: i64) -> String {
    let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let dt = Utc.timestamp_opt(timestamp, 0).unwrap().with_timezone(&beijing_tz);
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

// 添加带时间戳的日志并发送事件
fn add_query_log(app_handle: Option<&tauri::AppHandle>, message: &str) {
    let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let now = Utc::now().with_timezone(&beijing_tz);
    let log_message = format!("[{}] {}", now.format("%H:%M:%S"), message);
    
    // 发送事件到前端
    if let Some(handle) = app_handle {
        use tauri::Emitter;
        let _ = handle.emit("query-log", &log_message);
    }
    
    // 同时输出到控制台
    eprintln!("{}", log_message);
}

/// 检查表是否存在
fn check_table_exists(conn: &rusqlite::Connection, table_name: &str) -> Result<(), String> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            rusqlite::params![table_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查表是否存在失败: {}", e))?;
    if !exists {
        return Err(format!("数据库中不存在 {} 表", table_name));
    }
    Ok(())
}

// ============================================================
// 查询入口
// ============================================================

pub async fn execute_query(
    params: QueryParams,
    app_handle: Option<tauri::AppHandle>,
) -> Result<QueryResult, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 使用GMT+8时区格式化时间范围
    let start_time_str = format_gmt8_time(params.start_time);
    let end_time_str = format_gmt8_time(params.end_time);
    
    // 合并查询开始信息为一条日志
    add_query_log(app_handle_ref, &format!("开始查询 [{}] | 时间范围: {} - {}", 
        params.query_type, start_time_str, end_time_str));
    
    // 只支持宽表查询
    if params.query_type == "wide_table" {
        return execute_wide_table_query(params, app_handle).await;
    }
    
    Err(format!("不支持的查询类型: {}，仅支持 wide_table", params.query_type))
}

// ============================================================
// 导出宽表数据到CSV
// ============================================================

/// 直接导出宽表数据到CSV文件（本地rusqlite查询）
/// 返回导出的记录数
pub async fn export_wide_table_direct(
    db_path: String,
    start_time: i64,
    end_time: i64,
    output_path: String,
    app_handle: Option<tauri::AppHandle>,
) -> Result<usize, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 使用GMT+8时区格式化时间范围
    let start_time_str = format_gmt8_time(start_time);
    let end_time_str = format_gmt8_time(end_time);
    
    add_query_log(app_handle_ref, &format!("开始导出宽表数据 | 时间范围: {} - {} | 输出: {}", 
        start_time_str, end_time_str, output_path));
    
    // 获取本地缓存数据库路径
    let local_db_path = get_cached_db_path(&db_path)?;
    add_query_log(app_handle_ref, "使用本地缓存数据库查询...");
    
    let start_time_ms = start_time * 1000i64;
    let end_time_ms = end_time * 1000i64;
    let output_path_clone = output_path.clone();
    
    // rusqlite::Connection 不是 Send，需要在阻塞线程中执行
    let result = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let conn = rusqlite::Connection::open_with_flags(
            &local_db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).map_err(|e| format!("打开本地数据库失败: {}", e))?;
        
        check_table_exists(&conn, "data_wide")?;
        
        let mut stmt = conn.prepare(
            "SELECT * FROM data_wide WHERE local_timestamp >= ?1 AND local_timestamp <= ?2 ORDER BY local_timestamp"
        ).map_err(|e| format!("准备SQL语句失败: {}", e))?;
        
        let column_count = stmt.column_count();
        let columns: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect();
        
        // 找到 local_timestamp 列的索引
        let ts_col_idx = columns.iter().position(|c| c == "local_timestamp");
        
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        
        // 创建文件，写入UTF-8 BOM
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&output_path_clone)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("写入BOM失败: {}", e))?;
        }
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&output_path_clone)
            .map_err(|e| format!("打开输出文件失败: {}", e))?;
        let mut wtr = csv::WriterBuilder::new()
            .quote_style(csv::QuoteStyle::NonNumeric)
            .from_writer(std::io::BufWriter::new(file));
        
        // 写入表头
        wtr.write_record(&columns)
            .map_err(|e| format!("写入CSV表头失败: {}", e))?;
        
        let mut row_count: usize = 0;
        let rows = stmt.query_map(
            rusqlite::params![start_time_ms, end_time_ms],
            |row| {
                let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    values.push(row.get::<_, rusqlite::types::Value>(i)?);
                }
                Ok(values)
            },
        ).map_err(|e| format!("执行查询失败: {}", e))?;
        
        for row_result in rows {
            let values = row_result.map_err(|e| format!("读取行数据失败: {}", e))?;
            let mut record: Vec<String> = Vec::with_capacity(column_count);
            
            for (i, val) in values.iter().enumerate() {
                let field = if Some(i) == ts_col_idx {
                    match val {
                        rusqlite::types::Value::Integer(ms) => {
                            let secs = *ms / 1000;
                            let millis = (*ms % 1000) as u32;
                            match Utc.timestamp_opt(secs, millis * 1_000_000) {
                                chrono::LocalResult::Single(dt) => {
                                    let dt_bj = dt.with_timezone(&beijing_tz);
                                    format!("'{}.{:03}", dt_bj.format("%Y-%m-%d %H:%M:%S"), millis)
                                }
                                _ => format!("'{}", ms),
                            }
                        }
                        rusqlite::types::Value::Real(f) => {
                            let ms = *f as i64;
                            let secs = ms / 1000;
                            let millis = (ms % 1000) as u32;
                            match Utc.timestamp_opt(secs, millis * 1_000_000) {
                                chrono::LocalResult::Single(dt) => {
                                    let dt_bj = dt.with_timezone(&beijing_tz);
                                    format!("'{}.{:03}", dt_bj.format("%Y-%m-%d %H:%M:%S"), millis)
                                }
                                _ => format!("'{}", f),
                            }
                        }
                        rusqlite::types::Value::Null => String::new(),
                        other => format!("'{}", sqlite_value_to_csv_field(other)),
                    }
                } else {
                    sqlite_value_to_csv_field(val)
                };
                record.push(field);
            }
            
            wtr.write_record(&record)
                .map_err(|e| format!("写入CSV行失败: {}", e))?;
            row_count += 1;
        }
        
        wtr.flush().map_err(|e| format!("刷新CSV文件失败: {}", e))?;
        Ok(row_count)
    })
    .await
    .map_err(|e| format!("执行数据库查询线程失败: {}", e))??;
    
    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    add_query_log(app_handle_ref, &format!("导出完成 | {} 条记录 | 文件大小: {:.2}MB", 
        result, file_size as f64 / 1024.0 / 1024.0));
    
    Ok(result)
}

// ============================================================
// 导出需量数据到CSV
// ============================================================

/// 直接导出需量数据到CSV文件（本地rusqlite查询）
/// 返回导出的记录数
pub async fn export_demand_results_direct(
    db_path: String,
    start_time: i64,
    end_time: i64,
    output_path: String,
    app_handle: Option<tauri::AppHandle>,
) -> Result<usize, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 使用GMT+8时区格式化时间范围
    let start_time_str = format_gmt8_time(start_time);
    let end_time_str = format_gmt8_time(end_time);
    
    add_query_log(app_handle_ref, &format!("开始导出需量数据 | 时间范围: {} - {} | 输出: {}", 
        start_time_str, end_time_str, output_path));
    
    // 获取本地缓存数据库路径
    let local_db_path = get_cached_db_path(&db_path)?;
    add_query_log(app_handle_ref, "使用本地缓存数据库查询...");
    
    let output_path_clone = output_path.clone();
    
    let result = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let conn = rusqlite::Connection::open_with_flags(
            &local_db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).map_err(|e| format!("打开本地数据库失败: {}", e))?;
        
        check_table_exists(&conn, "demand_results")?;
        
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, meter_sn, calculated_demand FROM demand_results WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp ASC"
        ).map_err(|e| format!("准备SQL语句失败: {}", e))?;
        
        let columns = vec!["id", "timestamp", "meter_sn", "calculated_demand"];
        let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
        
        // 创建文件，先写BOM
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&output_path_clone)
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("写入BOM失败: {}", e))?;
        }
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&output_path_clone)
            .map_err(|e| format!("打开输出文件失败: {}", e))?;
        let mut wtr = csv::WriterBuilder::new()
            .quote_style(csv::QuoteStyle::NonNumeric)
            .from_writer(std::io::BufWriter::new(file));
        
        wtr.write_record(&columns)
            .map_err(|e| format!("写入CSV表头失败: {}", e))?;
        
        let mut row_count: usize = 0;
        let rows = stmt.query_map(
            rusqlite::params![start_time, end_time],
            |row| {
                // id, timestamp, meter_sn, calculated_demand
                Ok((
                    row.get::<_, rusqlite::types::Value>(0)?,
                    row.get::<_, rusqlite::types::Value>(1)?,
                    row.get::<_, rusqlite::types::Value>(2)?,
                    row.get::<_, rusqlite::types::Value>(3)?,
                ))
            },
        ).map_err(|e| format!("执行查询失败: {}", e))?;
        
        for row_result in rows {
            let (id_val, ts_val, meter_val, demand_val) = row_result
                .map_err(|e| format!("读取行数据失败: {}", e))?;
            
            let id_field = sqlite_value_to_csv_field(&id_val);
            
            // timestamp 列：秒级时间戳格式化（无毫秒）
            let ts_field = match &ts_val {
                rusqlite::types::Value::Integer(secs) => {
                    match Utc.timestamp_opt(*secs, 0) {
                        chrono::LocalResult::Single(dt) => {
                            let dt_bj = dt.with_timezone(&beijing_tz);
                            format!("'{}", dt_bj.format("%Y-%m-%d %H:%M:%S"))
                        }
                        _ => format!("'{}", secs),
                    }
                }
                rusqlite::types::Value::Real(f) => {
                    let secs = *f as i64;
                    match Utc.timestamp_opt(secs, 0) {
                        chrono::LocalResult::Single(dt) => {
                            let dt_bj = dt.with_timezone(&beijing_tz);
                            format!("'{}", dt_bj.format("%Y-%m-%d %H:%M:%S"))
                        }
                        _ => format!("'{}", f),
                    }
                }
                rusqlite::types::Value::Null => String::new(),
                other => format!("'{}", sqlite_value_to_csv_field(other)),
            };
            
            let meter_field = sqlite_value_to_csv_field(&meter_val);
            let demand_field = sqlite_value_to_csv_field(&demand_val);
            
            wtr.write_record(&[id_field, ts_field, meter_field, demand_field])
                .map_err(|e| format!("写入CSV行失败: {}", e))?;
            row_count += 1;
        }
        
        wtr.flush().map_err(|e| format!("刷新CSV文件失败: {}", e))?;
        Ok(row_count)
    })
    .await
    .map_err(|e| format!("执行数据库查询线程失败: {}", e))??;
    
    let file_size = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    add_query_log(app_handle_ref, &format!("导出完成 | {} 条记录 | 文件大小: {:.2}MB", 
        result, file_size as f64 / 1024.0 / 1024.0));
    
    Ok(result)
}

// ============================================================
// SQL查询（前端显示用）
// ============================================================

/// 执行SQL查询并返回结果（本地rusqlite）
/// 返回 (结果数据, 列名列表)
async fn execute_sql_query(db_path: &str, sql: &str, app_handle: Option<&tauri::AppHandle>) -> Result<(Vec<serde_json::Value>, Vec<String>), String> {
    let app_handle_ref = app_handle;
    
    let local_db_path = get_cached_db_path(db_path)?;
    add_query_log(app_handle_ref, "使用本地缓存数据库执行查询...");
    
    let sql_owned = sql.to_string();
    
    let (results, columns) = tokio::task::spawn_blocking(move || -> Result<(Vec<serde_json::Value>, Vec<String>), String> {
        let conn = rusqlite::Connection::open_with_flags(
            &local_db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).map_err(|e| format!("打开本地数据库失败: {}", e))?;
        
        let mut stmt = conn.prepare(&sql_owned)
            .map_err(|e| format!("SQL语句错误: {}", e))?;
        
        let column_count = stmt.column_count();
        let columns: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect();
        
        let mut results: Vec<serde_json::Value> = Vec::new();
        
        let rows = stmt.query_map([], |row| {
            let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(column_count);
            for i in 0..column_count {
                values.push(row.get::<_, rusqlite::types::Value>(i)?);
            }
            Ok(values)
        }).map_err(|e| format!("执行查询失败: {}", e))?;
        
        for row_result in rows {
            let values = row_result.map_err(|e| format!("读取行数据失败: {}", e))?;
            let mut row_map = serde_json::Map::new();
            
            for (i, val) in values.iter().enumerate() {
                let json_val = match val {
                    rusqlite::types::Value::Null => serde_json::Value::Null,
                    rusqlite::types::Value::Integer(n) => serde_json::Value::Number((*n).into()),
                    rusqlite::types::Value::Real(f) => {
                        serde_json::Number::from_f64(*f)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    }
                    rusqlite::types::Value::Text(s) => serde_json::Value::String(s.clone()),
                    rusqlite::types::Value::Blob(b) => {
                        serde_json::Value::String(format!("[BLOB {} bytes]", b.len()))
                    }
                };
                row_map.insert(columns[i].clone(), json_val);
            }
            
            results.push(serde_json::Value::Object(row_map));
        }
        
        Ok((results, columns))
    })
    .await
    .map_err(|e| format!("执行数据库查询线程失败: {}", e))??;
    
    add_query_log(app_handle_ref, &format!("查询完成 | {} 行 | {} 列", results.len(), columns.len()));
    
    Ok((results, columns))
}

async fn execute_wide_table_query(params: QueryParams, app_handle: Option<tauri::AppHandle>) -> Result<QueryResult, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 直接从 data_wide 表查询（不兼容旧表）
    let start_time_ms = params.start_time * 1000; // 转换为毫秒
    let end_time_ms = params.end_time * 1000;
    
    let sql = format!(
        "SELECT * FROM data_wide WHERE local_timestamp >= {} AND local_timestamp <= {} ORDER BY local_timestamp ASC",
        start_time_ms, end_time_ms
    );
    
    // 执行查询，获取结果和列名
    let (results, columns) = execute_sql_query(&params.db_path, &sql, app_handle_ref).await?;
    
    if results.is_empty() {
        add_query_log(app_handle_ref, "查询结果为空");
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            total_rows: 0,
        });
    }
    
    let total_rows = results.len();
    add_query_log(app_handle_ref, &format!("宽表查询完成 | {} 行 | {} 列", total_rows, columns.len()));
    
    Ok(QueryResult {
        columns,
        rows: results,
        total_rows,
    })
}

// ============================================================
// 工具函数
// ============================================================

/// 将 rusqlite Value 转换为 CSV 字段字符串
fn sqlite_value_to_csv_field(val: &rusqlite::types::Value) -> String {
    match val {
        rusqlite::types::Value::Null => String::new(),
        rusqlite::types::Value::Integer(n) => n.to_string(),
        rusqlite::types::Value::Real(f) => f.to_string(),
        rusqlite::types::Value::Text(s) => s.clone(),
        rusqlite::types::Value::Blob(b) => format!("[BLOB {} bytes]", b.len()),
    }
}

