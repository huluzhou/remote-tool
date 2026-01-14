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
/// 返回 (结果数据, CSV文件路径)
async fn execute_sql_query(db_path: &str, sql: &str, app_handle: Option<&tauri::AppHandle>) -> Result<(Vec<serde_json::Value>, Option<String>), String> {
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
    
    // 解压CSV+Gzip文件并保存为CSV文件
    let (csv_file_path, csv_content) = {
        // 创建解压后的CSV文件路径
        let csv_path = local_temp_path.replace(".gz", "");
        let file = std::fs::File::open(&local_temp_path)
            .map_err(|e| format!("打开压缩文件失败: {}", e))?;
        let decoder = GzDecoder::new(file);
        
        // 将解压后的内容读取到内存
        use std::io::Read;
        let mut decoder_reader = BufReader::new(decoder);
        let mut csv_content = Vec::new();
        decoder_reader.read_to_end(&mut csv_content)
            .map_err(|e| format!("读取解压数据失败: {}", e))?;
        
        // 保存CSV文件
        std::fs::write(&csv_path, &csv_content)
            .map_err(|e| format!("写入CSV文件失败: {}", e))?;
        
        add_query_log(app_handle_ref, &format!("CSV文件已保存: {}", csv_path));
        (Some(csv_path), csv_content)
    };
    
    // 从内存中的CSV内容解析为JSON（供前端显示）
    let mut reader = csv::Reader::from_reader(csv_content.as_slice());
    
    let mut results = Vec::new();
    let headers = reader.headers()
        .map_err(|e| format!("读取CSV表头失败: {}", e))?
        .clone();
    
    for record in reader.records() {
        let record = record.map_err(|e| format!("读取CSV记录失败: {}", e))?;
        let mut row = serde_json::Map::new();
        
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
    
    Ok((results, csv_file_path))
}

/// 获取表的所有列名
async fn get_table_columns(db_path: &str, table_name: &str, app_handle: Option<&tauri::AppHandle>) -> Result<Vec<String>, String> {
    let sql = format!("PRAGMA table_info({})", table_name);
    let (results, _) = execute_sql_query(db_path, &sql, app_handle).await?;
    
    let columns: Vec<String> = results
        .into_iter()
        .filter_map(|row| {
            row.as_object()?
                .get("name")?
                .as_str()
                .map(|s| s.to_string())
        })
        .collect();
    
    Ok(columns)
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
    // 使用固定的列顺序，确保CSV列顺序正确
    let preferred_column_order = vec![
        "id",
        "device_sn",
        "device_type",
        "timestamp",
        "local_timestamp",
        "activePower",
        "reactivePower",
        "powerFactor",
    ];
    
    let sql = if include_ext {
        // 获取主表列名（用于验证列是否存在）
        let available_columns = match get_table_columns(&params.db_path, "device_data", app_handle_ref).await {
            Ok(cols) if !cols.is_empty() => {
                // 使用HashSet快速查找
                let cols_set: std::collections::HashSet<&str> = cols.iter().map(|s| s.as_str()).collect();
                // 按照preferred顺序排列，只包含实际存在的列
                preferred_column_order
                    .iter()
                    .filter(|col| cols_set.contains(*col))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            },
            _ => preferred_column_order.iter().map(|s| s.to_string()).collect(),
        };
        
        let select_fields = available_columns
            .iter()
            .map(|col| format!("d.{}", col))
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(
            "SELECT {} , e.payload_json as payload_json FROM device_data d LEFT JOIN device_data_ext e ON d.id = e.device_data_id WHERE {} ORDER BY d.timestamp ASC",
            select_fields, where_clause
        )
    } else {
        // 只查询主表
        let available_columns = match get_table_columns(&params.db_path, "device_data", app_handle_ref).await {
            Ok(cols) if !cols.is_empty() => {
                // 使用HashSet快速查找
                let cols_set: std::collections::HashSet<&str> = cols.iter().map(|s| s.as_str()).collect();
                // 按照preferred顺序排列，只包含实际存在的列
                preferred_column_order
                    .iter()
                    .filter(|col| cols_set.contains(*col))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            },
            _ => preferred_column_order.iter().map(|s| s.to_string()).collect(),
        };
        
        let select_fields = available_columns
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
    
    // 执行查询
    let (results, csv_file_path) = execute_sql_query(&params.db_path, &sql, app_handle_ref).await?;
    
    if results.is_empty() {
        add_query_log(app_handle_ref, "查询结果为空");
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            total_rows: 0,
            csv_file_path: None,
        });
    }
    
    // 从第一行获取列名
    let total_rows = results.len();
    let columns: Vec<String> = if let Some(first_row) = results.first() {
        if let Some(obj) = first_row.as_object() {
            obj.keys().cloned().collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    
    add_query_log(app_handle_ref, &format!("查询完成，共 {} 行，{} 列", total_rows, columns.len()));
    
    Ok(QueryResult {
        columns,
        rows: results,
        total_rows,
        csv_file_path,
    })
}

async fn query_command_data(_params: QueryParams, _app_handle: Option<tauri::AppHandle>) -> Result<QueryResult, String> {
    panic!("Not implemented");
}

async fn execute_wide_table_query(_params: QueryParams, _app_handle: Option<tauri::AppHandle>) -> Result<QueryResult, String> {
    panic!("Not implemented");
}