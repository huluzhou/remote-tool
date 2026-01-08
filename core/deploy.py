"""
部署模块 - 用于通过SSH部署和更新程序到远程服务器
"""

import logging
from pathlib import Path
from typing import Optional, Tuple, List, Dict
import os

from .ssh_client import SSHClient

logger = logging.getLogger(__name__)


class Deployer:
    """部署器，用于通过SSH部署程序到远程服务器"""
    
    # 部署配置
    INSTALL_DIR = "/opt/analysis"
    SERVICE_USER = "analysis"
    SERVICE_NAME = "analysis-collector"
    BINARY_NAME = "analysis-collector"
    SERVICE_FILE = f"/etc/systemd/system/{SERVICE_NAME}.service"
    
    # 本地文件路径（相对于项目根目录）
    BINARY_SOURCE_PATHS = [
        "target/release/analysis-collector",
        "target/x86_64-unknown-linux-musl/release/analysis-collector",
        "target/aarch64-unknown-linux-musl/release/analysis-collector",
        "target/aarch64-unknown-linux-gnu/release/analysis-collector",
    ]
    CONFIG_SOURCE = "config.toml.example"
    TOPO_SOURCE = "topo.json"
    
    def __init__(self, ssh_client: SSHClient, project_root: Optional[str] = None):
        """
        初始化部署器
        
        Args:
            ssh_client: SSH客户端实例（必须已连接）
            project_root: 项目根目录路径，如果为None则自动检测
        """
        self.ssh = ssh_client
        if not self.ssh.client:
            raise RuntimeError("SSH客户端未连接，请先调用connect()方法")
        
        # 自动检测项目根目录
        if project_root is None:
            # 从当前文件位置向上查找项目根目录
            current_file = Path(__file__).resolve()
            # query_tool/core/deploy.py -> query_tool -> 项目根目录
            project_root = current_file.parent.parent.parent
            # 如果项目根目录不存在，尝试从当前工作目录查找
            if not Path(project_root).exists():
                # 尝试查找包含Cargo.toml的目录
                cwd = Path.cwd()
                if (cwd / "Cargo.toml").exists():
                    project_root = cwd
                else:
                    # 向上查找
                    for parent in cwd.parents:
                        if (parent / "Cargo.toml").exists():
                            project_root = parent
                            break
        
        self.project_root = Path(project_root).resolve()
        logger.info(f"项目根目录: {self.project_root}")
    
    def find_binary(self) -> Optional[Path]:
        """
        查找可执行文件
        
        Returns:
            可执行文件路径，如果未找到返回None
        """
        for binary_path in self.BINARY_SOURCE_PATHS:
            full_path = self.project_root / binary_path
            if full_path.exists() and full_path.is_file():
                logger.info(f"找到可执行文件: {full_path}")
                return full_path
        
        logger.warning("未找到可执行文件")
        return None
    
    def check_remote_status(self) -> Dict[str, bool]:
        """
        检查远程服务器部署状态
        
        Returns:
            包含部署状态的字典
        """
        status = {
            "installed": False,
            "service_exists": False,
            "service_running": False,
            "service_enabled": False,
        }
        
        # 检查可执行文件是否存在
        exit_status, stdout, stderr = self.ssh.execute_command(
            f"test -f {self.INSTALL_DIR}/bin/{self.BINARY_NAME} && echo 'exists' || echo 'not_exists'"
        )
        status["installed"] = "exists" in stdout
        
        # 检查服务文件是否存在
        exit_status, stdout, stderr = self.ssh.execute_command(
            f"test -f {self.SERVICE_FILE} && echo 'exists' || echo 'not_exists'"
        )
        status["service_exists"] = "exists" in stdout
        
        # 检查服务是否运行
        if status["service_exists"]:
            exit_status, stdout, stderr = self.ssh.execute_command(
                f"systemctl is-active {self.SERVICE_NAME} 2>/dev/null && echo 'active' || echo 'inactive'"
            )
            status["service_running"] = "active" in stdout
            
            # 检查服务是否启用
            exit_status, stdout, stderr = self.ssh.execute_command(
                f"systemctl is-enabled {self.SERVICE_NAME} 2>/dev/null && echo 'enabled' || echo 'disabled'"
            )
            status["service_enabled"] = "enabled" in stdout
        
        return status
    
    def create_directories(self, check_existing: bool = False) -> Tuple[bool, str]:
        """
        创建远程目录结构
        
        Args:
            check_existing: 如果为True，检查目录是否已存在，存在则跳过
        
        Returns:
            (success, message) 元组
        """
        try:
            # 如果检查已存在，先检查目录是否存在
            if check_existing:
                exit_status, stdout, stderr = self.ssh.execute_command(
                    f"test -d {self.INSTALL_DIR}/bin && echo 'exists' || echo 'not_exists'"
                )
                if "exists" in stdout:
                    logger.info(f"目录结构已存在: {self.INSTALL_DIR}")
                    return True, "目录结构已存在，跳过创建"
            
            # 创建主目录和bin目录
            command = f"sudo mkdir -p {self.INSTALL_DIR}/bin"
            exit_status, stdout, stderr = self.ssh.execute_command(command)
            
            if exit_status != 0:
                return False, f"创建目录失败: {stderr}"
            
            logger.info(f"目录结构创建成功: {self.INSTALL_DIR}")
            return True, "目录结构创建成功"
        except Exception as e:
            logger.error(f"创建目录异常: {e}")
            return False, f"创建目录异常: {e}"
    
    def upload_file(self, local_path: Path, remote_path: str, use_sudo: bool = False) -> Tuple[bool, str]:
        """
        上传文件到远程服务器
        
        Args:
            local_path: 本地文件路径
            remote_path: 远程文件路径
            use_sudo: 是否使用sudo权限（用于写入系统目录）
        
        Returns:
            (success, message) 元组
        """
        try:
            if not local_path.exists():
                return False, f"本地文件不存在: {local_path}"
            
            # 先上传到临时目录
            temp_remote = f"/tmp/{local_path.name}"
            
            logger.info(f"上传文件: {local_path} -> {temp_remote}")
            if not self.ssh.upload_file(str(local_path), temp_remote, async_mode=True):
                return False, "文件上传失败"
            
            try:
                # 如果需要sudo权限，使用sudo移动文件
                # 先删除旧文件（如果存在），确保可以覆盖
                # 使用单引号转义路径，避免特殊字符问题
                if use_sudo:
                    command = f"sudo rm -f '{remote_path}' && sudo mv '{temp_remote}' '{remote_path}' && sudo chmod 644 '{remote_path}'"
                else:
                    command = f"rm -f '{remote_path}' && mv '{temp_remote}' '{remote_path}' && chmod 644 '{remote_path}'"
                
                exit_status, stdout, stderr = self.ssh.execute_command(command)
                
                if exit_status != 0:
                    return False, f"移动文件失败: {stderr}"
                
                logger.info(f"文件上传成功: {remote_path}")
                return True, "文件上传成功"
            finally:
                # 无论成功或失败，都清理临时文件
                try:
                    self.ssh.execute_command(f"rm -f {temp_remote}")
                except:
                    pass
        except Exception as e:
            logger.error(f"上传文件异常: {e}")
            return False, f"上传文件异常: {e}"
    
    def upload_binary(self, binary_path: Path) -> Tuple[bool, str]:
        """
        上传可执行文件
        
        Args:
            binary_path: 本地可执行文件路径
        
        Returns:
            (success, message) 元组
        """
        try:
            remote_path = f"{self.INSTALL_DIR}/bin/{self.BINARY_NAME}"
            
            # 先上传到临时位置
            temp_remote = f"/tmp/{self.BINARY_NAME}"
            logger.info(f"上传可执行文件: {binary_path} -> {temp_remote}")
            
            if not self.ssh.upload_file(str(binary_path), temp_remote, async_mode=True):
                return False, "可执行文件上传失败"
            
            try:
                # 先删除旧文件（如果存在），确保可以覆盖
                # 使用绝对路径并转义特殊字符，确保命令正确执行
                # 然后使用sudo移动并设置执行权限
                command = (
                    f"sudo rm -f '{remote_path}' && "
                    f"sudo mv '{temp_remote}' '{remote_path}' && "
                    f"sudo chmod +x '{remote_path}' && "
                    f"sudo chown root:root '{remote_path}'"
                )
                
                exit_status, stdout, stderr = self.ssh.execute_command(command)
                
                if exit_status != 0:
                    return False, f"设置可执行文件权限失败: {stderr}"
                
                # 验证文件是否真的被移动了
                verify_cmd = f"test -f '{remote_path}' && echo 'exists' || echo 'not_exists'"
                verify_exit, verify_stdout, verify_stderr = self.ssh.execute_command(verify_cmd)
                if "not_exists" in verify_stdout:
                    return False, f"文件移动后验证失败: 目标文件不存在"
                
                logger.info(f"可执行文件部署成功: {remote_path}")
                return True, "可执行文件部署成功"
            finally:
                # 无论成功或失败，都清理临时文件
                try:
                    self.ssh.execute_command(f"rm -f {temp_remote}")
                except:
                    pass
        except Exception as e:
            logger.error(f"上传可执行文件异常: {e}")
            return False, f"上传可执行文件异常: {e}"
    
    def create_user(self) -> Tuple[bool, str]:
        """
        创建运行用户（如果不存在）
        
        Returns:
            (success, message) 元组
        """
        try:
            # 检查用户是否存在
            exit_status, stdout, stderr = self.ssh.execute_command(
                f"id {self.SERVICE_USER} 2>/dev/null && echo 'exists' || echo 'not_exists'"
            )
            
            if "exists" in stdout:
                logger.info(f"用户 {self.SERVICE_USER} 已存在")
                return True, f"用户 {self.SERVICE_USER} 已存在"
            
            # 创建用户
            command = f"sudo useradd -r -s /bin/false {self.SERVICE_USER}"
            exit_status, stdout, stderr = self.ssh.execute_command(command)
            
            if exit_status != 0:
                return False, f"创建用户失败: {stderr}"
            
            logger.info(f"用户 {self.SERVICE_USER} 创建成功")
            return True, f"用户 {self.SERVICE_USER} 创建成功"
        except Exception as e:
            logger.error(f"创建用户异常: {e}")
            return False, f"创建用户异常: {e}"
    
    def set_permissions(self, use_root: bool = False) -> Tuple[bool, str]:
        """
        设置文件权限
        
        Args:
            use_root: 是否使用root用户运行服务
        
        Returns:
            (success, message) 元组
        """
        try:
            if use_root:
                # 使用root用户，只需要设置可执行文件权限
                command = f"sudo chmod +x {self.INSTALL_DIR}/bin/{self.BINARY_NAME}"
                exit_status, stdout, stderr = self.ssh.execute_command(command)
                
                if exit_status != 0:
                    return False, f"设置权限失败: {stderr}"
                
                return True, "权限设置成功（root用户）"
            else:
                # 创建用户并设置权限
                success, msg = self.create_user()
                if not success:
                    return False, msg
                
                # 设置目录和文件的所有者
                command = f"sudo chown -R {self.SERVICE_USER}:{self.SERVICE_USER} {self.INSTALL_DIR}"
                exit_status, stdout, stderr = self.ssh.execute_command(command)
                
                if exit_status != 0:
                    return False, f"设置所有者失败: {stderr}"
                
                return True, f"权限设置成功（用户: {self.SERVICE_USER}）"
        except Exception as e:
            logger.error(f"设置权限异常: {e}")
            return False, f"设置权限异常: {e}"
    
    def create_service_file(self, use_root: bool = False, force: bool = False) -> Tuple[bool, str]:
        """
        创建systemd服务文件
        
        Args:
            use_root: 是否使用root用户运行服务
            force: 是否强制更新（即使内容相同也更新）
        
        Returns:
            (success, message) 元组
        """
        try:
            if use_root:
                service_content = f"""[Unit]
Description=Analysis Data Collector
After=network.target

[Service]
Type=simple
WorkingDirectory={self.INSTALL_DIR}
ExecStart={self.INSTALL_DIR}/bin/{self.BINARY_NAME} --config {self.INSTALL_DIR}/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"""
            else:
                service_content = f"""[Unit]
Description=Analysis Data Collector
After=network.target

[Service]
Type=simple
User={self.SERVICE_USER}
WorkingDirectory={self.INSTALL_DIR}
ExecStart={self.INSTALL_DIR}/bin/{self.BINARY_NAME} --config {self.INSTALL_DIR}/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"""
            
            # 检查服务文件是否已存在且内容相同
            if not force:
                exit_status, existing_content, stderr = self.ssh.execute_command(
                    f"sudo cat '{self.SERVICE_FILE}' 2>/dev/null || echo ''"
                )
                if exit_status == 0 and existing_content.strip() == service_content.strip():
                    logger.info(f"服务文件内容未变化，跳过更新: {self.SERVICE_FILE}")
                    return True, "服务文件内容未变化，跳过更新"
            
            # 将服务文件内容写入本地临时文件
            import tempfile
            with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.service') as f:
                f.write(service_content)
                temp_local = f.name
            
            try:
                # 上传到远程临时位置
                temp_remote = "/tmp/analysis-collector.service"
                logger.info(f"上传服务文件: {temp_local} -> {temp_remote}")
                
                if not self.ssh.upload_file(temp_local, temp_remote, async_mode=True):
                    return False, "上传服务文件失败"
                
                try:
                    # 使用sudo移动服务文件到正确位置
                    # 先删除旧文件（如果存在），确保可以覆盖
                    command = f"sudo rm -f '{self.SERVICE_FILE}' && sudo mv '{temp_remote}' '{self.SERVICE_FILE}'"
                    exit_status, stdout, stderr = self.ssh.execute_command(command)
                    
                    if exit_status != 0:
                        return False, f"移动服务文件失败: {stderr}"
                    
                    # 重新加载systemd
                    exit_status, stdout, stderr = self.ssh.execute_command("sudo systemctl daemon-reload")
                    
                    if exit_status != 0:
                        return False, f"重新加载systemd失败: {stderr}"
                    
                    logger.info(f"服务文件创建成功: {self.SERVICE_FILE}")
                    return True, "服务文件创建成功"
                finally:
                    # 无论成功或失败，都清理远程临时文件
                    try:
                        self.ssh.execute_command(f"rm -f {temp_remote}")
                    except:
                        pass
            finally:
                # 清理本地临时文件
                try:
                    os.unlink(temp_local)
                except:
                    pass
        except Exception as e:
            logger.error(f"创建服务文件异常: {e}")
            return False, f"创建服务文件异常: {e}"
    
    def enable_service(self, start: bool = True) -> Tuple[bool, str]:
        """
        启用并启动服务
        
        Args:
            start: 是否立即启动服务
        
        Returns:
            (success, message) 元组
        """
        try:
            # 启用服务（开机自启）
            exit_status, stdout, stderr = self.ssh.execute_command(
                f"sudo systemctl enable {self.SERVICE_NAME}"
            )
            
            if exit_status != 0:
                return False, f"启用服务失败: {stderr}"
            
            logger.info("服务已启用（开机自启）")
            
            if start:
                # 启动服务
                exit_status, stdout, stderr = self.ssh.execute_command(
                    f"sudo systemctl start {self.SERVICE_NAME}"
                )
                
                if exit_status != 0:
                    return False, f"启动服务失败: {stderr}"
                
                # 等待一下，检查服务状态
                import time
                time.sleep(2)
                
                exit_status, stdout, stderr = self.ssh.execute_command(
                    f"systemctl is-active {self.SERVICE_NAME} 2>/dev/null && echo 'active' || echo 'inactive'"
                )
                
                if "active" not in stdout:
                    # 获取服务状态信息
                    exit_status2, stdout2, stderr2 = self.ssh.execute_command(
                        f"sudo systemctl status {self.SERVICE_NAME} --no-pager -l | head -20"
                    )
                    return False, f"服务启动失败，状态: {stdout}\n详细信息:\n{stdout2}"
                
                logger.info("服务启动成功")
                return True, "服务已启用并启动成功"
            else:
                return True, "服务已启用（未启动）"
        except Exception as e:
            logger.error(f"启用服务异常: {e}")
            return False, f"启用服务异常: {e}"
    
    def stop_service(self) -> Tuple[bool, str]:
        """
        停止服务
        
        Returns:
            (success, message) 元组
        """
        try:
            exit_status, stdout, stderr = self.ssh.execute_command(
                f"sudo systemctl stop {self.SERVICE_NAME}"
            )
            
            if exit_status != 0:
                # 服务可能未运行，这不是错误
                if "not loaded" in stderr.lower() or "not found" in stderr.lower():
                    return True, "服务未运行"
                return False, f"停止服务失败: {stderr}"
            
            logger.info("服务已停止")
            return True, "服务已停止"
        except Exception as e:
            logger.error(f"停止服务异常: {e}")
            return False, f"停止服务异常: {e}"
    
    def deploy(self, 
               binary_path: Optional[Path] = None,
               config_path: Optional[Path] = None,
               topo_path: Optional[Path] = None,
               upload_config: bool = True,
               upload_topo: bool = True,
               use_root: bool = False,
               start_service: bool = True) -> Tuple[bool, str, List[str]]:
        """
        执行部署或更新
        
        Args:
            binary_path: 可执行文件路径（必需）
            config_path: 配置文件路径，如果为None且upload_config为True则使用默认路径
            topo_path: 拓扑文件路径，如果为None且upload_topo为True则使用默认路径
            upload_config: 是否上传配置文件
            upload_topo: 是否上传拓扑文件
            use_root: 是否使用root用户运行服务
            start_service: 是否启动服务
        
        Returns:
            (success, message, log_messages) 元组
        """
        log_messages = []
        
        try:
            # 检查部署状态
            status = self.check_remote_status()
            is_update = status["installed"]
            
            log_messages.append(f"部署模式: {'更新' if is_update else '新部署'}")
            
            # 检查可执行文件
            if binary_path is None:
                return False, "未指定可执行文件路径", log_messages
            
            if not binary_path.exists():
                return False, f"可执行文件不存在: {binary_path}", log_messages
            
            log_messages.append(f"可执行文件: {binary_path}")
            
            # 如果是更新，先停止服务
            if is_update and status["service_running"]:
                log_messages.append("停止现有服务...")
                success, msg = self.stop_service()
                if not success:
                    log_messages.append(f"警告: {msg}")
                else:
                    log_messages.append(msg)
            
            # 创建目录结构（更新时检查是否已存在）
            log_messages.append("创建目录结构...")
            success, msg = self.create_directories(check_existing=is_update)
            if not success:
                return False, f"创建目录失败: {msg}", log_messages
            log_messages.append(msg)
            
            # 上传可执行文件
            log_messages.append("上传可执行文件...")
            success, msg = self.upload_binary(binary_path)
            if not success:
                return False, f"上传可执行文件失败: {msg}", log_messages
            log_messages.append(msg)
            
            # 上传配置文件
            if upload_config:
                if config_path is None:
                    # 使用默认路径
                    config_path = self.project_root / self.CONFIG_SOURCE
                    if not config_path.exists():
                        # 尝试使用config.toml
                        config_path = self.project_root / "config.toml"
                
                if config_path and config_path.exists():
                    log_messages.append("上传配置文件...")
                    success, msg = self.upload_file(
                        config_path,
                        f"{self.INSTALL_DIR}/config.toml",
                        use_sudo=True
                    )
                    if not success:
                        log_messages.append(f"警告: {msg}")
                    else:
                        log_messages.append(msg)
                else:
                    log_messages.append(f"警告: 配置文件不存在: {config_path}")
            
            # 上传拓扑文件
            if upload_topo:
                if topo_path is None:
                    # 使用默认路径
                    topo_path = self.project_root / self.TOPO_SOURCE
                
                if topo_path and topo_path.exists():
                    log_messages.append("上传拓扑文件...")
                    success, msg = self.upload_file(
                        topo_path,
                        f"{self.INSTALL_DIR}/topo.json",
                        use_sudo=True
                    )
                    if not success:
                        log_messages.append(f"警告: {msg}")
                    else:
                        log_messages.append(msg)
                else:
                    log_messages.append(f"警告: 拓扑文件不存在: {topo_path}")
            
            # 设置权限
            log_messages.append("设置权限...")
            success, msg = self.set_permissions(use_root=use_root)
            if not success:
                return False, f"设置权限失败: {msg}", log_messages
            log_messages.append(msg)
            
            # 创建或更新服务文件
            log_messages.append("创建服务文件...")
            success, msg = self.create_service_file(use_root=use_root)
            if not success:
                return False, f"创建服务文件失败: {msg}", log_messages
            log_messages.append(msg)
            
            # 启用并启动服务
            log_messages.append("启用服务...")
            success, msg = self.enable_service(start=start_service)
            if not success:
                return False, f"启用服务失败: {msg}", log_messages
            log_messages.append(msg)
            
            # 部署完成
            action = "更新" if is_update else "部署"
            final_message = f"{action}完成！"
            log_messages.append(final_message)
            
            return True, final_message, log_messages
            
        except Exception as e:
            logger.error(f"部署异常: {e}")
            error_msg = f"部署异常: {e}"
            log_messages.append(error_msg)
            return False, error_msg, log_messages
