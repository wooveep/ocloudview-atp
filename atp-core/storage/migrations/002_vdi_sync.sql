-- VDI 平台数据同步表结构
-- 对标 VDI 平台 MariaDB 表结构，用于本地缓存

-- ============================================
-- 1. 扩展 hosts 表（对标 VDI host 表 22 字段）
-- ============================================

ALTER TABLE hosts ADD COLUMN ip_v6 TEXT;
ALTER TABLE hosts ADD COLUMN status INTEGER DEFAULT 0;
ALTER TABLE hosts ADD COLUMN pool_id TEXT;
ALTER TABLE hosts ADD COLUMN vmc_id TEXT;
ALTER TABLE hosts ADD COLUMN manufacturer TEXT;
ALTER TABLE hosts ADD COLUMN model TEXT;
ALTER TABLE hosts ADD COLUMN cpu TEXT;
ALTER TABLE hosts ADD COLUMN cpu_size INTEGER;
ALTER TABLE hosts ADD COLUMN memory REAL;
ALTER TABLE hosts ADD COLUMN physical_memory REAL;
ALTER TABLE hosts ADD COLUMN domain_limit INTEGER DEFAULT 60;
ALTER TABLE hosts ADD COLUMN extranet_ip TEXT;
ALTER TABLE hosts ADD COLUMN extranet_ip_v6 TEXT;
ALTER TABLE hosts ADD COLUMN arch TEXT;
ALTER TABLE hosts ADD COLUMN domain_cap_xml TEXT;
ALTER TABLE hosts ADD COLUMN qemu_version INTEGER DEFAULT 0;
ALTER TABLE hosts ADD COLUMN libvirt_version INTEGER DEFAULT 0;
ALTER TABLE hosts ADD COLUMN cached_at DATETIME DEFAULT CURRENT_TIMESTAMP;

-- ============================================
-- 2. 新增 pools 表（资源池）
-- ============================================

