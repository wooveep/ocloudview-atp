-- 测试报告表
CREATE TABLE IF NOT EXISTS test_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scenario_name TEXT NOT NULL,
    description TEXT,
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    duration_ms INTEGER,
    total_steps INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    skipped_count INTEGER NOT NULL DEFAULT 0,
    passed BOOLEAN NOT NULL DEFAULT 0,
    tags TEXT, -- JSON array: ["tag1", "tag2"]
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 执行步骤表
CREATE TABLE IF NOT EXISTS execution_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id INTEGER NOT NULL,
    step_index INTEGER NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL, -- 'Success', 'Failed', 'Skipped'
    error TEXT,
    duration_ms INTEGER,
    output TEXT,
    FOREIGN KEY (report_id) REFERENCES test_reports(id) ON DELETE CASCADE
);

-- 场景库表
CREATE TABLE IF NOT EXISTS scenarios (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    definition TEXT NOT NULL, -- JSON/YAML
    tags TEXT, -- JSON array
    version INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 主机配置表
CREATE TABLE IF NOT EXISTS hosts (
    id TEXT PRIMARY KEY,
    host TEXT NOT NULL,
    uri TEXT NOT NULL,
    tags TEXT, -- JSON array
    metadata TEXT, -- JSON object
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 性能指标表
CREATE TABLE IF NOT EXISTS connection_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    host_id TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    total_connections INTEGER NOT NULL,
    active_connections INTEGER NOT NULL,
    total_requests INTEGER NOT NULL,
    total_errors INTEGER NOT NULL,
    avg_response_time REAL,
    FOREIGN KEY (host_id) REFERENCES hosts(id) ON DELETE CASCADE
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_reports_time ON test_reports(start_time);
CREATE INDEX IF NOT EXISTS idx_reports_scenario ON test_reports(scenario_name);
CREATE INDEX IF NOT EXISTS idx_reports_passed ON test_reports(passed);
CREATE INDEX IF NOT EXISTS idx_reports_created ON test_reports(created_at);

CREATE INDEX IF NOT EXISTS idx_steps_report ON execution_steps(report_id);
CREATE INDEX IF NOT EXISTS idx_steps_status ON execution_steps(status);

CREATE INDEX IF NOT EXISTS idx_scenarios_name ON scenarios(name);
CREATE INDEX IF NOT EXISTS idx_scenarios_updated ON scenarios(updated_at);

CREATE INDEX IF NOT EXISTS idx_hosts_host ON hosts(host);

CREATE INDEX IF NOT EXISTS idx_metrics_host_time ON connection_metrics(host_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_metrics_timestamp ON connection_metrics(timestamp);
