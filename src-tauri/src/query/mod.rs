use crate::ssh::SshClient;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    pub db_path: String,
    pub start_time: i64,
    pub end_time: i64,
    pub device_sn: Option<String>,
    pub include_ext: Option<bool>,
    pub query_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub total_rows: usize,
}

pub async fn execute_query(params: QueryParams) -> Result<QueryResult, String> {
    // 宽表查询需要特殊处理
    if params.query_type == "wide_table" {
        return execute_wide_table_query(params).await;
    }
    
    let sql = build_query_sql(&params)?;
    
    // 生成唯一的临时文件名，避免stdout缓冲区限制
    let temp_file = format!("/tmp/query_result_{}.json", Uuid::new_v4().simple());
    
    // 通过SSH执行Python脚本查询数据库
    let python_script = format!(
        r#"
import sqlite3
import json
import sys
import base64
import os
import signal

temp_file = "{}"

# 定义清理函数，删除临时文件
def cleanup():
    try:
        if os.path.exists(temp_file):
            os.unlink(temp_file)
    except Exception:
        pass

# 不注册 atexit，让 Rust 代码负责清理
# 只在信号处理时清理（作为安全网）
def signal_handler(signum, frame):
    cleanup()
    sys.exit(1)

signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

try:
    db_path = base64.b64decode("{}").decode('utf-8')
    sql = base64.b64decode("{}").decode('utf-8')
    
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    cursor.execute(sql)
    
    columns = [description[0] for description in cursor.description] if cursor.description else []
    
    results = []
    for row in cursor.fetchall():
        row_dict = {{}}
        for i, col in enumerate(columns):
            value = row[i]
            if value is None:
                row_dict[col] = None
            else:
                row_dict[col] = value
        results.append(row_dict)
    
    # 将JSON写入临时文件，避免stdout缓冲区限制
    with open(temp_file, 'w', encoding='utf-8') as f:
        json.dump(results, f, ensure_ascii=False, default=str)
    
    # 验证文件是否成功创建
    if not os.path.exists(temp_file):
        raise Exception("Failed to create temp file: " + temp_file)
    
    # 验证文件大小
    file_size = os.path.getsize(temp_file)
    if file_size == 0:
        raise Exception("Temp file is empty: " + temp_file)
    
    # 输出临时文件路径（不立即清理，由 Rust 代码负责清理）
    print(temp_file, flush=True)
    
    conn.close()
    # 正常退出时不清理，让 Rust 代码负责清理
    sys.exit(0)
except Exception as e:
    # 异常时清理
    cleanup()
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
"#,
        temp_file,
        general_purpose::STANDARD.encode(&params.db_path),
        general_purpose::STANDARD.encode(&sql)
    );

    // 使用 trap 命令确保即使被强制终止也能清理临时文件
    let command = format!("trap 'rm -f \"{}\" 2>/dev/null; exit 1' EXIT INT TERM; python3 << 'EOF'\n{}\nEOF", temp_file, python_script);
    
    // 记录临时文件路径，即使查询失败也要清理
    let remote_temp_file = temp_file.clone();
    
    match SshClient::execute_command(&command).await {
        Ok((exit_status, stdout, stderr)) => {
            // 如果执行失败，输出详细错误信息
            if exit_status != 0 {
                let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                let error_msg = if !stderr.trim().is_empty() {
                    format!("Query failed: {}", stderr)
                } else if !stdout.trim().is_empty() {
                    format!("Query failed (stdout): {}", stdout)
                } else {
                    format!("Query failed with exit status: {}", exit_status)
                };
                return Err(error_msg);
            }
            
            // 从stdout获取临时文件路径（去除所有空白字符）
            let temp_file_path = stdout.trim().trim_end_matches('\n').trim_end_matches('\r').to_string();
            
            if temp_file_path.is_empty() {
                let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                return Err(format!("Failed to get temp file path from Python script. stdout: '{}', stderr: '{}'", stdout, stderr));
            }
            
            // 验证远程文件是否存在，并获取文件信息
            let check_cmd = format!("if [ -f \"{}\" ]; then ls -lh \"{}\" | awk '{{print $5}}' && echo 'exists'; else echo 'not found'; fi", temp_file_path, temp_file_path);
            match SshClient::execute_command(&check_cmd).await {
                Ok((_, check_output, _)) => {
                    if !check_output.trim().contains("exists") {
                        let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                        // 尝试列出 /tmp 目录看看文件是否在其他位置
                        let debug_cmd = format!("ls -la /tmp/query_result_* 2>/dev/null | head -5 || echo 'no files found'");
                        let debug_info = SshClient::execute_command(&debug_cmd).await
                            .map(|(_, out, _)| format!("Debug: {}", out))
                            .unwrap_or_else(|_| "Debug: failed to list files".to_string());
                        return Err(format!("Remote temp file does not exist: {}. {}", temp_file_path, debug_info));
                    }
                }
                Err(e) => {
                    // 检查失败，但继续尝试下载
                    eprintln!("Warning: Failed to check file existence: {}", e);
                }
            }
            
            // 下载临时文件
            let local_temp_file = std::env::temp_dir().join(format!("query_result_{}.json", Uuid::new_v4().simple()));
            let local_temp_file_str = local_temp_file.to_str().ok_or("Invalid temp file path")?;
            
            // 确保清理本地临时文件
            let cleanup_local = || {
                let _ = std::fs::remove_file(&local_temp_file);
            };
            
            match SshClient::download_file(&temp_file_path, local_temp_file_str).await {
                Ok(_) => {
                    // 读取文件内容
                    let json_content = std::fs::read_to_string(&local_temp_file)
                        .map_err(|e| {
                            cleanup_local();
                            format!("Failed to read temp file: {}", e)
                        })?;
                    
                    // 清理本地临时文件
                    cleanup_local();
                    
                    // 确保清理远程临时文件
                    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                    
                    let results: Vec<serde_json::Value> = serde_json::from_str(&json_content)
                        .map_err(|e| format!("Failed to parse results: {}", e))?;
            
                    if results.is_empty() {
                        return Ok(QueryResult {
                            columns: Vec::new(),
                            rows: Vec::new(),
                            total_rows: 0,
                        });
                    }
                    
                    // 提取列名
                    let columns: Vec<String> = results[0]
                        .as_object()
                        .ok_or("Invalid result format")?
                        .keys()
                        .cloned()
                        .collect();
                    
                    let total_rows = results.len();
                    Ok(QueryResult {
                        columns,
                        rows: results,
                        total_rows,
                    })
                }
                Err(e) => {
                    cleanup_local();
                    // 确保清理远程临时文件
                    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                    Err(format!("Failed to download result file: {}", e))
                }
            }
        }
        Err(e) => {
            // SSH命令执行失败，确保清理远程临时文件
            let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
            Err(format!("SSH command failed: {}", e))
        }
    }
}

