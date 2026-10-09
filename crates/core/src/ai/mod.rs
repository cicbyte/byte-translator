//! AI 网关：供应商配置 → rig 模型句柄 → 单轮补全（非流式 + 流式）。
//!
//! 多供应商统一抽象（OpenAI 兼容 / Anthropic / Gemini / Ollama），
//! 补全一律走 rig；连通探测与模型列表用共享 reqwest 客户端（不耗 token）。
//! API Key 仅在本层与存储层流转，任何对外视图不回显。

pub mod prompt;

use anyhow::{anyhow, Result};
use futures::StreamExt;
use std::time::Duration;

use rig_core::completion::message::{AssistantContent, Message};
use rig_core::completion::{CompletionRequest, CompletionResponse};
use rig_core::driver::DynModel;
use rig_core::operation::Completion;
use rig_core::streaming::{Item, StreamEvent};

/// 单轮补全上限
const TURN_TIMEOUT: Duration = Duration::from_secs(300);
/// 流式空闲超时：网络半开/代理挂住时事件永不到达，此闸防永久挂起
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
/// 流式取消轮询节拍（同步取消标志靠周期 tick 检查）
const CANCEL_TICK: Duration = Duration::from_millis(200);

/// 接入协议：决定 rig 客户端构造方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    OpenAiCompatible,
    Anthropic,
    Gemini,
    Ollama,
}

impl Protocol {
    /// 按 provider_key 前缀匹配（大小写不敏感）；未知值回落 OpenAI 兼容
    pub fn parse(s: &str) -> Self {
        let s = s.trim().to_ascii_lowercase();
        if s.starts_with("anthropic") {
            Self::Anthropic
        } else if s.starts_with("gemini") {
            Self::Gemini
        } else if s.starts_with("ollama") {
            Self::Ollama
        } else {
            Self::OpenAiCompatible
        }
    }
}

/// 供应商配置（补全用完整形态；对外展示用脱敏视图）
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderConfig {
    pub provider_key: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: i64,
    pub temperature: f64,
}

/// 归一化供应商 base_url 为「协议根」：
/// 去首尾空白与尾部斜杠、剥离已拼好的 /chat/completions 与 /v1
/// （是否补 /v1 由各协议决定：OpenAI 兼容补，其余原生不补）
pub fn normalize_base_url(base_url: &str) -> String {
    let mut s = base_url.trim().to_string();
    while s.ends_with('/') {
        s.pop();
    }
    let lower = s.to_ascii_lowercase();
    if lower.ends_with("/chat/completions") {
        s.truncate(s.len() - "/chat/completions".len());
        while s.ends_with('/') {
            s.pop();
        }
    }
    if s.to_ascii_lowercase().ends_with("/v1") {
        for _ in 0..3 {
            s.pop();
        }
        while s.ends_with('/') {
            s.pop();
        }
    }
    s
}

/// 按协议构造 rig 模型句柄（协议分发收敛点：新增协议只改这里）
fn build_model(cfg: &ProviderConfig) -> DynModel<Completion> {
    let base = normalize_base_url(&cfg.base_url);
    let model_name = cfg.model.clone();
    match Protocol::parse(&cfg.provider_key) {
        Protocol::OpenAiCompatible => {
            rig_core::providers::openai::OpenAIConfig::new(cfg.api_key.clone())
                .with_base_url(format!("{base}/v1"))
                .client()
                .completion(model_name)
                .erase()
        }
        Protocol::Anthropic => {
            rig_core::providers::anthropic::wire::AnthropicConfig::new(cfg.api_key.clone())
                .with_base_url(&base)
                .client()
                .completion(model_name)
                .erase()
        }
        Protocol::Gemini => {
            rig_core::providers::gemini::GeminiConfig::new(cfg.api_key.clone())
                .with_base_url(&base)
                .client()
                .completion(model_name)
                .erase()
        }
        Protocol::Ollama => {
            let mut config =
                rig_core::providers::ollama::wire::OllamaConfig::new().with_base_url(&base);
            if !cfg.api_key.is_empty() {
                config = config.with_api_key(cfg.api_key.clone());
            }
            config.client().completion(model_name).erase()
        }
    }
}

fn build_request(cfg: &ProviderConfig, system: &str, user: &str) -> CompletionRequest {
    let mut req = CompletionRequest::new(Message::user(user))
        .preamble(system)
        .temperature(cfg.temperature);
    if cfg.max_tokens > 0 {
        req = req.max_tokens(cfg.max_tokens as u64);
    }
    req
}

