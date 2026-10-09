//! iciba 引擎真实接口冒烟：
//! `ICIBA_API_KEY=<自有key> cargo run -p byte-translator-core --example iciba_smoke -- [word]`
//! （缺省公开 Key 已停用；自有 Key 到 open.iciba.com 免费申请）

use byte_translator_core::translator::{iciba::IcibaTranslator, TranslateRequest, Translator};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let word = std::env::args().nth(1).unwrap_or_else(|| "love".into());
    let key = std::env::var("ICIBA_API_KEY").ok().filter(|k| !k.trim().is_empty());
    let engine = IcibaTranslator::new(key);
    let result = engine
        .translate(&TranslateRequest { text: word, from: None, to: None })
        .await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
