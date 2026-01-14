use anyhow::Result;
use csv::Writer;
use serde::Deserialize;
use serde_json::Value;
use chrono::{Utc, TimeZone, FixedOffset};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

// 配置结构体
#[derive(Debug, Deserialize, Clone)]
struct ExportConfig {
    main_table_fields: Vec<String>,
    #[serde(default)]
    extract_from_payload: HashMap<String, Vec<String>>,
    #[serde(default)]
    field_name_mapping: HashMap<String, String>,
}

// 全局配置缓存
static CONFIG: OnceLock<ExportConfig> = OnceLock::new();

// 默认配置
fn default_config() -> ExportConfig {
    ExportConfig {
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
fn load_config() -> ExportConfig {
    CONFIG.get_or_init(|| {
        // 1. 优先从可执行文件同目录读取
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let config_path = exe_dir.join("csv_export_config.toml");
                if config_path.exists() {
                    if let Ok(config) = parse_config_file(&config_path) {
                        return config;
                    }
                }
            }
        }
        
        // 2. 从项目根目录读取（CARGO_MANIFEST_DIR 是 src-tauri，父目录是项目根目录）
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config_path = project_root.parent()
            .map(|p| p.join("csv_export_config.toml"))
            .unwrap_or_else(|| project_root.join("csv_export_config.toml"));
        
        if config_path.exists() {
            if let Ok(config) = parse_config_file(&config_path) {
                return config;
            }
        }
        
        // 3. 使用默认配置
        default_config()
    }).clone()
}

// 解析配置文件
fn parse_config_file(path: &Path) -> Result<ExportConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: ExportConfig = toml::from_str(&content)?;
    Ok(config)
}

// 格式化时间戳（东八区）
fn format_timestamp(timestamp: i64, is_millis: bool) -> String {
    let dt = if is_millis {
        Utc.timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
            .single()
    } else {
        Utc.timestamp_opt(timestamp, 0).single()
    };
    
    match dt {
        Some(dt) => {
            let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
            let beijing_dt = dt.with_timezone(&beijing_tz);
            if is_millis {
                format!("{}.{:03}", beijing_dt.format("%Y/%m/%d %H:%M:%S"), timestamp % 1000)
            } else {
                beijing_dt.format("%Y-%m-%d %H:%M:%S").to_string()
            }
        }
        None => timestamp.to_string(),
    }
}

// 格式化值
fn format_value(value: Option<&Value>) -> String {
    match value {
        None => String::new(),
        Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(a)) => serde_json::to_string(a).unwrap_or_default(),
        Some(Value::Object(o)) => serde_json::to_string(o).unwrap_or_default(),
    }
}

