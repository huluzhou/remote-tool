use crate::ssh::SshClient;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use uuid::Uuid;
use chrono::{Utc, FixedOffset, TimeZone};
use std::collections::HashMap;
use tempfile::NamedTempFile;
use std::io::BufReader;
use flate2::read::GzDecoder;
use std::path::Path;
use std::sync::OnceLock;

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub total_rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csv_file_path: Option<String>, // 保存解压后的CSV文件路径，供导出时直接使用
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

pub async fn execute_query(
    params: QueryParams,
    app_handle: Option<tauri::AppHandle>,
) -> Result<QueryResult, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 记录查询开始信息
    add_query_log(app_handle_ref, "开始执行查询...");
    add_query_log(app_handle_ref, &format!("查询类型: {}", params.query_type));
    add_query_log(app_handle_ref, &format!("数据库路径: {}", params.db_path));
    
    // 使用GMT+8时区格式化时间范围
    let start_time_str = format_gmt8_time(params.start_time);
    let end_time_str = format_gmt8_time(params.end_time);
    add_query_log(app_handle_ref, &format!("时间范围: {} - {}", start_time_str, end_time_str));
    
    match params.query_type.as_str() {
        "device" => {
            return query_device_data(params, app_handle).await;
        }
        "command" => {
            return query_command_data(params, app_handle).await;
        }
        "wide_table" => {
            return execute_wide_table_query(params, app_handle).await;
        },   
        _ => {
            return Err(format!("Unknown query type: {}", params.query_type));
        }
    }
}

