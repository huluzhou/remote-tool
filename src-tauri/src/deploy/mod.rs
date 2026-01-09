use crate::ssh::SshClient;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct DeployConfig {
    pub binary_path: String,
    pub config_path: Option<String>,
    pub topo_path: Option<String>,
    pub upload_config: bool,
    pub upload_topo: bool,
    pub use_root: bool,
    pub start_service: bool,
}

#[derive(Debug, Serialize)]
pub struct DeployStatus {
    pub installed: bool,
    pub service_exists: bool,
    pub service_running: bool,
    pub service_enabled: bool,
}

const INSTALL_DIR: &str = "/opt/analysis";
const SERVICE_NAME: &str = "analysis-collector";
const BINARY_NAME: &str = "analysis-collector";
const SERVICE_FILE: &str = "/etc/systemd/system/analysis-collector.service";
const SERVICE_USER: &str = "analysis";

pub async fn check_deploy_status() -> Result<DeployStatus, String> {
    let mut status = DeployStatus {
        installed: false,
        service_exists: false,
        service_running: false,
        service_enabled: false,
    };
    
    // 检查可执行文件
    let check_binary = format!(
        "test -f {}/bin/{} && echo 'exists' || echo 'not_exists'",
        INSTALL_DIR, BINARY_NAME
    );
    match SshClient::execute_command(&check_binary).await {
        Ok((_, stdout, _)) => {
            status.installed = stdout.trim() == "exists";
        }
        Err(_) => {}
    }
    
    // 检查服务文件
    let check_service_file = format!(
        "test -f {} && echo 'exists' || echo 'not_exists'",
        SERVICE_FILE
    );
    match SshClient::execute_command(&check_service_file).await {
        Ok((_, stdout, _)) => {
            status.service_exists = stdout.trim() == "exists";
        }
        Err(_) => {}
    }
    
    if status.service_exists {
        // 检查服务状态
        let check_active = format!(
            "systemctl is-active {} 2>/dev/null && echo 'active' || echo 'inactive'",
            SERVICE_NAME
        );
        match SshClient::execute_command(&check_active).await {
            Ok((_, stdout, _)) => {
                status.service_running = stdout.trim() == "active";
            }
            Err(_) => {}
        }
        
        // 检查服务是否启用
        let check_enabled = format!(
            "systemctl is-enabled {} 2>/dev/null && echo 'enabled' || echo 'disabled'",
            SERVICE_NAME
        );
        match SshClient::execute_command(&check_enabled).await {
            Ok((_, stdout, _)) => {
                status.service_enabled = stdout.trim() == "enabled";
            }
            Err(_) => {}
        }
    }
    
    Ok(status)
}