fn build_query_sql(params: &QueryParams) -> Result<String, String> {
    let mut conditions = vec![
        format!("timestamp >= {}", params.start_time),
        format!("timestamp <= {}", params.end_time),
    ];
    
    if let Some(ref device_sn) = params.device_sn {
        let escaped = device_sn.replace("'", "''");
        conditions.push(format!("device_sn = '{}'", escaped));
    }
    
    let where_clause = conditions.join(" AND ");
    
    match params.query_type.as_str() {
        "device" => {
            let include_ext = params.include_ext.unwrap_or(false);
            if include_ext {
                Ok(format!(
                    "SELECT d.*, e.payload_json as payload_json FROM device_data d LEFT JOIN device_data_ext e ON d.id = e.device_data_id WHERE {} ORDER BY d.timestamp ASC",
                    where_clause
                ))
            } else {
                Ok(format!(
                    "SELECT * FROM device_data d WHERE {} ORDER BY d.timestamp ASC",
                    where_clause
                ))
            }
        }
        "command" => {
            Ok(format!(
                "SELECT id, timestamp, device_sn, name, value, local_timestamp FROM cmd_data WHERE {} ORDER BY timestamp ASC",
                where_clause
            ))
        }
        "wide_table" => {
            // 宽表查询在execute_wide_table_query中处理，这里不需要SQL
            Ok("".to_string())
        }
        _ => Err(format!("Unknown query type: {}", params.query_type)),
    }
}

