-- v3：KV 配置表（引擎配置等：iciba_api_key / default_engine）
CREATE TABLE IF NOT EXISTS sys_config (
    key   TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL DEFAULT ''
);
