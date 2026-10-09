//! 后端门面：本地（直连领域服务）与远程（REST）双模式。
//!
//! `--remote <URL>` 让其它机器免本地数据直连 `btr serve` 挂起的服务；
//! 查询类命令本地/远程输出同构（--json 契约一致）。
//! 供应商管理仅本地（写操作要求主力机执行）。

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

use byte_translator_core::db::Database;
use byte_translator_core::service::history::{HistoryQuery, HistoryService};
use byte_translator_core::service::provider::{ProviderInput, ProviderService};
use byte_translator_core::service::translate::{EngineChoice, TranslateOutcome, TranslateService};
use byte_translator_core::service::wordbook::WordbookService;
use byte_translator_core::translator::TranslateRequest;

pub enum Backend {
    Local {
        translate: TranslateService,
        history: HistoryService,
        wordbook: WordbookService,
        providers: ProviderService,
    },
    Remote {
        client: reqwest::Client,
        base: String,
        key: String,
    },
}

impl Backend {
    pub fn local(db: Arc<Database>) -> Self {
        Backend::Local {
            translate: TranslateService::production(db.clone()),
            history: HistoryService::new(db.clone()),
            wordbook: WordbookService::new(db.clone()),
            providers: ProviderService::new(db),
        }
    }

    pub fn remote(url: &str, key: &str) -> Self {
        let base = url.trim_end_matches('/').to_string();
        Backend::Remote {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("构建 HTTP 客户端失败"),
            base,
            key: key.to_string(),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Backend::Remote { .. })
    }

    fn require_local(&self, op: &str) -> Result<&Self> {
        if self.is_remote() {
            return Err(anyhow!("远程模式不支持「{op}」（请在数据所在机器本地执行）"));
        }
        Ok(self)
    }

    // ===== 翻译 =====

    pub async fn translate(
        &self,
        req: &TranslateRequest,
        choice: &EngineChoice,
    ) -> Result<TranslateOutcome> {
        match self {
            Backend::Local { translate, history, .. } => {
                let outcome = translate.translate(req, choice).await?;
                let _ = history.record(&outcome.result);
                Ok(outcome)
            }
            Backend::Remote { client, base, key } => {
                let engine = match choice {
                    EngineChoice::Auto => None,
                    EngineChoice::Named(n) => Some(n.clone()),
                };
                let body = serde_json::json!({
                    "text": req.text,
                    "from": req.from,
                    "to": req.to,
                    "engine": engine,
                });
                let v: serde_json::Value = self
                    .rest(client, base, key, reqwest::Method::POST, "/api/v1/translate", &body)
                    .await?;
                wire_to_outcome(v)
            }
        }
    }

    // ===== 历史 =====

    pub async fn history_list(&self, q: &HistoryQuery) -> Result<serde_json::Value> {
        match self {
            Backend::Local { history, .. } => {
                let (list, total) = history.search(q)?;
                Ok(serde_json::json!({ "total": total, "list": list }))
            }
            Backend::Remote { client, base, key } => {
                let mut url = format!(
                    "/api/v1/history?page={}&size={}",
                    q.page.max(1),
                    q.size.clamp(1, 100)
                );
                if !q.keyword.is_empty() {
                    url.push_str(&format!("&keyword={}", urlencode(&q.keyword)));
                }
                if !q.engine.is_empty() {
                    url.push_str(&format!("&engine={}", urlencode(&q.engine)));
                }
                if !q.from.is_empty() {
                    url.push_str(&format!("&from={}", urlencode(&q.from)));
                }
                if !q.to.is_empty() {
                    url.push_str(&format!("&to={}", urlencode(&q.to)));
                }
                self.rest(client, base, key, reqwest::Method::GET, &url, &serde_json::json!({}))
                    .await
            }
        }
    }

    // ===== 生词本 =====

    pub async fn wordbook_list(&self, keyword: &str, page: i64, size: i64) -> Result<serde_json::Value> {
        match self {
            Backend::Local { wordbook, .. } => {
                let (list, total) = wordbook.list(keyword, page, size)?;
                Ok(serde_json::json!({ "total": total, "list": list }))
            }
            Backend::Remote { client, base, key } => {
                let url = format!(
                    "/api/v1/wordbook?page={page}&size={size}&keyword={}",
                    urlencode(keyword)
                );
                self.rest(client, base, key, reqwest::Method::GET, &url, &serde_json::json!({}))
                    .await
            }
        }
    }

