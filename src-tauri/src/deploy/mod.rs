use crate::ssh::SshClient;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tauri::AppHandle;
use tempfile::NamedTempFile;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployFile {
    pub local_path: Option<String>,
    pub remote_path: Option<String>,
    pub download_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployConfig {
    pub files: Vec<DeployFile>,
    pub use_root: bool,
    pub restart_service: bool,
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
const SERVICE_NAME: &str = "ancol";
const BINARY_NAME: &str = "ancol";
const SERVICE_FILE: &str = "/etc/systemd/system/ancol.service";
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
    
    // 判断操作类型
    let upload_files: Vec<&DeployFile> = config.files.iter()
        .filter(|f| f.local_path.is_some() && f.remote_path.is_some())
        .collect();
    let download_files: Vec<&DeployFile> = config.files.iter()
        .filter(|f| f.remote_path.is_some() && f.download_path.is_some())
        .collect();
    
    let operation_type = if !upload_files.is_empty() {
        "上传"
    } else if !download_files.is_empty() {
        "下载"
    } else if config.restart_service {
        "重启服务"
    } else {
        return Err("无效的操作：没有选择上传、下载或重启服务".to_string());
    };
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("开始{}流程", operation_type));
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    
    // 显示操作配置
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("操作配置:"));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 操作类型: {}", operation_type));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 上传文件数: {}", upload_files.len()));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 下载文件数: {}", download_files.len()));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 运行用户: {}", if config.use_root { "root" } else { SERVICE_USER }));
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  - 重启服务: {}", config.restart_service));
    
    // 检查部署状态（仅在上传或重启服务时检查）
    let status = if !upload_files.is_empty() || config.restart_service {
        add_log_and_emit(app_handle.as_ref(), &mut logs, "检查当前部署状态...");
        match check_deploy_status().await {
            Ok(s) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  可执行文件: {}", if s.installed { "已安装" } else { "未安装" }));
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务文件: {}", if s.service_exists { "存在" } else { "不存在" }));
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务状态: {}", if s.service_running { "运行中" } else { "未运行" }));
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务启用: {}", if s.service_enabled { "已启用" } else { "未启用" }));
                Some(s)
            }
            Err(e) => {
                let err_msg = format!("检查部署状态失败: {}", e);
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ {}", err_msg));
                return Err(err_msg);
            }
        }
    } else {
        None
    };
    
    // 如果是上传操作且服务正在运行，先停止服务
    if !upload_files.is_empty() {
        if let Some(ref status) = status {
            if status.service_running {
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
        }
    }
    
    // 处理文件上传
    if !upload_files.is_empty() {
        // 创建目录结构（如果需要）
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
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ {}", err_msg));
                }
            }
            Err(e) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 创建目录失败: {}", e));
            }
        }
        
        // 上传每个文件
        for (idx, file) in upload_files.iter().enumerate() {
            let local_path = file.local_path.as_ref().unwrap();
            let remote_path = file.remote_path.as_ref().unwrap();
            
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("上传文件 {}/{}: {}", idx + 1, upload_files.len(), remote_path));
            
            // 检查本地文件
            match std::fs::metadata(local_path) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  本地文件: {}", local_path));
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  文件大小: {}", format_file_size(file_size)));
                }
                Err(e) => {
                    let err_msg = format!("无法读取本地文件 {}: {}", local_path, e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
            
            // 生成临时文件名
            let file_name = remote_path.split('/').last().unwrap_or("file");
            let temp_remote = format!("/tmp/{}", file_name);
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  上传到临时位置: {}", temp_remote));
            
            // 上传文件
            match SshClient::upload_file(local_path, &temp_remote).await {
                Ok(_) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 文件上传成功");
                }
                Err(e) => {
                    let err_msg = format!("上传文件失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
            
            // 移动到目标位置并设置权限
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("部署文件到目标位置: {}", remote_path));
            
            // 获取目标目录
            let remote_dir = remote_path.rsplit('/').skip(1).collect::<Vec<&str>>().join("/");
            let mkdir_cmd = if !remote_dir.is_empty() {
                format!("sudo mkdir -p '{}'", remote_dir)
            } else {
                String::new()
            };
            
            if !mkdir_cmd.is_empty() {
                match SshClient::execute_command(&mkdir_cmd).await {
                    Ok(_) => {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 目录创建成功"));
                    }
                    Err(e) => {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 创建目录失败: {}", e));
                    }
                }
            }
            
            // 判断文件类型，设置不同的权限
            let is_binary = remote_path.contains("/bin/") || !remote_path.ends_with(".toml") && !remote_path.ends_with(".json");
            let move_cmd = if is_binary {
                format!(
                    "sudo rm -f '{}' && sudo mv '{}' '{}' && sudo chmod +x '{}' && sudo chown root:root '{}'",
                    remote_path, temp_remote, remote_path, remote_path, remote_path
                )
            } else {
                format!(
                    "sudo rm -f '{}' && sudo mv '{}' '{}' && sudo chmod 644 '{}'",
                    remote_path, temp_remote, remote_path, remote_path
                )
            };
            
            match SshClient::execute_command(&move_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 文件部署成功: {}", remote_path));
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
                        }
                    } else {
                        let filtered_stderr = filter_benign_warnings(&stderr).unwrap_or_else(|| stderr.trim().to_string());
                        let err_msg = format!("部署文件失败: 退出码 {}, 错误: {}", exit_status, filtered_stderr);
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                        return Err(err_msg);
                    }
                }
                Err(e) => {
                    let err_msg = format!("部署文件失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        }
    }
    
    // 处理文件下载
    if !download_files.is_empty() {
        for (idx, file) in download_files.iter().enumerate() {
            let remote_path = file.remote_path.as_ref().unwrap();
            let download_path = file.download_path.as_ref().unwrap();
            
            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("下载文件 {}/{}: {} -> {}", idx + 1, download_files.len(), remote_path, download_path));
            
            // 检查远程文件是否存在
            let check_cmd = format!("test -f '{}' && echo 'exists' || echo 'not_exists'", remote_path);
            match SshClient::execute_command(&check_cmd).await {
                Ok((_, stdout, _)) => {
                    if stdout.trim() == "exists" {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 远程文件存在");
                    } else {
                        let err_msg = format!("远程文件不存在: {}", remote_path);
                        add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                        return Err(err_msg);
                    }
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 检查远程文件失败: {}", e));
                }
            }
            
            // 下载文件
            match SshClient::download_file(remote_path, download_path).await {
                Ok(_) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 文件下载成功: {}", download_path));
                }
                Err(e) => {
                    let err_msg = format!("下载文件失败: {}", e);
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                    return Err(err_msg);
                }
            }
        }
    }
    
    // 设置权限（仅在上传文件时设置）
    if !upload_files.is_empty() {
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
        
        // 创建服务文件（仅在上传文件时创建）
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
        
        // 使用 tempfile 创建跨平台临时文件
        let temp_service_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(e) => {
                let err_msg = format!("创建临时服务文件失败: {}", e);
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                return Err(err_msg);
            }
        };
        
        let temp_service_path = temp_service_file.path().to_string_lossy().to_string();
        
        match std::fs::write(&temp_service_path, &service_content) {
            Ok(_) => {
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✓ 临时服务文件已创建: {}", temp_service_path));
            }
            Err(e) => {
                let err_msg = format!("写入临时服务文件失败: {}", e);
                add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ✗ {}", err_msg));
                return Err(err_msg);
            }
        }
        
        // 远程临时文件路径
        let temp_remote = "/tmp/analysis-collector.service";
        
        add_log_and_emit(app_handle.as_ref(), &mut logs, "上传服务文件...");
        match SshClient::upload_file(&temp_service_path, temp_remote).await {
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
            temp_remote, SERVICE_FILE
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
    }
    
    // 处理服务重启
    if config.restart_service {
        // 获取服务状态用于重启
        let service_running = if let Some(ref s) = status {
            s.service_running
        } else {
            // 如果没有状态，重新检查
            add_log_and_emit(app_handle.as_ref(), &mut logs, "检查服务状态...");
            match check_deploy_status().await {
                Ok(s) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  服务状态: {}", if s.service_running { "运行中" } else { "未运行" }));
                    s.service_running
                }
                Err(e) => {
                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  ⚠️ 检查服务状态失败: {}，尝试直接重启服务", e));
                    // 直接尝试重启服务
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "重启服务...");
                    let restart_cmd = format!("sudo systemctl restart {}", SERVICE_NAME);
                    match SshClient::execute_command(&restart_cmd).await {
                        Ok((exit_status, stdout, stderr)) => {
                            if exit_status == 0 {
                                add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已重启");
                                if let Some(output) = filter_benign_warnings(&stdout) {
                                    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
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
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "重启服务完成！");
                    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
                    return Ok(logs);
                }
            }
        };
        
        if service_running {
            // 服务正在运行，执行重启
            add_log_and_emit(app_handle.as_ref(), &mut logs, "重启服务...");
            let restart_cmd = format!("sudo systemctl restart {}", SERVICE_NAME);
            match SshClient::execute_command(&restart_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已重启");
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
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
            // 服务未运行，执行启动
            add_log_and_emit(app_handle.as_ref(), &mut logs, "启动服务...");
            let start_cmd = format!("sudo systemctl start {}", SERVICE_NAME);
            match SshClient::execute_command(&start_cmd).await {
                Ok((exit_status, stdout, stderr)) => {
                    if exit_status == 0 {
                        add_log_and_emit(app_handle.as_ref(), &mut logs, "  ✓ 服务已启动");
                        if let Some(output) = filter_benign_warnings(&stdout) {
                            add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("  输出: {}", output));
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
        
        // 启用服务
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
    }
    
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    add_log_and_emit(app_handle.as_ref(), &mut logs, &format!("{}完成！", operation_type));
    add_log_and_emit(app_handle.as_ref(), &mut logs, "=========================================");
    
    Ok(logs)
}