CREATE TABLE IF NOT EXISTS pools (
    id TEXT PRIMARY KEY,
    name TEXT,
    status INTEGER,
    vmc_id TEXT,
    cpu_over INTEGER,
    memory_over INTEGER,
    arch TEXT,
    create_time DATETIME,
    update_time DATETIME,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_pools_vmc ON pools(vmc_id);
CREATE INDEX IF NOT EXISTS idx_pools_name ON pools(name);

-- ============================================
-- 3. 新增 ovs 表（OVS 网络）
-- ============================================

CREATE TABLE IF NOT EXISTS ovs (
    id TEXT PRIMARY KEY,
    name TEXT,
    vmc_id TEXT,
    remark TEXT,
    create_time DATETIME,
    update_time DATETIME,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ovs_vmc ON ovs(vmc_id);
CREATE INDEX IF NOT EXISTS idx_ovs_name ON ovs(name);

-- ============================================
-- 4. 新增 cascad_port_groups 表（级联端口组）
-- ============================================

CREATE TABLE IF NOT EXISTS cascad_port_groups (
    id TEXT PRIMARY KEY,
    host_id TEXT,
    physical_nic TEXT,
    ovs_id TEXT,
    create_time DATETIME,
    update_time DATETIME,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cascad_port_groups_host ON cascad_port_groups(host_id);
CREATE INDEX IF NOT EXISTS idx_cascad_port_groups_ovs ON cascad_port_groups(ovs_id);

-- ============================================
-- 5. 新增 domains 表（虚拟机 - 完整 60 字段）
-- ============================================

CREATE TABLE IF NOT EXISTS domains (
    id TEXT PRIMARY KEY,
    name TEXT,
    is_model INTEGER,
    status INTEGER,
    is_connected INTEGER,
    vmc_id TEXT,
    pool_id TEXT,
    host_id TEXT,
    last_successful_host_id TEXT,
    cpu INTEGER,
    memory INTEGER,
    iso_path TEXT,
    is_clone_domain INTEGER,
    clone_type TEXT,
    mother_id TEXT,
    snapshot_count INTEGER DEFAULT 0,
    freeze INTEGER,
    last_freeze_time DATETIME,
    command TEXT,
    os_name TEXT,
    os_edition TEXT,
    system_type TEXT,
    mainboard TEXT DEFAULT 'pc',
    bootloader TEXT DEFAULT 'bios',
    working_group TEXT,
    desktop_pool_id TEXT,
    user_id TEXT,
    remark TEXT,
    connect_time DATETIME,
    disconnect_time DATETIME,
    os_type TEXT,
    soundcard_type TEXT,
    domain_xml TEXT,
    affinity_ip TEXT,
    sockets INTEGER,
    cores INTEGER,
    threads INTEGER,
    original_ip TEXT,
    original_mac TEXT,
    is_recycle INTEGER,
    disable_alpha INTEGER,
    graphics_card_num INTEGER,
    independ_disk_cnt INTEGER DEFAULT 0,
    mouse_mode TEXT,
    domain_fake INTEGER DEFAULT 0,
    host_bios_enable INTEGER DEFAULT 0,
    host_model_enable INTEGER DEFAULT 0,
    nested_virtual INTEGER DEFAULT 0,
    admin_id TEXT,
    admin_name TEXT,
    allow_monitor INTEGER DEFAULT 1,
    agent_version TEXT,
    gpu_type TEXT,
    auto_join_domain INTEGER DEFAULT 0,
    vgpu_type TEXT DEFAULT 'qxl',
    keyboard_bus TEXT DEFAULT 'ps2',
    mouse_bus TEXT DEFAULT 'ps2',
    keep_alive INTEGER DEFAULT 0,
    create_time DATETIME,
    update_time DATETIME,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_domains_name ON domains(name);
CREATE INDEX IF NOT EXISTS idx_domains_host ON domains(host_id);
CREATE INDEX IF NOT EXISTS idx_domains_pool ON domains(pool_id);
CREATE INDEX IF NOT EXISTS idx_domains_status ON domains(status);
CREATE INDEX IF NOT EXISTS idx_domains_user ON domains(user_id);
CREATE INDEX IF NOT EXISTS idx_domains_desktop_pool ON domains(desktop_pool_id);
CREATE INDEX IF NOT EXISTS idx_domains_vmc ON domains(vmc_id);

-- ============================================
-- 6. 新增 storage_pools 表（存储池）
-- ============================================

CREATE TABLE IF NOT EXISTS storage_pools (
    id TEXT PRIMARY KEY,
    name TEXT,
    nfs_ip TEXT,
    nfs_path TEXT,
    status INTEGER,
    pool_type TEXT,
    path TEXT,
    vmc_id TEXT,
    pool_id TEXT,
    host_id TEXT,
    is_share INTEGER,
    is_iso INTEGER,
    remark TEXT,
    source_host_name TEXT,
    source_name TEXT,
    create_time DATETIME,
    update_time DATETIME,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_storage_pools_type ON storage_pools(pool_type);
CREATE INDEX IF NOT EXISTS idx_storage_pools_host ON storage_pools(host_id);
CREATE INDEX IF NOT EXISTS idx_storage_pools_pool ON storage_pools(pool_id);
CREATE INDEX IF NOT EXISTS idx_storage_pools_name ON storage_pools(name);

-- ============================================
-- 7. 新增 storage_volumes 表（存储卷）
-- ============================================

CREATE TABLE IF NOT EXISTS storage_volumes (
    id TEXT PRIMARY KEY,
    name TEXT,
    filename TEXT,
    storage_pool_id TEXT,
    domain_id TEXT,
    is_start_disk INTEGER,
    size INTEGER,
    domains TEXT,
    is_recycle INTEGER,
    read_iops_sec TEXT,
    write_iops_sec TEXT,
    read_bytes_sec TEXT,
    write_bytes_sec TEXT,
    bus_type TEXT DEFAULT 'virtio',
    create_time DATETIME,
    update_time DATETIME,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_storage_volumes_pool ON storage_volumes(storage_pool_id);
CREATE INDEX IF NOT EXISTS idx_storage_volumes_domain ON storage_volumes(domain_id);
CREATE INDEX IF NOT EXISTS idx_storage_volumes_filename ON storage_volumes(filename);

-- ============================================
-- 8. 迁移旧数据并删除旧表
-- ============================================

-- 将 domain_host_mappings 数据迁移到 domains 表
INSERT OR IGNORE INTO domains (id, name, host_id, os_type, update_time, cached_at)
SELECT
    domain_name,
    domain_name,
    host_id,
    os_type,
    updated_at,
    CURRENT_TIMESTAMP
FROM domain_host_mappings;

-- 删除旧表
DROP TABLE IF EXISTS domain_host_mappings;

-- 删除旧索引
DROP INDEX IF EXISTS idx_domain_host_mappings_host;
