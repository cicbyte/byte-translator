//! 供应商管理：ai_providers 表 CRUD、启用切换、连通测试。
//!
//! Key 只进不出：`list`/`detail` 一律返回脱敏视图（`api_key_set` 布尔），
//! 更新时 Key 留空表示保持原值。启用为单选语义：启用某供应商时其余自动停用。

use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::ai::{probe, ProviderConfig};
use crate::db::Database;

/// 供应商对外视图（脱敏）
#[derive(Debug, Clone, Serialize)]
pub struct ProviderView {
    pub id: i64,
    pub name: String,
    pub provider_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: i64,
    pub temperature: f64,
    pub is_enabled: bool,
    /// Key 是否已配置（不回显内容）
    pub api_key_set: bool,
    /// 可用模型列表
    pub models: Vec<String>,
}

/// 新增/更新输入；provider_key 唯一，api_key 留空表示保持原值（更新时）
#[derive(Debug, Clone)]
pub struct ProviderInput {
    pub name: String,
    pub provider_key: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: i64,
    pub temperature: f64,
    pub models: Vec<String>,
}

impl Default for ProviderInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            provider_key: String::new(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            max_tokens: 4096,
            temperature: 0.3,
            models: Vec::new(),
        }
    }
}

pub struct ProviderService {
    db: Arc<Database>,
}

impl ProviderService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 列出全部供应商（脱敏）
    pub fn list(&self) -> Result<Vec<ProviderView>> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, provider_key, base_url, model, max_tokens, temperature, \
             is_enabled, (api_key != ''), models FROM ai_providers \
             ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ProviderView {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    provider_key: r.get(2)?,
                    base_url: r.get(3)?,
                    model: r.get(4)?,
                    max_tokens: r.get(5)?,
                    temperature: r.get(6)?,
                    is_enabled: r.get::<_, i64>(7)? != 0,
                    api_key_set: r.get::<_, i64>(8)? != 0,
                    models: parse_models(&r.get::<_, String>(9)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 新增或更新（按 provider_key upsert）；api_key 空则保持已有值
    pub fn upsert(&self, input: &ProviderInput) -> Result<()> {
        if input.provider_key.trim().is_empty() {
            return Err(anyhow!("provider_key 不能为空"));
        }
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM ai_providers WHERE provider_key=?1",
                rusqlite::params![input.provider_key],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e),
            })?;
        match existing {
            Some(id) => {
                conn.execute(
                    "UPDATE ai_providers SET name=?2, base_url=?3, \
                     api_key=CASE WHEN ?4='' THEN api_key ELSE ?4 END, model=?5, \
                     max_tokens=?6, temperature=?7, models=?8, updated_at=CURRENT_TIMESTAMP \
                     WHERE id=?1",
                    rusqlite::params![
                        id,
                        input.name,
                        input.base_url,
                        input.api_key,
                        input.model,
                        input.max_tokens,
                        input.temperature,
                        serde_json::to_string(&input.models).unwrap_or_else(|_| "[]".into()),
                    ],
                )?;
            }
            None => {
                conn.execute(
                    "INSERT INTO ai_providers (name, provider_key, base_url, api_key, model, \
                     max_tokens, temperature, models) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        input.name,
                        input.provider_key,
                        input.base_url,
                        input.api_key,
                        input.model,
                        input.max_tokens,
                        input.temperature,
                        serde_json::to_string(&input.models).unwrap_or_else(|_| "[]".into()),
                    ],
                )?;
            }
        }
        Ok(())
    }

    /// 启用某供应商（单选：其余全部停用）
    pub fn set_enabled(&self, provider_key: &str, enabled: bool) -> Result<()> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let n = conn.execute(
            "UPDATE ai_providers SET is_enabled=0 WHERE is_enabled=1",
            [],
        )?;
        let _ = n;
        if enabled {
            let affected = conn.execute(
                "UPDATE ai_providers SET is_enabled=1, updated_at=CURRENT_TIMESTAMP \
                 WHERE provider_key=?1",
                rusqlite::params![provider_key],
            )?;
            if affected == 0 {
                return Err(anyhow!("供应商不存在: {provider_key}"));
            }
        }
        Ok(())
    }

    /// 删除供应商
    pub fn delete(&self, provider_key: &str) -> Result<()> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let affected = conn.execute(
            "DELETE FROM ai_providers WHERE provider_key=?1",
            rusqlite::params![provider_key],
        )?;
        if affected == 0 {
            return Err(anyhow!("供应商不存在: {provider_key}"));
        }
        Ok(())
    }

    /// 取启用中的供应商完整配置（补全用；无启用时报错并给出配置指引）
    pub fn enabled_config(&self) -> Result<ProviderConfig> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let row = conn.query_row(
            "SELECT provider_key, base_url, api_key, model, max_tokens, temperature, models \
             FROM ai_providers WHERE is_enabled=1 ORDER BY sort_order ASC, id ASC LIMIT 1",
            [],
            |r| {
                Ok(ProviderConfig {
                    provider_key: r.get(0)?,
                    base_url: r.get(1)?,
                    api_key: r.get(2)?,
                    model: r.get(3)?,
                    max_tokens: r.get(4)?,
                    temperature: r.get(5)?,
                    models: parse_models(&r.get::<_, String>(6)?),
                })
            },
        );
        match row {
            Ok(cfg) => Ok(cfg),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(anyhow!(
                "未配置启用的 AI 供应商，请先添加并启用（provider set / provider enable）"
            )),
            Err(e) => Err(anyhow!("查询 AI 供应商失败: {e}")),
        }
    }

    /// 连通测试（轻量探测，不耗 token）
    pub async fn test_connectivity(&self, provider_key: &str) -> Result<()> {
        let cfg = {
            let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
            conn.query_row(
                "SELECT provider_key, base_url, api_key, model, max_tokens, temperature, models \
                 FROM ai_providers WHERE provider_key=?1",
                rusqlite::params![provider_key],
                |r| {
                    Ok(ProviderConfig {
                        provider_key: r.get(0)?,
                        base_url: r.get(1)?,
                        api_key: r.get(2)?,
                        model: r.get(3)?,
                        max_tokens: r.get(4)?,
                        temperature: r.get(5)?,
                        models: parse_models(&r.get::<_, String>(6)?),
                    })
                },
            )
            .map_err(|_| anyhow!("供应商不存在: {provider_key}"))?
        };
        probe(&cfg).await
    }
}


