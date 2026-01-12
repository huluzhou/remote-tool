use crate::ssh::SshClient;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tauri::AppHandle;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployConfig {
    pub binary_path: Option<String>,
    pub config_path: Option<String>,
    pub topo_path: Option<String>,
    pub upload_binary: Option<bool>,
    pub upload_config: bool,
    pub upload_topo: bool,
    pub use_root: bool,
    pub start_service: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    
    // 辅助函数：从输出中提取结果（处理多行输出和警告）
    fn extract_result(output: &str) -> Option<bool> {
        let lines: Vec<&str> = output.lines().collect();
        // 查找包含 'exists' 或 'not_exists' 的行
        for line in &lines {
            let trimmed = line.trim();
            if trimmed == "exists" {
                return Some(true);
            } else if trimmed == "not_exists" {
                return Some(false);
            }
        }
        // 如果没有找到明确的结果，检查是否包含关键字
        let output_lower = output.to_lowercase();
        if output_lower.contains("exists") && !output_lower.contains("not_exists") {
            Some(true)
        } else if output_lower.contains("not_exists") {
            Some(false)
        } else {
            None
        }
    }
    
    // 辅助函数：从输出中提取状态（处理多行输出，如 'active'/'inactive' 或 'enabled'/'disabled'）
    fn extract_status(output: &str, positive: &str, negative: &str) -> Option<bool> {
        let lines: Vec<&str> = output.lines().collect();
        // 查找包含状态关键字的行
        for line in &lines {
            let trimmed = line.trim();
            if trimmed == positive {
                return Some(true);
            } else if trimmed == negative {
                return Some(false);
            }
        }
        // 如果没有找到明确的结果，检查是否包含关键字
        let output_lower = output.to_lowercase();
        if output_lower.contains(positive) && !output_lower.contains(negative) {
            Some(true)
        } else if output_lower.contains(negative) {
            Some(false)
        } else {
            None
        }
    }
    
    // 检查可执行文件
    let check_binary = format!(
        "test -f {}/bin/{} && echo 'exists' || echo 'not_exists'",
        INSTALL_DIR, BINARY_NAME
    );
    match SshClient::execute_command(&check_binary).await {
        Ok((exit_status, stdout, stderr)) => {
            status.installed = extract_result(&stdout).unwrap_or(false);
            // 调试信息：记录命令执行结果
            eprintln!("[DEBUG] 检查可执行文件: 命令='{}', 退出码={}, stdout='{}', stderr='{}', 提取结果={}, 最终结果={}", 
                check_binary, exit_status, stdout.trim(), stderr.trim(), 
                extract_result(&stdout).map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
                status.installed);
        }
        Err(e) => {
            eprintln!("[DEBUG] 检查可执行文件失败: 命令='{}', 错误='{}'", check_binary, e);
        }
    }
    
    // 检查服务文件（使用sudo，因为/etc/systemd/system需要root权限）
    let check_service_file = format!(
        "sudo test -f {} && echo 'exists' || echo 'not_exists'",
        SERVICE_FILE
    );
    match SshClient::execute_command(&check_service_file).await {
        Ok((exit_status, stdout, stderr)) => {
            status.service_exists = extract_result(&stdout).unwrap_or(false);
            // 调试信息：记录命令执行结果
            eprintln!("[DEBUG] 检查服务文件(sudo): 命令='{}', 退出码={}, stdout='{}', stderr='{}', 提取结果={}, 最终结果={}", 
                check_service_file, exit_status, stdout.trim(), stderr.trim(),
                extract_result(&stdout).map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
                status.service_exists);
        }
        Err(e) => {
            eprintln!("[DEBUG] 检查服务文件(sudo)失败: 命令='{}', 错误='{}'", check_service_file, e);
            // 如果sudo失败，尝试不使用sudo（某些系统可能配置了无密码sudo）
            let check_service_file_no_sudo = format!(
                "test -f {} && echo 'exists' || echo 'not_exists'",
                SERVICE_FILE
            );
            match SshClient::execute_command(&check_service_file_no_sudo).await {
                Ok((exit_status, stdout, stderr)) => {
                    status.service_exists = extract_result(&stdout).unwrap_or(false);
                    eprintln!("[DEBUG] 检查服务文件(无sudo): 命令='{}', 退出码={}, stdout='{}', stderr='{}', 提取结果={}, 最终结果={}", 
                        check_service_file_no_sudo, exit_status, stdout.trim(), stderr.trim(),
                        extract_result(&stdout).map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
                        status.service_exists);
                }
                Err(e2) => {
                    eprintln!("[DEBUG] 检查服务文件(无sudo)失败: 命令='{}', 错误='{}'", check_service_file_no_sudo, e2);
                }
            }
        }
    }
    
    if status.service_exists {
        // 检查服务状态
        let check_active = format!(
            "systemctl is-active {} 2>/dev/null && echo 'active' || echo 'inactive'",
            SERVICE_NAME
        );
        match SshClient::execute_command(&check_active).await {
            Ok((exit_status, stdout, stderr)) => {
                status.service_running = extract_status(&stdout, "active", "inactive").unwrap_or(false);
                // 调试信息：记录命令执行结果
                eprintln!("[DEBUG] 检查服务运行状态: 命令='{}', 退出码={}, stdout='{}', stderr='{}', 提取结果={}, 最终结果={}", 
                    check_active, exit_status, stdout.trim(), stderr.trim(),
                    extract_status(&stdout, "active", "inactive").map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
                    status.service_running);
            }
            Err(e) => {
                eprintln!("[DEBUG] 检查服务运行状态失败: 命令='{}', 错误='{}'", check_active, e);
            }
        }
        
        // 检查服务是否启用
        let check_enabled = format!(
            "systemctl is-enabled {} 2>/dev/null && echo 'enabled' || echo 'disabled'",
            SERVICE_NAME
        );
        match SshClient::execute_command(&check_enabled).await {
            Ok((exit_status, stdout, stderr)) => {
                status.service_enabled = extract_status(&stdout, "enabled", "disabled").unwrap_or(false);
                // 调试信息：记录命令执行结果
                eprintln!("[DEBUG] 检查服务启用状态: 命令='{}', 退出码={}, stdout='{}', stderr='{}', 提取结果={}, 最终结果={}", 
                    check_enabled, exit_status, stdout.trim(), stderr.trim(),
                    extract_status(&stdout, "enabled", "disabled").map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()),
                    status.service_enabled);
            }
            Err(e) => {
                eprintln!("[DEBUG] 检查服务启用状态失败: 命令='{}', 错误='{}'", check_enabled, e);
            }
        }
    } else {
        eprintln!("[DEBUG] 服务文件不存在，跳过服务状态检查");
    }
    
    // 输出最终状态摘要
    eprintln!("[DEBUG] 状态检查完成: installed={}, service_exists={}, service_running={}, service_enabled={}", 
        status.installed, status.service_exists, status.service_running, status.service_enabled);
    
    Ok(status)
}

