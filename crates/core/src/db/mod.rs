//! SQLite 数据库：rusqlite bundled，`PRAGMA user_version` 版本化迁移。
//!
//! 单连接 + std::sync::Mutex 保护，WAL 模式保证崩溃安全与并发读不阻塞写。
//!
//! 迁移机制：MIGRATIONS 按版本号升序声明 (version, sql)；启动时读
//! user_version，跳过已应用版本，逐个在事务内执行未应用迁移并同事务推进
//! user_version——崩溃时整体回滚，不留半应用状态。
//!
//! 版本号纪律：严格递增，不得复用已发布构建用过的编号（迁移按
//! `user_version >= 条目版本` 跳过，复用编号会让跑过旧构建的库漏迁移）；
//! 编号空洞无害——runner 按本表遍历而非逐号执行。

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// v1：全量 schema（历史 / 生词本 / AI 供应商 / 词典缓存）
const SCHEMA_SQL: &str = include_str!("schema.sql");

/// 版本化迁移表：后续 schema 演进在此追加 (版本号, 迁移 SQL)。
const MIGRATIONS: &[(i64, &str)] = &[(1, SCHEMA_SQL)];

/// 当前 schema 目标版本（MIGRATIONS 最大版本）
const LATEST_VERSION: i64 = 1;

/// 数据库句柄：持 Mutex<Connection>，单连接串行化（避免 SQLite 写锁竞争）
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 打开/创建数据库并执行迁移
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建数据库目录失败：{}", parent.display()))?;
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("打开数据库失败：{}", db_path.display()))?;

        // 关键 pragma（显式设确保连接级生效；journal_mode 不在迁移事务内执行）
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        // CLI 与桌面端可能并发打开同一库（WAL 允许并发读，写写冲突时等锁而非报错）
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        migrate(&conn).context("执行数据库迁移失败")?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 获取连接 guard（持锁期间串行化所有 DB 操作）
    pub fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock()
    }
}

/// 版本化迁移：应用所有 version > user_version 的迁移，每个迁移单事务
fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current >= LATEST_VERSION {
        return Ok(());
    }
    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction().context("开启迁移事务失败")?;
        tx.execute_batch(sql)
            .with_context(|| format!("执行迁移 v{version} 失败"))?;
        tx.pragma_update(None, "user_version", version)
            .with_context(|| format!("推进迁移版本到 v{version} 失败"))?;
        tx.commit()
            .with_context(|| format!("提交迁移 v{version} 失败"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> (tempfile::TempDir, Database) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.db");
        let db = Database::open(&path).unwrap();
        (tmp, db)
    }

    #[test]
    fn schema_migrates_idempotently() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let db = Database::open(tmp.path()).expect("首次迁移");
        {
            let conn = db.lock().unwrap();
            let v: i64 = conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(v, LATEST_VERSION);
        }
        // 重复打开：无迁移可执行，幂等
        let db2 = Database::open(tmp.path()).expect("二次迁移");
        let conn = db2.lock().unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, LATEST_VERSION);
    }

    #[test]
    fn four_tables_crud_smoke() {
        let (_tmp, db) = db();
        let conn = db.lock().unwrap();

        // history
        conn.execute(
            "INSERT INTO history (text, result_json, engine, from_lang, to_lang) \
             VALUES ('hello', '{}', 'iciba', 'en', 'zh')",
            [],
        )
        .unwrap();
        let (engine,): (String,) = conn
            .query_row("SELECT engine FROM history WHERE text='hello'", [], |r| {
                Ok((r.get(0)?,))
            })
            .unwrap();
        assert_eq!(engine, "iciba");

        // wordbook：唯一索引防重复收藏
        conn.execute(
            "INSERT INTO wordbook (text, note) VALUES ('love', 'n. 爱')",
            [],
        )
        .unwrap();
        let dup = conn.execute(
            "INSERT INTO wordbook (text, note) VALUES ('love', '重复')",
            [],
        );
        assert!(dup.is_err(), "同词条重复收藏应被唯一索引拒绝");
        conn.execute("DELETE FROM wordbook WHERE text='love'", [])
            .unwrap();

        // ai_providers
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key, base_url, model, is_enabled) \
             VALUES ('测试供应商', 'test', 'https://example.com', 'm1', 1)",
            [],
        )
        .unwrap();
        let (enabled,): (i64,) = conn
            .query_row(
                "SELECT is_enabled FROM ai_providers WHERE provider_key='test'",
                [],
                |r| Ok((r.get(0)?,)),
            )
            .unwrap();
        assert_eq!(enabled, 1);
        conn.execute("DELETE FROM ai_providers WHERE provider_key='test'", [])
            .unwrap();

        // dict_cache：复合主键 (word, lang)，upsert 语义可用 ON CONFLICT
        conn.execute(
            "INSERT INTO dict_cache (word, lang, result_json) VALUES ('love', 'en', '{\"k\":1}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dict_cache (word, lang, result_json) VALUES ('love', 'en', '{\"k\":2}') \
             ON CONFLICT (word, lang) DO UPDATE SET result_json=excluded.result_json",
            [],
        )
        .unwrap();
        let (json,): (String,) = conn
            .query_row(
                "SELECT result_json FROM dict_cache WHERE word='love' AND lang='en'",
                [],
                |r| Ok((r.get(0)?,)),
            )
            .unwrap();
        assert_eq!(json, r#"{"k":2}"#);
    }

    #[test]
    fn wal_mode_enabled() {
        let (_tmp, db) = db();
        let conn = db.lock().unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
}
