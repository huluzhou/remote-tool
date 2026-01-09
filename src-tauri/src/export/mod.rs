use anyhow::Result;
use csv::Writer;
use serde_json::Value;
use chrono::{Utc, TimeZone};

pub async fn export_to_csv(data: Value, file_path: String) -> Result<(), String> {
    let rows = data
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or("Invalid data format")?;
    
    if rows.is_empty() {
        return Err("No data to export".to_string());
    }
    
    // 提取列名
    let columns: Vec<String> = rows[0]
        .as_object()
        .ok_or("Invalid row format")?
        .keys()
        .cloned()
        .collect();
    
    // 创建CSV写入器
    let mut wtr = Writer::from_path(&file_path)
        .map_err(|e| format!("Failed to create CSV file: {}", e))?;
    
    // 写入表头
    wtr.write_record(&columns)
        .map_err(|e| format!("Failed to write header: {}", e))?;
    
    // 写入数据
    for row in rows {
        let obj = row.as_object().ok_or("Invalid row format")?;
        let record: Vec<String> = columns
            .iter()
            .map(|col| {
                let value = obj.get(col);
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

fn format_timestamp(timestamp: i64, is_millis: bool) -> String {
    let dt = if is_millis {
        Utc.timestamp_opt(timestamp / 1000, ((timestamp % 1000) * 1_000_000) as u32)
            .single()
    } else {
        Utc.timestamp_opt(timestamp, 0).single()
    };
    
    match dt {
        Some(dt) => {
            let beijing_tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
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