/// 执行SQL查询并返回结果（通过SSH执行Python脚本）
/// 返回 (结果数据, 列名列表, CSV文件路径)
async fn execute_sql_query(db_path: &str, sql: &str, app_handle: Option<&tauri::AppHandle>) -> Result<(Vec<serde_json::Value>, Vec<String>, Option<String>), String> {
    let app_handle_ref = app_handle;
    
    // 将SQL和路径进行base64编码，避免shell注入
    let sql_b64 = general_purpose::STANDARD.encode(sql.as_bytes());
    let db_path_b64 = general_purpose::STANDARD.encode(db_path.as_bytes());
    
    // 创建临时文件路径（CSV+Gzip格式）
    let mut uuid_buffer = [0u8; 32];
    let temp_file = format!("/tmp/query_result_{}.csv.gz", Uuid::new_v4().simple().encode_lower(&mut uuid_buffer));
    
    // 创建Python脚本来执行查询
    let python_script = format!(r#"
import sqlite3
import csv
import gzip
import sys
import base64
import os
import json

try:
    # 解码路径和SQL
    db_path = base64.b64decode("{}").decode('utf-8')
    sql = base64.b64decode("{}").decode('utf-8')
    temp_file = "{}"
    
    # 连接数据库
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    # 执行查询
    cursor.execute(sql)
    
    # 获取列名
    columns = [description[0] for description in cursor.description] if cursor.description else []
    
    if not columns:
        # 如果没有列，创建空文件
        with gzip.open(temp_file, 'wt', encoding='utf-8', newline='') as f:
            pass
        print(temp_file)
        conn.close()
        sys.exit(0)
    
    # 将CSV写入临时文件并压缩，避免stdout缓冲区限制
    with gzip.open(temp_file, 'wt', encoding='utf-8', newline='') as gz_file:
        writer = csv.DictWriter(gz_file, fieldnames=columns, extrasaction='ignore')
        writer.writeheader()
        
        for row in cursor.fetchall():
            row_dict = {{}}
            for i, col in enumerate(columns):
                value = row[i]
                # 处理None值，转换为空字符串（CSV标准）
                if value is None:
                    row_dict[col] = ''
                else:
                    # 转换为字符串（CSV只支持字符串）
                    row_dict[col] = str(value)
            writer.writerow(row_dict)
    
    # 输出临时文件路径
    print(temp_file)
    
    conn.close()
    sys.exit(0)
except Exception as e:
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
"#, db_path_b64, sql_b64, temp_file);
    println!("python_script: {}", python_script);
    // 使用heredoc方式执行Python脚本
    let mut eof_uuid_buffer = [0u8; 32];
    let eof_uuid_str = Uuid::new_v4().simple().encode_lower(&mut eof_uuid_buffer);
    let eof_marker = format!("PYTHON_SCRIPT_EOF_{}", &eof_uuid_str[..8]);
    let command = format!("python3 << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
    
    add_query_log(app_handle_ref, "执行SQL查询...");
    
    // 执行命令
    let (exit_status, stdout, stderr) = SshClient::execute_command(&command)
        .await
        .map_err(|e| format!("执行查询命令失败: {}", e))?;
    
    // 如果python3不存在，尝试python
    let (exit_status, stdout, stderr) = if exit_status != 0 && stderr.to_lowercase().contains("command not found") {
        add_query_log(app_handle_ref, "python3 未找到，尝试使用 python");
        let command = format!("python << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
        SshClient::execute_command(&command)
            .await
            .map_err(|e| format!("执行查询命令失败: {}", e))?
    } else {
        (exit_status, stdout, stderr)
    };
    
    // 如果执行失败，处理错误
    if exit_status != 0 {
        // 尝试解析错误信息
        let error_msg = if let Ok(error_data) = serde_json::from_str::<HashMap<String, String>>(&stderr) {
            error_data.get("error").cloned().unwrap_or_else(|| stderr.clone())
        } else {
            stderr.clone()
        };
        return Err(format!("SQL查询失败: {}", error_msg));
    }
    
    // 从stdout获取远程临时文件路径
    let remote_temp_file = stdout.trim();
    add_query_log(app_handle_ref, &format!("远程临时文件: {}", remote_temp_file));
    
    // 创建本地临时文件（二进制模式，用于gzip文件）
    let local_temp_file = NamedTempFile::new()
        .map_err(|e| format!("创建本地临时文件失败: {}", e))?;
    let local_temp_path = local_temp_file.path().to_string_lossy().to_string();
    
    // 使用SFTP下载文件
    add_query_log(app_handle_ref, "下载查询结果文件...");
    SshClient::download_file(remote_temp_file, &local_temp_path)
        .await
        .map_err(|e| format!("下载结果文件失败: {}", e))?;
    
    // 获取文件大小
    let file_size = std::fs::metadata(&local_temp_path)
        .map_err(|e| format!("获取文件信息失败: {}", e))?
        .len();
    add_query_log(app_handle_ref, &format!("文件下载成功: {} 字节 ({:.2} MB)", file_size, file_size as f64 / 1024.0 / 1024.0));
    
    // 清理远程临时文件
    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
    
    // 解压CSV+Gzip文件并读取到内存（不保存到磁盘）
    let csv_content = {
        // 打开gzip文件并解压
        let file = std::fs::File::open(&local_temp_path)
            .map_err(|e| format!("打开压缩文件失败: {}", e))?;
        let decoder = GzDecoder::new(file);
        
        // 将解压后的内容读取到内存
        use std::io::Read;
        let mut decoder_reader = BufReader::new(decoder);
        let mut csv_content = Vec::new();
        decoder_reader.read_to_end(&mut csv_content)
            .map_err(|e| format!("读取解压数据失败: {}", e))?;
        
        csv_content
    };
    
    // 数据已读取到内存，可以删除临时文件了
    let _ = std::fs::remove_file(&local_temp_path);
    
    // 从内存中的CSV内容解析为JSON（供前端显示）
    let mut reader = csv::Reader::from_reader(csv_content.as_slice());
    
    let mut results = Vec::new();
    let headers = reader.headers()
        .map_err(|e| format!("读取CSV表头失败: {}", e))?
        .clone();
    
    // 提取列名列表（保持CSV中的顺序，即数据库中的顺序）
    let columns: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
    
    for record in reader.records() {
        let record = record.map_err(|e| format!("读取CSV记录失败: {}", e))?;
        let mut row = serde_json::Map::new();
        
        // 按照CSV headers的顺序插入，保持列顺序
        for (i, field) in record.iter().enumerate() {
            let header = headers.get(i).unwrap_or("");
            let value: serde_json::Value = if field.is_empty() {
                serde_json::Value::Null
            } else {
                // 尝试转换为数字
                if let Ok(int_val) = field.parse::<i64>() {
                    serde_json::Value::Number(int_val.into())
                } else if let Ok(float_val) = field.parse::<f64>() {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(float_val)
                            .unwrap_or_else(|| serde_json::Number::from(0))
                    )
                } else {
                    serde_json::Value::String(field.to_string())
                }
            };
            row.insert(header.to_string(), value);
        }
        
        results.push(serde_json::Value::Object(row));
    }
    
    add_query_log(app_handle_ref, &format!("查询返回 {} 行", results.len()));
    
    // 数据已在内存中，不需要文件路径（导出时从内存数据生成）
    Ok((results, columns, None))
}

/// 获取表的所有列名（按数据库中的顺序）
async fn get_table_columns(db_path: &str, table_name: &str, app_handle: Option<&tauri::AppHandle>) -> Result<Vec<String>, String> {
    let sql = format!("PRAGMA table_info({})", table_name);
    let (results, _columns, _) = execute_sql_query(db_path, &sql, app_handle).await?;
    
    // PRAGMA table_info 返回的列顺序就是数据库中的列顺序
    // 从结果中提取 name 字段，保持顺序
    let table_columns: Vec<String> = results
        .into_iter()
        .filter_map(|row| {
            row.as_object()?
                .get("name")?
                .as_str()
                .map(|s| s.to_string())
        })
        .collect();
    
    Ok(table_columns)
}

async fn query_device_data(params: QueryParams, app_handle: Option<tauri::AppHandle>) -> Result<QueryResult, String> {
    let app_handle_ref = app_handle.as_ref();
    
    add_query_log(app_handle_ref, "开始查询设备数据...");
    
    // 构建WHERE条件
    let mut conditions = vec![
        format!("d.timestamp >= {}", params.start_time),
        format!("d.timestamp <= {}", params.end_time),
    ];
    
    if let Some(ref device_sn) = params.device_sn {
        // 转义单引号，防止SQL注入
        let escaped_device_sn = device_sn.replace("'", "''");
        conditions.push(format!("d.device_sn = '{}'", escaped_device_sn));
        add_query_log(app_handle_ref, &format!("设备序列号: {}", device_sn));
    }
    
    let where_clause = conditions.join(" AND ");
    let include_ext = params.include_ext.unwrap_or(false);
    
    add_query_log(app_handle_ref, &format!("包含扩展表: {}", include_ext));
    
    // 构建SQL查询
    // 使用数据库中的实际列顺序，而不是硬编码的顺序
    let sql = if include_ext {
        // 获取主表列名（按数据库中的顺序）
        let main_columns = match get_table_columns(&params.db_path, "device_data", app_handle_ref).await {
            Ok(cols) if !cols.is_empty() => cols,
            _ => {
                // 如果获取失败，使用默认列（按常见顺序）
                vec!["id".to_string(), "device_sn".to_string(), "device_type".to_string(), 
                     "timestamp".to_string(), "local_timestamp".to_string(), 
                     "activePower".to_string(), "reactivePower".to_string(), "powerFactor".to_string()]
            },
        };
        
        let select_fields = main_columns
            .iter()
            .map(|col| format!("d.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(
            "SELECT {} , e.payload_json as payload_json FROM device_data d LEFT JOIN device_data_ext e ON d.id = e.device_data_id WHERE {} ORDER BY d.timestamp ASC",
            select_fields, where_clause
        )
    } else {
        // 只查询主表（使用数据库中的实际列顺序）
        let main_columns = match get_table_columns(&params.db_path, "device_data", app_handle_ref).await {
            Ok(cols) if !cols.is_empty() => cols,
            _ => {
                // 如果获取失败，使用默认列（按常见顺序）
                vec!["id".to_string(), "device_sn".to_string(), "device_type".to_string(), 
                     "timestamp".to_string(), "local_timestamp".to_string(), 
                     "activePower".to_string(), "reactivePower".to_string(), "powerFactor".to_string()]
            },
        };
        
        let select_fields = main_columns
            .iter()
            .map(|col| format!("d.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(
            "SELECT {} FROM device_data d WHERE {} ORDER BY d.timestamp ASC",
            select_fields, where_clause
        )
    };
    
    add_query_log(app_handle_ref, "执行SQL查询...");
    
    // 执行查询，获取结果、列名和CSV文件路径
    let (results, columns, csv_file_path) = execute_sql_query(&params.db_path, &sql, app_handle_ref).await?;
    
    if results.is_empty() {
        add_query_log(app_handle_ref, "查询结果为空");
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            total_rows: 0,
            csv_file_path: None,
        });
    }
    
    // 使用从CSV headers中获取的列名（保持数据库中的顺序）
    let total_rows = results.len();
    
    add_query_log(app_handle_ref, &format!("查询完成，共 {} 行，{} 列", total_rows, columns.len()));
    
    Ok(QueryResult {
        columns,
        rows: results,
        total_rows,
        csv_file_path,
    })
}

async fn query_command_data(params: QueryParams, app_handle: Option<tauri::AppHandle>) -> Result<QueryResult, String> {
    let app_handle_ref = app_handle.as_ref();
    
    add_query_log(app_handle_ref, "开始查询命令数据...");
    
    // 构建WHERE条件
    let mut conditions = vec![
        format!("timestamp >= {}", params.start_time),
        format!("timestamp <= {}", params.end_time),
    ];
    
    if let Some(ref device_sn) = params.device_sn {
        // 转义单引号，防止SQL注入
        let escaped_device_sn = device_sn.replace("'", "''");
        conditions.push(format!("device_sn = '{}'", escaped_device_sn));
        add_query_log(app_handle_ref, &format!("设备序列号: {}", device_sn));
    }
    
    let where_clause = conditions.join(" AND ");
    
    // 构建SQL查询 - 查询cmd_data表
    let sql = format!(
        "SELECT id, timestamp, device_sn, name, value, local_timestamp FROM cmd_data WHERE {} ORDER BY timestamp ASC",
        where_clause
    );
    
    add_query_log(app_handle_ref, "执行SQL查询...");
    
    // 执行查询，获取结果、列名和CSV文件路径
    let (results, columns, csv_file_path) = execute_sql_query(&params.db_path, &sql, app_handle_ref).await?;
    
    if results.is_empty() {
        add_query_log(app_handle_ref, "查询结果为空");
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            total_rows: 0,
            csv_file_path: None,
        });
    }
    
    let total_rows = results.len();
    
    add_query_log(app_handle_ref, &format!("查询完成，共 {} 行，{} 列", total_rows, columns.len()));
    
    Ok(QueryResult {
        columns,
        rows: results,
        total_rows,
        csv_file_path,
    })
}