// 添加时间戳的日志辅助函数（使用 GMT+8 时区）
fn log_with_time(message: &str) -> String {
    use chrono::{Utc, FixedOffset};
    let beijing_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let now = Utc::now().with_timezone(&beijing_tz);
    format!("[{}] {}", now.format("%H:%M:%S"), message)
}

// 添加日志并实时发送事件
fn add_log_and_emit(app_handle: Option<&AppHandle>, logs: &mut Vec<String>, message: &str) {
    let log_message = log_with_time(message);
    logs.push(log_message.clone());
    
    // 如果提供了 AppHandle，实时发送事件
    if let Some(handle) = app_handle {
        use tauri::Emitter;
        let _ = handle.emit("deploy-log", &log_message);
    }
}

// 过滤无害的警告信息
fn filter_benign_warnings(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    
    // 过滤常见的无害 sudo 警告
    let benign_patterns = [
        "unable to resolve host",
        "unable to resolve hostname",
        "sudo: unable to resolve host",
        "sudo: unable to resolve hostname",
    ];
    
    let lower = trimmed.to_lowercase();
    for pattern in &benign_patterns {
        if lower.contains(pattern) {
            return None; // 过滤掉这个警告
        }
    }
    
    Some(trimmed.to_string())
}

// 格式化文件大小
fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.2} KB", size as f64 / 1024.0)
    } else {
        format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
    }
}

