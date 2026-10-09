-- byte-translator 数据库 schema v1
-- 启动时由 db 模块的版本化迁移执行。表名小写蛇形。
-- 连接级 PRAGMA（journal_mode/foreign_keys/synchronous）在 Database::open 显式设置，
-- PRAGMA 事务安全级别不允许在迁移事务内执行，此处不声明。

-- ============ 翻译历史 ============
CREATE TABLE IF NOT EXISTS history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    text        TEXT NOT NULL,
    result_json TEXT NOT NULL DEFAULT '{}',
    engine      TEXT NOT NULL DEFAULT '',
    from_lang   TEXT NOT NULL DEFAULT '',
    to_lang     TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_history_created_at ON history (created_at);
CREATE INDEX IF NOT EXISTS idx_history_engine ON history (engine);

-- ============ 生词本 ============
CREATE TABLE IF NOT EXISTS wordbook (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    text       TEXT NOT NULL,
    note       TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
-- 同一词条只收藏一次
CREATE UNIQUE INDEX IF NOT EXISTS idx_wordbook_text ON wordbook (text);

-- ============ AI 供应商 ============
CREATE TABLE IF NOT EXISTS ai_providers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         VARCHAR(100) NOT NULL,
    provider_key VARCHAR(80) NOT NULL UNIQUE,
    base_url     VARCHAR(500) NOT NULL DEFAULT '',
    api_key      VARCHAR(500) NOT NULL DEFAULT '',  -- 仅 Rust 侧读取，不回显任何接口
    model        VARCHAR(200) NOT NULL DEFAULT '',
    max_tokens   INTEGER NOT NULL DEFAULT 4096,
    temperature  REAL NOT NULL DEFAULT 0.7,
    is_enabled   INTEGER NOT NULL DEFAULT 0,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_ai_providers_key ON ai_providers (provider_key);

-- ============ 词典结果缓存 ============
CREATE TABLE IF NOT EXISTS dict_cache (
    word        TEXT NOT NULL,
    lang        TEXT NOT NULL DEFAULT '',
    result_json TEXT NOT NULL,
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (word, lang)
);