fn text_from_response(resp: &CompletionResponse) -> String {
    let mut text = String::new();
    for block in &resp.choice {
        if let AssistantContent::Text(t) = block {
            text.push_str(t.text());
        }
    }
    text
}

/// 单轮补全（非流式）：system 指令 + user 输入 → 文本输出
pub async fn complete(cfg: &ProviderConfig, system: &str, user: &str) -> Result<String> {
    let model = build_model(cfg);
    let req = build_request(cfg, system, user);
    let resp = tokio::time::timeout(TURN_TIMEOUT, model.call(req))
        .await
        .map_err(|_| anyhow!("LLM 请求超时（300s）"))?
        .map_err(|e| anyhow!("LLM 请求失败: {e}"))?;
    Ok(text_from_response(&resp))
}

/// 单轮流式补全：`on_delta(text)` 增量回调；`should_stop` 为 true 时中断。
/// 返回拼接的完整文本。
pub async fn complete_stream(
    cfg: &ProviderConfig,
    system: &str,
    user: &str,
    mut on_delta: impl FnMut(&str),
    should_stop: impl Fn() -> bool,
) -> Result<String> {
    let model = build_model(cfg);
    let req = build_request(cfg, system, user);
    let mut stream = model
        .stream(req)
        .map_err(|e| anyhow!("LLM 流式请求失败: {e}"))?;

    let mut full = String::new();
    let mut cancel_tick = tokio::time::interval(CANCEL_TICK);
    cancel_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        let item = tokio::select! {
            biased;
            _ = cancel_tick.tick() => {
                if should_stop() {
                    return Err(anyhow!("已取消"));
                }
                continue;
            }
            item = tokio::time::timeout(STREAM_IDLE_TIMEOUT, stream.next()) => {
                match item {
                    Err(_) => return Err(anyhow!("LLM 流式空闲超时（120s 无事件）")),
                    Ok(None) => break,
                    Ok(Some(Ok(ev))) => ev,
                    Ok(Some(Err(e))) => return Err(anyhow!("LLM 流中断: {e}")),
                }
            }
        };
        match item {
            Item::Event(StreamEvent::Text { text, .. }) => {
                full.push_str(&text);
                on_delta(&text);
            }
            // Reasoning / Arguments / Start / End 与未建模负载：翻译场景不消费
            _ => {}
        }
    }
    Ok(full)
}

/// 连通测试（轻量探测，不耗 token）：OpenAI 兼容端点 GET /models
pub async fn probe(cfg: &ProviderConfig) -> Result<()> {
    let base = normalize_base_url(&cfg.base_url);
    let url = match Protocol::parse(&cfg.provider_key) {
        Protocol::OpenAiCompatible => format!("{base}/v1/models"),
        _ => format!("{base}/api/tags"),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut req = client.get(&url);
    if !cfg.api_key.is_empty() {
        req = req.bearer_auth(&cfg.api_key);
    }
    let resp = req.send().await.map_err(|e| anyhow!("连接失败: {e}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!("服务返回 {}", resp.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_parse() {
        assert_eq!(Protocol::parse("deepseek"), Protocol::OpenAiCompatible);
        assert_eq!(Protocol::parse("zhipu"), Protocol::OpenAiCompatible);
        assert_eq!(Protocol::parse("openai"), Protocol::OpenAiCompatible);
        assert_eq!(Protocol::parse(""), Protocol::OpenAiCompatible);
        assert_eq!(Protocol::parse("anthropic"), Protocol::Anthropic);
        assert_eq!(Protocol::parse("Gemini-2"), Protocol::Gemini);
        assert_eq!(Protocol::parse("Ollama-3"), Protocol::Ollama);
    }

    #[test]
    fn base_url_normalization() {
        assert_eq!(normalize_base_url(" https://api.deepseek.com/ "), "https://api.deepseek.com");
        assert_eq!(
            normalize_base_url("https://api.deepseek.com/v1/"),
            "https://api.deepseek.com"
        );
        assert_eq!(
            normalize_base_url("https://api.x.com/v1/chat/completions"),
            "https://api.x.com"
        );
        assert_eq!(normalize_base_url("http://localhost:11434/"), "http://localhost:11434");
    }
}
