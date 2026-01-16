use csv::Writer;
use serde_json::Value;
use chrono::{Utc, TimeZone, FixedOffset};

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

// 主导出函数（从内存数据直接导出到CSV文件，仅支持宽表）
pub async fn export_to_csv(
    data: crate::query::QueryResult,
    output_path: String,
    _query_type: Option<String>,
) -> Result<(), String> {
    // 只支持宽表导出
    export_wide_table_from_memory(&data, &output_path).await
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