async fn execute_wide_table_query(params: QueryParams) -> Result<QueryResult, String> {
    let include_ext = params.include_ext.unwrap_or(false);
    
    // 生成唯一的临时文件名，避免stdout缓冲区限制
    let temp_file = format!("/tmp/query_result_{}.json", Uuid::new_v4().simple());
    
    // 构建宽表查询的Python脚本
    let python_script = format!(
        r#"
import sqlite3
import json
import sys
import base64
from collections import defaultdict
import os
import signal

temp_file = "{}"

# 定义清理函数，删除临时文件
def cleanup():
    try:
        if os.path.exists(temp_file):
            os.unlink(temp_file)
    except Exception:
        pass

# 不注册 atexit，让 Rust 代码负责清理
# 只在信号处理时清理（作为安全网）
def signal_handler(signum, frame):
    cleanup()
    sys.exit(1)

signal.signal(signal.SIGINT, signal_handler)
signal.signal(signal.SIGTERM, signal_handler)

try:
    db_path = base64.b64decode("{}").decode('utf-8')
    start_time = {}
    end_time = {}
    include_ext = {}
    
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    # 1. 查询所有设备数据（不限制device_sn）
    device_sql = "SELECT d.*"
    if include_ext:
        device_sql += ", e.payload_json as payload_json FROM device_data d LEFT JOIN device_data_ext e ON d.id = e.device_data_id"
    else:
        device_sql += " FROM device_data d"
    device_sql += f" WHERE d.timestamp >= {{start_time}} AND d.timestamp <= {{end_time}} ORDER BY d.timestamp ASC"
    
    cursor.execute(device_sql)
    device_data = []
    for row in cursor.fetchall():
        row_dict = {{}}
        for key in row.keys():
            value = row[key]
            if value is None:
                row_dict[key] = None
            else:
                row_dict[key] = value
        device_data.append(row_dict)
    
    # 2. 查询指令数据
    cmd_sql = f"SELECT id, timestamp, device_sn, name, value, local_timestamp FROM cmd_data WHERE timestamp >= {{start_time}} AND timestamp <= {{end_time}} ORDER BY timestamp ASC"
    cursor.execute(cmd_sql)
    command_data = []
    for row in cursor.fetchall():
        row_dict = {{}}
        for key in row.keys():
            value = row[key]
            if value is None:
                row_dict[key] = None
            else:
                row_dict[key] = value
        command_data.append(row_dict)
    
    # 3. 加载CSV导出配置（如果存在）
    import os
    from pathlib import Path
    try:
        import toml
        has_toml = True
    except ImportError:
        try:
            import tomllib as toml
            has_toml = True
        except ImportError:
            has_toml = False
    
    # 默认配置
    main_table_fields = ["id", "device_sn", "device_type", "timestamp", "local_timestamp", "activePower", "reactivePower", "powerFactor"]
    extract_config = {{}}
    field_mapping = {{}}
    
    if has_toml:
        # 尝试加载配置文件
        config_paths = [
            Path(db_path).parent / "csv_export_config.toml",
            Path("/tmp") / "csv_export_config.toml",
            Path.home() / "csv_export_config.toml",
        ]
        for config_path in config_paths:
            if config_path.exists():
                try:
                    if hasattr(toml, 'load'):
                        with open(config_path, 'r', encoding='utf-8') as f:
                            config = toml.load(f)
                    else:
                        with open(config_path, 'rb') as f:
                            config = toml.load(f)
                    main_table_fields = config.get("main_table_fields", main_table_fields)
                    extract_config = config.get("extract_from_payload", {{}})
                    field_mapping = config.get("field_name_mapping", {{}})
                    break
                except Exception:
                    pass
    
    # 排除元数据字段，只保留数据字段
    metadata_fields = {{"id", "device_sn", "device_type", "timestamp", "local_timestamp"}}
    data_fields = [f for f in main_table_fields if f not in metadata_fields]
    
    # 4. 按 local_timestamp 合并数据
    wide_table = defaultdict(dict)
    
    # 处理设备数据
    for row in device_data:
        local_ts = row.get('local_timestamp')
        if local_ts is None:
            continue
        
        # 使用 local_timestamp（毫秒）作为主键
        if local_ts not in wide_table:
            wide_table[local_ts]['local_timestamp'] = local_ts
        
        device_sn = row.get('device_sn', '')
        device_type = row.get('device_type', '')
        
        if not device_sn:
            continue
        
        # 添加主表字段（使用设备序列号作为前缀）
        for key in data_fields:
            if key in row:
                value = row[key]
                column_name = f"{{device_sn}}_{{key}}"
                wide_table[local_ts][column_name] = value
        
        # 如果包含扩展表数据，从 payload_json 中提取字段
        if include_ext and 'payload_json' in row:
            payload_json = row.get('payload_json')
            if payload_json:
                try:
                    if isinstance(payload_json, str):
                        payload_data = json.loads(payload_json)
                    else:
                        payload_data = payload_json
                    
                    # 获取该设备类型需要提取的字段列表
                    fields_to_extract = extract_config.get(device_type, extract_config.get('default', []))
                    
                    # 提取字段
                    for field_key in fields_to_extract:
                        if isinstance(payload_data, dict):
                            value = payload_data.get(field_key)
                            if value is not None:
                                output_field_name = field_mapping.get(field_key, field_key)
                                column_name = f"{{device_sn}}_{{output_field_name}}"
                                wide_table[local_ts][column_name] = value
                except (json.JSONDecodeError, TypeError):
                    pass
    
    # 处理指令数据
    for cmd_row in command_data:
        local_ts = cmd_row.get('local_timestamp')
        if local_ts is None:
            continue
        
        # 使用 local_timestamp（毫秒）作为主键
        if local_ts not in wide_table:
            wide_table[local_ts]['local_timestamp'] = local_ts
        
        cmd_device_sn = cmd_row.get('device_sn', '')
        cmd_name = cmd_row.get('name', '')
        cmd_value = cmd_row.get('value')
        
        if cmd_name:
            if cmd_device_sn:
                # 使用设备序列号+指令名作为列名
                column_name = f"{{cmd_device_sn}}_{{cmd_name}}"
            else:
                # 如果没有设备序列号，直接使用指令名
                column_name = cmd_name
            wide_table[local_ts][column_name] = cmd_value
    
    # 转换为列表并排序
    result = list(wide_table.values())
    result.sort(key=lambda x: x.get('local_timestamp', 0))
    
    # 将JSON写入临时文件，避免stdout缓冲区限制
    with open(temp_file, 'w', encoding='utf-8') as f:
        json.dump(result, f, ensure_ascii=False, default=str)
    
    # 验证文件是否成功创建
    if not os.path.exists(temp_file):
        raise Exception("Failed to create temp file: " + temp_file)
    
    # 验证文件大小
    file_size = os.path.getsize(temp_file)
    if file_size == 0:
        raise Exception("Temp file is empty: " + temp_file)
    
    # 输出临时文件路径（不立即清理，由 Rust 代码负责清理）
    print(temp_file, flush=True)
    
    conn.close()
    # 正常退出时不清理，让 Rust 代码负责清理
    sys.exit(0)
except Exception as e:
    # 异常时清理
    cleanup()
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    import traceback
    traceback.print_exc()
    sys.exit(1)
"#,
        temp_file,
        general_purpose::STANDARD.encode(&params.db_path),
        params.start_time,
        params.end_time,
        if include_ext { "True" } else { "False" }
    );

    // 使用 trap 命令确保即使被强制终止也能清理临时文件
    let command = format!("trap 'rm -f \"{}\" 2>/dev/null; exit 1' EXIT INT TERM; python3 << 'EOF'\n{}\nEOF", temp_file, python_script);
    
    // 记录临时文件路径，即使查询失败也要清理
    let remote_temp_file = temp_file.clone();
    
    match SshClient::execute_command(&command).await {
        Ok((exit_status, stdout, stderr)) => {
            // 如果执行失败，确保清理临时文件
            if exit_status != 0 {
                let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                return Err(format!("Wide table query failed: {}", stderr));
            }
            
            // 从stdout获取临时文件路径（去除所有空白字符）
            let temp_file_path = stdout.trim().trim_end_matches('\n').trim_end_matches('\r').to_string();
            
            if temp_file_path.is_empty() {
                let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                return Err("Failed to get temp file path from Python script".to_string());
            }
            
            // 验证远程文件是否存在
            let check_cmd = format!("test -f \"{}\" && echo 'exists' || echo 'not found'", temp_file_path);
            match SshClient::execute_command(&check_cmd).await {
                Ok((_, check_output, _)) => {
                    if !check_output.trim().contains("exists") {
                        let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                        return Err(format!("Remote temp file does not exist: {}", temp_file_path));
                    }
                }
                Err(e) => {
                    // 检查失败，但继续尝试下载
                    eprintln!("Warning: Failed to check file existence: {}", e);
                }
            }
            
            // 下载临时文件
            let local_temp_file = std::env::temp_dir().join(format!("query_result_{}.json", Uuid::new_v4().simple()));
            let local_temp_file_str = local_temp_file.to_str().ok_or("Invalid temp file path")?;
            
            // 确保清理本地临时文件
            let cleanup_local = || {
                let _ = std::fs::remove_file(&local_temp_file);
            };
            
            match SshClient::download_file(&temp_file_path, local_temp_file_str).await {
                Ok(_) => {
                    // 读取文件内容
                    let json_content = std::fs::read_to_string(&local_temp_file)
                        .map_err(|e| {
                            cleanup_local();
                            format!("Failed to read temp file: {}", e)
                        })?;
                    
                    // 清理本地临时文件
                    cleanup_local();
                    
                    // 确保清理远程临时文件
                    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                    
                    let results: Vec<serde_json::Value> = serde_json::from_str(&json_content)
                        .map_err(|e| format!("Failed to parse results: {}", e))?;
            
                    if results.is_empty() {
                        return Ok(QueryResult {
                            columns: Vec::new(),
                            rows: Vec::new(),
                            total_rows: 0,
                        });
                    }
                    
                    // 提取列名（宽表的所有列）
                    let mut all_columns = std::collections::HashSet::new();
                    for row in &results {
                        if let Some(obj) = row.as_object() {
                            for key in obj.keys() {
                                all_columns.insert(key.clone());
                            }
                        }
                    }
                    
                    let mut columns: Vec<String> = all_columns.into_iter().collect();
                    columns.sort();
                    // 确保 local_timestamp 在最前面
                    if let Some(pos) = columns.iter().position(|x| x == "local_timestamp") {
                        columns.remove(pos);
                        columns.insert(0, "local_timestamp".to_string());
                    }
                    
                    let total_rows = results.len();
                    Ok(QueryResult {
                        columns,
                        rows: results,
                        total_rows,
                    })
                }
                Err(e) => {
                    cleanup_local();
                    // 确保清理远程临时文件
                    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
                    Err(format!("Failed to download result file: {}", e))
                }
            }
        }
        Err(e) => {
            // SSH命令执行失败，确保清理远程临时文件
            let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
            Err(format!("SSH command failed: {}", e))
        }
    }
}