/// models 列 JSON 解析（坏值容错为空）
fn parse_models(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

impl ProviderService {
    /// 拉取供应商可用模型列表（GET /v1/models 或 Ollama /api/tags，不耗 token）
    pub async fn fetch_models(&self, provider_key: &str) -> Result<Vec<String>> {
        let cfg = {
            let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
            conn.query_row(
                "SELECT provider_key, base_url, api_key, model, max_tokens, temperature, models \
                 FROM ai_providers WHERE provider_key=?1",
                rusqlite::params![provider_key],
                |r| {
                    Ok(ProviderConfig {
                        provider_key: r.get(0)?,
                        base_url: r.get(1)?,
                        api_key: r.get(2)?,
                        model: r.get(3)?,
                        max_tokens: r.get(4)?,
                        temperature: r.get(5)?,
                        models: parse_models(&r.get::<_, String>(6)?),
                    })
                },
            )
            .map_err(|_| anyhow!("供应商不存在: {provider_key}"))?
        };
        crate::ai::fetch_models(&cfg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> ProviderService {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("provider-test.db");
        std::mem::forget(tmp);
        ProviderService::new(Arc::new(Database::open(&path).unwrap()))
    }

    fn input(key: &str) -> ProviderInput {
        ProviderInput {
            name: format!("供应商 {key}"),
            provider_key: key.into(),
            base_url: "https://api.example.com".into(),
            api_key: "sk-test".into(),
            model: "m1".into(),
            models: vec!["m1".into(), "m2".into()],
            ..Default::default()
        }
    }

    #[test]
    fn list_masks_api_key() {
        let svc = service();
        svc.upsert(&input("deepseek")).unwrap();
        let list = svc.list().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].api_key_set, "api_key_set 应为 true");
        // 序列化结果不含真实 Key
        let json = serde_json::to_string(&list[0]).unwrap();
        assert!(!json.contains("sk-test"), "对外视图不得回显 Key");
    }

    #[test]
    fn upsert_then_update_keeps_blank_key() {
        let svc = service();
        svc.upsert(&input("deepseek")).unwrap();
        // 更新：换模型、Key 留空
        let mut upd = input("deepseek");
        upd.api_key = String::new();
        upd.model = "m2".into();
        svc.upsert(&upd).unwrap();

        let cfg = svc.enabled_config();
        assert!(cfg.is_err(), "未启用时 enabled_config 应报错");

        svc.set_enabled("deepseek", true).unwrap();
        let cfg = svc.enabled_config().unwrap();
        assert_eq!(cfg.model, "m2");
        assert_eq!(cfg.api_key, "sk-test", "Key 留空更新应保持原值");
    }

    #[test]
    fn enable_is_exclusive() {
        let svc = service();
        svc.upsert(&input("deepseek")).unwrap();
        svc.upsert(&input("zhipu")).unwrap();
        svc.set_enabled("deepseek", true).unwrap();
        svc.set_enabled("zhipu", true).unwrap();
        let cfg = svc.enabled_config().unwrap();
        assert_eq!(cfg.provider_key, "zhipu", "后启用的应独占启用位");
        let list = svc.list().unwrap();
        let enabled: Vec<_> = list.iter().filter(|p| p.is_enabled).collect();
        assert_eq!(enabled.len(), 1);
    }

    #[test]
    fn delete_and_enable_missing() {
        let svc = service();
        svc.upsert(&input("deepseek")).unwrap();
        svc.delete("deepseek").unwrap();
        assert!(svc.delete("deepseek").is_err());
        assert!(svc.set_enabled("nope", true).is_err());
    }
}
