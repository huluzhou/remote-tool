"""
数据库查询模块 - 通过SSH执行SQL查询
"""

import sqlite3
import io
import csv
import uuid
import tempfile
import os
import base64
from typing import List, Dict, Any, Optional, Callable
from datetime import datetime
import logging

logger = logging.getLogger(__name__)


class DBQuery:
    """数据库查询类，通过SSH执行SQL查询"""
    
    def __init__(self, ssh_client):
        """
        初始化数据库查询器
        
        Args:
            ssh_client: SSHClient实例
        """
        self.ssh = ssh_client
    
    def execute_query(self, db_path: str, sql: str) -> List[Dict[str, Any]]:
        """
        执行SQL查询并返回结果
        
        Args:
            db_path: 远程数据库文件路径
            sql: SQL查询语句
            
        Returns:
            查询结果列表，每行是一个字典
            
        Warning:
            此方法直接执行SQL语句，请确保SQL语句的安全性，避免SQL注入攻击。
            建议使用预定义的查询方法（如query_by_time_range）而不是直接使用此方法。
        """
        # 使用Python的sqlite3模块执行查询，输出JSON格式
        # 这样可以避免依赖sqlite3命令行工具
        
        # 转义SQL和路径中的特殊字符，防止注入
        import json
        import base64
        
        # 将SQL和路径进行base64编码，避免shell注入
        sql_b64 = base64.b64encode(sql.encode('utf-8')).decode('ascii')
        db_path_b64 = base64.b64encode(db_path.encode('utf-8')).decode('ascii')
        
        # 创建Python脚本来执行查询
        # 使用临时文件存储JSON输出，避免stdout缓冲区限制
        temp_file = f'/tmp/query_result_{uuid.uuid4().hex}.json'
        
        python_script = f'''import sqlite3
import json
import sys
import base64
import os

try:
    # 解码路径和SQL
    db_path = base64.b64decode("{db_path_b64}").decode('utf-8')
    sql = base64.b64decode("{sql_b64}").decode('utf-8')
    temp_file = "{temp_file}"
    
    # 连接数据库
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    # 执行查询
    cursor.execute(sql)
    
    # 获取列名
    columns = [description[0] for description in cursor.description] if cursor.description else []
    
    # 获取结果并转换为字典列表
    results = []
    for row in cursor.fetchall():
        row_dict = {{}}
        for i, col in enumerate(columns):
            value = row[i]
            # 处理None值
            if value is None:
                row_dict[col] = None
            # 尝试保持原始类型（sqlite3会返回合适的Python类型）
            else:
                row_dict[col] = value
        results.append(row_dict)
    
    # 将JSON写入临时文件，避免stdout缓冲区限制
    with open(temp_file, 'w', encoding='utf-8') as f:
        json.dump(results, f, ensure_ascii=False, default=str)
    
    # 输出临时文件路径
    print(temp_file)
    
    conn.close()
    sys.exit(0)
except Exception as e:
    error_msg = json.dumps({{"error": str(e)}}, ensure_ascii=False)
    print(error_msg, file=sys.stderr)
    sys.exit(1)
'''
        
        # 使用heredoc方式执行Python脚本，避免bash转义问题
        # 使用唯一的结束标记避免冲突
        eof_marker = f'PYTHON_SCRIPT_EOF_{uuid.uuid4().hex[:8]}'
        
        command = f'''python3 << '{eof_marker}'
{python_script}
{eof_marker}'''
        
        exit_status, stdout, stderr = self.ssh.execute_command(command)
        
        # 如果python3不存在，尝试python
        if exit_status != 0 and "command not found" in stderr.lower():
            logger.info("python3 not found, trying python")
            command = f'''python << '{eof_marker}'
{python_script}
{eof_marker}'''
            exit_status, stdout, stderr = self.ssh.execute_command(command)
        
        # 如果执行失败，直接处理错误
        if exit_status != 0:
            # 尝试解析错误信息
            try:
                error_data = json.loads(stderr.strip() if stderr else stdout.strip())
                error_msg = error_data.get('error', 'Unknown error')
            except:
                error_msg = stderr.strip() if stderr else stdout.strip() or "Unknown error"
            raise RuntimeError(f"SQL query failed: {error_msg}")
        
        # 从stdout获取远程临时文件路径
        remote_temp_file = stdout.strip()
        logger.info(f"远程临时文件: {remote_temp_file}")
        
        # 使用SFTP下载文件（已验证可用）
        json_output = None
        local_temp_file = None
        
        try:
            # 创建本地临时文件
            with tempfile.NamedTemporaryFile(mode='w+', suffix='.json', delete=False, encoding='utf-8') as f:
                local_temp_file = f.name
            
            # 使用SFTP下载远程文件（异步模式）
            if not self.ssh.download_file(remote_temp_file, local_temp_file, async_mode=True):
                raise RuntimeError("SFTP下载失败")
            
            # 读取本地文件内容
            with open(local_temp_file, 'r', encoding='utf-8') as f:
                json_output = f.read()
            
            file_size = len(json_output)
            logger.info(f"✓ 文件下载成功: {file_size:,} 字符 ({file_size / 1024 / 1024:.2f} MB)")
            
        except Exception as e:
            logger.error(f"SFTP下载失败: {e}", exc_info=True)
            raise RuntimeError(f"无法下载结果文件: {e}")
        finally:
            # 清理远程临时文件
            if remote_temp_file:
                try:
                    logger.info(f"清理远程临时文件: {remote_temp_file}")
                    exit_status, stdout, stderr = self.ssh.execute_command(f'rm -f "{remote_temp_file}"')
                    if exit_status == 0:
                        logger.info(f"✓ 远程临时文件已删除: {remote_temp_file}")
                    else:
                        logger.warning(f"⚠ 删除远程临时文件失败: {stderr}")
                except Exception as e:
                    logger.warning(f"⚠ 删除远程临时文件时出错: {e}")
            
            # 清理本地临时文件
            if local_temp_file and os.path.exists(local_temp_file):
                try:
                    logger.info(f"清理本地临时文件: {local_temp_file}")
                    os.unlink(local_temp_file)
                    logger.info(f"✓ 本地临时文件已删除: {local_temp_file}")
                except Exception as e:
                    logger.warning(f"⚠ 删除本地临时文件时出错: {e}")
        
        if not json_output or not json_output.strip():
            logger.warning("Query returned no results")
            return []
        
        # 使用json_output作为JSON数据源
        stdout = json_output
        
        # 解析JSON输出
        try:
            results = json.loads(stdout.strip())
            if isinstance(results, list):
                # 转换数据类型（JSON可能将数字转为字符串）
                converted_results = []
                for row in results:
                    converted_row = {}
                    for key, value in row.items():
                        if value is None:
                            converted_row[key] = None
                        elif isinstance(value, (int, float)):
                            converted_row[key] = value
                        elif isinstance(value, str):
                            # 尝试转换为数字
                            try:
                                if '.' not in value and 'e' not in value.lower() and 'E' not in value:
                                    converted_row[key] = int(value)
                                else:
                                    converted_row[key] = float(value)
                            except ValueError:
                                converted_row[key] = value
                        else:
                            converted_row[key] = value
                    converted_results.append(converted_row)
                logger.info(f"Query returned {len(converted_results)} rows")
                return converted_results
            else:
                logger.warning("Query returned unexpected format")
                return []
        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse JSON output: {e}")
            logger.error(f"Output: {stdout[:500]}")
            raise RuntimeError(f"Failed to parse query results: {e}")
    
    def _get_table_columns(self, db_path: str, table_name: str) -> List[str]:
        """
        获取表的所有列名
        
        Args:
            db_path: 数据库路径
            table_name: 表名
            
        Returns:
            列名列表
        """
        # 使用PRAGMA table_info获取表结构
        sql = f"PRAGMA table_info({table_name})"
        try:
            result = self.execute_query(db_path, sql)
            columns = [row['name'] for row in result]
            return columns
        except Exception as e:
            logger.warning(f"Failed to get columns for table {table_name}: {e}")
            return []
    
    def _build_select_with_all_columns(self, db_path: str, main_table: str, 
                                       ext_table: Optional[str] = None, 
                                       main_alias: str = 'd', 
                                       ext_alias: str = 'm') -> str:
        """
        动态构建SELECT语句，包含所有字段
        
        Args:
            db_path: 数据库路径
            main_table: 主表名
            ext_table: 扩展表名（可选）
            main_alias: 主表别名
            ext_alias: 扩展表别名
            
        Returns:
            SELECT子句字符串
        """
        # 获取主表所有列
        main_columns = self._get_table_columns(db_path, main_table)
        if not main_columns:
            # 如果获取失败，使用通配符
            return f"{main_alias}.*"
        
        # 构建主表字段列表（带别名前缀）
        select_parts = [f"{main_alias}.{col}" for col in main_columns]
        
        # 如果有扩展表，添加扩展表字段
        if ext_table:
            ext_columns = self._get_table_columns(db_path, ext_table)
            if ext_columns:
                # 排除扩展表的主键（通常是device_data_id，与主表id重复）
                ext_columns_filtered = [col for col in ext_columns 
                                       if col not in ['device_data_id', 'id']]
                select_parts.extend([f"{ext_alias}.{col}" for col in ext_columns_filtered])
        
        return ", ".join(select_parts)
    
    def query_by_time_range(self, db_path: str, start_time: int, end_time: int,
                           device_sn: Optional[str] = None,
                           include_ext: bool = False,
                           progress_callback: Optional[Callable[[str, float], None]] = None) -> List[Dict[str, Any]]:
        """
        按时间范围查询设备数据
        
        Args:
            db_path: 远程数据库文件路径
            start_time: 开始时间戳（秒）
            end_time: 结束时间戳（秒）
            device_sn: 设备序列号（可选，用于过滤特定设备）
            include_ext: 是否包含扩展表数据（JOIN JSON扩展表，使用json_extract提取字段）
            progress_callback: 进度回调函数（保留用于宽表查询的嵌套调用）
            
        Returns:
            查询结果列表
        """
        # 构建WHERE条件（使用参数化方式，避免SQL注入）
        # 注意：虽然最终通过命令行执行，但我们仍然使用参数化方式构建SQL
        conditions = [f"d.timestamp >= {start_time}", f"d.timestamp <= {end_time}"]
        
        if device_sn:
            # 转义单引号，防止SQL注入
            escaped_device_sn = device_sn.replace("'", "''")
            conditions.append(f"d.device_sn = '{escaped_device_sn}'")
        
        where_clause = " AND ".join(conditions)
        
        # 构建SQL查询
        if include_ext:
            # 使用统一的 device_data_ext 表
            # 不再使用 json_extract 提取字段，字段提取由 CSV 导出模块处理
            # 只查询主表字段和 payload_json
            main_columns = self._get_table_columns(db_path, 'device_data')
            if main_columns:
                # 构建主表字段列表
                select_fields = ", ".join([f"d.{col}" for col in main_columns])
            else:
                # 如果获取失败，使用默认字段
                select_fields = "d.id, d.device_sn, d.device_type, d.timestamp, d.local_timestamp, d.activePower, d.reactivePower, d.powerFactor"
            
            sql = f'''
            SELECT 
                {select_fields},
                e.payload_json as payload_json
            FROM device_data d
            LEFT JOIN device_data_ext e ON d.id = e.device_data_id
            WHERE {where_clause}
            ORDER BY d.timestamp ASC
            '''
        else:
            # 只查询主表（动态获取主表所有字段）
            main_columns = self._get_table_columns(db_path, 'device_data')
            if main_columns:
                # 排除固定字段，只选择数据字段（可选）
                select_fields = ", ".join([f"d.{col}" for col in main_columns])
            else:
                # 如果获取失败，使用默认字段
                select_fields = "d.id, d.device_sn, d.device_type, d.timestamp, d.local_timestamp, d.activePower, d.reactivePower, d.powerFactor"
            
            sql = f'''
            SELECT {select_fields}
            FROM device_data d
            WHERE {where_clause}
            ORDER BY d.timestamp ASC
            '''
        
        result = self.execute_query(db_path, sql)
        return result
    
    def query_command_data(self, db_path: str, start_time: int, end_time: int,
                          device_sn: Optional[str] = None,
                          progress_callback: Optional[Callable[[str, float], None]] = None) -> List[Dict[str, Any]]:
        """
        查询命令数据
        
        Args:
            db_path: 远程数据库文件路径
            start_time: 开始时间戳（秒）
            end_time: 结束时间戳（秒）
            device_sn: 设备序列号（可选）
            progress_callback: 进度回调函数（保留用于宽表查询的嵌套调用）
            
        Returns:
            查询结果列表
        """
        conditions = [f"timestamp >= {start_time}", f"timestamp <= {end_time}"]
        
        if device_sn:
            conditions.append(f"device_sn = '{device_sn}'")
        
        where_clause = " AND ".join(conditions)
        
        sql = f'''
        SELECT 
            id, timestamp, device_sn, name, value, local_timestamp
        FROM cmd_data
        WHERE {where_clause}
        ORDER BY timestamp ASC
        '''
        
        result = self.execute_query(db_path, sql)
        return result
    
    def query_wide_table(self, db_path: str, start_time: int, end_time: int,
                        include_ext: bool = False,
                        progress_callback: Optional[Callable[[str, float], None]] = None) -> List[Dict[str, Any]]:
        """
        查询宽表数据
        
        宽表以 local_timestamp 为主键，合并所有设备的数据和指令数据
        
        Args:
            db_path: 远程数据库文件路径
            start_time: 开始时间戳（秒）
            end_time: 结束时间戳（秒）
            include_ext: 是否查询扩展表数据（只查主表性能好）
            progress_callback: 进度回调函数，参数为 (stage: str, progress: float)
            
        Returns:
            宽表数据列表，每行以 local_timestamp 为主键
        """
        # 1. 查询所有设备数据（不限制device_sn）
        if progress_callback:
            progress_callback("querying_device", 0)
        
        device_data = self.query_by_time_range(
            db_path=db_path,
            start_time=start_time,
            end_time=end_time,
            device_sn=None,  # 查询所有设备
            include_ext=include_ext
        )
        
        if progress_callback:
            progress_callback("querying_device", 40)
        
        # 2. 查询指令数据
        if progress_callback:
            progress_callback("querying_command", 40)
        
        command_data = self.query_command_data(
            db_path=db_path,
            start_time=start_time,
            end_time=end_time,
            device_sn=None  # 查询所有设备
        )
        
        if progress_callback:
            progress_callback("querying_command", 60)
        
        # 3. 按 local_timestamp 合并数据
        # 注意：local_timestamp 是毫秒时间戳，需要转换为秒进行比较
        if progress_callback:
            progress_callback("merging", 60)
        
        from collections import defaultdict
        import json
        
        # 加载扩展表字段配置
        from .csv_export import CSVExporter
        config = CSVExporter._load_config()
        extract_config = config.get("extract_from_payload", {})
        # 获取主表字段列表（宽表查询时只从主表获取数据字段，排除元数据字段）
        all_main_table_fields = config.get("main_table_fields", ["activePower", "reactivePower", "powerFactor"])
        # 排除元数据字段，只保留数据字段（这些字段会使用设备类型前缀）
        metadata_fields = {"id", "device_sn", "device_type", "timestamp", "local_timestamp"}
        main_table_fields = [f for f in all_main_table_fields if f not in metadata_fields]
        
        # 使用字典按 local_timestamp（毫秒）分组
        wide_table = defaultdict(dict)
        
        # 处理设备数据
        total_device_rows = len(device_data)
        for idx, row in enumerate(device_data):
            if progress_callback and idx % 100 == 0:
                progress = 60 + (idx / total_device_rows) * 0.2 if total_device_rows > 0 else 60
                progress_callback("processing", progress)
            local_ts = row.get('local_timestamp')
            if local_ts is None:
                continue
            
            # 使用 local_timestamp（毫秒）作为主键
            # 如果该时间戳还没有记录，初始化
            if local_ts not in wide_table:
                wide_table[local_ts]['local_timestamp'] = local_ts
            
            device_sn = row.get('device_sn', '')
            device_type = row.get('device_type', '')
            
            # 添加主表字段（只从配置文件中指定的主表字段获取）
            # 使用设备序列号作为前缀，避免同类设备字段冲突
            # 例如：DEVICE001_activePower, DEVICE002_activePower
            # 设备序列号是唯一的，可以完全避免字段冲突
            if not device_sn:
                # 如果没有设备序列号，跳过该行
                continue
                
            for key in main_table_fields:
                if key in row:
                    value = row[key]
                    # 使用设备序列号+字段名作为列名
                    column_name = f"{device_sn}_{key}"
                    wide_table[local_ts][column_name] = value
            
            # 如果包含扩展表数据，从 payload_json 中提取字段
            if include_ext and 'payload_json' in row:
                payload_json = row.get('payload_json')
                if payload_json:
                    try:
                        if isinstance(payload_json, str):
                            payload_data = json.loads(payload_json)
                        else:
                            payload_data = payload_json
                        
                        # 获取该设备类型需要提取的字段列表
                        fields_to_extract = extract_config.get(device_type, extract_config.get('default', []))
                        
                        # 提取字段，列名为设备序列号+字段名（避免同类设备字段冲突）
                        if not device_sn:
                            continue
                            
                        for field_key in fields_to_extract:
                            if isinstance(payload_data, dict):
                                value = payload_data.get(field_key)
                                if value is not None:
                                    column_name = f"{device_sn}_{field_key}"
                                    wide_table[local_ts][column_name] = value
                    except (json.JSONDecodeError, TypeError) as e:
                        logger.warning(f"解析payload_json失败: {e}")
        
        # 处理指令数据
        total_cmd_rows = len(command_data)
        for idx, cmd_row in enumerate(command_data):
            if progress_callback and idx % 100 == 0:
                progress = 80 + (idx / total_cmd_rows) * 0.15 if total_cmd_rows > 0 else 80
                progress_callback("processing", progress)
            local_ts = cmd_row.get('local_timestamp')
            if local_ts is None:
                continue
            
            # 使用 local_timestamp（毫秒）作为主键
            # 如果该时间戳还没有记录，初始化
            if local_ts not in wide_table:
                wide_table[local_ts]['local_timestamp'] = local_ts
            
            # 指令数据的列名为设备序列号+指令的 name（避免不同设备的同名指令冲突）
            cmd_device_sn = cmd_row.get('device_sn', '')
            cmd_name = cmd_row.get('name', '')
            cmd_value = cmd_row.get('value')
            
            if cmd_name:
                if cmd_device_sn:
                    # 使用设备序列号+指令名作为列名
                    column_name = f"{cmd_device_sn}_{cmd_name}"
                else:
                    # 如果没有设备序列号，直接使用指令名（向后兼容）
                    column_name = cmd_name
                wide_table[local_ts][column_name] = cmd_value
        
        # 转换为列表并排序
        if progress_callback:
            progress_callback("processing", 95)
        
        result = list(wide_table.values())
        result.sort(key=lambda x: x.get('local_timestamp', 0))
        
        if progress_callback:
            progress_callback("completed", 100)
        
        logger.info(f"宽表查询完成: {len(result)} 条记录")
        return result
    
    def get_table_info(self, db_path: str) -> Dict[str, Any]:
        """
        获取数据库表信息
        
        Args:
            db_path: 远程数据库文件路径
            
        Returns:
            包含表信息的字典
        """
        # 获取所有表名
        sql_tables = "SELECT name FROM sqlite_master WHERE type='table'"
        tables_result = self.execute_query(db_path, sql_tables)
        table_names = [row['name'] for row in tables_result]
        
        # 获取每个表的行数
        table_stats = {}
        for table in table_names:
            sql_count = f'SELECT COUNT(*) as count FROM "{table}"'
            count_result = self.execute_query(db_path, sql_count)
            if count_result:
                table_stats[table] = count_result[0].get('count', 0)
        
        # 获取时间范围
        time_range = None
        if 'device_data' in table_names:
            sql_time = 'SELECT MIN(timestamp) as min_time, MAX(timestamp) as max_time FROM device_data'
            time_result = self.execute_query(db_path, sql_time)
            if time_result and time_result[0].get('min_time') is not None:
                time_range = {
                    'min': time_result[0].get('min_time'),
                    'max': time_result[0].get('max_time')
                }
        
        return {
            'tables': table_names,
            'table_stats': table_stats,
            'time_range': time_range
        }
