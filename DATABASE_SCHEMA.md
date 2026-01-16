# 数据库表结构文档

## 设计说明

### 新表设计（data_wide）⭐

采用**单宽表**设计，用于存储设备数据和命令数据：
- **主键**：`local_timestamp`（毫秒级时间戳）
- **动态列**：根据配置文件和实际数据动态创建
- **列名格式**：`{device_sn}_{table_field_name}` 或 `{device_sn}_{cmd_name}`
- **体积优化**：只存储配置的字段，减少数据库体积
- **宽表查询**：设备数据和命令数据在同一张表中，查询方便

### 旧表设计（备份）

采用**主表+JSON扩展表**设计（保留作为备份）：
- **主表**：存储公共字段和元数据，字段根据配置动态创建
- **JSON扩展表**：统一使用一个扩展表，将所有动态字段以JSON格式存储

这种设计的优势：
- 灵活性：支持任意JSON字段，无需预定义
- 可扩展性：添加新字段无需修改表结构
- 查询能力：使用SQLite的`json_extract()`函数查询JSON字段

## 表结构

### data_wide（新表 - 宽表设计）⭐

存储设备数据和命令数据的宽表，采用动态列设计。

**固定字段：**

| 字段名 | 类型 | 说明 | 约束 |
|--------|------|------|------|
| local_timestamp | INTEGER | 本地时间戳（毫秒） | PRIMARY KEY |

**动态列（根据配置和实际数据动态创建）：**

**设备数据列：**
- 格式：`{device_sn}_{table_field_name}`
- 类型：REAL
- 示例：
  - `METER001_active_power` - 电表METER001的有功功率
  - `STORAGE001_soc` - 储能设备STORAGE001的SOC（从嵌套路径 `battery.soc` 提取）
  - `PV001_active_power` - 光伏设备PV001的有功功率

**命令数据列：**
- 格式：`{device_sn}_{cmd_name}`
- 类型：REAL
- 示例：
  - `METER001_activePowerLimit` - 电表METER001的有功功率限制命令
  - `STORAGE001_chargeLimit` - 储能设备STORAGE001的充电限制命令

**列创建规则：**
1. 首次插入新设备/字段时，自动使用 `ALTER TABLE` 添加列
2. 列名必须符合SQLite标识符规范（字母、数字、下划线，不能以数字开头）
3. 列名使用设备SN和表字段名组合，确保唯一性

**数据存储规则：**
- 使用 `INSERT ... ON CONFLICT(local_timestamp) DO UPDATE SET ...` 语法
- 如果 `local_timestamp` 已存在，则更新对应列的值
- 如果 `local_timestamp` 不存在，则插入新行
- 设备数据和命令数据可能在不同时间到达，如果时间戳相同则合并到同一行

**配置依赖：**
- **拓扑文件**：用于获取设备SN到设备类型的映射
- **字段映射配置**：根据设备类型决定存储哪些字段
  - 配置格式：`table_field_name = "json_path"`
  - 支持嵌套路径：`soc = "battery.soc"` 表示从 `payload.battery.soc` 提取值
  - 表字段名用于生成列名：`{device_sn}_{table_field_name}`

**查询示例：**

```sql
-- 查询所有设备在某个时间点的数据
SELECT 
    local_timestamp,
    METER001_active_power,
    METER001_reactive_power,
    STORAGE001_soc,
    STORAGE001_soh,
    PV001_active_power,
    METER001_activePowerLimit
FROM data_wide
WHERE local_timestamp BETWEEN ? AND ?
ORDER BY local_timestamp;

-- 查询特定设备的数据
SELECT 
    local_timestamp,
    METER001_active_power,
    METER001_reactive_power,
    METER001_activePowerLimit
FROM data_wide
WHERE local_timestamp >= ?
ORDER BY local_timestamp DESC
LIMIT 100;
```

**设计优势：**
- ✅ **体积最小**：只存储配置的字段，无冗余数据
- ✅ **查询简单**：一次SELECT即可获取同一时间点的所有数据
- ✅ **时间对齐**：设备数据和命令数据在同一行，便于分析
- ✅ **动态扩展**：新设备/字段自动添加列，无需手动维护

---

### device_data（主表 - 旧表，备份）

存储所有设备的公共字段和元数据。主表字段根据字段映射配置动态创建。

**固定字段：**