pub async fn deploy_application(config: DeployConfig) -> Result<Vec<String>, String> {
    let mut logs = Vec::new();
    
    // 检查部署状态
    let status = check_deploy_status().await.map_err(|e| e.to_string())?;
    let is_update = status.installed;
    
    logs.push(format!("部署模式: {}", if is_update { "更新" } else { "新部署" }));
    
    // 如果是更新，先停止服务
    if is_update && status.service_running {
        logs.push("停止现有服务...".to_string());
        let stop_cmd = format!("sudo systemctl stop {}", SERVICE_NAME);
        match SshClient::execute_command(&stop_cmd).await {
            Ok(_) => logs.push("服务已停止".to_string()),
            Err(e) => logs.push(format!("警告: 停止服务失败: {}", e)),
        }
    }
    
    // 创建目录结构
    logs.push("创建目录结构...".to_string());
    let mkdir_cmd = format!("sudo mkdir -p {}/bin", INSTALL_DIR);
    match SshClient::execute_command(&mkdir_cmd).await {
        Ok(_) => logs.push("目录结构创建成功".to_string()),
        Err(e) => return Err(format!("创建目录失败: {}", e)),
    }
    
    // 上传可执行文件
    logs.push("上传可执行文件...".to_string());
    let temp_remote = format!("/tmp/{}", BINARY_NAME);
    SshClient::upload_file(&config.binary_path, &temp_remote)
        .await
        .map_err(|e| format!("上传可执行文件失败: {}", e))?;
    
    let move_cmd = format!(
        "sudo rm -f '{}/bin/{}' && sudo mv '{}' '{}/bin/{}' && sudo chmod +x '{}/bin/{}' && sudo chown root:root '{}/bin/{}'",
        INSTALL_DIR, BINARY_NAME, temp_remote, INSTALL_DIR, BINARY_NAME, INSTALL_DIR, BINARY_NAME, INSTALL_DIR, BINARY_NAME
    );
    match SshClient::execute_command(&move_cmd).await {
        Ok(_) => logs.push("可执行文件部署成功".to_string()),
        Err(e) => return Err(format!("部署可执行文件失败: {}", e)),
    }
    
    // 上传配置文件
    if config.upload_config {
        if let Some(ref config_path) = config.config_path {
            logs.push("上传配置文件...".to_string());
            let temp_config = "/tmp/config.toml";
            SshClient::upload_file(config_path, temp_config)
                .await
                .map_err(|e| format!("上传配置文件失败: {}", e))?;
            
            let move_config_cmd = format!(
                "sudo mv '{}' '{}/config.toml' && sudo chmod 644 '{}/config.toml'",
                temp_config, INSTALL_DIR, INSTALL_DIR
            );
            match SshClient::execute_command(&move_config_cmd).await {
                Ok(_) => logs.push("配置文件上传成功".to_string()),
                Err(e) => logs.push(format!("警告: 配置文件上传失败: {}", e)),
            }
        }
    }
    
    // 上传拓扑文件
    if config.upload_topo {
        if let Some(ref topo_path) = config.topo_path {
            logs.push("上传拓扑文件...".to_string());
            let temp_topo = "/tmp/topo.json";
            SshClient::upload_file(topo_path, temp_topo)
                .await
                .map_err(|e| format!("上传拓扑文件失败: {}", e))?;
            
            let move_topo_cmd = format!(
                "sudo mv '{}' '{}/topo.json' && sudo chmod 644 '{}/topo.json'",
                temp_topo, INSTALL_DIR, INSTALL_DIR
            );
            match SshClient::execute_command(&move_topo_cmd).await {
                Ok(_) => logs.push("拓扑文件上传成功".to_string()),
                Err(e) => logs.push(format!("警告: 拓扑文件上传失败: {}", e)),
            }
        }
    }
    
    // 设置权限
    logs.push("设置权限...".to_string());
    if !config.use_root {
        // 创建用户
        let create_user_cmd = format!(
            "id {} 2>/dev/null || sudo useradd -r -s /bin/false {}",
            SERVICE_USER, SERVICE_USER
        );
        SshClient::execute_command(&create_user_cmd).await.ok();
        
        let chown_cmd = format!(
            "sudo chown -R {}:{} {}",
            SERVICE_USER, SERVICE_USER, INSTALL_DIR
        );
        SshClient::execute_command(&chown_cmd).await.ok();
    }
    
    // 创建服务文件
    logs.push("创建服务文件...".to_string());
    let service_content = if config.use_root {
        format!(
            r#"[Unit]
Description=Analysis Data Collector
After=network.target

[Service]
Type=simple
WorkingDirectory={}
ExecStart={}/bin/{} --config {}/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target"#,
            INSTALL_DIR, INSTALL_DIR, BINARY_NAME, INSTALL_DIR
        )
    } else {
        format!(
            r#"[Unit]
Description=Analysis Data Collector
After=network.target

[Service]
Type=simple
User={}
WorkingDirectory={}
ExecStart={}/bin/{} --config {}/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target"#,
            SERVICE_USER, INSTALL_DIR, INSTALL_DIR, BINARY_NAME, INSTALL_DIR
        )
    };
    
    let temp_service = "/tmp/analysis-collector.service";
    std::fs::write(temp_service, &service_content)
        .map_err(|e| format!("创建临时服务文件失败: {}", e))?;
    
    SshClient::upload_file(temp_service, temp_service)
        .await
        .map_err(|e| format!("上传服务文件失败: {}", e))?;
    
    let move_service_cmd = format!(
        "sudo mv '{}' '{}' && sudo systemctl daemon-reload",
        temp_service, SERVICE_FILE
    );
    match SshClient::execute_command(&move_service_cmd).await {
        Ok(_) => logs.push("服务文件创建成功".to_string()),
        Err(e) => return Err(format!("创建服务文件失败: {}", e)),
    }
    
    // 启用并启动服务
    if config.start_service {
        logs.push("启用服务...".to_string());
        let enable_cmd = format!("sudo systemctl enable {}", SERVICE_NAME);
        SshClient::execute_command(&enable_cmd).await.ok();
        
        let start_cmd = format!("sudo systemctl start {}", SERVICE_NAME);
        match SshClient::execute_command(&start_cmd).await {
            Ok(_) => logs.push("服务已启用并启动成功".to_string()),
            Err(e) => return Err(format!("启动服务失败: {}", e)),
        }
    }
    
    logs.push(format!("{}完成！", if is_update { "更新" } else { "部署" }));
    
    Ok(logs)
}