// 配置结构体（用于宽表查询）
#[derive(Debug, Deserialize, Clone)]
struct WideTableConfig {
    main_table_fields: Vec<String>,
    #[serde(default)]
    extract_from_payload: HashMap<String, Vec<String>>,
    #[serde(default)]
    field_name_mapping: HashMap<String, String>,
}

// 全局配置缓存
static WIDE_TABLE_CONFIG: OnceLock<WideTableConfig> = OnceLock::new();

// 默认配置
fn default_wide_table_config() -> WideTableConfig {
    WideTableConfig {
        main_table_fields: vec![
            "id".to_string(),
            "device_sn".to_string(),
            "device_type".to_string(),
            "timestamp".to_string(),
            "local_timestamp".to_string(),
            "activePower".to_string(),
            "reactivePower".to_string(),
        ],
        extract_from_payload: HashMap::new(),
        field_name_mapping: HashMap::new(),
    }
}

// 加载配置文件
fn load_wide_table_config() -> WideTableConfig {
    WIDE_TABLE_CONFIG.get_or_init(|| {
        // 1. 优先从可执行文件同目录读取
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let config_path = exe_dir.join("csv_export_config.toml");
                if config_path.exists() {
                    if let Ok(config) = parse_wide_table_config_file(&config_path) {
                        return config;
                    }
                }
            }
        }
        
        // 2. 从项目根目录读取
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config_path = project_root.parent()
            .map(|p| p.join("csv_export_config.toml"))
            .unwrap_or_else(|| project_root.join("csv_export_config.toml"));
        
        if config_path.exists() {
            if let Ok(config) = parse_wide_table_config_file(&config_path) {
                return config;
            }
        }
        
        // 3. 使用默认配置
        default_wide_table_config()
    }).clone()
}

