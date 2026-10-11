//! 免 Key 翻译引擎：Google 社区接口与 MyMemory。
//!
//! 两者都是公开 Web 接口（非官方付费 API）：无需配置开箱即用，
//! 稳定性依赖服务方政策，作为「免 Key 即用」分组提供。

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use super::{EngineKind, TranslateRequest, TranslationResult, Translator};

/// 公共：构造 Text 形态结果
fn text_result(engine: &str, from: &str, to: &str, req_text: &str, translation: &str) -> TranslationResult {
    TranslationResult::plain(engine, from, to, req_text, translation)
}

// ============ Google（translate_a/single 社区接口，免 Key） ============

pub struct GoogleTranslator {
    client: reqwest::Client,
}

impl GoogleTranslator {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }
}

#[async_trait]
impl Translator for GoogleTranslator {
    fn name(&self) -> &'static str {
        "google"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }
    fn supports(&self, _from: &str, _to: &str) -> bool {
        true
    }
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let from = req.from.as_deref().unwrap_or("auto");
        let to = req.to.as_deref().ok_or_else(|| anyhow!("google 引擎需要目标语言"))?;
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={from}&tl={to}&dt=t&q={}",
            urlencode(&req.text),
        );
        let resp: Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("google 请求失败: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("google 响应解析失败: {e}"))?;
        // 响应形状：[[["译文","原文",..],..], .., "检测语言", ..]
        let mut out = String::new();
        if let Some(segs) = resp.get(0).and_then(|v| v.as_array()) {
            for seg in segs {
                if let Some(t) = seg.get(0).and_then(|v| v.as_str()) {
                    out.push_str(t);
                }
            }
        }
        if out.is_empty() {
            return Err(anyhow!("google 未返回译文"));
        }
        let detected = resp.get(2).and_then(|v| v.as_str()).unwrap_or(from).to_string();
        Ok(text_result(self.name(), &detected, to, &req.text, &out))
    }
}

// ============ MyMemory（免 Key，需具体语言对） ============

pub struct MyMemoryTranslator {
    client: reqwest::Client,
}

impl MyMemoryTranslator {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }
}

#[async_trait]
impl Translator for MyMemoryTranslator {
    fn name(&self) -> &'static str {
        "mymemory"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }
    fn supports(&self, from: &str, to: &str) -> bool {
        // langpair 需要具体语言对；调度层归一化后 from/to 总有具体值
        !from.is_empty() && !to.is_empty() && from != "auto"
    }
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let from = req.from.as_deref().filter(|s| !s.is_empty() && *s != "auto").unwrap_or("en");
        let to = req.to.as_deref().filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("mymemory 需要目标语言"))?;
        let url = format!(
            "https://api.mymemory.translated.net/get?q={}&langpair={from}|{to}",
            urlencode(&req.text),
        );
        let resp: Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("mymemory 请求失败: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("mymemory 响应解析失败: {e}"))?;
        let status = resp.get("responseStatus").and_then(|v| v.as_i64()).unwrap_or(0);
        let text = resp
            .get("responseData")
            .and_then(|d| d.get("translatedText"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if status != 200 || text.is_empty() {
            let detail = if text.is_empty() { "未返回译文" } else { &text };
            return Err(anyhow!("mymemory 失败（status={status}）: {detail}"));
        }
        Ok(text_result(self.name(), from, to, &req.text, &text))
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