// 过滤字段并提取payload_json中的字段
fn filter_and_extract_fields(data: &[Value], config: &ExportConfig) -> Vec<HashMap<String, Value>> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let main_table_fields = &config.main_table_fields;
    let extract_config = &config.extract_from_payload;
    let field_mapping = &config.field_name_mapping;
    
    let mut result = Vec::new();
    
    for row in data {
        if let Some(obj) = row.as_object() {
            let mut new_row = HashMap::new();
            // 获取设备类型，转换为大写以匹配配置（配置中的设备类型通常是大写）
            let device_type = obj.get("device_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_uppercase())
                .unwrap_or_else(|| "default".to_string());
            
            // 1. 保留主表字段
            for field in main_table_fields {
                if let Some(value) = obj.get(field) {
                    new_row.insert(field.clone(), value.clone());
                }
            }
            
            // 2. 从payload_json中提取配置的字段
            if let Some(payload_json) = obj.get("payload_json") {
                // 跳过空值和空字符串
                if !payload_json.is_null() {
                    let payload_data: Option<HashMap<String, Value>> = match payload_json {
                        Value::String(s) => {
                            // 跳过空字符串
                            if s.is_empty() {
                                None
                            } else {
                                serde_json::from_str(s).ok()
                            }
                        }
                        Value::Object(o) => {
                            if o.is_empty() {
                                None
                            } else {
                                Some(o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            }
                        }
                        _ => None,
                    };
                    
                    if let Some(payload) = payload_data {
                        // 获取该设备类型需要提取的字段列表
                        // 先尝试精确匹配，再尝试默认配置
                        let fields_to_extract = extract_config.get(&device_type)
                            .or_else(|| extract_config.get("default"))
                            .cloned()
                            .unwrap_or_default();
                        
                        // 如果配置了该设备类型的字段，进行提取
                        if !fields_to_extract.is_empty() {
                            // 提取字段
                            for field_key in fields_to_extract {
                                if let Some(value) = payload.get(&field_key) {
                                    // 跳过null值
                                    if !value.is_null() {
                                        // 应用字段名映射
                                        let output_field_name = field_mapping.get(&field_key)
                                            .cloned()
                                            .unwrap_or(field_key.clone());
                                        new_row.insert(output_field_name, value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            result.push(new_row);
        }
    }
    
    result
}

// 为数据添加格式化时间戳
fn add_formatted_timestamps(data: Vec<HashMap<String, Value>>) -> Vec<HashMap<String, Value>> {
    data.into_iter().map(|mut row| {
        // 格式化timestamp（秒级）
        if let Some(timestamp) = row.get("timestamp") {
            if let Some(ts) = timestamp.as_i64() {
                row.insert("timestamp".to_string(), Value::String(format_timestamp(ts, false)));
            }
        }
        
        // 格式化local_timestamp（毫秒级）
        if let Some(local_timestamp) = row.get("local_timestamp") {
            if let Some(ts) = local_timestamp.as_i64() {
                row.insert("local_timestamp".to_string(), Value::String(format_timestamp(ts, true)));
            }
        }
        
        row
    }).collect()
}

// 重新排列列顺序（普通查询）
fn reorder_columns(data: Vec<HashMap<String, Value>>) -> Vec<HashMap<String, Value>> {
    if data.is_empty() {
        return data;
    }
    
    let priority_columns = vec![
        "id".to_string(),
        "device_sn".to_string(),
        "device_type".to_string(),
        "timestamp".to_string(),
        "local_timestamp".to_string(),
    ];
    
    data.into_iter().map(|row| {
        let mut new_row = HashMap::new();
        
        // 先添加优先级列
        for col in &priority_columns {
            if let Some(value) = row.get(col) {
                new_row.insert(col.clone(), value.clone());
            }
        }
        
        // 再添加其他列
        for (key, value) in row {
            if !priority_columns.contains(&key) {
                new_row.insert(key, value);
            }
        }
        
        new_row
    }).collect()
}

// 准备普通查询数据用于导出
fn prepare_for_export(data: &[Value], config: &ExportConfig) -> Vec<HashMap<String, Value>> {
    // 1. 过滤字段，提取payload_json中的字段
    let filtered = filter_and_extract_fields(data, config);
    
    // 2. 格式化时间戳
    let formatted = add_formatted_timestamps(filtered);
    
    // 3. 重新排列列顺序
    reorder_columns(formatted)
}

// 主导出函数（从内存数据直接导出到CSV文件）
pub async fn export_to_csv(
    data: crate::query::QueryResult,
    output_path: String,
    query_type: Option<String>,
) -> Result<(), String> {
    match query_type.as_deref() {
        Some("device") => export_device_data_from_memory(&data, &output_path).await,
        Some("command") => export_command_data_from_memory(&data, &output_path).await,
        Some("wide_table") => export_wide_table_from_memory(&data, &output_path).await,
        _ => {
            // 默认按设备数据处理（向后兼容）
            export_device_data_from_memory(&data, &output_path).await
        }
    }
}

// 从内存数据导出设备数据：处理payload_json字段提取、格式化时间戳、重新排列列顺序
async fn export_device_data_from_memory(
    data: &crate::query::QueryResult,
    output_path: &str,
) -> Result<(), String> {
    use std::io::Write;
    
    // 检查是否包含payload_json字段
    let has_payload_json = data.columns.iter().any(|h| h == "payload_json");
    
    // 加载配置
    let config = load_config();
    
    // 如果包含payload_json，需要处理扩展表字段
    let processed_data = if has_payload_json {
        // 使用prepare_for_export处理数据（会提取payload_json中的字段）
        prepare_for_export(&data.rows, &config)
    } else {
        // 没有payload_json，只格式化时间戳和重新排列列顺序
        let filtered: Vec<HashMap<String, Value>> = data.rows.iter()
            .filter_map(|row| row.as_object().map(|obj| {
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            }))
            .collect();
        let formatted = add_formatted_timestamps(filtered);
        reorder_columns(formatted)
    };
    
    if processed_data.is_empty() {
        return Err("No data to export after processing".to_string());
    }
    
    // 收集所有行的所有字段名
    let mut all_fieldnames = std::collections::HashSet::new();
    for row in &processed_data {
        all_fieldnames.extend(row.keys().cloned());
    }
    
    // 构建列名顺序：主表字段（按配置顺序）+ 扩展表字段（按字母顺序）
    let main_fields = &config.main_table_fields;
    let mut fieldnames: Vec<String> = Vec::new();
    
    // 1. 添加主表字段（按配置顺序，只包含实际存在的字段）
    for field in main_fields {
        if all_fieldnames.contains(field) {
            fieldnames.push(field.clone());
        }
    }
    
    // 2. 添加扩展表字段（按字母顺序）
    let mut ext_fields: Vec<String> = all_fieldnames.iter()
        .filter(|f| !main_fields.contains(f))
        .cloned()
        .collect();
    ext_fields.sort();
    fieldnames.extend(ext_fields);
    
    // 创建输出文件并写入UTF-8 BOM
    let mut file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    
    file.write_all(&[0xEF, 0xBB, 0xBF])
        .map_err(|e| format!("Failed to write BOM: {}", e))?;
    
    // 创建CSV写入器
    let mut wtr = Writer::from_writer(file);
    
    // 写入表头
    wtr.write_record(&fieldnames)
        .map_err(|e| format!("Failed to write header: {}", e))?;
    
    // 写入数据
    for row in &processed_data {
        let record: Vec<String> = fieldnames
            .iter()
            .map(|col| {
                let value = row.get(col);
                format_value(value)
            })
            .collect();
        
        wtr.write_record(&record)
            .map_err(|e| format!("Failed to write record: {}", e))?;
    }
    
    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV file: {}", e))?;
    
    Ok(())
}

// 从内存数据导出指令数据：格式化时间戳、保持原始列顺序
async fn export_command_data_from_memory(
    data: &crate::query::QueryResult,
    output_path: &str,
) -> Result<(), String> {
    use std::io::Write;
    
    // 创建输出文件并写入UTF-8 BOM
    let mut file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    
    file.write_all(&[0xEF, 0xBB, 0xBF])
        .map_err(|e| format!("Failed to write BOM: {}", e))?;
    
    // 创建CSV写入器
    let mut wtr = Writer::from_writer(file);
    
    // 写入表头（保持原始顺序）
    wtr.write_record(&data.columns)
        .map_err(|e| format!("Failed to write header: {}", e))?;
    
    // 写入数据行（格式化时间戳字段）
    for row in &data.rows {
        if let Some(obj) = row.as_object() {
            let mut record = Vec::new();
            for col in &data.columns {
                let value = obj.get(col);
                let formatted_value = if col == "timestamp" || col == "local_timestamp" {
                    if let Some(v) = value {
                        if let Some(ts) = v.as_i64() {
                            if col == "timestamp" {
                                format_timestamp(ts, false)
                            } else {
                                format_timestamp(ts, true)
                            }
                        } else {
                            format_value(value)
                        }
                    } else {
                        String::new()
                    }
                } else {
                    format_value(value)
                };
                record.push(formatted_value);
            }
            
            wtr.write_record(&record)
                .map_err(|e| format!("Failed to write record: {}", e))?;
        }
    }
    
    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV file: {}", e))?;
    
    Ok(())
}

// 从内存数据导出宽表：格式化local_timestamp、保持原始列顺序
async fn export_wide_table_from_memory(
    data: &crate::query::QueryResult,
    output_path: &str,
) -> Result<(), String> {
    use std::io::Write;
    
    // 创建输出文件并写入UTF-8 BOM
    let mut file = std::fs::File::create(output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    
    file.write_all(&[0xEF, 0xBB, 0xBF])
        .map_err(|e| format!("Failed to write BOM: {}", e))?;
    
    // 创建CSV写入器
    let mut wtr = Writer::from_writer(file);
    
    // 写入表头（保持原始顺序）
    wtr.write_record(&data.columns)
        .map_err(|e| format!("Failed to write header: {}", e))?;
    
    // 写入数据行（格式化local_timestamp字段）
    for row in &data.rows {
        if let Some(obj) = row.as_object() {
            let mut record = Vec::new();
            for col in &data.columns {
                let value = obj.get(col);
                let formatted_value = if col == "local_timestamp" {
                    if let Some(v) = value {
                        if let Some(ts) = v.as_i64() {
                            format_timestamp(ts, true)
                        } else {
                            format_value(value)
                        }
                    } else {
                        String::new()
                    }
                } else {
                    format_value(value)
                };
                record.push(formatted_value);
            }
            
            wtr.write_record(&record)
                .map_err(|e| format!("Failed to write record: {}", e))?;
        }
    }
    
    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV file: {}", e))?;
    
    Ok(())
}