// 解析配置文件
fn parse_wide_table_config_file(path: &Path) -> Result<WideTableConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: WideTableConfig = toml::from_str(&content)?;
    Ok(config)
}

async fn execute_wide_table_query(params: QueryParams, app_handle: Option<tauri::AppHandle>) -> Result<QueryResult, String> {
    let app_handle_ref = app_handle.as_ref();
    
    add_query_log(app_handle_ref, "开始执行宽表查询...");
    
    // 加载配置
    let config = load_wide_table_config();
    
    // 获取主表字段（排除元数据字段）
    let metadata_fields: std::collections::HashSet<&str> = ["id", "device_sn", "device_type", "timestamp", "local_timestamp"]
        .iter()
        .copied()
        .collect();
    let main_table_fields: Vec<String> = config.main_table_fields
        .iter()
        .filter(|f| !metadata_fields.contains(f.as_str()))
        .cloned()
        .collect();
    
    add_query_log(app_handle_ref, &format!("主表数据字段: {:?}", main_table_fields));
    
    // 1. 查询所有设备数据（不限制device_sn）
    add_query_log(app_handle_ref, "查询所有设备数据...");
    let include_ext = params.include_ext.unwrap_or(false);
    
    let device_params = QueryParams {
        db_path: params.db_path.clone(),
        start_time: params.start_time,
        end_time: params.end_time,
        device_sn: None, // 查询所有设备
        include_ext: Some(include_ext),
        query_type: "device".to_string(),
    };
    
    let device_result = query_device_data(device_params, app_handle.clone()).await?;
    add_query_log(app_handle_ref, &format!("设备数据查询完成: {} 行", device_result.total_rows));
    
    // 2. 查询所有命令数据
    add_query_log(app_handle_ref, "查询所有命令数据...");
    let command_params = QueryParams {
        db_path: params.db_path.clone(),
        start_time: params.start_time,
        end_time: params.end_time,
        device_sn: None, // 查询所有设备
        include_ext: None,
        query_type: "command".to_string(),
    };
    
    let command_result = query_command_data(command_params, app_handle.clone()).await?;
    add_query_log(app_handle_ref, &format!("命令数据查询完成: {} 行", command_result.total_rows));
    
    // 3. 在内存中合并数据，按 local_timestamp 分组
    add_query_log(app_handle_ref, "合并数据...");
    
    // 使用HashMap按local_timestamp（毫秒）分组
    let mut wide_table: HashMap<i64, serde_json::Map<String, serde_json::Value>> = HashMap::new();
    
    // 处理设备数据
    for row in device_result.rows {
        let row_obj = match row.as_object() {
            Some(obj) => obj,
            None => continue,
        };
        
        let local_ts = row_obj.get("local_timestamp")
            .and_then(|v| v.as_i64())
            .or_else(|| {
                // 如果local_timestamp是字符串，尝试解析
                row_obj.get("local_timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
            });
        
        let local_ts = match local_ts {
            Some(ts) => ts,
            None => continue,
        };
        
        // 如果该时间戳还没有记录，初始化
        if !wide_table.contains_key(&local_ts) {
            let mut init_row = serde_json::Map::new();
            init_row.insert("local_timestamp".to_string(), serde_json::Value::Number(local_ts.into()));
            wide_table.insert(local_ts, init_row);
        }
        
        let device_sn = row_obj.get("device_sn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let device_sn = match device_sn {
            Some(sn) if !sn.is_empty() => sn,
            _ => continue, // 如果没有设备序列号，跳过
        };
        
        let device_type = row_obj.get("device_type")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        // 添加主表字段（使用设备序列号作为前缀）
        for field in &main_table_fields {
            if let Some(value) = row_obj.get(field) {
                let column_name = format!("{}_{}", device_sn, field);
                wide_table.get_mut(&local_ts).unwrap().insert(column_name, value.clone());
            }
        }
        
        // 如果包含扩展表数据，从payload_json中提取字段
        if include_ext {
            if let Some(payload_json) = row_obj.get("payload_json") {
                if !payload_json.is_null() {
                    // 解析payload_json
                    let payload_data: Option<serde_json::Map<String, serde_json::Value>> = match payload_json {
                        serde_json::Value::String(s) => {
                            serde_json::from_str(s).ok()
                        },
                        serde_json::Value::Object(map) => Some(map.clone()),
                        _ => None,
                    };
                    
                    if let Some(payload_map) = payload_data {
                        // 获取该设备类型需要提取的字段列表
                        let fields_to_extract = config.extract_from_payload
                            .get(device_type)
                            .or_else(|| config.extract_from_payload.get("default"))
                            .cloned()
                            .unwrap_or_default();
                        
                        // 提取字段，列名为设备序列号+字段名
                        for field_key in fields_to_extract {
                            if let Some(value) = payload_map.get(&field_key) {
                                let column_name = format!("{}_{}", device_sn, field_key);
                                wide_table.get_mut(&local_ts).unwrap().insert(column_name, value.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 处理命令数据
    for row in command_result.rows {
        let row_obj = match row.as_object() {
            Some(obj) => obj,
            None => continue,
        };
        
        let local_ts = row_obj.get("local_timestamp")
            .and_then(|v| v.as_i64())
            .or_else(|| {
                row_obj.get("local_timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
            });
        
        let local_ts = match local_ts {
            Some(ts) => ts,
            None => continue,
        };
        
        // 如果该时间戳还没有记录，初始化
        if !wide_table.contains_key(&local_ts) {
            let mut init_row = serde_json::Map::new();
            init_row.insert("local_timestamp".to_string(), serde_json::Value::Number(local_ts.into()));
            wide_table.insert(local_ts, init_row);
        }
        
        let cmd_device_sn = row_obj.get("device_sn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let cmd_name = row_obj.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let cmd_value = row_obj.get("value").cloned();
        
        if let Some(name) = cmd_name {
            let column_name = if let Some(sn) = cmd_device_sn {
                if !sn.is_empty() {
                    format!("{}_{}", sn, name)
                } else {
                    name.clone()
                }
            } else {
                name.clone()
            };
            
            if let Some(value) = cmd_value {
                wide_table.get_mut(&local_ts).unwrap().insert(column_name, value);
            }
        }
    }
    
    // 转换为列表并排序
    let mut result_rows: Vec<serde_json::Value> = wide_table
        .into_values()
        .map(|map| serde_json::Value::Object(map))
        .collect();
    
    result_rows.sort_by_key(|row| {
        row.as_object()
            .and_then(|obj| obj.get("local_timestamp"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
    });
    
    add_query_log(app_handle_ref, &format!("数据合并完成: {} 行", result_rows.len()));
    
    // 提取所有列名（从所有行中收集）
    let mut all_columns: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in &result_rows {
        if let Some(obj) = row.as_object() {
            for key in obj.keys() {
                all_columns.insert(key.clone());
            }
        }
    }
    
    // 将列名转换为有序列表（local_timestamp优先，然后按字母顺序）
    let mut columns: Vec<String> = all_columns.into_iter().collect();
    columns.sort_by(|a, b| {
        if a == "local_timestamp" {
            std::cmp::Ordering::Less
        } else if b == "local_timestamp" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });
    
    // 数据已在内存中，不需要生成CSV文件（导出时从内存数据生成）
    let csv_file_path = None;
    
    let total_rows = result_rows.len();
    add_query_log(app_handle_ref, &format!("宽表查询完成，共 {} 行，{} 列", total_rows, columns.len()));
    
    Ok(QueryResult {
        columns,
        rows: result_rows,
        total_rows,
        csv_file_path,
    })
}