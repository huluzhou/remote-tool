"""
Tkinter UI界面 - 提供图形化界面用于数据库查询
"""

try:
    import tkinter as tk
    from tkinter import ttk, messagebox, filedialog
    import tkinter.font as tkfont
except ImportError:
    import sys
    print("错误: tkinter 未安装")
    print("")
    print("在 Ubuntu/Debian 系统上，请运行:")
    print("  sudo apt-get install python3-tk")
    print("")
    print("在 CentOS/RHEL 系统上，请运行:")
    print("  sudo yum install python3-tkinter")
    print("")
    sys.exit(1)
from datetime import datetime, timedelta
from pathlib import Path
import json
import logging
import base64
import threading
from typing import Optional, Callable

from core.ssh_client import SSHClient
from core.db_query import DBQuery
from core.csv_export import CSVExporter
from core.deploy import Deployer

logger = logging.getLogger(__name__)


class QueryToolUI:
    """数据库查询工具UI"""
    
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.geometry("800x700")
        
        # 设置中文字体
        self.setup_fonts()
        
        # 注意：窗口标题已在main()函数中设置
        # 窗口标题由窗口管理器控制，某些窗口管理器可能不支持UTF-8中文
        # 如果标题显示乱码，请在main()函数中使用英文标题
        
        # SSH连接配置
        self.ssh_client: Optional[SSHClient] = None
        self.db_query: Optional[DBQuery] = None
        
        # 配置保存路径
        self.config_file = Path.home() / ".query_tool_config.json"
        
        # 创建UI
        self.create_widgets()
        
        # 加载保存的配置
        self.load_config()
        
        # 如果字体有问题，显示警告
        if hasattr(self, '_font_warning') and self._font_warning:
            import tkinter.messagebox as msgbox
            msgbox.showwarning(
                "字体提示",
                "未检测到中文字体，中文可能显示为方框或乱码。\n\n"
                "建议安装中文字体：\n"
                "  Ubuntu/Debian: sudo apt-get install fonts-wqy-microhei\n"
                "  CentOS/RHEL:   sudo yum install wqy-microhei-fonts\n\n"
                "安装后请重启应用程序。"
            )
    
    def setup_fonts(self):
        """设置支持中文的字体"""
        import platform
        
        system = platform.system()
        
        # 获取所有可用字体
        try:
            all_fonts = list(tkfont.families())
        except:
            all_fonts = []
        
        # 中文字体关键词
        chinese_keywords = [
            "wenquanyi", "wenquan", "noto", "source han", "sourcehan",
            "simhei", "simsun", "microsoft yahei", "yahei",
            "pingfang", "stheit", "arial unicode", "dejavu"
        ]
        
        # 尝试使用系统中文字体
        chinese_fonts = []
        
        if system == "Linux":
            # Linux系统常见中文字体（按优先级）
            preferred_fonts = [
                "WenQuanYi Micro Hei",
                "WenQuanYi Zen Hei", 
                "Noto Sans CJK SC",
                "Noto Sans SC",
                "Source Han Sans CN",
                "Source Han Sans SC",
                "DejaVu Sans",
                "SimHei",
            ]
            # 从系统字体中查找包含中文关键词的字体
            for font in all_fonts:
                font_lower = font.lower()
                if any(keyword in font_lower for keyword in chinese_keywords):
                    if font not in preferred_fonts:
                        preferred_fonts.append(font)
            chinese_fonts = preferred_fonts
        elif system == "Windows":
            # Windows系统中文字体
            chinese_fonts = [
                "Microsoft YaHei",
                "SimHei",
                "SimSun",
                "KaiTi",
                "Microsoft JhengHei",
            ]
        elif system == "Darwin":  # macOS
            chinese_fonts = [
                "PingFang SC",
                "STHeiti",
                "Arial Unicode MS",
                "Hiragino Sans GB",
            ]
        
        # 查找可用的字体
        available_font = None
        
        # 方法1: 尝试预定义的字体列表
        for font_name in chinese_fonts:
            try:
                test_font = tkfont.Font(family=font_name, size=10)
                actual_family = test_font.actual()["family"]
                # 检查字体是否真的可用
                # 如果实际字体名匹配，或者实际字体名包含中文字体关键词
                if (actual_family == font_name or 
                    any(kw in actual_family.lower() for kw in chinese_keywords) or
                    font_name.lower() in actual_family.lower()):
                    available_font = font_name
                    logger.info(f"找到中文字体: {font_name} (实际: {actual_family})")
                    break
            except Exception as e:
                logger.debug(f"测试字体 {font_name} 失败: {e}")
                continue
        
        # 方法2: 如果没找到，从系统字体中查找
        if not available_font and all_fonts:
            for font in all_fonts:
                font_lower = font.lower()
                if any(keyword in font_lower for keyword in chinese_keywords):
                    try:
                        test_font = tkfont.Font(family=font, size=10)
                        available_font = font
                        logger.info(f"从系统字体中找到: {font}")
                        break
                    except:
                        continue
        
        # 方法3: 如果还是找不到，尝试使用支持Unicode的字体
        if not available_font:
            # 尝试一些可能支持Unicode的字体（按优先级）
            unicode_fonts = ["DejaVu Sans", "Liberation Sans", "Ubuntu", "Arial", "Sans"]
            for font_name in unicode_fonts:
                try:
                    # 检查字体是否在系统字体列表中
                    if all_fonts and font_name in all_fonts:
                        test_font = tkfont.Font(family=font_name, size=10)
                        available_font = font_name
                        logger.info(f"使用Unicode字体: {font_name}")
                        break
                except Exception as e:
                    logger.debug(f"测试Unicode字体 {font_name} 失败: {e}")
                    continue
        
        # 方法4: 最后使用默认字体
        if not available_font:
            default_font = tkfont.nametofont("TkDefaultFont")
            default_family = default_font.actual()["family"]
            default_font.configure(size=10)
            available_font = default_family
            logger.warning(f"未找到中文字体，使用默认字体: {default_family}")
            # 保存警告信息，稍后在UI中显示
            self._font_warning = True
        else:
            self._font_warning = False
        
        # 配置ttk样式使用中文字体
        style = ttk.Style()
        try:
            style.configure("TLabel", font=(available_font, 10))
            style.configure("TButton", font=(available_font, 10))
            style.configure("TEntry", font=(available_font, 10))
            style.configure("TText", font=(available_font, 10))
            style.configure("TLabelFrame", font=(available_font, 10))
            style.configure("TFrame", font=(available_font, 10))
            style.configure("TCheckbutton", font=(available_font, 10))
            style.configure("TCombobox", font=(available_font, 10))
        except Exception as e:
            logger.warning(f"设置ttk样式字体失败: {e}")
        
        # 配置全局默认字体
        try:
            self.root.option_add("*Font", (available_font, 10))
            # 为Text组件单独设置
            default_text_font = tkfont.Font(family=available_font, size=10)
            self.root.option_add("*Text.Font", default_text_font)
        except Exception as e:
            logger.warning(f"设置全局字体失败: {e}")
        
        # 保存字体供后续使用
        self.chinese_font = available_font
        self.chinese_font_obj = tkfont.Font(family=available_font, size=10)
        
        logger.info(f"最终使用字体: {available_font}")
    
    def create_widgets(self):
        """创建UI组件"""
        # 创建主框架
        main_frame = ttk.Frame(self.root, padding="10")
        main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))
        
        # 配置网格权重
        self.root.columnconfigure(0, weight=1)
        self.root.rowconfigure(0, weight=1)
        main_frame.columnconfigure(1, weight=1)
        
        # === SSH连接配置区域 ===
        ssh_frame = ttk.LabelFrame(main_frame, text="SSH连接配置", padding="10")
        ssh_frame.grid(row=0, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)
        ssh_frame.columnconfigure(1, weight=1)
        
        # SSH连接指令
        ttk.Label(ssh_frame, text="SSH连接指令:").grid(row=0, column=0, sticky=tk.W, padx=5, pady=5)
        self.ssh_command_var = tk.StringVar()
        ssh_entry = ttk.Entry(ssh_frame, textvariable=self.ssh_command_var, width=50)
        ssh_entry.grid(row=0, column=1, sticky=(tk.W, tk.E), padx=5, pady=5)
        
        # 提示文本处理
        placeholder = "ssh user_name@10.60.100.105 -p 2222"
        self.ssh_command_var.set(placeholder)
        ssh_entry.config(foreground="gray")
        
        def on_focus_in(event):
            if self.ssh_command_var.get() == placeholder:
                ssh_entry.delete(0, tk.END)
                ssh_entry.config(foreground="black")
        
        def on_focus_out(event):
            if not self.ssh_command_var.get().strip():
                self.ssh_command_var.set(placeholder)
                ssh_entry.config(foreground="gray")
        
        ssh_entry.bind("<FocusIn>", on_focus_in)
        ssh_entry.bind("<FocusOut>", on_focus_out)
        
        # 密码
        ttk.Label(ssh_frame, text="密码:").grid(row=1, column=0, sticky=tk.W, padx=5, pady=5)
        self.password_var = tk.StringVar()
        ttk.Entry(ssh_frame, textvariable=self.password_var, show="*", width=50).grid(row=1, column=1, sticky=(tk.W, tk.E), padx=5, pady=5)
        
        # 连接按钮和状态
        button_frame = ttk.Frame(ssh_frame)
        button_frame.grid(row=2, column=0, columnspan=2, pady=10)
        self.connect_btn = ttk.Button(button_frame, text="连接", command=self.connect_ssh)
        self.connect_btn.grid(row=0, column=0, padx=5)
        self.connection_status = ttk.Label(button_frame, text="未连接", foreground="red")
        self.connection_status.grid(row=0, column=1, padx=10)
        
        # === 数据库配置区域 ===
        db_frame = ttk.LabelFrame(main_frame, text="数据库配置", padding="10")
        db_frame.grid(row=1, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)
        db_frame.columnconfigure(1, weight=1)
        
        ttk.Label(db_frame, text="数据库路径:").grid(row=0, column=0, sticky=tk.W, padx=5, pady=5)
        self.db_path_var = tk.StringVar(value="/mnt/analysis/data/device_data.db")
        ttk.Entry(db_frame, textvariable=self.db_path_var, width=50).grid(row=0, column=1, sticky=(tk.W, tk.E), padx=5, pady=5)
        
        # === 查询配置区域 ===
        query_frame = ttk.LabelFrame(main_frame, text="查询配置", padding="10")
        query_frame.grid(row=2, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)
        query_frame.columnconfigure(1, weight=1)
        
        # 查询类型选择
        ttk.Label(query_frame, text="查询类型:").grid(row=0, column=0, sticky=tk.W, padx=5, pady=5)
        self.query_type_var = tk.StringVar(value="wide_table")
        query_type_frame = ttk.Frame(query_frame)
        query_type_frame.grid(row=0, column=1, sticky=tk.W, padx=5, pady=5)
        ttk.Radiobutton(query_type_frame, text="设备数据", variable=self.query_type_var, 
                       value="device", command=self.on_query_type_changed).grid(row=0, column=0, padx=5)
        ttk.Radiobutton(query_type_frame, text="指令数据", variable=self.query_type_var, 
                       value="command", command=self.on_query_type_changed).grid(row=0, column=1, padx=5)
        ttk.Radiobutton(query_type_frame, text="宽表", variable=self.query_type_var, 
                       value="wide_table", command=self.on_query_type_changed).grid(row=0, column=2, padx=5)
        
        # 设备序列号（根据查询类型显示必填/可选）
        self.device_sn_label = ttk.Label(query_frame, text="设备序列号:")
        self.device_sn_label.grid(row=1, column=0, sticky=tk.W, padx=5, pady=5)
        self.device_sn_var = tk.StringVar()
        self.device_sn_entry = ttk.Entry(query_frame, textvariable=self.device_sn_var, width=30)
        self.device_sn_entry.grid(row=1, column=1, sticky=(tk.W, tk.E), padx=5, pady=5)
        
        # 时间范围
        ttk.Label(query_frame, text="开始时间:").grid(row=2, column=0, sticky=tk.W, padx=5, pady=5)
        self.start_time_var = tk.StringVar()
        start_time_frame = ttk.Frame(query_frame)
        start_time_frame.grid(row=2, column=1, sticky=(tk.W, tk.E), padx=5, pady=5)
        ttk.Entry(start_time_frame, textvariable=self.start_time_var, width=20).grid(row=0, column=0, padx=(0, 5))
        ttk.Button(start_time_frame, text="今天", command=lambda: self.set_time_range("today")).grid(row=0, column=1, padx=2)
        ttk.Button(start_time_frame, text="昨天", command=lambda: self.set_time_range("yesterday")).grid(row=0, column=2, padx=2)
        ttk.Button(start_time_frame, text="最近7天", command=lambda: self.set_time_range("7days")).grid(row=0, column=3, padx=2)
        
        ttk.Label(query_frame, text="结束时间:").grid(row=3, column=0, sticky=tk.W, padx=5, pady=5)
        self.end_time_var = tk.StringVar()
        end_time_frame = ttk.Frame(query_frame)
        end_time_frame.grid(row=3, column=1, sticky=(tk.W, tk.E), padx=5, pady=5)
        ttk.Entry(end_time_frame, textvariable=self.end_time_var, width=20).grid(row=0, column=0, padx=(0, 5))
        ttk.Button(end_time_frame, text="现在", command=lambda: self.set_end_time_now()).grid(row=0, column=1, padx=2)
        
        # 查询选项（设备数据和宽表查询时显示）
        self.include_ext_var = tk.BooleanVar(value=True)
        self.include_ext_check = ttk.Checkbutton(query_frame, text="包含扩展表数据", variable=self.include_ext_var)
        self.include_ext_check.grid(row=4, column=0, columnspan=2, sticky=tk.W, padx=5, pady=5)
        
        # 查询按钮
        self.query_btn = ttk.Button(query_frame, text="执行查询", command=self.execute_query, state=tk.DISABLED)
        self.query_btn.grid(row=5, column=0, columnspan=2, pady=10)
        
        # 进度条
        self.progress_var = tk.StringVar(value="")
        self.progress_label = ttk.Label(query_frame, textvariable=self.progress_var)
        self.progress_label.grid(row=6, column=0, columnspan=2, pady=5)
        
        # 使用determinate模式，初始值为0，确保打开软件时进度条为空
        self.progress_bar = ttk.Progressbar(query_frame, mode='determinate', length=300, maximum=100)
        self.progress_bar.grid(row=7, column=0, columnspan=2, sticky=(tk.W, tk.E), pady=5)
        
        # === 结果显示区域 ===
        result_frame = ttk.LabelFrame(main_frame, text="查询结果", padding="10")
        result_frame.grid(row=3, column=0, columnspan=2, sticky=(tk.W, tk.E, tk.N, tk.S), pady=5)
        result_frame.columnconfigure(0, weight=1)
        result_frame.rowconfigure(0, weight=1)
        main_frame.rowconfigure(3, weight=1)
        
        # 结果文本框（使用中文字体）
        text_font = self.chinese_font_obj if hasattr(self, 'chinese_font_obj') else tkfont.Font(family=self.chinese_font, size=10)
        self.result_text = tk.Text(result_frame, height=10, wrap=tk.NONE, font=text_font)
        self.result_text.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))
        
        # 滚动条
        scrollbar_y = ttk.Scrollbar(result_frame, orient=tk.VERTICAL, command=self.result_text.yview)
        scrollbar_y.grid(row=0, column=1, sticky=(tk.N, tk.S))
        self.result_text.configure(yscrollcommand=scrollbar_y.set)
        
        scrollbar_x = ttk.Scrollbar(result_frame, orient=tk.HORIZONTAL, command=self.result_text.xview)
        scrollbar_x.grid(row=1, column=0, sticky=(tk.W, tk.E))
        self.result_text.configure(xscrollcommand=scrollbar_x.set)
        
        # 导出按钮和部署按钮
        export_frame = ttk.Frame(main_frame)
        export_frame.grid(row=4, column=0, columnspan=2, pady=5)
        ttk.Button(export_frame, text="导出为CSV", command=self.export_csv, state=tk.DISABLED).grid(row=0, column=0, padx=5)
        ttk.Button(export_frame, text="保存配置", command=self.save_config).grid(row=0, column=1, padx=5)
        self.deploy_btn = ttk.Button(export_frame, text="部署/更新", command=self.show_deploy_dialog, state=tk.DISABLED)
        self.deploy_btn.grid(row=0, column=2, padx=5)
        
        # 存储查询结果和查询类型
        self.query_results = []
        self.query_type = None  # 保存当前查询类型，用于导出时区分
        
        # 初始化查询类型UI状态
        self.on_query_type_changed()
    
    def parse_ssh_command(self, ssh_command: str) -> tuple:
        """
        解析SSH连接指令
        
        支持的格式：
        - ssh user@host
        - ssh user@host -p port
        - ssh user@host:port
        - user@host
        - user@host:port
        
        Returns:
            (username, host, port) 元组，如果解析失败返回 (None, None, None)
        """
        import re
        
        ssh_command = ssh_command.strip()
        if not ssh_command:
            return None, None, None
        
        # 移除开头的 "ssh " 前缀（如果存在）
        ssh_command = re.sub(r'^ssh\s+', '', ssh_command, flags=re.IGNORECASE)
        
        # 默认端口
        port = 2222
        
        # 提取端口（-p port 格式）
        port_match = re.search(r'-p\s+(\d+)', ssh_command, re.IGNORECASE)
        if port_match:
            port = int(port_match.group(1))
            ssh_command = re.sub(r'-p\s+\d+', '', ssh_command, flags=re.IGNORECASE).strip()
        
        # 提取用户名和主机
        # 格式：user@host 或 user@host:port
        match = re.match(r'([^@]+)@([^:]+)(?::(\d+))?', ssh_command)
        if match:
            username = match.group(1).strip()
            host = match.group(2).strip()
            # 如果命令行中有 :port 格式，使用它（优先级高于 -p）
            if match.group(3):
                port = int(match.group(3))
            return username, host, port
        
        # 如果格式不匹配，尝试直接解析为 host:port 或 host
        if '@' not in ssh_command:
            # 可能是 host:port 或 host
            parts = ssh_command.split(':')
            if len(parts) == 2:
                host = parts[0].strip()
                port = int(parts[1].strip())
                return None, host, port
            elif len(parts) == 1:
                host = parts[0].strip()
                return None, host, port
        
        return None, None, None
    
    def set_time_range(self, range_type: str):
        """设置时间范围"""
        now = datetime.now()
        
        if range_type == "today":
            start = now.replace(hour=0, minute=0, second=0, microsecond=0)
            self.start_time_var.set(str(int(start.timestamp())))
            self.end_time_var.set(str(int(now.timestamp())))
        elif range_type == "yesterday":
            yesterday = now - timedelta(days=1)
            start = yesterday.replace(hour=0, minute=0, second=0, microsecond=0)
            end = yesterday.replace(hour=23, minute=59, second=59, microsecond=999999)
            self.start_time_var.set(str(int(start.timestamp())))
            self.end_time_var.set(str(int(end.timestamp())))
        elif range_type == "7days":
            start = now - timedelta(days=7)
            self.start_time_var.set(str(int(start.timestamp())))
            self.end_time_var.set(str(int(now.timestamp())))
    
    def set_end_time_now(self):
        """设置结束时间为当前时间"""
        self.end_time_var.set(str(int(datetime.now().timestamp())))
    
    def connect_ssh(self):
        """连接SSH服务器"""
        try:
            # 解析SSH连接指令
            ssh_command = self.ssh_command_var.get().strip()
            placeholder = "ssh user_name@10.60.100.105 -p 2222"
            if not ssh_command or ssh_command == placeholder:
                messagebox.showerror("错误", "请输入SSH连接指令\n例如: ssh user_name@10.60.100.105 -p 2222")
                return
            
            username, host, port = self.parse_ssh_command(ssh_command)
            
            if not host:
                messagebox.showerror("错误", "无法解析SSH连接指令\n请使用格式: ssh user@host -p port\n或: user@host:port")
                return
            
            if not username:
                username = "root"  # 默认用户名
            
            password = self.password_var.get() or None
            if not password:
                messagebox.showerror("错误", "请输入密码")
                return
            
            # 创建SSH客户端
            self.ssh_client = SSHClient(
                host=host,
                port=port,
                username=username,
                password=password,
                key_file=None  # 简化版本不支持密钥文件
            )
            
            # 连接
            self.connection_status.config(text="正在连接...", foreground="orange")
            self.root.update()
            
            success, error_message = self.ssh_client.connect()
            
            if success:
                self.db_query = DBQuery(self.ssh_client)
                self.connection_status.config(text="已连接", foreground="green")
                self.query_btn.config(state=tk.NORMAL)
                self.deploy_btn.config(state=tk.NORMAL)
                messagebox.showinfo("成功", f"SSH连接成功\n{username}@{host}:{port}")
            else:
                self.connection_status.config(text="连接失败", foreground="red")
                # 显示详细的错误信息
                messagebox.showerror("SSH连接失败", error_message if error_message else "SSH连接失败，请检查配置")
                
        except ValueError as e:
            logger.error(f"Parse error: {e}")
            messagebox.showerror("错误", f"解析SSH指令失败: {e}\n请检查格式是否正确")
        except Exception as e:
            logger.error(f"Connection error: {e}")
            self.connection_status.config(text="连接失败", foreground="red")
            messagebox.showerror("错误", f"连接失败: {e}")
    
    def on_query_type_changed(self):
        """查询类型改变时的回调"""
        query_type = self.query_type_var.get()
        if query_type == "command":
            # 指令查询：隐藏扩展表选项，设备序列号为必填
            self.include_ext_check.grid_remove()
            self.device_sn_label.config(text="设备序列号: *")
        elif query_type == "wide_table":
            # 宽表查询：显示扩展表选项，设备序列号为可选（不显示）
            self.include_ext_check.grid()
            self.device_sn_label.config(text="设备序列号:")
            # 宽表查询不需要设备序列号，可以隐藏或禁用
            self.device_sn_entry.config(state=tk.DISABLED)
            self.device_sn_var.set("")
        else:
            # 设备查询：显示扩展表选项，设备序列号为必填
            self.include_ext_check.grid()
            self.device_sn_label.config(text="设备序列号: *")
            self.device_sn_entry.config(state=tk.NORMAL)
    
    def update_progress(self, message: str = "", progress: float = None):
        """更新进度条状态"""
        if message:
            self.progress_var.set(message)
        if progress is not None:
            # 更新进度条的值（0-100）
            self.progress_bar['value'] = progress
        self.root.update_idletasks()
    
    def execute_query(self):
        """执行查询"""
        if not self.db_query:
            messagebox.showerror("错误", "请先连接SSH服务器")
            return
        
        # 检查是否已有查询在运行
        if hasattr(self, '_query_thread') and self._query_thread.is_alive():
            messagebox.showwarning("警告", "查询正在进行中，请稍候...")
            return
        
        try:
            # 获取查询参数
            db_path = self.db_path_var.get().strip()
            if not db_path:
                messagebox.showerror("错误", "请输入数据库路径")
                return
            
            start_time_str = self.start_time_var.get().strip()
            end_time_str = self.end_time_var.get().strip()
            
            if not start_time_str or not end_time_str:
                messagebox.showerror("错误", "请设置时间范围")
                return
            
            start_time = int(start_time_str)
            end_time = int(end_time_str)
            
            query_type = self.query_type_var.get()
            
            # 对于设备和指令查询，设备序列号是必填的
            device_sn = self.device_sn_var.get().strip()
            if query_type in ["device", "command"]:
                if not device_sn:
                    messagebox.showerror("错误", "设备和指令查询必须指定设备序列号")
                    return
                device_sn = device_sn  # 保持为字符串
            elif query_type == "wide_table":
                device_sn = None  # 宽表查询不使用设备序列号
            else:
                device_sn = device_sn if device_sn else None
            
            # 初始化UI状态
            self.result_text.delete(1.0, tk.END)
            self.result_text.insert(tk.END, "正在查询，请稍候...\n")
            self.query_btn.config(state=tk.DISABLED)
            # 重置进度条为0
            self.progress_bar['value'] = 0
            self.progress_var.set("正在连接数据库...")
            self.root.update()
            
            # 定义进度回调函数（主要用于宽表查询）
            def progress_callback(stage: str, progress: float = None):
                """进度回调函数"""
                messages = {
                    "querying_device": "正在查询设备数据...",
                    "querying_command": "正在查询指令数据...",
                    "merging": "正在合并数据...",
                    "processing": "正在处理数据...",
                    "completed": "查询完成"
                }
                message = messages.get(stage, stage)
                
                # 如果没有提供具体的进度值，根据阶段设置估算进度
                if progress is None:
                    stage_progress = {
                        "querying_device": 30,
                        "querying_command": 60,
                        "merging": 80,
                        "processing": 90,
                        "completed": 100
                    }
                    progress = stage_progress.get(stage, 0)
                
                # 更新进度条和消息
                message = f"{message} ({progress:.1f}%)"
                self.root.after(0, lambda p=progress, m=message: self.update_progress(m, p))
            
            # 在后台线程中执行查询
            def query_thread():
                try:
                    if query_type == "command":
                        # 简单查询，只显示查询中状态
                        self.root.after(0, lambda: self.update_progress("正在查询指令数据...", 50))
                        results = self.db_query.query_command_data(
                            db_path=db_path,
                            start_time=start_time,
                            end_time=end_time,
                            device_sn=device_sn
                        )
                    elif query_type == "wide_table":
                        # 宽表查询，使用完整的进度回调，复用 include_ext_var
                        include_ext = self.include_ext_var.get()
                        results = self.db_query.query_wide_table(
                            db_path=db_path,
                            start_time=start_time,
                            end_time=end_time,
                            include_ext=include_ext,
                            progress_callback=progress_callback
                        )
                    else:
                        # 简单查询，只显示查询中状态
                        self.root.after(0, lambda: self.update_progress("正在查询设备数据...", 50))
                        include_ext = self.include_ext_var.get()
                        results = self.db_query.query_by_time_range(
                            db_path=db_path,
                            start_time=start_time,
                            end_time=end_time,
                            device_sn=device_sn,
                            include_ext=include_ext
                        )
                    
                    # 查询完成，更新UI（保存查询类型）
                    self.root.after(0, lambda q=query_type, r=results: self._on_query_completed(r, q))
                except Exception as e:
                    logger.error(f"Query error: {e}", exc_info=True)
                    self.root.after(0, lambda: self._on_query_error(str(e)))
            
            # 启动查询线程
            self._query_thread = threading.Thread(target=query_thread, daemon=True)
            self._query_thread.start()
            
        except Exception as e:
            logger.error(f"Query error: {e}")
            # 重置进度条为0
            self.progress_bar['value'] = 0
            self.progress_var.set("")
            self.query_btn.config(state=tk.NORMAL)
            self.result_text.delete(1.0, tk.END)
            self.result_text.insert(tk.END, f"查询失败: {e}\n")
            messagebox.showerror("错误", f"查询失败: {e}")
    
    def _on_query_completed(self, results, query_type=None):
        """查询完成回调"""
        # 显示100%完成
        self.progress_bar['value'] = 100
        self.progress_var.set("查询完成 (100%)")
        self.root.update_idletasks()
        self.query_btn.config(state=tk.NORMAL)
        
        # 保存查询结果和查询类型
        self.query_results = results
        self.query_type = query_type if query_type else self.query_type_var.get()
        self.result_text.delete(1.0, tk.END)
        
        if results:
            # 显示前几行作为预览
            preview_lines = min(10, len(results))
            self.result_text.insert(tk.END, f"查询成功！共 {len(results)} 条记录\n\n")
            self.result_text.insert(tk.END, "前10条记录预览:\n")
            self.result_text.insert(tk.END, "-" * 80 + "\n")
            
            for i, row in enumerate(results[:preview_lines]):
                self.result_text.insert(tk.END, f"记录 {i+1}:\n")
                for key, value in row.items():
                    self.result_text.insert(tk.END, f"  {key}: {value}\n")
                self.result_text.insert(tk.END, "\n")
            
            if len(results) > preview_lines:
                self.result_text.insert(tk.END, f"... 还有 {len(results) - preview_lines} 条记录\n")
            
            # 启用导出按钮
            for widget in self.root.winfo_children():
                for child in widget.winfo_children():
                    if isinstance(child, ttk.Frame):
                        for btn in child.winfo_children():
                            if isinstance(btn, ttk.Button) and btn.cget("text") == "导出为CSV":
                                btn.config(state=tk.NORMAL)
        else:
            self.result_text.insert(tk.END, "查询成功，但没有找到匹配的记录\n")
    
    def _on_query_error(self, error_msg: str):
        """查询错误回调"""
        # 重置进度条为0
        self.progress_bar['value'] = 0
        self.progress_var.set("")
        self.query_btn.config(state=tk.NORMAL)
        self.result_text.delete(1.0, tk.END)
        self.result_text.insert(tk.END, f"查询失败: {error_msg}\n")
        messagebox.showerror("错误", f"查询失败: {error_msg}")
    
    def export_csv(self):
        """导出为CSV"""
        if not self.query_results:
            messagebox.showwarning("警告", "没有可导出的数据")
            return
        
        try:
            # 根据查询类型选择不同的导出流程
            if self.query_type == "wide_table":
                # 宽表使用单独的导出流程
                formatted_results = CSVExporter.prepare_wide_table_for_export(self.query_results)
            else:
                # 普通查询使用原有流程
                formatted_results = CSVExporter.prepare_for_export(self.query_results)
            
            # 选择保存路径
            filename = CSVExporter.generate_filename("query_result")
            file_path = filedialog.asksaveasfilename(
                title="保存CSV文件",
                defaultextension=".csv",
                initialfile=filename,
                filetypes=[("CSV files", "*.csv"), ("All files", "*.*")]
            )
            
            if file_path:
                if CSVExporter.export_to_csv(formatted_results, file_path):
                    messagebox.showinfo("成功", f"数据已导出到: {file_path}")
                else:
                    messagebox.showerror("错误", "导出失败")
        except Exception as e:
            logger.error(f"Export error: {e}")
            messagebox.showerror("错误", f"导出失败: {e}")
    
    def save_config(self):
        """保存配置"""
        config = {
            "ssh_command": self.ssh_command_var.get(),
            "db_path": self.db_path_var.get(),
        }
        
        # 保存密码（使用base64编码，非明文存储）
        password = self.password_var.get()
        if password:
            # 使用base64编码密码（简单编码，不是真正的加密）
            encoded_password = base64.b64encode(password.encode('utf-8')).decode('ascii')
            config["password_encoded"] = encoded_password
        
        # 保存部署相关配置（如果存在）
        if hasattr(self, 'deploy_config_path') and self.deploy_config_path:
            config["deploy_config_path"] = str(self.deploy_config_path)
        if hasattr(self, 'deploy_topo_path') and self.deploy_topo_path:
            config["deploy_topo_path"] = str(self.deploy_topo_path)
        if hasattr(self, 'deploy_binary_path') and self.deploy_binary_path:
            config["deploy_binary_path"] = str(self.deploy_binary_path)
        
        try:
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(config, f, indent=2, ensure_ascii=False)
            config_path = str(self.config_file.absolute())
            messagebox.showinfo("成功", f"配置已保存\n\n保存位置:\n{config_path}")
        except Exception as e:
            logger.error(f"Save config error: {e}")
            messagebox.showerror("错误", f"保存配置失败: {e}")
    
    def load_config(self):
        """加载配置"""
        if self.config_file.exists():
            try:
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    config = json.load(f)
                
                # 兼容旧格式配置
                if "ssh_command" in config:
                    self.ssh_command_var.set(config.get("ssh_command", ""))
                elif "host" in config:
                    # 从旧格式构建SSH命令
                    host = config.get("host", "")
                    port = config.get("port", "22")
                    username = config.get("username", "root")
                    if host:
                        if port and port != "22":
                            self.ssh_command_var.set(f"ssh {username}@{host} -p {port}")
                        else:
                            self.ssh_command_var.set(f"ssh {username}@{host}")
                
                self.db_path_var.set(config.get("db_path", "/mnt/analysis/data/device_data.db"))
                
                # 加载密码（如果保存了）
                if "password_encoded" in config:
                    try:
                        decoded_password = base64.b64decode(config["password_encoded"]).decode('utf-8')
                        self.password_var.set(decoded_password)
                    except Exception as e:
                        logger.warning(f"Failed to decode password: {e}")
                
                # 加载部署相关配置
                if "deploy_config_path" in config:
                    self.deploy_config_path = Path(config["deploy_config_path"])
                if "deploy_topo_path" in config:
                    self.deploy_topo_path = Path(config["deploy_topo_path"])
                if "deploy_binary_path" in config:
                    self.deploy_binary_path = Path(config["deploy_binary_path"])
            except Exception as e:
                logger.warning(f"Load config error: {e}")
        
        # 初始化部署路径变量（如果不存在）
        if not hasattr(self, 'deploy_config_path'):
            self.deploy_config_path = None
        if not hasattr(self, 'deploy_topo_path'):
            self.deploy_topo_path = None
        if not hasattr(self, 'deploy_binary_path'):
            self.deploy_binary_path = None
    
    def show_deploy_dialog(self):
        """显示部署对话框"""
        if not self.ssh_client or not self.ssh_client.client:
            messagebox.showerror("错误", "请先连接SSH服务器")
            return
        
        # 创建部署对话框
        dialog = tk.Toplevel(self.root)
        dialog.title("部署/更新程序")
        dialog.geometry("600x750")  # 增加窗口高度，确保按钮可见
        dialog.transient(self.root)
        dialog.grab_set()
        
        # 主框架
        main_frame = ttk.Frame(dialog, padding="10")
        main_frame.pack(fill=tk.BOTH, expand=True)
        
        # 部署配置
        config_frame = ttk.LabelFrame(main_frame, text="部署配置", padding="10")
        config_frame.pack(fill=tk.X, pady=5)
        
        # 上传配置文件
        self.upload_config_var = tk.BooleanVar(value=True)
        ttk.Checkbutton(config_frame, text="上传配置文件 (config.toml)", 
                       variable=self.upload_config_var).pack(anchor=tk.W, pady=2)
        
        # 上传拓扑文件
        self.upload_topo_var = tk.BooleanVar(value=True)
        ttk.Checkbutton(config_frame, text="上传拓扑文件 (topo.json)", 
                       variable=self.upload_topo_var).pack(anchor=tk.W, pady=2)
        
        # 运行用户
        user_frame = ttk.Frame(config_frame)
        user_frame.pack(fill=tk.X, pady=5)
        ttk.Label(user_frame, text="运行用户:").pack(side=tk.LEFT, padx=5)
        self.use_root_var = tk.BooleanVar(value=True)  # 默认使用root用户
        ttk.Radiobutton(user_frame, text="普通用户 (analysis)", 
                       variable=self.use_root_var, value=False).pack(side=tk.LEFT, padx=5)
        ttk.Radiobutton(user_frame, text="Root用户", 
                       variable=self.use_root_var, value=True).pack(side=tk.LEFT, padx=5)
        
        # 启动服务
        self.start_service_var = tk.BooleanVar(value=True)
        ttk.Checkbutton(config_frame, text="部署后启动服务", 
                       variable=self.start_service_var).pack(anchor=tk.W, pady=2)
        
        # 部署状态显示
        status_frame = ttk.LabelFrame(main_frame, text="部署日志", padding="10")
        status_frame.pack(fill=tk.BOTH, expand=True, pady=5)
        
        # 日志文本框
        log_text = tk.Text(status_frame, height=15, wrap=tk.WORD)
        log_text.pack(fill=tk.BOTH, expand=True)
        
        scrollbar = ttk.Scrollbar(status_frame, orient=tk.VERTICAL, command=log_text.yview)
        scrollbar.pack(side=tk.RIGHT, fill=tk.Y)
        log_text.configure(yscrollcommand=scrollbar.set)
        
        # 配置文本标签
        log_text.tag_config("success", foreground="green")
        log_text.tag_config("error", foreground="red")
        log_text.tag_config("warning", foreground="orange")
        
        # 文件路径显示和修改按钮
        file_frame = ttk.LabelFrame(main_frame, text="文件路径", padding="10")
        file_frame.pack(fill=tk.X, pady=5)
        
        # 可执行文件路径
        binary_frame = ttk.Frame(file_frame)
        binary_frame.pack(fill=tk.X, pady=2)
        ttk.Label(binary_frame, text="可执行文件:").pack(side=tk.LEFT, padx=5)
        # 使用Label组件支持换行，wraplength会根据窗口大小动态调整
        binary_path_label = tk.Label(binary_frame, text="未设置", foreground="gray", 
                                    anchor=tk.W, justify=tk.LEFT, wraplength=400)
        binary_path_label.pack(side=tk.LEFT, padx=5, fill=tk.X, expand=True)
        ttk.Button(binary_frame, text="修改", command=lambda: self.select_binary_file(dialog, binary_path_label)).pack(side=tk.RIGHT, padx=5)
        
        # 配置文件路径
        config_frame = ttk.Frame(file_frame)
        config_frame.pack(fill=tk.X, pady=2)
        ttk.Label(config_frame, text="配置文件:").pack(side=tk.LEFT, padx=5)
        config_path_label = tk.Label(config_frame, text="未设置", foreground="gray",
                                     anchor=tk.W, justify=tk.LEFT, wraplength=400)
        config_path_label.pack(side=tk.LEFT, padx=5, fill=tk.X, expand=True)
        ttk.Button(config_frame, text="修改", command=lambda: self.select_config_file(dialog, config_path_label)).pack(side=tk.RIGHT, padx=5)
        
        # 拓扑文件路径
        topo_frame = ttk.Frame(file_frame)
        topo_frame.pack(fill=tk.X, pady=2)
        ttk.Label(topo_frame, text="拓扑文件:").pack(side=tk.LEFT, padx=5)
        topo_path_label = tk.Label(topo_frame, text="未设置", foreground="gray",
                                  anchor=tk.W, justify=tk.LEFT, wraplength=400)
        topo_path_label.pack(side=tk.LEFT, padx=5, fill=tk.X, expand=True)
        ttk.Button(topo_frame, text="修改", command=lambda: self.select_topo_file(dialog, topo_path_label)).pack(side=tk.RIGHT, padx=5)
        
        # 绑定窗口大小变化事件，动态调整wraplength
        def update_wraplength(event=None):
            # 获取文件框架的宽度，减去标签、按钮和边距的宽度
            frame_width = file_frame.winfo_width()
            if frame_width > 1:  # 确保窗口已经渲染
                # 估算可用宽度：总宽度 - 标签宽度(80) - 按钮宽度(60) - 边距(40)
                available_width = max(200, frame_width - 180)
                binary_path_label.config(wraplength=available_width)
                config_path_label.config(wraplength=available_width)
                topo_path_label.config(wraplength=available_width)
        
        dialog.bind('<Configure>', update_wraplength)
        file_frame.bind('<Configure>', update_wraplength)
        
        # 初始化文件路径显示
        self.update_file_paths(binary_path_label, config_path_label, topo_path_label)
        
        # 按钮框架
        button_frame = ttk.Frame(main_frame)
        button_frame.pack(fill=tk.X, pady=10)
        
        deploy_button = ttk.Button(button_frame, text="开始部署", 
                                  command=lambda: self.execute_deploy(dialog, log_text, deploy_button))
        deploy_button.pack(side=tk.LEFT, padx=5)
        
        close_button = ttk.Button(button_frame, text="关闭", command=dialog.destroy)
        close_button.pack(side=tk.RIGHT, padx=5)
        
        # 初始化日志
        log_text.insert(tk.END, "准备部署...\n")
        log_text.insert(tk.END, "请配置部署选项，然后点击'开始部署'按钮\n\n")
        log_text.see(tk.END)
        
        # 保存标签引用以便后续更新
        dialog.binary_path_label = binary_path_label
        dialog.config_path_label = config_path_label
        dialog.topo_path_label = topo_path_label
    
    def execute_deploy(self, dialog, log_text, deploy_button):
        """执行部署（异步）"""
        import threading
        import logging
        
        # 创建自定义日志处理器，将日志输出到UI
        class UITextHandler(logging.Handler):
            def __init__(self, log_text_widget, dialog_window):
                super().__init__()
                self.log_text = log_text_widget
                self.dialog = dialog_window
                # 只捕获INFO及以上级别的日志
                self.setLevel(logging.INFO)
                # 设置格式，只显示消息部分（去掉时间戳和模块名）
                formatter = logging.Formatter('%(message)s')
                self.setFormatter(formatter)
            
            def emit(self, record):
                try:
                    # 过滤掉paramiko的详细连接日志，只保留关键信息
                    if 'paramiko.transport' in record.name:
                        # 只显示连接成功和认证成功的消息
                        if 'Connected' in record.getMessage() or 'Authentication' in record.getMessage():
                            msg = f"SSH连接: {record.getMessage()}"
                        elif 'sftp' in record.name.lower() and 'Opened' in record.getMessage():
                            # 跳过SFTP连接打开的详细日志
                            return
                        else:
                            # 跳过其他paramiko详细日志
                            return
                    else:
                        msg = self.format(record)
                    
                    # 在UI线程中更新日志（使用闭包避免lambda问题）
                    def update_ui(message=msg):
                        self.log_text.insert(tk.END, f"{message}\n")
                        self.log_text.see(tk.END)
                    
                    self.dialog.after(0, update_ui)
                except Exception:
                    pass  # 忽略日志处理错误
        
        def deploy_thread():
            # 添加自定义日志处理器
            ui_handler = UITextHandler(log_text, dialog)
            # 只捕获部署和SSH相关的日志
            deploy_logger = logging.getLogger('query_tool.core.deploy')
            ssh_logger = logging.getLogger('query_tool.core.ssh_client')
            
            # 保存原始级别（如果logger没有显式设置级别，level会是NOTSET）
            deploy_level = deploy_logger.level if deploy_logger.level != logging.NOTSET else None
            ssh_level = ssh_logger.level if ssh_logger.level != logging.NOTSET else None
            
            # 确保日志级别足够低以捕获INFO日志
            deploy_logger.setLevel(logging.INFO)
            ssh_logger.setLevel(logging.INFO)
            
            # 添加处理器
            deploy_logger.addHandler(ui_handler)
            ssh_logger.addHandler(ui_handler)
            
            try:
                # 禁用部署按钮
                deploy_button.config(state=tk.DISABLED)
                
                def log(msg, tag=None):
                    # 在UI线程中更新日志
                    dialog.after(0, lambda: (
                        log_text.insert(tk.END, f"{msg}\n", tag),
                        log_text.see(tk.END)
                    ))
                
                log("=" * 50)
                log("开始部署流程...")
                
                # 创建部署器
                from pathlib import Path
                project_root = Path(__file__).parent.parent.parent.parent
                
                # 使用保存的路径或查找默认位置
                binary_path = None
                if self.deploy_binary_path and self.deploy_binary_path.exists():
                    binary_path = self.deploy_binary_path
                    log(f"使用保存的可执行文件: {binary_path}")
                else:
                    # 尝试查找默认位置
                    for possible_path in [
                        project_root / "target" / "release" / "analysis-collector",
                        project_root / "target" / "x86_64-unknown-linux-musl" / "release" / "analysis-collector",
                        project_root / "target" / "aarch64-unknown-linux-musl" / "release" / "analysis-collector",
                        project_root / "target" / "aarch64-unknown-linux-gnu" / "release" / "analysis-collector",
                    ]:
                        if possible_path.exists():
                            binary_path = possible_path
                            self.deploy_binary_path = binary_path
                            log(f"找到可执行文件: {binary_path}")
                            break
                
                if not binary_path:
                    # 需要在主线程中选择文件
                    log("✗ 未找到可执行文件，请先通过'修改'按钮选择文件", "error")
                    deploy_button.config(state=tk.NORMAL)
                    return
                
                log(f"使用可执行文件: {binary_path}")
                
                # 使用保存的配置文件路径
                config_path = None
                if self.upload_config_var.get():
                    if self.deploy_config_path and self.deploy_config_path.exists():
                        config_path = self.deploy_config_path
                        log(f"使用保存的配置文件: {config_path}")
                    else:
                        # 尝试查找默认位置
                        default_config = project_root / "config.toml.example"
                        if not default_config.exists():
                            default_config = project_root / "config.toml"
                        
                        if default_config.exists():
                            config_path = default_config
                            self.deploy_config_path = config_path
                            log(f"使用默认配置文件: {config_path}")
                        else:
                            log("警告: 未找到配置文件，将跳过配置文件上传", "warning")
                            self.upload_config_var.set(False)
                
                # 使用保存的拓扑文件路径
                topo_path = None
                if self.upload_topo_var.get():
                    if self.deploy_topo_path and self.deploy_topo_path.exists():
                        topo_path = self.deploy_topo_path
                        log(f"使用保存的拓扑文件: {topo_path}")
                    else:
                        # 尝试查找默认位置
                        default_topo = project_root / "topo.json"
                        if default_topo.exists():
                            topo_path = default_topo
                            self.deploy_topo_path = topo_path
                            log(f"使用默认拓扑文件: {topo_path}")
                        else:
                            log("警告: 未找到拓扑文件，将跳过拓扑文件上传", "warning")
                            self.upload_topo_var.set(False)
                
                deployer = Deployer(self.ssh_client, project_root)
                
                # 检查远程状态
                log("检查远程服务器状态...")
                status = deployer.check_remote_status()
                if status["installed"]:
                    log("检测到已部署的程序，将执行更新操作")
                else:
                    log("未检测到已部署的程序，将执行新部署操作")
                
                # 执行部署
                success, message, log_messages = deployer.deploy(
                    binary_path=binary_path,
                    config_path=config_path if self.upload_config_var.get() else None,
                    topo_path=topo_path if self.upload_topo_var.get() else None,
                    use_root=self.use_root_var.get(),
                    start_service=self.start_service_var.get()
                )
                
                # 显示日志
                for log_msg in log_messages:
                    log(log_msg)
                
                log("=" * 50)
                if success:
                    log(f"✓ {message}", "success")
                    dialog.after(0, lambda: messagebox.showinfo("成功", message))
                else:
                    log(f"✗ 部署失败: {message}", "error")
                    dialog.after(0, lambda: messagebox.showerror("部署失败", message))
                
            except Exception as e:
                logger.error(f"部署异常: {e}")
                error_msg = f"部署异常: {e}"
                log_text.insert(tk.END, f"✗ {error_msg}\n", "error")
                log_text.see(tk.END)
                dialog.after(0, lambda: messagebox.showerror("错误", error_msg))
            finally:
                # 移除日志处理器并恢复原始级别
                deploy_logger.removeHandler(ui_handler)
                ssh_logger.removeHandler(ui_handler)
                if deploy_level is not None:
                    deploy_logger.setLevel(deploy_level)
                else:
                    deploy_logger.setLevel(logging.NOTSET)
                if ssh_level is not None:
                    ssh_logger.setLevel(ssh_level)
                else:
                    ssh_logger.setLevel(logging.NOTSET)
                # 重新启用部署按钮
                deploy_button.config(state=tk.NORMAL)
        
        # 在后台线程中执行部署
        thread = threading.Thread(target=deploy_thread, daemon=True)
        thread.start()
    
    def update_file_paths(self, binary_label, config_label, topo_label):
        """更新文件路径显示"""
        if self.deploy_binary_path and self.deploy_binary_path.exists():
            binary_label.config(text=str(self.deploy_binary_path), foreground="black")
        else:
            binary_label.config(text="未设置", foreground="gray")
        
        if self.deploy_config_path and self.deploy_config_path.exists():
            config_label.config(text=str(self.deploy_config_path), foreground="black")
        else:
            config_label.config(text="未设置", foreground="gray")
        
        if self.deploy_topo_path and self.deploy_topo_path.exists():
            topo_label.config(text=str(self.deploy_topo_path), foreground="black")
        else:
            topo_label.config(text="未设置", foreground="gray")
    
    def select_binary_file(self, dialog, label):
        """选择可执行文件"""
        from pathlib import Path
        project_root = Path(__file__).parent.parent.parent.parent
        
        default_binary = self.deploy_binary_path if self.deploy_binary_path and self.deploy_binary_path.exists() else None
        if not default_binary:
            for possible_path in [
                project_root / "target" / "release" / "analysis-collector",
                project_root / "target" / "x86_64-unknown-linux-musl" / "release" / "analysis-collector",
                project_root / "target" / "aarch64-unknown-linux-musl" / "release" / "analysis-collector",
                project_root / "target" / "aarch64-unknown-linux-gnu" / "release" / "analysis-collector",
            ]:
                if possible_path.exists():
                    default_binary = possible_path
                    break
        
        file_path = filedialog.askopenfilename(
            title="选择可执行文件",
            initialdir=str(default_binary.parent if default_binary else project_root / "target"),
            initialfile=default_binary.name if default_binary else "",
            filetypes=[
                ("所有文件", "*.*"),
                ("可执行文件", "*"),
            ]
        )
        
        if file_path:
            self.deploy_binary_path = Path(file_path)
            label.config(text=str(self.deploy_binary_path), foreground="black")
            self.save_config()  # 自动保存配置
    
    def select_config_file(self, dialog, label):
        """选择配置文件"""
        from pathlib import Path
        project_root = Path(__file__).parent.parent.parent.parent
        
        default_config = self.deploy_config_path if self.deploy_config_path and self.deploy_config_path.exists() else None
        if not default_config:
            default_config = project_root / "config.toml.example"
            if not default_config.exists():
                default_config = project_root / "config.toml"
        
        file_path = filedialog.askopenfilename(
            title="选择配置文件",
            initialdir=str(project_root),
            initialfile=default_config.name if default_config.exists() else "config.toml",
            filetypes=[
                ("配置文件", "*.toml"),
                ("所有文件", "*.*")
            ]
        )
        
        if file_path:
            self.deploy_config_path = Path(file_path)
            label.config(text=str(self.deploy_config_path), foreground="black")
            self.save_config()  # 自动保存配置
    
    def select_topo_file(self, dialog, label):
        """选择拓扑文件"""
        from pathlib import Path
        project_root = Path(__file__).parent.parent.parent.parent
        
        default_topo = self.deploy_topo_path if self.deploy_topo_path and self.deploy_topo_path.exists() else None
        if not default_topo:
            default_topo = project_root / "topo.json"
        
        file_path = filedialog.askopenfilename(
            title="选择拓扑文件",
            initialdir=str(project_root),
            initialfile=default_topo.name if default_topo.exists() else "topo.json",
            filetypes=[
                ("JSON文件", "*.json"),
                ("所有文件", "*.*")
            ]
        )
        
        if file_path:
            self.deploy_topo_path = Path(file_path)
            label.config(text=str(self.deploy_topo_path), foreground="black")
            self.save_config()  # 自动保存配置


def main():
    """启动UI应用"""
    import sys
    import os
    
    # 设置环境变量确保UTF-8编码
    os.environ.setdefault('LANG', 'en_US.UTF-8')
    os.environ.setdefault('LC_ALL', 'en_US.UTF-8')
    
    root = tk.Tk()
    
    # 窗口标题：某些窗口管理器可能不支持UTF-8中文，使用英文标题更可靠
    # 如果需要中文标题，可以取消下面的注释，但可能在某些系统上显示为乱码
    root.title("Database Query Tool")  # 英文标题，避免乱码
    # root.title("数据库查询工具")  # 中文标题（如果窗口管理器支持UTF-8）
    
    app = QueryToolUI(root)
    root.mainloop()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    main()
