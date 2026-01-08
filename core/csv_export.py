"""
CSV导出模块 - 将查询结果导出为CSV文件
"""

import csv
import json
from typing import List, Dict, Any, Optional
from pathlib import Path
from datetime import datetime
import logging

logger = logging.getLogger(__name__)

# 尝试导入 toml 库
try:
    import toml
except ImportError:
    # 如果toml库未安装，尝试使用tomllib（Python 3.11+）
    try:
        import tomllib as toml
    except ImportError:
        logger.error("需要安装 toml 库: pip install toml")
        raise ImportError("需要安装 toml 库: pip install toml")


class CSVExporter:
    """CSV导出器"""
    
    _config: Optional[Dict[str, Any]] = None
    
    @staticmethod
    def _load_config(config_path: Optional[str] = None) -> Dict[str, Any]:
        """
        加载CSV导出配置文件
        
        Args:
            config_path: 配置文件路径，如果为None则使用默认路径
            
        Returns:
            配置字典
        """
        if CSVExporter._config is not None:
            return CSVExporter._config
        
        if config_path is None:
            # 处理打包后的路径
            import sys
            if getattr(sys, 'frozen', False):
                # 打包后的可执行文件，配置文件在可执行文件同目录
                base_path = Path(sys.executable).parent
                # 配置文件在可执行文件同目录
                config_path = base_path / "csv_export_config.toml"
            else:
                # 开发环境，配置文件在模块目录
                config_path = Path(__file__).parent.parent / "csv_export_config.toml"
        
        config_file = Path(config_path)
        
        if not config_file.exists():
            logger.warning(f"配置文件不存在: {config_path}，使用默认配置")
            CSVExporter._config = {
                "main_table_fields": [
                    "id", "device_sn", "device_type", "timestamp", 
                    "local_timestamp", "activePower", "reactivePower", "powerFactor"
                ],
                "extract_from_payload": {},
                "field_name_mapping": {}
            }
            return CSVExporter._config
        
        try:
            # toml 库使用文本模式，tomllib 使用二进制模式
            if hasattr(toml, 'load'):
                # 使用 toml 库（文本模式）
                with open(config_file, 'r', encoding='utf-8') as f:
                    CSVExporter._config = toml.load(f)
            else:
                # 使用 tomllib（二进制模式）
                with open(config_file, 'rb') as f:
                    CSVExporter._config = toml.load(f)
            logger.info(f"已加载配置文件: {config_path}")
            return CSVExporter._config
        except Exception as e:
            logger.error(f"加载配置文件失败: {e}，使用默认配置")
            CSVExporter._config = {
                "main_table_fields": [
                    "id", "device_sn", "device_type", "timestamp", 
                    "local_timestamp", "activePower", "reactivePower", "powerFactor"
                ],
                "extract_from_payload": {},
                "field_name_mapping": {}
            }
            return CSVExporter._config
    
    @staticmethod
    def filter_and_extract_fields(data: List[Dict[str, Any]], 
                                  config_path: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        过滤数据，只保留主表字段和配置的扩展字段，从payload_json中提取指定字段
        
        Args:
            data: 原始数据列表
            config_path: 配置文件路径
            
        Returns:
            过滤后的数据列表（不包含payload_json列）
        """
        if not data:
            return data
        
        config = CSVExporter._load_config(config_path)
        main_table_fields = config.get("main_table_fields", [])
        extract_config = config.get("extract_from_payload", {})
        field_mapping = config.get("field_name_mapping", {})
        
        result = []
        for row in data:
            new_row = {}
            device_type = row.get('device_type', 'default')
            
            # 1. 保留主表字段
            for field in main_table_fields:
                if field in row:
                    new_row[field] = row[field]
            
            # 2. 从payload_json中提取配置的字段
            payload_json = row.get('payload_json')
            if payload_json:
                try:
                    if isinstance(payload_json, str):
                        payload_data = json.loads(payload_json)
                    else:
                        payload_data = payload_json
                    
                    # 获取该设备类型需要提取的字段列表
                    fields_to_extract = extract_config.get(device_type, extract_config.get('default', []))
                    
                    # 提取字段
                    for field_key in fields_to_extract:
                        # 从JSON中提取字段值
                        if isinstance(payload_data, dict):
                            value = payload_data.get(field_key)
                        else:
                            value = None
                        
                        # 应用字段名映射（如果配置了）
                        output_field_name = field_mapping.get(field_key, field_key)
                        new_row[output_field_name] = value
                        
                except (json.JSONDecodeError, TypeError) as e:
                    logger.warning(f"解析payload_json失败: {e}")
            
            # 注意：不保留payload_json列，也不保留其他未在配置中指定的字段
            # 只保留主表字段和从payload_json中提取的配置字段
            
            result.append(new_row)
        
        return result
    
    @staticmethod
    def export_to_csv(data: List[Dict[str, Any]], output_path: str, 
                     encoding: str = 'utf-8-sig') -> bool:
        """
        将数据导出为CSV文件
        
        Args:
            data: 要导出的数据列表（字典列表）
            output_path: 输出文件路径
            encoding: 文件编码，默认utf-8-sig（Excel兼容）
            
        Returns:
            True if successful, False otherwise
        """
        if not data:
            logger.warning("No data to export")
            return False
        
        try:
            # 确保输出目录存在
            output_file = Path(output_path)
            output_file.parent.mkdir(parents=True, exist_ok=True)
            
            # 收集所有行的所有字段名（避免某些行缺少字段导致错误）
            # 这对于宽表特别重要，因为不同时间戳的行可能包含不同的字段
            all_fieldnames = set()
            for row in data:
                all_fieldnames.update(row.keys())
            
            # 转换为列表并排序，确保列顺序一致
            # local_timestamp 优先，其他列按字母顺序
            fieldnames = sorted(list(all_fieldnames))
            if 'local_timestamp' in fieldnames:
                fieldnames.remove('local_timestamp')
                fieldnames.insert(0, 'local_timestamp')
            
            # 确保所有行都有所有字段（缺失的字段设为None）
            normalized_data = []
            for row in data:
                normalized_row = {}
                for field in fieldnames:
                    normalized_row[field] = row.get(field, None)
                normalized_data.append(normalized_row)
            
            # 写入CSV文件
            with open(output_path, 'w', newline='', encoding=encoding) as f:
                writer = csv.DictWriter(f, fieldnames=fieldnames)
                writer.writeheader()
                writer.writerows(normalized_data)
            
            logger.info(f"Exported {len(data)} rows to {output_path}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to export CSV: {e}")
            return False
    
    @staticmethod
    def generate_filename(prefix: str = "query_result", 
                         extension: str = "csv") -> str:
        """
        生成带时间戳的文件名
        
        Args:
            prefix: 文件名前缀
            extension: 文件扩展名
            
        Returns:
            生成的文件名
        """
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        return f"{prefix}_{timestamp}.{extension}"
    
    @staticmethod
    def format_timestamp(timestamp: int, is_millis: bool = False) -> str:
        """
        格式化时间戳为可读字符串（东八区 UTC+8）
        
        Args:
            timestamp: 时间戳
            is_millis: 是否为毫秒时间戳
            
        Returns:
            格式化后的时间字符串（东八区）
        """
        from datetime import timezone, timedelta
        
        # 东八区时区
        tz_beijing = timezone(timedelta(hours=8))
        
        if is_millis:
            # 提取秒和毫秒部分
            seconds = timestamp / 1000
            milliseconds = timestamp % 1000
            dt = datetime.fromtimestamp(seconds, tz=timezone.utc)
            # 转换为东八区
            dt_beijing = dt.astimezone(tz_beijing)
            # 格式化为 yyyy/m/d h:mm:ss.000 格式
            return dt_beijing.strftime("%Y/%m/%d %H:%M:%S") + f".{milliseconds:03d}"
        else:
            dt = datetime.fromtimestamp(timestamp, tz=timezone.utc)
            # 转换为东八区
            dt_beijing = dt.astimezone(tz_beijing)
            return dt_beijing.strftime("%Y-%m-%d %H:%M:%S")
    
    @staticmethod
    def add_formatted_timestamps(data: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        将时间戳列替换为格式化后的时间戳（东八区，便于阅读）
        只保留格式化后的时间戳，不保留原始时间戳值
        
        Args:
            data: 原始数据列表
            
        Returns:
            时间戳已格式化的数据列表
        """
        result = []
        for row in data:
            new_row = row.copy()
            
            # 替换timestamp列为格式化后的值（东八区）
            if 'timestamp' in row and row['timestamp'] is not None:
                new_row['timestamp'] = CSVExporter.format_timestamp(
                    row['timestamp'], is_millis=False
                )
            
            # 替换local_timestamp列为格式化后的值（东八区）
            if 'local_timestamp' in row and row['local_timestamp'] is not None:
                new_row['local_timestamp'] = CSVExporter.format_timestamp(
                    row['local_timestamp'], is_millis=True
                )
            
            result.append(new_row)
        
        return result
    
    @staticmethod
    def reorder_columns(data: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        重新排列列顺序，将时间戳相关列放在前面
        
        Args:
            data: 数据列表
            
        Returns:
            重新排列列顺序的数据列表
        """
        if not data:
            return data
        
        # 定义列顺序（时间戳相关列优先）
        priority_columns = [
            'id', 'device_sn', 'device_type',
            'timestamp', 'local_timestamp'
        ]
        
        result = []
        for row in data:
            new_row = {}
            # 先添加优先级列
            for col in priority_columns:
                if col in row:
                    new_row[col] = row[col]
            # 再添加其他列
            for key, value in row.items():
                if key not in priority_columns:
                    new_row[key] = value
            result.append(new_row)
        
        return result
    
    @staticmethod
    def prepare_for_export(data: List[Dict[str, Any]], 
                          config_path: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        准备数据用于导出：过滤字段、格式化时间戳、重新排列列顺序
        
        Args:
            data: 原始数据列表
            config_path: 配置文件路径
            
        Returns:
            处理后的数据列表
        """
        # 1. 过滤字段，只保留主表字段和配置的扩展字段，从payload_json中提取字段
        filtered_data = CSVExporter.filter_and_extract_fields(data, config_path)
        
        # 2. 格式化时间戳
        formatted_data = CSVExporter.add_formatted_timestamps(filtered_data)
        
        # 3. 重新排列列顺序
        reordered_data = CSVExporter.reorder_columns(formatted_data)
        
        return reordered_data
    
    @staticmethod
    def prepare_wide_table_for_export(data: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        准备宽表数据用于导出：格式化时间戳、重新排列列顺序
        
        宽表数据特点：
        - 按 local_timestamp 排序，每个时间戳一行
        - 包含所有设备的所有字段（主表字段+扩展表字段）
        - 字段名格式为 {device_sn}_{field_name}，使用设备序列号避免字段冲突（包括同类设备）
        - 指令字段格式为 {device_sn}_{cmd_name}，避免不同设备的同名指令冲突
        - 指令信息和数据信息可能不同时到达，所以一行可能只有指令或只有数据
        - 保留所有字段，只格式化时间戳和重新排列列顺序
        
        Args:
            data: 宽表数据列表（来自 query_wide_table）
            
        Returns:
            处理后的数据列表
        """
        if not data:
            return data
        
        # 1. 格式化时间戳（宽表只有 local_timestamp）
        formatted_data = []
        for row in data:
            new_row = row.copy()
            # 格式化 local_timestamp（毫秒时间戳）
            if 'local_timestamp' in new_row and new_row['local_timestamp'] is not None:
                new_row['local_timestamp'] = CSVExporter.format_timestamp(
                    new_row['local_timestamp'], is_millis=True
                )
            formatted_data.append(new_row)
        
        # 2. 重新排列列顺序（宽表：local_timestamp 优先，其他列按字母顺序）
        # 这样可以确保所有设备的所有字段都包含在CSV中
        result = []
        for row in formatted_data:
            new_row = {}
            # 先添加 local_timestamp（如果存在）
            if 'local_timestamp' in row:
                new_row['local_timestamp'] = row['local_timestamp']
            # 再添加其他列（按字母顺序，确保列顺序一致）
            other_keys = sorted([k for k in row.keys() if k != 'local_timestamp'])
            for key in other_keys:
                new_row[key] = row[key]
            result.append(new_row)
        
        return result