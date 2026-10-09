//! LLM 引擎真实接口冒烟：
//! `LLM_BASE_URL=... LLM_API_KEY=... LLM_MODEL=... cargo run -p byte-translator-core --example llm_smoke -- [text]`

use byte_translator_core::ai::ProviderConfig;
use byte_translator_core::translator::llm::LlmTranslator;
use byte_translator_core::translator::{TranslateRequest, Translator};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let text = std::env::args().nth(1).unwrap_or_else(|| "我想回家了".into());
    let provider = ProviderConfig {
        provider_key: "openai".into(),
        base_url: std::env::var("LLM_BASE_URL").unwrap_or_default(),
        api_key: std::env::var("LLM_API_KEY").unwrap_or_default(),
        model: std::env::var("LLM_MODEL").unwrap_or_default(),
        max_tokens: 0,
        temperature: 0.3,
        models: Vec::new(),
    };
    let engine = LlmTranslator::new(provider);
    let result = engine
        .translate(&TranslateRequest { text, from: None, to: None })
        .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
