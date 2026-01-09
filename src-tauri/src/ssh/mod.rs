use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};

pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_file: Option<String>,
}

pub struct SshClient {
    session: Arc<Mutex<Session>>,
}

static SSH_SESSION: Mutex<Option<Arc<Mutex<Session>>>> = Mutex::new(None);

impl SshClient {
    pub async fn connect(config: SshConfig) -> Result<()> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr)
            .with_context(|| format!("Failed to connect to {}", addr))?;
        
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // 尝试使用密钥文件
        if let Some(ref key_file) = config.key_file {
            if std::path::Path::new(key_file).exists() {
                session.userauth_pubkey_file(
                    &config.username,
                    None,
                    std::path::Path::new(key_file),
                    None,
                )?;
            }
        }

        // 如果密钥认证失败或没有密钥，尝试密码认证
        if !session.authenticated() {
            if let Some(ref password) = config.password {
                session.userauth_password(&config.username, password)?;
            } else {
                anyhow::bail!("Authentication failed: no password or key file provided");
            }
        }

        if !session.authenticated() {
            anyhow::bail!("Authentication failed");
        }

        let session = Arc::new(Mutex::new(session));
        *SSH_SESSION.lock().unwrap() = Some(session.clone());

        Ok(())
    }

    pub async fn disconnect() {
        *SSH_SESSION.lock().unwrap() = None;
    }

    pub fn get_session() -> Result<Arc<Mutex<Session>>> {
        SSH_SESSION
            .lock()
            .unwrap()
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))
            .map(|s| s.clone())
    }

    pub async fn execute_command(command: &str) -> Result<(i32, String, String)> {
        let session = Self::get_session()?;
        let mut channel = session.lock().unwrap().channel_session()?;
        
        channel.exec(command)?;
        
        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;
        
        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;
        
        channel.wait_close()?;
        let exit_status = channel.exit_status()?;
        
        Ok((exit_status, stdout, stderr))
    }

    pub async fn upload_file(local_path: &str, remote_path: &str) -> Result<()> {
        let session = Self::get_session()?;
        let mut sftp = session.lock().unwrap().sftp()?;
        
        let mut local_file = std::fs::File::open(local_path)
            .with_context(|| format!("Failed to open local file: {}", local_path))?;
        
        let mut remote_file = sftp.create(std::path::Path::new(remote_path))?;
        
        std::io::copy(&mut local_file, &mut remote_file)?;
        
        Ok(())
    }

    pub async fn download_file(remote_path: &str, local_path: &str) -> Result<()> {
        let session = Self::get_session()?;
        let mut sftp = session.lock().unwrap().sftp()?;
        
        let mut remote_file = sftp.open(std::path::Path::new(remote_path))?;
        let mut local_file = std::fs::File::create(local_path)
            .with_context(|| format!("Failed to create local file: {}", local_path))?;
        
        std::io::copy(&mut remote_file, &mut local_file)?;
        
        Ok(())
    }
}