| 字段名 | 类型 | 说明 | 约束 |
|--------|------|------|------|
| id | INTEGER | 主键，自增 | PRIMARY KEY |
| device_sn | TEXT | 设备序列号 | NOT NULL |
| device_type | TEXT | 设备类型 | NOT NULL, CHECK IN (''METER'',''STORAGE'',''PV'',''CHARGER'',''none'') |
| timestamp | INTEGER | 设备时间戳（秒） | NOT NULL |
| local_timestamp | INTEGER | 本地时间戳（毫秒） | NOT NULL |
| version | INTEGER | 数据版本 | DEFAULT 1 |
| data_source | TEXT | 数据来源 | DEFAULT ''NNG'' |
| quality | INTEGER | 数据质量评分 | DEFAULT 100 |

**动态字段（根据字段映射配置）：**

默认情况下，主表包含以下字段（如果字段映射配置中定义了）：

| 字段名 | 类型 | 说明 | 配置来源 |
|--------|------|------|----------|
| activePower | REAL | 有功功率 | 所有设备类型默认都有 |
| reactivePower | REAL | 无功功率 | 如果字段映射配置中定义了 `reactive_power` |
| powerFactor | REAL | 功率因数 | 如果字段映射配置中定义了 `power_factor` |

**注意：**
- `ACfrequqncy` 字段已从主表中移除，现在仅存储在JSON扩展表中
- 主表字段可以通过字段映射配置自定义，不同设备类型可以有不同的字段映射
- 如果字段映射配置中未定义某个字段（如 `power_factor`），则主表中不会包含该字段
- `device_type` 为 `'none'` 时表示设备不在拓扑配置中，此时会存储所有接收到的设备数据

**索引：**
- `idx_device_sn` ON (device_sn)
- `idx_timestamp` ON (timestamp)
- `idx_device_type` ON (device_type)
- `idx_device_sn_timestamp` ON (device_sn, timestamp)

---

### device_data_ext（统一JSON扩展表）

存储所有设备的动态字段，使用JSON格式。不再为每种设备类型创建单独的扩展表。

| 字段名 | 类型 | 说明 | 约束 |
|--------|------|------|------|
| device_data_id | INTEGER | 关联主表ID | PRIMARY KEY, FOREIGN KEY -> device_data(id) ON DELETE CASCADE |
| payload_json | TEXT | 完整的JSON数据，包含所有动态字段 | NOT NULL |

**设计说明：**
- 所有设备的动态字段都存储在 `payload_json` 列中
- 使用SQLite的JSON函数（`json_extract()`）查询JSON字段
- 支持任意JSON结构，无需预定义字段

---

### cmd_data（命令数据表 - 旧表，备份）

存储命令数据，独立于设备数据表。新系统使用 `data_wide` 表。

| 字段名 | 类型 | 说明 | 约束 |
|--------|------|------|------|
| id | INTEGER | 主键，自增 | PRIMARY KEY |
| timestamp | INTEGER | 时间戳（秒） | NOT NULL |
| device_sn | TEXT | 设备序列号 | NOT NULL |
| name | TEXT | 命令名称 | NOT NULL |
| value | REAL | 命令值 | NOT NULL |
| local_timestamp | INTEGER | 本地时间戳（毫秒） | NOT NULL |

**索引：**
- `idx_cmd_device_sn` ON (device_sn)
- `idx_cmd_timestamp` ON (timestamp)

---

## 查询示例

### 新表 data_wide 查询示例

#### 查询所有设备在某个时间点的数据

```sql
SELECT 
    local_timestamp,
    METER001_active_power,
    METER001_reactive_power,
    STORAGE001_soc,
    STORAGE001_soh,
    PV001_active_power,
    METER001_activePowerLimit,
    STORAGE001_chargeLimit
FROM data_wide
WHERE local_timestamp BETWEEN ? AND ?
ORDER BY local_timestamp;
```

#### 查询特定设备的数据

```sql
SELECT 
    local_timestamp,
    METER001_active_power,
    METER001_reactive_power,
    METER001_activePowerLimit
FROM data_wide
WHERE local_timestamp >= ?
ORDER BY local_timestamp DESC
LIMIT 100;
```

#### 查询时间范围内的数据（导出宽表）

```sql
SELECT *
FROM data_wide
WHERE local_timestamp BETWEEN ? AND ?
ORDER BY local_timestamp;
```

