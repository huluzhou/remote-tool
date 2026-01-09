use async_ssh2_tokio::{Client, AuthMethod, ServerCheckMethod};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_file: Option<String>,
}

pub struct SshClient;

static SSH_CLIENT: Mutex<Option<Arc<Client>>> = Mutex::new(None);
static SSH_CONFIG: Mutex<Option<SshConfig>> = Mutex::new(None);

impl SshClient {
    /// 连接到 SSH 服务器（类似 paramiko 的连接方式，针对 JumpServer 优化）
    pub async fn connect(config: SshConfig) -> Result<()> {
        let addr = (&config.host[..], config.port);
        
        // 尝试使用密钥文件认证（类似 paramiko 的 key_filename）
        // 注意：async-ssh2-tokio 默认只使用指定的认证方法，相当于 paramiko 的
        // look_for_keys=False（不自动查找密钥）和 allow_agent=False（不使用 SSH 代理）
        if let Some(ref key_file) = config.key_file {
            if std::path::Path::new(key_file).exists() {
                let auth = AuthMethod::with_key_file(key_file, None);
                match Client::connect(
                    addr,
                    &config.username,
                    auth,
                    ServerCheckMethod::NoCheck, // 自动接受服务器密钥（类似 AutoAddPolicy）
                )
                    .await
                {
                    Ok(client) => {
                        // 密钥认证成功
                        let client_arc = Arc::new(client);
                        *SSH_CLIENT.lock().unwrap() = Some(client_arc);
                        *SSH_CONFIG.lock().unwrap() = Some(config);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("密钥认证失败: {}, 尝试密码认证", e);
                        // 继续尝试密码
                    }
                }
            }
        }
        
        // 使用密码认证（类似 paramiko 的 password）
        // 注意：AuthMethod::with_password() 只使用密码认证，不会尝试密钥或代理
        // 这相当于 paramiko 的 look_for_keys=False 和 allow_agent=False（对 JumpServer 很重要）
        if let Some(ref password) = config.password {
            let auth = AuthMethod::with_password(password);
            let client = Client::connect(
                addr,
                &config.username,
                auth,
                ServerCheckMethod::NoCheck, // 自动接受服务器密钥（类似 AutoAddPolicy）
            )
                .await
            .with_context(|| format!("SSH 连接失败: {}@{}:{}", config.username, config.host, config.port))?;
            
                let client_arc = Arc::new(client);
                *SSH_CLIENT.lock().unwrap() = Some(client_arc);
            *SSH_CONFIG.lock().unwrap() = Some(config);
                Ok(())
        } else {
            anyhow::bail!(
                "缺少认证信息\n\n\
                服务器: {}:{}\n\
                用户名: {}\n\n\
                请提供密码或密钥文件",
                config.host, config.port, config.username
            )
        }
    }

    /// 断开 SSH 连接
    pub async fn disconnect() {
        if let Some(client) = SSH_CLIENT.lock().unwrap().take() {
            // Client 在 Drop 时会自动关闭连接
            drop(client);
        }
        *SSH_CONFIG.lock().unwrap() = None;
    }

    /// 获取 SSH 客户端
    pub fn get_client() -> Result<Arc<Client>> {
        SSH_CLIENT
            .lock()
            .unwrap()
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未连接，请先调用 connect()"))
            .map(|c| c.clone())
    }

    /// 执行 SSH 命令（类似 paramiko 的 exec_command）
    pub async fn execute_command(command: &str) -> Result<(i32, String, String)> {
        let client = Self::get_client()?;
        
        // 执行命令（async-ssh2-tokio 提供了便捷的 execute 方法）
        let result = client
            .execute(command)
            .await
            .with_context(|| format!("执行命令失败: {}", command))?;
        
        Ok((
            result.exit_status as i32,
            result.stdout,
            result.stderr,
        ))
    }

    /// 上传文件到远程服务器（使用 SFTP，类似 paramiko 的 put）
    pub async fn upload_file(local_path: &str, remote_path: &str) -> Result<()> {
        let client = Self::get_client()?;
        
        // upload_file(本地路径, 远程路径, 权限, 块大小, 是否覆盖)
        client
            .upload_file(local_path, remote_path, None, None, true)
            .await
            .with_context(|| format!("上传文件失败: {} -> {}", local_path, remote_path))?;
        
        Ok(())
    }

    /// 从远程服务器下载文件（使用 SFTP，类似 paramiko 的 get）
    pub async fn download_file(remote_path: &str, local_path: &str) -> Result<()> {
        let client = Self::get_client()?;
        
        // download_file(远程路径, 本地路径)
        client
            .download_file(remote_path, local_path)
            .await
            .with_context(|| format!("下载文件失败: {} -> {}", remote_path, local_path))?;
        
        Ok(())
    }
}
