# OCloudView-ATP 代码架构分析文档

**生成日期**: 2026-01-05

---

## 1. 项目结构概览

```
ocloudview-atp/
├── atp-application/          # 应用层
│   ├── cli/                  # CLI 主程序 (atp 命令)
│   ├── http-api/             # HTTP API (TODO)
│   └── scenarios/            # 场景库 (空)
├── atp-core/                 # 核心模块
│   ├── executor/             # 场景执行器
│   ├── protocol/             # 协议层 (QMP/QGA/SPICE/VirtIO)
│   ├── transport/            # 传输层 (Libvirt 连接)
│   ├── vdiplatform/          # VDI 平台 API
│   ├── storage/              # SQLite 存储
│   ├── verification-server/  # 验证服务器
│   ├── ssh-executor/         # SSH 执行器
│   └── gluster/              # Gluster 存储
├── atp-common/               # 共享类型
├── guest-verifier/           # Guest 验证代理
│   ├── verifier-agent/
│   └── verifier-core/
└── docs/                     # 文档
```

---

## 2. CLI 入口调用链

```mermaid
graph TD
    subgraph "入口"
        MAIN["main.rs::main()"]
    end
    
    subgraph "命令模块"
        HOST["host::handle()"]
        VDI["vdi::handle()"]
        PS["powershell::handle()"]
        DB["db::handle()"]
        SCENARIO["scenario::handle()"]
        REPORT["report::handle()"]
    end
    
    MAIN --> HOST
    MAIN --> VDI
    MAIN --> PS
    MAIN --> DB
    MAIN --> SCENARIO
    MAIN --> REPORT
```

---

## 3. VDI 模块函数调用关系

### 3.1 调用图

```mermaid
graph TB
    subgraph "入口"
        H["handle()"]
    end
    
    subgraph "查询命令"
        LH["list_hosts()"]
        LV["list_vms()"]
        SH["sync_hosts()"]
        DL["disk_location()"]
    end
    
    subgraph "批量操作"
        BS["batch_start_vms()"]
        BA["batch_assign_vms()"]
        BR["batch_rename_vms()"]
        BJ["batch_set_auto_join_domain()"]
    end
    
    subgraph "辅助函数"
        GVM["get_matching_vms()"]
        MP["matches_pattern()"]
        ODT["output_disk_location_table()"]
        ODJ["output_disk_location_json()"]
    end
    
    subgraph "公共模块 common.rs"
        VDI_CLIENT["create_vdi_client()"]
        HOST_MAP["build_host_id_to_name_map_from_json()"]
        LIBVIRT["connect_libvirt()"]
    end
    
    H --> LH & LV & SH & DL
    H --> BS & BA & BR & BJ
    
    BS & BA & BR & BJ --> GVM
    GVM --> MP
    
    DL --> ODT & ODJ
    
    LH & LV & SH & DL --> VDI_CLIENT
    LH & LV & SH --> HOST_MAP
    DL --> LIBVIRT
```

### 3.2 函数列表

| 函数 | 行号 | 功能 |
|------|------|------|
| `handle()` | 47-120 | 命令分发 |
| `verify_consistency()` | 123-324 | VDI-libvirt 一致性验证 |
| `list_hosts()` | 393-426 | 列出 VDI 主机 |
| `list_vms()` | 428-477 | 列出 VDI 虚拟机 |
| `sync_hosts()` | 479-522 | 同步主机到本地 |
| `disk_location()` | 524-673 | 磁盘存储位置查询 |
| `output_disk_location_table()` | 675-735 | 表格输出 |
| `output_disk_location_json()` | 739-801 | JSON 输出 |
| `matches_pattern()` | 807-842 | 模式匹配 |
| `get_matching_vms()` | 844-891 | 获取匹配虚拟机 |
| `batch_start_vms()` | 893-985 | 批量启动 |
| `batch_assign_vms()` | 987-1139 | 批量分配 |
| `batch_rename_vms()` | 1141-1252 | 批量重命名 |
| `batch_set_auto_join_domain()` | 1254-1363 | 批量设置自动加域 |

---

## 4. PowerShell 模块调用关系

```mermaid
graph TB
    subgraph "入口"
        H["handle()"]
    end
    
    subgraph "核心函数"
        EXEC["exec_powershell()"]
        QGA["execute_ps_via_qga()"]
        RES["resolve_targets()"]
        LIST["list_vms()"]
    end
    
    subgraph "外部依赖"
        VDI["VdiClient"]
        CONN["HostConnection"]
        QGAP["QgaProtocol"]
    end
    
    H --> EXEC & LIST
    EXEC --> RES --> VDI
    EXEC --> CONN --> QGA --> QGAP
```

| 函数 | 功能 |
|------|------|
| `exec_powershell()` | 执行 PowerShell 命令 |
| `execute_ps_via_qga()` | 通过 QGA 执行 |
| `resolve_targets()` | 解析目标虚拟机 |
| `list_vms()` | 列出可用虚拟机 |

---

## 5. DB 模块调用关系

```mermaid
graph LR
    H["handle()"] --> B["backup_database()"]
    H --> R["restore_database()"]
    H --> L["list_backups()"]
    H --> D["delete_backup()"]
    H --> C["cleanup_backups()"]
    
    B & R & L & D & C --> BM["BackupManager"]
```

---

## 6. Scenario 模块调用关系

```mermaid
graph TB
    H["handle()"] --> RS["run_scenario()"]
    H --> LS["list_scenarios()"]
    
    RS --> RUNNER["ScenarioRunner"]
    RS --> TM["TransportManager"]
    RS --> PR["ProtocolRegistry"]
    RS --> VDI["create_vdi_client()"]
    RS --> ST["Storage"]
```

---

## 7. 核心模块依赖

```mermaid
graph TB
    subgraph "应用层"
        CLI["atp-cli"]
    end
    
    subgraph "核心层"
        EXEC["atp-executor"]
        PROTO["atp-protocol"]
        TRANS["atp-transport"]
        VDI["atp-vdiplatform"]
        STOR["atp-storage"]
        VERIFY["verification-server"]
    end
    
    CLI --> EXEC & PROTO & TRANS & VDI & STOR
    EXEC --> PROTO & TRANS & VDI & STOR & VERIFY
    PROTO --> TRANS
```

---

## 8. Common 模块公共函数

| 函数 | 功能 | 调用者 |
|------|------|--------|
| `create_vdi_client()` | 创建并登录 VDI | vdi, powershell, scenario |
| `build_host_id_to_name_map_from_json()` | 构建主机 ID 映射 | vdi, powershell |
| `connect_libvirt()` | 连接 libvirt | vdi |
| `LibvirtConnectionResult` | 连接结果结构体 | vdi |