### 旧表查询示例（备份）

#### 查询所有设备的公共字段（主表）

```sql
SELECT device_sn, device_type, timestamp, activePower, reactivePower
FROM device_data
WHERE timestamp BETWEEN ? AND ?
ORDER BY timestamp DESC;
```

#### 查询设备数据并提取JSON字段（使用 json_extract）

```sql
SELECT 
    d.device_sn, 
    d.device_type, 
    d.timestamp, 
    d.activePower,
    json_extract(e.payload_json, ''$.activeEnergy'') as active_energy,
    json_extract(e.payload_json, ''$.ACfrequqncy'') as ac_frequency
FROM device_data d
LEFT JOIN device_data_ext e ON d.id = e.device_data_id
WHERE d.device_type = ''METER''
  AND d.device_sn = ?
ORDER BY d.timestamp DESC
LIMIT 100;
```

#### 查询储能设备的SOC和SOH（从JSON提取）

```sql
SELECT 
    d.device_sn, 
    d.timestamp, 
    d.activePower,
    json_extract(e.payload_json, ''$.SOC-BAT-001'') as soc,
    json_extract(e.payload_json, ''$.SOH-BAT-001'') as soh
FROM device_data d
LEFT JOIN device_data_ext e ON d.id = e.device_data_id
WHERE d.device_type = ''STORAGE''
  AND d.device_sn = ?
ORDER BY d.timestamp DESC;
```

---

## 字段映射配置

新表 `data_wide` 的列由字段映射配置决定。配置格式（`config.toml`）：

```toml
[field_mapping]
# 电表字段映射
[field_mapping.meter]
# 表字段名 = JSON路径（相对于payload）
active_power = "activePower"
reactive_power = "reactivePower"
power_factor = "powerFactor"

# 储能设备字段映射
[field_mapping.storage]
active_power = "ActiveP"
reactive_power = "ActivePalg"
# 嵌套字段示例：从 payload.battery.soc 提取值，存储到表的 {sn}_soc 列
soc = "battery.soc"
soh = "battery.soh"

# 光伏设备字段映射
[field_mapping.pv]
active_power = "activePower"
reactive_power = "reactivePower"
power_factor = "powerFactor"

# 充电桩字段映射
[field_mapping.charger]
active_power = "OutP"
# 更深层嵌套示例
# temperature = "status.sensors.temperature"
```

**配置说明：**
- **键名**（`table_field_name`）：表字段名，用于生成列名 `{device_sn}_{table_field_name}`
- **值**（`json_path`）：JSON路径，相对于 `payload`，支持嵌套（使用点号分隔）
- **嵌套路径**：例如 `soc = "battery.soc"` 表示从 `payload.battery.soc` 提取值
- **列名生成**：配置 `soc = "battery.soc"` → 列名 `STORAGE001_soc`（假设设备SN是STORAGE001）

**工作流程：**
1. 接收数据，获取设备SN
2. 通过拓扑文件查询设备类型
3. 根据设备类型从字段映射配置获取需要采集的字段列表
4. 根据嵌套路径从JSON中提取值（例如：`payload.battery.soc`）
5. 使用表字段名生成列名（例如：`STORAGE001_soc`）
6. 只存储配置中定义的字段，忽略其他字段

---

## 注意事项

### 新表 data_wide

1. **动态列管理**：列根据配置和实际数据动态创建，首次插入新设备/字段时会自动添加列
2. **列数限制**：SQLite默认支持最多2000列，如果设备很多或字段很多，可能达到限制
3. **稀疏数据**：大部分列可能为NULL，但SQLite对NULL值只占用1字节标记，开销较小
4. **UPSERT合并**：同一时间戳的设备数据和命令数据会自动合并到同一行
5. **拓扑依赖**：必须配置拓扑文件，用于获取设备SN到设备类型的映射
6. **字段映射**：所有字段映射完全依赖配置文件，无默认值

### 旧表（备份）

1. **SQLite版本**：需要SQLite 3.38+ 才能使用完整的JSON函数支持
2. **JSON性能**：JSON查询可能比直接列查询慢，对于频繁查询的字段建议存储在主表中
3. **字段映射**：主表字段由配置决定，修改配置后需要运行 `migrate_table_fields()` 更新表结构
4. **数据完整性**：所有原始JSON数据都存储在扩展表中，主表只存储常用字段用于快速查询
