use crate::ssh::SshClient;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub db_path: String,
    pub start_time: i64,
    pub end_time: i64,
    pub device_sn: Option<String>,
    pub include_ext: Option<bool>,
    pub query_type: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub total_rows: usize,
}

pub async fn execute_query(params: QueryParams) -> Result<QueryResult, String> {
    let sql = build_query_sql(&params)?;
    
    // 通过SSH执行Python脚本查询数据库
    let python_script = format!(
        r#"
import sqlite3
import json
import sys
import base64

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
    
    print(json.dumps(results, ensure_ascii=False, default=str))
    
    conn.close()
    sys.exit(0)
except Exception as e:
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
"#,
        general_purpose::STANDARD.encode(&params.db_path),
        general_purpose::STANDARD.encode(&sql)
    );

    let command = format!("python3 << 'EOF'\n{}\nEOF", python_script);
    
    match SshClient::execute_command(&command).await {
        Ok((exit_status, stdout, stderr)) => {
            if exit_status != 0 {
                return Err(format!("Query failed: {}", stderr));
            }
            
            let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
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
            
            Ok(QueryResult {
                columns,
                rows: results,
                total_rows: results.len(),
            })
        }
        Err(e) => Err(format!("SSH command failed: {}", e)),
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
            // 宽表查询需要特殊处理，这里简化实现
            // 实际应该调用专门的宽表查询逻辑
            Err("Wide table query not fully implemented yet".to_string())
        }
        _ => Err(format!("Unknown query type: {}", params.query_type)),
    }
}
