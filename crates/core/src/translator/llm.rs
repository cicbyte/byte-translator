//! LLM 翻译引擎：供应商配置 + 翻译提示词 → rig 单轮补全。

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use super::{EngineKind, TranslateRequest, TranslationResult, Translator};
use crate::ai::{self, prompt, ProviderConfig};
use crate::lang::detect_zh_en;

/// LLM 引擎（任意语言对）。持有供应商配置快照——切换供应商后需重建实例。
pub struct LlmTranslator {
    provider: ProviderConfig,
}

impl LlmTranslator {
    pub fn new(provider: ProviderConfig) -> Self {
        Self { provider }
    }

    fn resolve_langs(req: &TranslateRequest) -> (String, String) {
        match (&req.from, &req.to) {
            (Some(f), Some(t)) => (f.clone(), t.clone()),
            _ => {
                let (f, t) = detect_zh_en(&req.text);
                (f.to_string(), t.to_string())
            }
        }
    }
}

#[async_trait]
impl Translator for LlmTranslator {
    fn name(&self) -> &'static str {
        "llm"
    }

    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }

    fn supports(&self, _from: &str, _to: &str) -> bool {
        true
    }

    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let (from, to) = Self::resolve_langs(req);
        let system = prompt::translation_prompt(Some(&from), Some(&to));
        let text = ai::complete(&self.provider, &system, &req.text).await?;
        let text = text.trim();
        if text.is_empty() {
            return Err(anyhow!("LLM 返回空结果"));
        }
        Ok(TranslationResult::plain(self.name(), &from, &to, &req.text, text))
    }
}

/// 流式翻译：增量回调 + 取消标志（REST SSE / 桌面端用）
pub async fn translate_stream(
    provider: &ProviderConfig,
    req: &TranslateRequest,
    on_delta: impl FnMut(&str),
    should_stop: impl Fn() -> bool,
) -> Result<TranslationResult> {
    let (from, to) = LlmTranslator::resolve_langs(req);
    let system = prompt::translation_prompt(Some(&from), Some(&to));
    let text = ai::complete_stream(provider, &system, &req.text, on_delta, should_stop).await?;
    let text = text.trim();
    if text.is_empty() {
        return Err(anyhow!("LLM 返回空结果"));
    }
    Ok(TranslationResult::plain("llm", &from, &to, &req.text, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_langs_auto_detection() {
        let req = TranslateRequest { text: "你好".into(), from: None, to: None };
        assert_eq!(LlmTranslator::resolve_langs(&req), ("zh".into(), "en".into()));
        let req = TranslateRequest { text: "hello".into(), from: None, to: None };
        assert_eq!(LlmTranslator::resolve_langs(&req), ("en".into(), "zh".into()));
        let req = TranslateRequest { text: "こんにちは".into(), from: Some("ja".into()), to: Some("en".into()) };
        assert_eq!(LlmTranslator::resolve_langs(&req), ("ja".into(), "en".into()));
    }

    #[test]
    fn llm_supports_any_pair() {
        let t = LlmTranslator::new(ProviderConfig {
            provider_key: "openai".into(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            max_tokens: 0,
            temperature: 0.3,
            models: Vec::new(),
        });
        assert!(t.supports("ja", "fr"));
        assert_eq!(t.kind(), EngineKind::Text);
        assert_eq!(t.name(), "llm");
    }
}
