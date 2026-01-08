#!/usr/bin/env python3
"""
使用示例 - 演示如何将查询工具集成到其他应用
"""

from query_tool import SSHClient, DBQuery, CSVExporter
from datetime import datetime, timedelta

def example_basic_query():
    """基本查询示例"""
    print("=== 基本查询示例 ===")
    
    # 连接SSH
    with SSHClient(
        host="192.168.1.100",
        username="root",
        password="your_password"
    ) as ssh:
        # 创建查询器
        db_query = DBQuery(ssh)
        
        # 计算时间范围（最近24小时）
        end_time = int(datetime.now().timestamp())
        start_time = int((datetime.now() - timedelta(hours=24)).timestamp())
        
        # 执行查询
        results = db_query.query_by_time_range(
            db_path="/opt/analysis/data/device_data.db",
            start_time=start_time,
            end_time=end_time,
            include_ext=True
        )
        
        print(f"查询到 {len(results)} 条记录")
        
        # 导出为CSV
        if results:
            CSVExporter.export_to_csv(results, "query_result.csv")
            print("数据已导出到 query_result.csv")


def example_device_specific_query():
    """查询特定设备数据"""
    print("\n=== 查询特定设备数据 ===")
    
    with SSHClient(host="192.168.1.100", username="root", password="password") as ssh:
        db_query = DBQuery(ssh)
        
        # 查询特定设备的数据
        results = db_query.query_by_time_range(
            db_path="/opt/analysis/data/device_data.db",
            start_time=int((datetime.now() - timedelta(days=7)).timestamp()),
            end_time=int(datetime.now().timestamp()),
            device_sn="METER001",
            device_type="METER",
            include_ext=True
        )
        
        print(f"设备 METER001 的数据: {len(results)} 条记录")


def example_custom_sql():
    """自定义SQL查询示例"""
    print("\n=== 自定义SQL查询 ===")
    
    with SSHClient(host="192.168.1.100", username="root", password="password") as ssh:
        db_query = DBQuery(ssh)
        
        # 执行自定义SQL
        sql = """
        SELECT device_type, COUNT(*) as count
        FROM device_data
        GROUP BY device_type
        """
        
        results = db_query.execute_query(
            db_path="/opt/analysis/data/device_data.db",
            sql=sql
        )
        
        print("设备类型统计:")
        for row in results:
            print(f"  {row['device_type']}: {row['count']} 条记录")


def example_database_info():
    """获取数据库信息"""
    print("\n=== 数据库信息 ===")
    
    with SSHClient(host="192.168.1.100", username="root", password="password") as ssh:
        db_query = DBQuery(ssh)
        
        info = db_query.get_table_info("/opt/analysis/data/device_data.db")
        
        print("数据库表:")
        for table in info['tables']:
            print(f"  - {table}: {info['table_stats'].get(table, 0)} 行")
        
        if info['time_range']:
            min_time = datetime.fromtimestamp(info['time_range']['min'])
            max_time = datetime.fromtimestamp(info['time_range']['max'])
            print(f"\n时间范围: {min_time} 到 {max_time}")


def example_with_formatted_timestamps():
    """使用格式化时间戳导出"""
    print("\n=== 格式化时间戳导出 ===")
    
    with SSHClient(host="192.168.1.100", username="root", password="password") as ssh:
        db_query = DBQuery(ssh)
        
        results = db_query.query_by_time_range(
            db_path="/opt/analysis/data/device_data.db",
            start_time=int((datetime.now() - timedelta(days=1)).timestamp()),
            end_time=int(datetime.now().timestamp())
        )
        
        # 添加格式化的时间戳列
        formatted_results = CSVExporter.add_formatted_timestamps(results)
        
        # 导出
        CSVExporter.export_to_csv(formatted_results, "formatted_result.csv")
        print("已导出带格式化时间戳的数据")


if __name__ == "__main__":
    print("数据库查询工具使用示例\n")
    print("注意：请修改示例中的SSH连接信息后再运行\n")
    
    # 取消注释以运行示例
    # example_basic_query()
    # example_device_specific_query()
    # example_custom_sql()
    # example_database_info()
    # example_with_formatted_timestamps()
    
    print("\n请查看代码中的示例函数，了解如何使用查询工具")