pub async fn deploy_application(app_handle: Option<AppHandle>, config: DeployConfig) -> Result<Vec<String>, String> {
    let mut logs = Vec::new();
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    add_log_and_emit(app_handle.as_ref(), &mut logs, "开始部署流程");
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    
    // 显示部署配置
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("部署配置:"));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 上传可执行文件: {}", config.upload_binary.unwrap_or(false)));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 上传配置文件: {}", config.upload_config));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 上传拓扑文件: {}", config.upload_topo));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 运行用户: {}", if config.use_root { "root" } else { SERVICE_USER }));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 部署后启动服务: {}", config.start_service));
    
    // 检查部署状态
    add_log_and_emit(app_handle.as_ref(), &mut logs, "检查当前部署状态...");
    let status = match check_deploy_status().await {
        Ok(s) => {
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  可执行文件: {}", if s.installed { "已安装" } else { "未安装" }));
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务文件: {}", if s.service_exists { "存在" } else { "不存在" }));
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务状态: {}", if s.service_running { "运行中" } else { "未运行" }));
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务启用: {}", if s.service_enabled { "已启用" } else { "未启用" }));
            s
        }
        Err(e) => {
            let err_msg = format!("检查部署状态失败: {}", e);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ {}", err_msg));
            return Err(err_msg);
        }
    };
    
    let is_update = status.installed;
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("部署模式: {}", if is_update { "更新" } else { "新部署" }));
    
    // 判断是否需要重启服务
    // 如果只上传配置文件或拓扑文件，且服务正在运行，需要重启服务以加载新配置
    let need_restart = status.service_running && (
        config.upload_config || 
        config.upload_topo
    );
    
    // 如果是更新（上传可执行文件）或需要重启服务（上传配置文件/拓扑文件），先停止服务
    if (is_update && status.service_running) || need_restart {
        add_log_and_emit(app_handle.as_ref(), &mut logs, "停止现有服务...");
        let stop_cmd = format!("sudo systemctl stop {}", SERVICE_NAME);
        match SshClient::execute_command(&stop_cmd).await {
            Ok((exit_status, stdout, stderr)) => {
                if exit_status == 0 {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已停止");
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                    }
                } else {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 停止服务返回非零退出码: {}", exit_status));
                    if let Some(error) = filter_benign_warnings(&stderr) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  错误: {}", error));
                    }
                }
            }
            Err(e) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ 停止服务失败: {}", e));
            }
        }
    }
    
    // 创建目录结构
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("创建目录结构: {}/bin", INSTALL_DIR));
    let mkdir_cmd = format!("sudo mkdir -p {}/bin", INSTALL_DIR);
    match SshClient::execute_command(&mkdir_cmd).await {
            Ok((exit_status, stdout, stderr)) => {
                if exit_status == 0 {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 目录结构创建成功");
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                    }
                } else {
                    let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                    let err_msg = format!("创建目录失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        Err(e) => {
            let err_msg = format!("创建目录失败: {}", e);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
            return Err(err_msg);
        }
    }
    
    // 上传可执行文件（如果选择）
    if config.upload_binary.unwrap_or(false) {
        if let Some(ref binary_path) = config.binary_path {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "上传可执行文件...");
            
            // 检查本地文件
            match std::fs::metadata(binary_path) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  本地文件: {}", binary_path));
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  文件大小: {}", format_file_size(file_size)));
                }
                Err(e) => {
                    let err_msg = format!("无法读取本地文件 {}: {}", binary_path, e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
            
            let temp_remote = format!("/tmp/{}", BINARY_NAME);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  上传到临时位置: {}", temp_remote));
            
            match SshClient::upload_file(binary_path, &temp_remote).await {
                Ok(_) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 文件上传成功");
                }
                Err(e) => {
                    let err_msg = format!("上传可执行文件失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
            
            // 验证远程文件
            add_log_and_emit(app_handle.as_ref(), &mut logs, "验证远程文件...");
            let verify_cmd = format!("test -f {} && ls -lh {} | awk '{{print $5}}'", temp_remote, temp_remote);
            match SshClient::execute_command(&verify_cmd).await {
                Ok((_, stdout, _)) => {
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  远程文件大小: {}", output));
                    }
                }
                Err(_) => {}
            }
            
            // 使用 rm -f 确保能覆盖已存在的文件，然后移动并设置权限
            add_log_and_emit(app_handle.as_ref(), &mut logs, "部署可执行文件到目标位置...");
            let move_cmd = format!(
                "sudo rm -f '{}/bin/{}' && sudo mv '{}' '{}/bin/{}' && sudo chmod +x '{}/bin/{}' && sudo chown root:root '{}/bin/{}'",
                INSTALL_DIR, BINARY_NAME, temp_remote, INSTALL_DIR, BINARY_NAME, INSTALL_DIR, BINARY_NAME, INSTALL_DIR, BINARY_NAME
            );
            match SshClient::execute_command(&move_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 可执行文件部署成功: {}/bin/{}", INSTALL_DIR, BINARY_NAME));
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                        }
                    } else {
                        let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                        let err_msg = format!("部署可执行文件失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr);
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                        return Err(err_msg);
                    }
                }
                Err(e) => {
                    let err_msg = format!("部署可执行文件失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        } else {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "  ⚠️ 警告: 选择了上传可执行文件但未提供文件路径");
        }
    }
    
    // 上传配置文件（如果选择）
    if config.upload_config {
        if let Some(ref config_path) = config.config_path {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "上传配置文件...");
            
            // 检查本地文件
            match std::fs::metadata(config_path) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  本地文件: {}", config_path));
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  文件大小: {}", format_file_size(file_size)));
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 无法读取本地文件 {}: {}", config_path, e));
                }
            }
            
            let temp_config = "/tmp/config.toml";
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  上传到临时位置: {}", temp_config));
            
            match SshClient::upload_file(config_path, temp_config).await {
                Ok(_) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 文件上传成功");
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ 上传配置文件失败: {}", e));
                }
            }
            
            // 使用 rm -f 确保能覆盖已存在的文件
            add_log_and_emit(app_handle.as_ref(), &mut logs, "部署配置文件到目标位置...");
            let move_config_cmd = format!(
                "sudo rm -f '{}/config.toml' && sudo mv '{}' '{}/config.toml' && sudo chmod 644 '{}/config.toml'",
                INSTALL_DIR, temp_config, INSTALL_DIR, INSTALL_DIR
            );
            match SshClient::execute_command(&move_config_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 配置文件部署成功: {}/config.toml", INSTALL_DIR));
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                        }
                    } else {
                        let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                        if !filtered_stderr.is_empty() {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 配置文件部署失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr));
                        }
                    }
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 配置文件部署失败: {}", e));
                }
            }
        } else {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "  ⚠️ 警告: 选择了上传配置文件但未提供文件路径");
        }
    }
    
    // 上传拓扑文件（如果选择）
    if config.upload_topo {
        if let Some(ref topo_path) = config.topo_path {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "上传拓扑文件...");
            
            // 检查本地文件
            match std::fs::metadata(topo_path) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  本地文件: {}", topo_path));
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  文件大小: {}", format_file_size(file_size)));
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 无法读取本地文件 {}: {}", topo_path, e));
                }
            }
            
            let temp_topo = "/tmp/topo.json";
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  上传到临时位置: {}", temp_topo));
            
            match SshClient::upload_file(topo_path, temp_topo).await {
                Ok(_) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 文件上传成功");
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ 上传拓扑文件失败: {}", e));
                }
            }
            
            // 使用 rm -f 确保能覆盖已存在的文件
            add_log_and_emit(app_handle.as_ref(), &mut logs, "部署拓扑文件到目标位置...");
            let move_topo_cmd = format!(
                "sudo rm -f '{}/topo.json' && sudo mv '{}' '{}/topo.json' && sudo chmod 644 '{}/topo.json'",
                INSTALL_DIR, temp_topo, INSTALL_DIR, INSTALL_DIR
            );
            match SshClient::execute_command(&move_topo_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 拓扑文件部署成功: {}/topo.json", INSTALL_DIR));
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                        }
                    } else {
                        let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                        if !filtered_stderr.is_empty() {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 拓扑文件部署失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr));
                        }
                    }
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 拓扑文件部署失败: {}", e));
                }
            }
        } else {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "  ⚠️ 警告: 选择了上传拓扑文件但未提供文件路径");
        }
    }
    
    // 设置权限
    add_log_and_emit(app_handle.as_ref(), &mut logs, "设置权限...");
    if !config.use_root {
        // 创建用户
        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("创建运行用户: {}", SERVICE_USER));
        let create_user_cmd = format!(
            "id {} 2>/dev/null || sudo useradd -r -s /bin/false {}",
            SERVICE_USER, SERVICE_USER
        );
        match SshClient::execute_command(&create_user_cmd).await {
            Ok((exit_status, stdout, stderr)) => {
                if exit_status == 0 {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 用户 {} 已存在或创建成功", SERVICE_USER));
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                    }
                } else {
                    if let Some(error) = filter_benign_warnings(&stderr) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 创建用户失败: 退出码 {}, 错误: {}", exit_status, error));
                    }
                }
            }
            Err(e) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 创建用户失败: {}", e));
            }
        }
        
        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("设置目录所有者: {}:{}", SERVICE_USER, SERVICE_USER));
        let chown_cmd = format!(
            "sudo chown -R {}:{} {}",
            SERVICE_USER, SERVICE_USER, INSTALL_DIR
        );
        match SshClient::execute_command(&chown_cmd).await {
            Ok((exit_status, stdout, stderr)) => {
                if exit_status == 0 {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 权限设置成功");
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                    }
                } else {
                    if let Some(error) = filter_benign_warnings(&stderr) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 权限设置失败: 退出码 {}, 错误: {}", exit_status, error));
                    }
                }
            }
            Err(e) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 权限设置失败: {}", e));
            }
        }
    } else {
        add_log_and_emit(app_handle.as_ref(), &mut logs, "使用 root 用户运行，跳过权限设置");
    }
    
    // 创建服务文件
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("创建服务文件: {}", SERVICE_FILE));
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
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "生成服务文件内容...");
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  工作目录: {}", INSTALL_DIR));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  可执行文件: {}/bin/{}", INSTALL_DIR, BINARY_NAME));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  配置文件: {}/config.toml", INSTALL_DIR));
    
    let temp_service = "/tmp/analysis-collector.service";
    match std::fs::write(temp_service, &service_content) {
        Ok(_) => {
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 临时服务文件已创建: {}", temp_service));
        }
        Err(e) => {
            let err_msg = format!("创建临时服务文件失败: {}", e);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
            return Err(err_msg);
        }
    }
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "上传服务文件...");
    match SshClient::upload_file(temp_service, temp_service).await {
        Ok(_) => {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务文件上传成功");
        }
        Err(e) => {
            let err_msg = format!("上传服务文件失败: {}", e);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
            return Err(err_msg);
        }
    }
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "部署服务文件并重新加载 systemd...");
    let move_service_cmd = format!(
        "sudo mv '{}' '{}' && sudo systemctl daemon-reload",
        temp_service, SERVICE_FILE
    );
    match SshClient::execute_command(&move_service_cmd).await {
            Ok((exit_status, stdout, stderr)) => {
                if exit_status == 0 {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 服务文件部署成功: {}", SERVICE_FILE));
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ systemd 已重新加载");
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                    }
                } else {
                    let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                    let err_msg = format!("创建服务文件失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        Err(e) => {
            let err_msg = format!("创建服务文件失败: {}", e);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
            return Err(err_msg);
        }
    }
    
    // 启用并启动/重启服务
    // 如果需要重启服务（上传配置文件或拓扑文件），即使 start_service 为 false 也要重启
    if config.start_service || need_restart {
        if need_restart && !config.start_service {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "检测到配置文件或拓扑文件更新，需要重启服务以加载新配置...");
        }
        
        add_log_and_emit(app_handle.as_ref(), &mut logs, "启用服务...");
        let enable_cmd = format!("sudo systemctl enable {}", SERVICE_NAME);
        match SshClient::execute_command(&enable_cmd).await {
            Ok((exit_status, stdout, stderr)) => {
                if exit_status == 0 {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已启用");
                    if let Some(output) = filter_benign_warnings(&stdout) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                    }
                } else {
                    if let Some(error) = filter_benign_warnings(&stderr) {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 启用服务失败: 退出码 {}, 错误: {}", exit_status, error));
                    }
                }
            }
            Err(e) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 启用服务失败: {}", e));
            }
        }
        
        // 如果服务之前已经在运行（需要重启），使用 restart；否则使用 start
        if need_restart {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "重启服务以加载新配置...");
            let restart_cmd = format!("sudo systemctl restart {}", SERVICE_NAME);
            match SshClient::execute_command(&restart_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已重启");
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                        }
                        
                        // 验证服务状态
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "验证服务状态...");
                        let status_cmd = format!("sudo systemctl status {} --no-pager -l", SERVICE_NAME);
                        match SshClient::execute_command(&status_cmd).await {
                            Ok((_, status_output, _)) => {
                                if let Some(output) = filter_benign_warnings(&status_output) {
                                    let status_lines: Vec<&str> = output.lines().take(3).collect();
                                    for line in status_lines {
                                        if !line.trim().is_empty() {
                                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  {}", line));
                                        }
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    } else {
                        let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                        let err_msg = format!("重启服务失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr);
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                        return Err(err_msg);
                    }
                }
                Err(e) => {
                    let err_msg = format!("重启服务失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        } else {
            add_log_and_emit(app_handle.as_ref(), &mut logs, "启动服务...");
            let start_cmd = format!("sudo systemctl start {}", SERVICE_NAME);
            match SshClient::execute_command(&start_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已启动");
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                        }
                        
                        // 验证服务状态
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "验证服务状态...");
                        let status_cmd = format!("sudo systemctl status {} --no-pager -l", SERVICE_NAME);
                        match SshClient::execute_command(&status_cmd).await {
                            Ok((_, status_output, _)) => {
                                if let Some(output) = filter_benign_warnings(&status_output) {
                                    let status_lines: Vec<&str> = output.lines().take(3).collect();
                                    for line in status_lines {
                                        if !line.trim().is_empty() {
                                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  {}", line));
                                        }
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    } else {
                        let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                        let err_msg = format!("启动服务失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr);
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                        return Err(err_msg);
                    }
                }
                Err(e) => {
                    let err_msg = format!("启动服务失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        }
    } else {
        add_log_and_emit(app_handle.as_ref(), &mut logs, "跳过服务启动（未选择启动服务选项）");
    }
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("{}完成！", if is_update { "更新" } else { "部署" }));
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    
    Ok(logs)
}
