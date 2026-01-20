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

/// 直接导出宽表数据到CSV文件（流式处理，不加载到内存）
/// 返回导出的记录数
pub async fn export_wide_table_direct(
    db_path: String,
    start_time: i64,
    end_time: i64,
    output_path: String,
    app_handle: Option<tauri::AppHandle>,
) -> Result<usize, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 设置SSH日志回调，将SSH日志发送到查询日志
    if let Some(handle) = app_handle_ref {
        let handle_clone = handle.clone();
        crate::ssh::SshClient::set_log_callback(move |message: &str| {
            // 添加时间戳并发送到查询日志
            let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
            let now = Utc::now().with_timezone(&beijing_tz);
            let log_message = format!("[{}] {}", now.format("%H:%M:%S"), message);
            
            // 发送到前端
            use tauri::Emitter;
            let _ = handle_clone.emit("query-log", &log_message);
            
            // 同时输出到控制台
            eprintln!("{}", log_message);
        });
    }
    
    // 使用GMT+8时区格式化时间范围
    let start_time_str = format_gmt8_time(start_time);
    let end_time_str = format_gmt8_time(end_time);
    
    // 合并导出开始信息为一条日志
    add_query_log(app_handle_ref, &format!("开始导出宽表数据 | 时间范围: {} - {} | 输出: {}", 
        start_time_str, end_time_str, output_path));
    
    // 将参数进行base64编码，避免shell注入
    let db_path_b64 = general_purpose::STANDARD.encode(db_path.as_bytes());
    
    // 创建远程临时文件路径（CSV+Gzip格式，最高压缩级别）
    let mut uuid_buffer = [0u8; 32];
    let temp_file = format!("/tmp/wide_table_export_{}.csv.gz", Uuid::new_v4().simple().encode_lower(&mut uuid_buffer));
    
    // 创建Python脚本来执行流式查询和压缩
    // 使用gzip最高压缩级别（compresslevel=9）和流式处理（fetchmany）
    let python_script = format!(r#"
import sqlite3
import csv
import gzip
import sys
import base64
import os
import json
from datetime import datetime, timezone, timedelta

try:
    # 解码路径
    db_path = base64.b64decode("{}").decode('utf-8')
    temp_file = "{}"
    start_time_ms = {} * 1000  # 转换为毫秒
    end_time_ms = {} * 1000
    
    # 东八区时区
    beijing_tz = timezone(timedelta(hours=8))
    
    # 格式化毫秒时间戳为可读时间格式（东八区）
    # 在值前加单引号，强制Excel将其识别为文本（Excel会将单引号开头的值识别为文本）
    # 注意：单引号在CSV中不会被转义，所以Excel能正确识别
    def format_timestamp_ms(timestamp_ms):
        if timestamp_ms is None:
            return ''
        try:
            # 将毫秒时间戳转换为datetime对象（UTC）
            dt = datetime.fromtimestamp(timestamp_ms / 1000.0, tz=timezone.utc)
            # 转换为东八区
            dt_beijing = dt.astimezone(beijing_tz)
            # 格式化：YYYY-MM-DD HH:MM:SS.mmm（使用横线分隔日期，Excel更友好）
            milliseconds = int(timestamp_ms % 1000)
            formatted_time = dt_beijing.strftime("%Y-%m-%d %H:%M:%S")
            time_str = formatted_time + ".{{0:03d}}".format(milliseconds)
            # 在值前加单引号，强制Excel将其识别为文本
            # Excel会将单引号开头的值识别为文本，不会尝试解析为时间类型
            # 单引号在CSV中不是特殊字符，不会被转义，所以Excel能正确识别
            return "'" + time_str
        except (ValueError, OSError, OverflowError):
            # 如果转换失败，返回原始值（也加单引号保护）
            return "'" + str(timestamp_ms)
    
    # 连接数据库
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    # 执行查询（使用参数化查询避免SQL注入）
    sql = "SELECT * FROM data_wide WHERE local_timestamp >= ? AND local_timestamp <= ? ORDER BY local_timestamp"
    cursor.execute(sql, (start_time_ms, end_time_ms))
    
    # 获取列名
    columns = [description[0] for description in cursor.description] if cursor.description else []
    
    if not columns:
        # 如果没有列，创建空文件
        with gzip.open(temp_file, 'wt', encoding='utf-8', newline='', compresslevel=9) as f:
            pass
        print(json.dumps({{"file": temp_file, "rows": 0}}))
        conn.close()
        sys.exit(0)
    
    # 流式写入CSV到临时文件并压缩（最高压缩级别）
    # 使用fetchmany分批读取，避免一次性加载所有数据到内存
    row_count = 0
    batch_size = 1000  # 每批处理1000行
    
    with gzip.open(temp_file, 'wt', encoding='utf-8', newline='', compresslevel=9) as gz_file:
        # 配置CSV writer使用QUOTE_NONNUMERIC，确保非数字值（包括时间字符串）都被引号括起来
        # 这样可以确保Excel正确识别文本值，不会尝试解析为时间类型
        writer = csv.DictWriter(gz_file, fieldnames=columns, extrasaction='ignore', quoting=csv.QUOTE_NONNUMERIC)
        writer.writeheader()
        
        # 分批读取数据
        while True:
            rows = cursor.fetchmany(batch_size)
            if not rows:
                break
            
            for row in rows:
                row_dict = {{}}
                for i, col in enumerate(columns):
                    value = row[i]
                    # 处理None值，转换为空字符串（CSV标准）
                    if value is None:
                        row_dict[col] = ''
                    elif col == 'local_timestamp':
                        # 将local_timestamp列从毫秒时间戳转换为可读时间格式
                        # 注意：由于使用QUOTE_NONNUMERIC，字符串值会自动被引号括起来
                        row_dict[col] = format_timestamp_ms(value)
                    else:
                        # 转换为字符串（CSV只支持字符串）
                        # 如果是数字，保持为数字类型（不会被引号括起来）
                        # 如果是字符串，会被引号括起来
                        if isinstance(value, (int, float)):
                            row_dict[col] = value
                        else:
                            row_dict[col] = str(value)
                writer.writerow(row_dict)
                row_count += 1
    
    # 输出临时文件路径和行数
    result = json.dumps({{"file": temp_file, "rows": row_count}}, ensure_ascii=False)
    print(result)
    
    conn.close()
    sys.exit(0)
except Exception as e:
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
"#, db_path_b64, temp_file, start_time, end_time);
    
    add_query_log(app_handle_ref, "执行查询并压缩数据...");
    
    // 使用heredoc方式执行Python脚本
    let mut eof_uuid_buffer = [0u8; 32];
    let eof_uuid_str = Uuid::new_v4().simple().encode_lower(&mut eof_uuid_buffer);
    let eof_marker = format!("PYTHON_SCRIPT_EOF_{}", &eof_uuid_str[..8]);
    let command = format!("python3 << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
    
    // 执行命令
    let (exit_status, stdout, stderr) = SshClient::execute_command(&command)
        .await
        .map_err(|e| format!("执行查询命令失败: {}", e))?;
    
    // 如果python3不存在，尝试python
    let (exit_status, stdout, stderr) = if exit_status != 0 && stderr.to_lowercase().contains("command not found") {
        add_query_log(app_handle_ref, "使用 python 替代 python3");
        let command = format!("python << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
        SshClient::execute_command(&command)
            .await
            .map_err(|e| format!("执行查询命令失败: {}", e))?
    } else {
        (exit_status, stdout, stderr)
    };
    
    // 如果执行失败，处理错误
    if exit_status != 0 {
        let error_msg = if let Ok(error_data) = serde_json::from_str::<HashMap<String, String>>(&stderr) {
            error_data.get("error").cloned().unwrap_or_else(|| stderr.clone())
        } else {
            stderr.clone()
        };
        return Err(format!("SQL查询失败: {}", error_msg));
    }
    
    // 解析输出，获取临时文件路径和行数
    let result: HashMap<String, serde_json::Value> = serde_json::from_str(&stdout.trim())
        .map_err(|e| format!("解析查询结果失败: {}", e))?;
    
    let remote_temp_file = result.get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "未找到临时文件路径".to_string())?;
    let row_count = result.get("rows")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    
    // 创建本地临时文件（用于下载压缩文件）
    let local_temp_file = NamedTempFile::new()
        .map_err(|e| format!("创建本地临时文件失败: {}", e))?;
    let local_temp_path = local_temp_file.path().to_string_lossy().to_string();
    
    // 使用SFTP下载文件
    add_query_log(app_handle_ref, "下载文件...");
    SshClient::download_file(remote_temp_file, &local_temp_path)
        .await
        .map_err(|e| format!("下载结果文件失败: {}", e))?;
    
    // 获取压缩文件大小
    let compressed_size = std::fs::metadata(&local_temp_path)
        .map_err(|e| format!("获取文件信息失败: {}", e))?
        .len();
    
    // 清理远程临时文件
    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
    
    // 流式解压并直接写入目标CSV文件（不加载到内存）
    {
        use std::io::{Read, Write};
        
        // 打开压缩文件
        let file = std::fs::File::open(&local_temp_path)
            .map_err(|e| format!("打开压缩文件失败: {}", e))?;
        let decoder = GzDecoder::new(file);
        
        // 创建目标CSV文件（带UTF-8 BOM，Excel兼容）
        let mut output_file = std::fs::File::create(&output_path)
            .map_err(|e| format!("创建输出文件失败: {}", e))?;
        
        // 写入UTF-8 BOM
        output_file.write_all(&[0xEF, 0xBB, 0xBF])
            .map_err(|e| format!("写入BOM失败: {}", e))?;
        
        // 流式复制：从解压器直接写入目标文件
        let mut decoder_reader = BufReader::new(decoder);
        let mut buffer = [0u8; 8192]; // 8KB缓冲区
        loop {
            let bytes_read = decoder_reader.read(&mut buffer)
                .map_err(|e| format!("读取解压数据失败: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            output_file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("写入CSV文件失败: {}", e))?;
        }
        
        output_file.flush()
            .map_err(|e| format!("刷新CSV文件失败: {}", e))?;
    }
    
    // 清理本地临时文件
    let _ = std::fs::remove_file(&local_temp_path);
    
    // 获取最终文件大小
    let final_size = std::fs::metadata(&output_path)
        .map_err(|e| format!("获取输出文件信息失败: {}", e))?
        .len();
    
    // 合并最终信息为一条日志
    add_query_log(app_handle_ref, &format!("导出完成 | {} 条记录 | 压缩: {:.2}MB | 解压: {:.2}MB", 
        row_count, 
        compressed_size as f64 / 1024.0 / 1024.0,
        final_size as f64 / 1024.0 / 1024.0));
    
    // 清除SSH日志回调
    crate::ssh::SshClient::clear_log_callback();
    
    Ok(row_count)
}

/// 直接导出需量数据到CSV文件（流式处理，不加载到内存）
/// 返回导出的记录数
pub async fn export_demand_results_direct(
    db_path: String,
    start_time: i64,
    end_time: i64,
    output_path: String,
    app_handle: Option<tauri::AppHandle>,
) -> Result<usize, String> {
    let app_handle_ref = app_handle.as_ref();
    
    // 设置SSH日志回调，将SSH日志发送到查询日志
    if let Some(handle) = app_handle_ref {
        let handle_clone = handle.clone();
        crate::ssh::SshClient::set_log_callback(move |message: &str| {
            // 添加时间戳并发送到查询日志
            let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
            let now = Utc::now().with_timezone(&beijing_tz);
            let log_message = format!("[{}] {}", now.format("%H:%M:%S"), message);
            
            // 发送到前端
            use tauri::Emitter;
            let _ = handle_clone.emit("query-log", &log_message);
            
            // 同时输出到控制台
            eprintln!("{}", log_message);
        });
    }
    
    // 使用GMT+8时区格式化时间范围
    let start_time_str = format_gmt8_time(start_time);
    let end_time_str = format_gmt8_time(end_time);
    
    // 合并导出开始信息为一条日志
    add_query_log(app_handle_ref, &format!("开始导出需量数据 | 时间范围: {} - {} | 输出: {}", 
        start_time_str, end_time_str, output_path));
    
    // 将参数进行base64编码，避免shell注入
    let db_path_b64 = general_purpose::STANDARD.encode(db_path.as_bytes());
    
    // 创建远程临时文件路径（CSV+Gzip格式，最高压缩级别）
    let mut uuid_buffer = [0u8; 32];
    let temp_file = format!("/tmp/demand_results_export_{}.csv.gz", Uuid::new_v4().simple().encode_lower(&mut uuid_buffer));
    
    // 创建Python脚本来执行流式查询和压缩
    // 使用gzip最高压缩级别（compresslevel=9）和流式处理（fetchmany）
    let python_script = format!(r#"
import sqlite3
import csv
import gzip
import sys
import base64
import os
import json
from datetime import datetime, timezone, timedelta

try:
    # 解码路径
    db_path = base64.b64decode("{}").decode('utf-8')
    temp_file = "{}"
    start_time = {}  # 秒级时间戳
    end_time = {}    # 秒级时间戳
    
    # 东八区时区
    beijing_tz = timezone(timedelta(hours=8))
    
    # 格式化秒级时间戳为可读时间格式（东八区）
    # 在值前加单引号，强制Excel将其识别为文本（Excel会将单引号开头的值识别为文本）
    # 注意：单引号在CSV中不会被转义，所以Excel能正确识别
    def format_timestamp(timestamp):
        if timestamp is None:
            return ''
        try:
            # 将秒级时间戳转换为datetime对象（UTC）
            dt = datetime.fromtimestamp(timestamp, tz=timezone.utc)
            # 转换为东八区
            dt_beijing = dt.astimezone(beijing_tz)
            # 格式化：YYYY-MM-DD HH:MM:SS（使用横线分隔日期，Excel更友好）
            formatted_time = dt_beijing.strftime("%Y-%m-%d %H:%M:%S")
            # 在值前加单引号，强制Excel将其识别为文本
            # Excel会将单引号开头的值识别为文本，不会尝试解析为时间类型
            # 单引号在CSV中不是特殊字符，不会被转义，所以Excel能正确识别
            return "'" + formatted_time
        except (ValueError, OSError, OverflowError):
            # 如果转换失败，返回原始值（也加单引号保护）
            return "'" + str(timestamp)
    
    # 连接数据库
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    # 执行查询（使用参数化查询避免SQL注入）
    sql = "SELECT id, timestamp, meter_sn, calculated_demand FROM demand_results WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp ASC"
    cursor.execute(sql, (start_time, end_time))
    
    # 定义列名
    columns = ['id', 'timestamp', 'meter_sn', 'calculated_demand']
    
    # 流式写入CSV到临时文件并压缩（最高压缩级别）
    # 使用fetchmany分批读取，避免一次性加载所有数据到内存
    row_count = 0
    batch_size = 1000  # 每批处理1000行
    
    with gzip.open(temp_file, 'wt', encoding='utf-8', newline='', compresslevel=9) as gz_file:
        # 配置CSV writer使用QUOTE_NONNUMERIC，确保非数字值（包括时间字符串）都被引号括起来
        # 这样可以确保Excel正确识别文本值，不会尝试解析为时间类型
        writer = csv.DictWriter(gz_file, fieldnames=columns, extrasaction='ignore', quoting=csv.QUOTE_NONNUMERIC)
        writer.writeheader()
        
        # 分批读取数据
        while True:
            rows = cursor.fetchmany(batch_size)
            if not rows:
                break
            
            for row in rows:
                row_dict = {{}}
                row_dict['id'] = row[0] if row[0] is not None else ''
                # 格式化时间戳
                row_dict['timestamp'] = format_timestamp(row[1])
                row_dict['meter_sn'] = row[2] if row[2] is not None else ''
                # calculated_demand 是数字，保持为数字类型
                row_dict['calculated_demand'] = row[3] if row[3] is not None else 0.0
                writer.writerow(row_dict)
                row_count += 1
    
    # 输出临时文件路径和行数
    result = json.dumps({{"file": temp_file, "rows": row_count}}, ensure_ascii=False)
    print(result)
    
    conn.close()
    sys.exit(0)
except Exception as e:
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
"#, db_path_b64, temp_file, start_time, end_time);
    
    add_query_log(app_handle_ref, "执行查询并压缩数据...");
    
    // 使用heredoc方式执行Python脚本
    let mut eof_uuid_buffer = [0u8; 32];
    let eof_uuid_str = Uuid::new_v4().simple().encode_lower(&mut eof_uuid_buffer);
    let eof_marker = format!("PYTHON_SCRIPT_EOF_{}", &eof_uuid_str[..8]);
    let command = format!("python3 << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
    
    // 执行命令
    let (exit_status, stdout, stderr) = SshClient::execute_command(&command)
        .await
        .map_err(|e| format!("执行查询命令失败: {}", e))?;
    
    // 如果python3不存在，尝试python
    let (exit_status, stdout, stderr) = if exit_status != 0 && stderr.to_lowercase().contains("command not found") {
        add_query_log(app_handle_ref, "使用 python 替代 python3");
        let command = format!("python << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
        SshClient::execute_command(&command)
            .await
            .map_err(|e| format!("执行查询命令失败: {}", e))?
    } else {
        (exit_status, stdout, stderr)
    };
    
    // 如果执行失败，处理错误
    if exit_status != 0 {
        let error_msg = if let Ok(error_data) = serde_json::from_str::<HashMap<String, String>>(&stderr) {
            error_data.get("error").cloned().unwrap_or_else(|| stderr.clone())
        } else {
            stderr.clone()
        };
        return Err(format!("SQL查询失败: {}", error_msg));
    }
    
    // 解析输出，获取临时文件路径和行数
    let result: HashMap<String, serde_json::Value> = serde_json::from_str(&stdout.trim())
        .map_err(|e| format!("解析查询结果失败: {}", e))?;
    
    let remote_temp_file = result.get("file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "未找到临时文件路径".to_string())?;
    let row_count = result.get("rows")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    
    // 创建本地临时文件（用于下载压缩文件）
    let local_temp_file = NamedTempFile::new()
        .map_err(|e| format!("创建本地临时文件失败: {}", e))?;
    let local_temp_path = local_temp_file.path().to_string_lossy().to_string();
    
    // 使用SFTP下载文件
    add_query_log(app_handle_ref, "下载文件...");
    SshClient::download_file(remote_temp_file, &local_temp_path)
        .await
        .map_err(|e| format!("下载结果文件失败: {}", e))?;
    
    // 获取压缩文件大小
    let compressed_size = std::fs::metadata(&local_temp_path)
        .map_err(|e| format!("获取文件信息失败: {}", e))?
        .len();
    
    // 清理远程临时文件
    let _ = SshClient::execute_command(&format!("rm -f \"{}\"", remote_temp_file)).await;
    
    // 流式解压并直接写入目标CSV文件（不加载到内存）
    {
        use std::io::{Read, Write};
        
        // 打开压缩文件
        let file = std::fs::File::open(&local_temp_path)
            .map_err(|e| format!("打开压缩文件失败: {}", e))?;
        let decoder = GzDecoder::new(file);
        
        // 创建目标CSV文件（带UTF-8 BOM，Excel兼容）
        let mut output_file = std::fs::File::create(&output_path)
            .map_err(|e| format!("创建输出文件失败: {}", e))?;
        
        // 写入UTF-8 BOM
        output_file.write_all(&[0xEF, 0xBB, 0xBF])
            .map_err(|e| format!("写入BOM失败: {}", e))?;
        
        // 流式复制：从解压器直接写入目标文件
        let mut decoder_reader = BufReader::new(decoder);
        let mut buffer = [0u8; 8192]; // 8KB缓冲区
        loop {
            let bytes_read = decoder_reader.read(&mut buffer)
                .map_err(|e| format!("读取解压数据失败: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            output_file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("写入CSV文件失败: {}", e))?;
        }
        
        output_file.flush()
            .map_err(|e| format!("刷新CSV文件失败: {}", e))?;
    }
    
    // 清理本地临时文件
    let _ = std::fs::remove_file(&local_temp_path);
    
    // 获取最终文件大小
    let final_size = std::fs::metadata(&output_path)
        .map_err(|e| format!("获取输出文件信息失败: {}", e))?
        .len();
    
    // 合并最终信息为一条日志
    add_query_log(app_handle_ref, &format!("导出完成 | {} 条记录 | 压缩: {:.2}MB | 解压: {:.2}MB", 
        row_count, 
        compressed_size as f64 / 1024.0 / 1024.0,
        final_size as f64 / 1024.0 / 1024.0));
    
    // 清除SSH日志回调
    crate::ssh::SshClient::clear_log_callback();
    
    Ok(row_count)
}

/// 执行SQL查询并返回结果（通过SSH执行Python脚本）
/// 返回 (结果数据, 列名列表)
async fn execute_sql_query(db_path: &str, sql: &str, app_handle: Option<&tauri::AppHandle>) -> Result<(Vec<serde_json::Value>, Vec<String>), String> {
    let app_handle_ref = app_handle;
    
    // 设置SSH日志回调，将SSH日志发送到查询日志
    if let Some(handle) = app_handle_ref {
        let handle_clone = handle.clone();
        crate::ssh::SshClient::set_log_callback(move |message: &str| {
            // 添加时间戳并发送到查询日志
            let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
            let now = Utc::now().with_timezone(&beijing_tz);
            let log_message = format!("[{}] {}", now.format("%H:%M:%S"), message);
            
            // 发送到前端
            use tauri::Emitter;
            let _ = handle_clone.emit("query-log", &log_message);
            
            // 同时输出到控制台
            eprintln!("{}", log_message);
        });
    }
    
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
        with gzip.open(temp_file, 'wt', encoding='utf-8', newline='', compresslevel=9) as f:
            pass
        print(temp_file)
        conn.close()
        sys.exit(0)
    
    # 将CSV写入临时文件并压缩（最高压缩级别），避免stdout缓冲区限制
    with gzip.open(temp_file, 'wt', encoding='utf-8', newline='', compresslevel=9) as gz_file:
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
    // 使用heredoc方式执行Python脚本
    let mut eof_uuid_buffer = [0u8; 32];
    let eof_uuid_str = Uuid::new_v4().simple().encode_lower(&mut eof_uuid_buffer);
    let eof_marker = format!("PYTHON_SCRIPT_EOF_{}", &eof_uuid_str[..8]);
    let command = format!("python3 << '{}'\n{}\n{}", eof_marker, python_script, eof_marker);
    
    // 执行命令
    let (exit_status, stdout, stderr) = SshClient::execute_command(&command)
        .await
        .map_err(|e| format!("执行查询命令失败: {}", e))?;
    
    // 如果python3不存在，尝试python
    let (exit_status, stdout, stderr) = if exit_status != 0 && stderr.to_lowercase().contains("command not found") {
        add_query_log(app_handle_ref, "使用 python 替代 python3");
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
    
    // 创建本地临时文件（二进制模式，用于gzip文件）
    let local_temp_file = NamedTempFile::new()
        .map_err(|e| format!("创建本地临时文件失败: {}", e))?;
    let local_temp_path = local_temp_file.path().to_string_lossy().to_string();
    
    // 使用SFTP下载文件
    add_query_log(app_handle_ref, "下载查询结果...");
    SshClient::download_file(remote_temp_file, &local_temp_path)
        .await
        .map_err(|e| format!("下载结果文件失败: {}", e))?;
    
    // 获取文件大小
    let file_size = std::fs::metadata(&local_temp_path)
        .map_err(|e| format!("获取文件信息失败: {}", e))?
        .len();
    
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
    
    add_query_log(app_handle_ref, &format!("查询完成 | {} 行 | 文件大小: {:.2}MB", 
        results.len(), file_size as f64 / 1024.0 / 1024.0));
    
    // 清除SSH日志回调
    crate::ssh::SshClient::clear_log_callback();
    
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