use crate::ssh::{SshClient, SshConfig};
use crate::query::{QueryParams, QueryResult};
use crate::export;
use crate::deploy::{DeployConfig, DeployStatus};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConfigDto {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_file: Option<String>,
}

#[tauri::command]
pub async fn ssh_connect(config: SshConfigDto) -> Result<serde_json::Value, String> {
    let ssh_config = SshConfig {
        host: config.host.clone(),
        port: config.port,
        username: config.username.clone(),
        password: config.password.clone(),
        key_file: config.key_file.clone(),
    };

    match SshClient::connect(ssh_config).await {
        Ok(_) => Ok(serde_json::json!({ "success": true })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

#[tauri::command]
pub async fn ssh_disconnect() -> Result<(), String> {
    SshClient::disconnect().await;
    Ok(())
}

#[tauri::command]
pub async fn execute_query(
    app: tauri::AppHandle,
    params: QueryParams,
) -> Result<QueryResult, String> {
    crate::query::execute_query(params, Some(app)).await
}

#[tauri::command]
pub async fn export_to_csv(
    data: serde_json::Value,
    file_path: String,
    query_type: Option<String>,
) -> Result<(), String> {
    // 将JSON值反序列化为QueryResult
    let query_result: QueryResult = serde_json::from_value(data)
        .map_err(|e| format!("解析查询结果失败: {}", e))?;
    
    export::export_to_csv(query_result, file_path, query_type).await
}

#[tauri::command]
pub async fn check_deploy_status() -> Result<DeployStatus, String> {
    crate::deploy::check_deploy_status().await
}

#[tauri::command]
pub async fn deploy_application(
    app: tauri::AppHandle,
    config: DeployConfig,
) -> Result<serde_json::Value, String> {
    match crate::deploy::deploy_application(Some(app), config).await {
        Ok(logs) => Ok(serde_json::json!({
            "success": true,
            "logs": logs
        })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string(),
            "logs": Vec::<String>::new()
        })),
    }
}
