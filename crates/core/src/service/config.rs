//! KV 配置：引擎级设置持久化（iciba_api_key / default_engine 等）。
//!
//! 存 DB 而非各端本地——CLI / 桌面 / 服务端共享同一份引擎配置。
//! 约定键：
//! - `iciba_api_key`：词典引擎 Key（空 = 用内置公共 Key）
//! - `default_engine`：默认引擎策略（auto / iciba / llm）

use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::db::Database;

pub const KEY_ICIBA_API_KEY: &str = "iciba_api_key";
pub const KEY_DEFAULT_ENGINE: &str = "default_engine";

#[derive(Clone)]
pub struct ConfigService {
    db: Arc<Database>,
}

impl ConfigService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 读配置；不存在返回 None
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let row = conn.query_row(
            "SELECT value FROM sys_config WHERE key=?1",
            rusqlite::params![key],
            |r| r.get::<_, String>(0),
        );
        match row {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow!("读取配置失败: {e}")),
        }
    }

    /// 写配置（upsert；空值也存——显式清空语义）
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO sys_config (key, value) VALUES (?1, ?2) \
             ON CONFLICT (key) DO UPDATE SET value=excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// 读 iciba Key 并注入进程环境（各端启动时调用一次）。
    /// 优先级：已有环境变量 > DB 配置。
    pub fn apply_iciba_key(&self) -> Result<()> {
        if std::env::var("ICIBA_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false) {
            return Ok(());
        }
        if let Some(k) = self.get(KEY_ICIBA_API_KEY)? {
            if !k.trim().is_empty() {
                std::env::set_var("ICIBA_API_KEY", k.trim());
            }
        }
        Ok(())
    }

    /// 默认引擎策略（缺省 auto）
    pub fn default_engine(&self) -> String {
        self.get(KEY_DEFAULT_ENGINE)
            .ok()
            .flatten()
            .filter(|v| ["auto", "iciba", "llm"].contains(&v.as_str()))
            .unwrap_or_else(|| "auto".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> ConfigService {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cfg-test.db");
        std::mem::forget(tmp);
        ConfigService::new(Arc::new(Database::open(&path).unwrap()))
    }

    #[test]
    fn roundtrip_and_default_engine() {
        let s = svc();
        assert_eq!(s.get(KEY_ICIBA_API_KEY).unwrap(), None);
        s.set(KEY_ICIBA_API_KEY, "my-key").unwrap();
        assert_eq!(s.get(KEY_ICIBA_API_KEY).unwrap().as_deref(), Some("my-key"));
        // upsert 覆盖
        s.set(KEY_ICIBA_API_KEY, "").unwrap();
        assert_eq!(s.get(KEY_ICIBA_API_KEY).unwrap().as_deref(), Some(""));

        assert_eq!(s.default_engine(), "auto");
        s.set(KEY_DEFAULT_ENGINE, "llm").unwrap();
        assert_eq!(s.default_engine(), "llm");
        // 非法值回落 auto
        s.set(KEY_DEFAULT_ENGINE, "baidu").unwrap();
        assert_eq!(s.default_engine(), "auto");
    }
}
