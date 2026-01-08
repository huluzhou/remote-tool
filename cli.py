#!/usr/bin/env python3
"""
命令行接口 - 提供CLI方式使用查询工具
"""

import argparse
import sys
from datetime import datetime, timedelta
from pathlib import Path
import logging

from .core.ssh_client import SSHClient
from .core.db_query import DBQuery
from .core.csv_export import CSVExporter

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def parse_time(time_str: str) -> int:
    """
    解析时间字符串为时间戳
    
    支持格式：
    - 时间戳（整数）
    - "YYYY-MM-DD HH:MM:SS"
    - "YYYY-MM-DD"
    - "today", "yesterday", "now"
    - "Ndays" (N天前)
    """
    time_str = time_str.strip().lower()
    
    # 特殊关键字
    if time_str == "now":
        return int(datetime.now().timestamp())
    elif time_str == "today":
        dt = datetime.now().replace(hour=0, minute=0, second=0, microsecond=0)
        return int(dt.timestamp())
    elif time_str == "yesterday":
        dt = datetime.now() - timedelta(days=1)
        dt = dt.replace(hour=0, minute=0, second=0, microsecond=0)
        return int(dt.timestamp())
    elif time_str.endswith("days") or time_str.endswith("day"):
        try:
            days = int(time_str.replace("days", "").replace("day", "").strip())
            dt = datetime.now() - timedelta(days=days)
            return int(dt.timestamp())
        except ValueError:
            pass
    
    # 尝试解析为时间戳
    try:
        return int(time_str)
    except ValueError:
        pass
    
    # 尝试解析为日期时间字符串
    try:
        if len(time_str) == 10:  # YYYY-MM-DD
            dt = datetime.strptime(time_str, "%Y-%m-%d")
        else:  # YYYY-MM-DD HH:MM:SS
            dt = datetime.strptime(time_str, "%Y-%m-%d %H:%M:%S")
        return int(dt.timestamp())
    except ValueError:
        pass
    
    raise ValueError(f"无法解析时间字符串: {time_str}")


def main():
    parser = argparse.ArgumentParser(
        description="数据库查询工具 - 通过SSH连接远程服务器查询SQLite数据库",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 查询今天的数据
  %(prog)s -H 192.168.1.100 -u root -p password -s today -e now

  # 查询指定设备最近7天的数据
  %(prog)s -H 192.168.1.100 -u root -p password -s 7days -e now --device-sn METER001

  # 查询并导出为CSV
  %(prog)s -H 192.168.1.100 -u root -p password -s "2024-01-01" -e "2024-01-31" -o result.csv
        """
    )
    
    # SSH连接参数
    parser.add_argument("-H", "--host", required=True, help="SSH服务器地址")
    parser.add_argument("-P", "--port", type=int, default=22, help="SSH端口 (默认: 22)")
    parser.add_argument("-u", "--username", default="root", help="SSH用户名 (默认: root)")
    parser.add_argument("-p", "--password", help="SSH密码")
    parser.add_argument("-k", "--key-file", help="SSH私钥文件路径")
    
    # 数据库参数
    parser.add_argument("-d", "--db-path", default="/opt/analysis/data/device_data.db",
                       help="远程数据库文件路径 (默认: /opt/analysis/data/device_data.db)")
    
    # 查询参数
    parser.add_argument("-s", "--start-time", required=True,
                       help="开始时间 (支持: 时间戳, 'YYYY-MM-DD', 'YYYY-MM-DD HH:MM:SS', 'today', 'yesterday', 'Ndays')")
    parser.add_argument("-e", "--end-time", required=True,
                       help="结束时间 (支持: 时间戳, 'YYYY-MM-DD', 'YYYY-MM-DD HH:MM:SS', 'now', 'today')")
    parser.add_argument("--device-sn", help="设备序列号 (可选)")
    parser.add_argument("--include-ext", action="store_true", default=True,
                       help="包含扩展表数据 (默认: True)")
    parser.add_argument("--no-ext", dest="include_ext", action="store_false",
                       help="不包含扩展表数据")
    
    # 输出参数
    parser.add_argument("-o", "--output", help="输出CSV文件路径 (如果不指定，输出到标准输出)")
    parser.add_argument("--format", choices=["csv", "json"], default="csv",
                       help="输出格式 (默认: csv)")
    parser.add_argument("-v", "--verbose", action="store_true", help="详细输出")
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    try:
        # 解析时间
        start_time = parse_time(args.start_time)
        end_time = parse_time(args.end_time)
        
        if start_time >= end_time:
            logger.error("开始时间必须小于结束时间")
            sys.exit(1)
        
        logger.info(f"连接SSH服务器: {args.host}:{args.port}")
        logger.info(f"查询时间范围: {datetime.fromtimestamp(start_time)} 到 {datetime.fromtimestamp(end_time)}")
        
        # 连接SSH
        with SSHClient(
            host=args.host,
            port=args.port,
            username=args.username,
            password=args.password,
            key_file=args.key_file
        ) as ssh:
            if not ssh.client:
                logger.error("SSH连接失败")
                sys.exit(1)
            
            # 执行查询
            db_query = DBQuery(ssh)
            logger.info("执行查询...")
            
            results = db_query.query_by_time_range(
                db_path=args.db_path,
                start_time=start_time,
                end_time=end_time,
                device_sn=args.device_sn,
                include_ext=args.include_ext
            )
            
            logger.info(f"查询完成，共 {len(results)} 条记录")
            
            if not results:
                logger.warning("没有找到匹配的记录")
                sys.exit(0)
            
            # 输出结果
            if args.output:
                # 导出到文件
                if args.format == "csv":
                    # 准备数据：过滤字段、格式化时间戳、重新排列列顺序
                    formatted_results = CSVExporter.prepare_for_export(results)
                    if CSVExporter.export_to_csv(formatted_results, args.output):
                        logger.info(f"数据已导出到: {args.output}")
                    else:
                        logger.error("导出失败")
                        sys.exit(1)
                else:  # json
                    import json
                    with open(args.output, 'w', encoding='utf-8') as f:
                        json.dump(results, f, indent=2, ensure_ascii=False, default=str)
                    logger.info(f"数据已导出到: {args.output}")
            else:
                # 输出到标准输出
                if args.format == "csv":
                    import csv
                    import io
                    output = io.StringIO()
                    if results:
                        writer = csv.DictWriter(output, fieldnames=results[0].keys())
                        writer.writeheader()
                        writer.writerows(results)
                    print(output.getvalue())
                else:  # json
                    import json
                    print(json.dumps(results, indent=2, ensure_ascii=False, default=str))
    
    except KeyboardInterrupt:
        logger.info("用户中断")
        sys.exit(1)
    except Exception as e:
        logger.error(f"错误: {e}", exc_info=args.verbose)
        sys.exit(1)


if __name__ == "__main__":
    main()