    pub async fn wordbook_add(&self, text: &str, note: &str) -> Result<serde_json::Value> {
        match self {
            Backend::Local { wordbook, .. } => {
                wordbook.add(text, note)?;
                Ok(serde_json::json!({ "ok": true, "text": text }))
            }
            Backend::Remote { client, base, key } => {
                let body = serde_json::json!({ "text": text, "note": note });
                self.rest(client, base, key, reqwest::Method::POST, "/api/v1/wordbook", &body)
                    .await
            }
        }
    }

    pub async fn wordbook_remove(&self, id: i64) -> Result<serde_json::Value> {
        match self {
            Backend::Local { wordbook, .. } => {
                wordbook.remove(id)?;
                Ok(serde_json::json!({ "ok": true, "id": id }))
            }
            Backend::Remote { client, base, key } => {
                let url = format!("/api/v1/wordbook/{id}");
                self.rest(client, base, key, reqwest::Method::DELETE, &url, &serde_json::json!({}))
                    .await
            }
        }
    }

    // ===== 供应商（仅本地）=====

    pub async fn provider_list(&self) -> Result<serde_json::Value> {
        let Backend::Local { providers, .. } = self.require_local("provider list")? else {
            unreachable!()
        };
        let list = providers.list()?;
        Ok(serde_json::json!({ "list": list }))
    }

    pub async fn provider_set(&self, input: &ProviderInput, enable: Option<bool>) -> Result<serde_json::Value> {
        let Backend::Local { providers, .. } = self.require_local("provider set")? else {
            unreachable!()
        };
        providers.upsert(input)?;
        if let Some(on) = enable {
            if on {
                providers.set_enabled(&input.provider_key, true)?;
            }
        }
        Ok(serde_json::json!({ "ok": true, "provider_key": input.provider_key }))
    }

    pub async fn provider_enable(&self, key: &str, on: bool) -> Result<serde_json::Value> {
        let Backend::Local { providers, .. } = self.require_local("provider enable")? else {
            unreachable!()
        };
        providers.set_enabled(key, on)?;
        Ok(serde_json::json!({ "ok": true, "provider_key": key, "enabled": on }))
    }

    pub async fn provider_delete(&self, key: &str) -> Result<serde_json::Value> {
        let Backend::Local { providers, .. } = self.require_local("provider delete")? else {
            unreachable!()
        };
        providers.delete(key)?;
        Ok(serde_json::json!({ "ok": true, "provider_key": key }))
    }

    pub async fn provider_test(&self, key: &str) -> Result<serde_json::Value> {
        let Backend::Local { providers, .. } = self.require_local("provider test")? else {
            unreachable!()
        };
        providers.test_connectivity(key).await?;
        Ok(serde_json::json!({ "ok": true, "provider_key": key, "reachable": true }))
    }

    // ===== REST 基座 =====

    async fn rest(
        &self,
        client: &reqwest::Client,
        base: &str,
        key: &str,
        method: reqwest::Method,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{base}{path}");
        let has_body = method != reqwest::Method::GET && method != reqwest::Method::DELETE;
        let mut req = client
            .request(method, &url)
            .header("x-api-key", key)
            .header("host", host_of(base)?);
        if has_body {
            req = req.json(body);
        }
        let resp = req.send().await.context("远程请求失败（服务未启动？）")?;
        let status = resp.status();
        let v: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        if !status.is_success() {
            let msg = v["error"].as_str().unwrap_or("未知错误");
            return Err(anyhow!("远程服务返回 {status}: {msg}"));
        }
        Ok(v)
    }
}

/// REST 响应 → 调度产出（与本地渲染共用结构）
fn wire_to_outcome(v: serde_json::Value) -> Result<TranslateOutcome> {
    if v["ok"].as_bool() != Some(true) {
        let msg = v["error"].as_str().unwrap_or("远程翻译失败");
        return Err(anyhow!("{msg}"));
    }
    let result: byte_translator_core::translator::TranslationResult =
        serde_json::from_value(v["result"].clone()).context("远程结果解析失败")?;
    let fallback_from = v["fallback_from"].as_str().map(String::from);
    Ok(TranslateOutcome { result, fallback_from })
}

fn host_of(base: &str) -> Result<String> {
    let rest = base
        .strip_prefix("http://")
        .or_else(|| base.strip_prefix("https://"))
        .ok_or_else(|| anyhow!("--remote 需以 http:// 或 https:// 开头"))?;
    Ok(rest.split('/').next().unwrap_or(rest).to_string())
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
