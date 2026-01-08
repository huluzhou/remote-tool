"""
SSH客户端模块 - 用于连接远程服务器并执行命令
"""

import paramiko
import socket
from typing import Optional, Tuple, Callable
from pathlib import Path
import logging
import threading
import queue

logger = logging.getLogger(__name__)


class SSHClient:
    """SSH客户端，用于连接远程服务器"""
    
    def __init__(self, host: str, port: int = 22, username: str = "root", 
                 password: Optional[str] = None, key_file: Optional[str] = None,
                 timeout: int = 30):
        """
        初始化SSH客户端
        
        Args:
            host: 服务器地址
            port: SSH端口，默认22
            username: 用户名，默认root
            password: 密码（如果使用密钥认证，可以为None）
            key_file: 私钥文件路径（可选）
            timeout: 连接超时时间（秒），默认10秒
        """
        self.host = host
        self.port = port
        self.username = username
        self.password = password
        self.key_file = key_file
        self.timeout = timeout
        self.client: Optional[paramiko.SSHClient] = None
        self.sftp: Optional[paramiko.SFTPClient] = None
    
    def connect(self) -> Tuple[bool, str]:
        """
        连接到SSH服务器
        
        Returns:
            (success, error_message) 元组
            - success: True if connection successful, False otherwise
            - error_message: 详细的错误信息（如果失败）
        """
        try:
            self.client = paramiko.SSHClient()
            self.client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
            
            # 尝试使用密钥文件
            if self.key_file and Path(self.key_file).exists():
                try:
                    self.client.connect(
                        hostname=self.host,
                        port=self.port,
                        username=self.username,
                        key_filename=self.key_file,
                        timeout=self.timeout,
                        look_for_keys=False,  # 禁用自动查找密钥
                        allow_agent=False    # 禁用SSH代理
                    )
                    logger.info(f"Connected to {self.host} using key file")
                    return True, ""
                except Exception as e:
                    logger.warning(f"Failed to connect with key file: {e}, trying password")
            
            # 使用密码认证
            if self.password:
                try:
                    self.client.connect(
                        hostname=self.host,
                        port=self.port,
                        username=self.username,
                        password=self.password,
                        timeout=self.timeout,
                        look_for_keys=False,  # 禁用自动查找密钥（对JumpServer很重要）
                        allow_agent=False     # 禁用SSH代理（对JumpServer很重要）
                    )
                    logger.info(f"Connected to {self.host} using password")
                    return True, ""
                except socket.timeout:
                    error_msg = (
                        f"连接超时\n\n"
                        f"服务器: {self.host}:{self.port}\n"
                        f"超时时间: {self.timeout}秒\n\n"
                        f"可能的原因：\n"
                        f"1. 服务器地址或端口不正确\n"
                        f"2. 网络连接问题\n"
                        f"3. 防火墙阻止连接\n"
                        f"4. 服务器未运行或SSH服务未启动"
                    )
                    logger.error(f"Connection timeout to {self.host}:{self.port}")
                    return False, error_msg
                except paramiko.AuthenticationException as e:
                    error_msg = (
                        f"认证失败\n\n"
                        f"服务器: {self.host}:{self.port}\n"
                        f"用户名: {self.username}\n\n"
                        f"可能的原因：\n"
                        f"1. 用户名或密码错误\n"
                        f"2. 账户被禁用\n"
                        f"3. 服务器不允许密码认证（需要使用密钥）\n\n"
                        f"错误详情: {str(e)}"
                    )
                    logger.error(f"Authentication failed for {self.username}@{self.host}: {e}")
                    return False, error_msg
                except paramiko.SSHException as e:
                    error_str = str(e)
                    # 检查是否是传输关闭错误（通常在认证阶段发生）
                    if "transport shut down" in error_str.lower() or "saw eof" in error_str.lower():
                        error_msg = (
                            f"SSH连接在认证过程中被关闭\n\n"
                            f"服务器: {self.host}:{self.port}\n"
                            f"用户名: {self.username}\n\n"
                            f"可能的原因：\n"
                            f"1. 用户名或密码错误（服务器在认证失败后关闭连接）\n"
                            f"2. 服务器达到最大认证尝试次数限制\n"
                            f"3. 服务器配置不允许该用户登录\n"
                            f"4. 网络连接不稳定导致中断\n"
                            f"5. 服务器SSH服务异常\n\n"
                            f"错误详情: {error_str}"
                        )
                    else:
                        error_msg = (
                            f"SSH连接错误\n\n"
                            f"服务器: {self.host}:{self.port}\n\n"
                            f"可能的原因：\n"
                            f"1. SSH服务未运行\n"
                            f"2. 端口不正确\n"
                            f"3. SSH协议版本不兼容\n\n"
                            f"错误详情: {error_str}"
                        )
                    logger.error(f"SSH error connecting to {self.host}:{self.port}: {e}")
                    return False, error_msg
                except socket.gaierror as e:
                    error_msg = (
                        f"无法解析主机名\n\n"
                        f"主机: {self.host}\n\n"
                        f"可能的原因：\n"
                        f"1. 主机名或IP地址拼写错误\n"
                        f"2. DNS解析失败\n"
                        f"3. 网络配置问题\n\n"
                        f"错误详情: {str(e)}"
                    )
                    logger.error(f"DNS resolution failed for {self.host}: {e}")
                    return False, error_msg
                except ConnectionRefusedError as e:
                    error_msg = (
                        f"连接被拒绝\n\n"
                        f"服务器: {self.host}:{self.port}\n\n"
                        f"可能的原因：\n"
                        f"1. 端口不正确\n"
                        f"2. SSH服务未在该端口运行\n"
                        f"3. 防火墙阻止连接\n"
                        f"4. 服务器拒绝连接\n\n"
                        f"错误详情: {str(e)}"
                    )
                    logger.error(f"Connection refused to {self.host}:{self.port}: {e}")
                    return False, error_msg
                except OSError as e:
                    error_msg = (
                        f"网络错误\n\n"
                        f"服务器: {self.host}:{self.port}\n\n"
                        f"可能的原因：\n"
                        f"1. 网络不可达\n"
                        f"2. 路由问题\n"
                        f"3. 防火墙阻止\n\n"
                        f"错误详情: {str(e)}"
                    )
                    logger.error(f"Network error connecting to {self.host}:{self.port}: {e}")
                    return False, error_msg
                except Exception as e:
                    error_msg = (
                        f"连接失败\n\n"
                        f"服务器: {self.host}:{self.port}\n"
                        f"用户名: {self.username}\n\n"
                        f"错误类型: {type(e).__name__}\n"
                        f"错误详情: {str(e)}"
                    )
                    logger.error(f"Failed to connect to {self.host}:{self.port}: {e}")
                    return False, error_msg
            else:
                error_msg = (
                    f"缺少认证信息\n\n"
                    f"服务器: {self.host}:{self.port}\n"
                    f"用户名: {self.username}\n\n"
                    f"请提供密码或密钥文件"
                )
                logger.error("No password or valid key file provided")
                return False, error_msg
                
        except Exception as e:
            error_msg = (
                f"连接初始化失败\n\n"
                f"错误类型: {type(e).__name__}\n"
                f"错误详情: {str(e)}"
            )
            logger.error(f"Failed to initialize connection: {e}")
            return False, error_msg
    
    def execute_command(self, command: str, timeout: Optional[int] = None) -> Tuple[int, str, str]:
        """
        执行SSH命令
        
        Args:
            command: 要执行的命令
            timeout: 超时时间（秒），None表示使用默认超时
            
        Returns:
            (exit_status, stdout, stderr) 元组
        """
        if not self.client:
            raise RuntimeError("Not connected. Call connect() first.")
        
        try:
            stdin, stdout, stderr = self.client.exec_command(command)
            
            # 对于大输出，使用分块读取避免内存问题
            stdout_chunks = []
            stderr_chunks = []
            
            # 设置超时（如果提供）
            if timeout:
                import socket
                stdout.channel.settimeout(timeout)
                stderr.channel.settimeout(timeout)
            
            # 先读取stderr（通常较小且重要，避免被stdout读取阻塞）
            # 使用非阻塞方式，但paramiko的read()会阻塞直到有数据或EOF
            # 所以我们需要在读取stdout之前先尝试读取stderr
            import threading
            import queue
            
            stderr_queue = queue.Queue()
            stderr_done = threading.Event()
            
            def read_stderr():
                try:
                    while True:
                        chunk = stderr.read(1024 * 1024)  # 1MB chunks
                        if chunk:
                            stderr_queue.put(chunk)
                        else:
                            break
                except Exception as e:
                    logger.warning(f"Error reading stderr: {e}")
                finally:
                    stderr_done.set()
            
            # 启动stderr读取线程
            stderr_thread = threading.Thread(target=read_stderr, daemon=True)
            stderr_thread.start()
            
            # 分块读取stdout（支持大文件）
            while True:
                chunk = stdout.read(1024 * 1024)  # 1MB chunks
                if not chunk:
                    break
                stdout_chunks.append(chunk)
            
            # 等待stderr读取完成
            stderr_done.wait(timeout=5)  # 最多等待5秒
            
            # 收集stderr数据
            while not stderr_queue.empty():
                stderr_chunks.append(stderr_queue.get())
            
            # 读取剩余的stderr（如果还有）
            while True:
                chunk = stderr.read(1024 * 1024)
                if not chunk:
                    break
                stderr_chunks.append(chunk)
            
            # 等待命令完成并获取退出状态
            exit_status = stdout.channel.recv_exit_status()
            
            # 合并chunks并解码
            stdout_text = b''.join(stdout_chunks).decode('utf-8', errors='replace')
            stderr_text = b''.join(stderr_chunks).decode('utf-8', errors='replace')
            
            return exit_status, stdout_text, stderr_text
        except Exception as e:
            logger.error(f"Failed to execute command: {e}")
            raise
    
    def get_sftp(self) -> paramiko.SFTPClient:
        """
        获取SFTP客户端（用于文件传输）
        
        Returns:
            SFTP客户端对象
        """
        if not self.client:
            raise RuntimeError("Not connected. Call connect() first.")
        
        if not self.sftp:
            self.sftp = self.client.open_sftp()
        
        return self.sftp
    
    def download_file(self, remote_path: str, local_path: str, 
                     async_mode: bool = False,
                     callback: Optional[Callable[[bool, Optional[str]], None]] = None) -> bool:
        """
        从远程服务器下载文件（使用SFTP）
        
        Args:
            remote_path: 远程文件路径
            local_path: 本地保存路径
            async_mode: 是否异步执行（默认False，保持向后兼容）
            callback: 异步模式下的回调函数 callback(success: bool, error: Optional[str])
            
        Returns:
            True if successful, False otherwise
            注意：异步模式下会等待完成后再返回
        """
        def _download():
            try:
                sftp = self.get_sftp()
                sftp.get(remote_path, local_path)
                logger.info(f"Downloaded {remote_path} to {local_path}")
                return True, None
            except Exception as e:
                error_msg = str(e)
                logger.error(f"Failed to download file: {e}")
                return False, error_msg
        
        if async_mode:
            # 异步模式：在后台线程执行，但等待完成
            result_queue = queue.Queue()
            
            def _download_with_result():
                success, error = _download()
                result_queue.put((success, error))
                if callback:
                    callback(success, error)
            
            thread = threading.Thread(target=_download_with_result, daemon=True)
            thread.start()
            
            # 等待结果
            try:
                success, error = result_queue.get(timeout=300)  # 5分钟超时
                return success
            except queue.Empty:
                logger.error("Download timeout")
                if callback:
                    callback(False, "下载超时")
                return False
        else:
            # 同步模式：直接执行
            success, error = _download()
            return success
    
    def download_file_scp(self, remote_path: str, local_path: str) -> bool:
        """
        使用SCP协议下载文件（通过paramiko Transport，不需要系统scp命令）
        注意：JumpServer可能不支持SCP，此方法主要用于普通SSH服务器
        
        Args:
            remote_path: 远程文件路径
            local_path: 本地保存路径
            
        Returns:
            True if successful, False otherwise
        """
        try:
            # 使用paramiko的Transport创建SCP连接
            # 注意：paramiko本身不直接支持SCP，但可以通过SFTP实现类似功能
            # 实际上，SFTP和SCP在paramiko中都是通过open_sftp()实现
            # 所以这里直接使用SFTP方法（更可靠）
            return self.download_file(remote_path, local_path)
        except Exception as e:
            logger.warning(f"SCP/SFTP download failed: {e}")
            return False
    
    def upload_file(self, local_path: str, remote_path: str,
                   async_mode: bool = False,
                   callback: Optional[Callable[[bool, Optional[str]], None]] = None) -> bool:
        """
        上传文件到远程服务器
        
        Args:
            local_path: 本地文件路径
            remote_path: 远程保存路径
            async_mode: 是否异步执行（默认False，保持向后兼容）
            callback: 异步模式下的回调函数 callback(success: bool, error: Optional[str])
            
        Returns:
            True if successful, False otherwise
            注意：异步模式下会等待完成后再返回
        """
        def _upload():
            try:
                sftp = self.get_sftp()
                sftp.put(local_path, remote_path)
                logger.info(f"Uploaded {local_path} to {remote_path}")
                return True, None
            except Exception as e:
                error_msg = str(e)
                logger.error(f"Failed to upload file: {e}")
                return False, error_msg
        
        if async_mode:
            # 异步模式：在后台线程执行，但等待完成
            result_queue = queue.Queue()
            
            def _upload_with_result():
                success, error = _upload()
                result_queue.put((success, error))
                if callback:
                    callback(success, error)
            
            thread = threading.Thread(target=_upload_with_result, daemon=True)
            thread.start()
            
            # 等待结果
            try:
                success, error = result_queue.get(timeout=300)  # 5分钟超时
                return success
            except queue.Empty:
                logger.error("Upload timeout")
                if callback:
                    callback(False, "上传超时")
                return False
        else:
            # 同步模式：直接执行
            success, error = _upload()
            return success
    
    def close(self):
        """关闭SSH连接"""
        if self.sftp:
            self.sftp.close()
            self.sftp = None
        
        if self.client:
            self.client.close()
            self.client = None
        
        logger.info("SSH connection closed")
    
    def __enter__(self):
        """上下文管理器入口"""
        success, error_msg = self.connect()
        if not success:
            raise RuntimeError(f"SSH connection failed: {error_msg}")
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """上下文管理器出口"""
        self.close()
    
    def __repr__(self) -> str:
        return f"SSHClient(host={self.host}, port={self.port}, username={self.username})"
