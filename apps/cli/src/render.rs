//! 人类可读渲染：词典卡片 / 译文 + 引擎标注 / 降级提示。

use byte_translator_core::service::translate::TranslateOutcome;
use byte_translator_core::translator::TranslationResult;

/// 词典卡片：词条 + 音标 + 引擎标注，逐词性释义，编号例句
fn render_dictionary(result: &TranslationResult) {
    let phonetic = if result.phonetic.is_empty() {
        String::new()
    } else {
        format!("  [{}]", result.phonetic.join("] ["))
    };
    println!(" {}{}   ~  {}", result.text, phonetic, result.engine);
    println!();
    for part in &result.parts {
        println!("  - {} {}", part.pos, part.mean);
    }
    if !result.examples.is_empty() {
        println!();
        for (i, ex) in result.examples.iter().enumerate() {
            println!("  {}. {}", i + 1, ex.orig);
            println!("     {}", ex.trans);
        }
    }
}

/// 译文形态：原文 + 引擎标注，译文缩进
fn render_plain(result: &TranslationResult) {
    println!(" {}   ~  {}", result.text, result.engine);
    println!();
    for line in result.translation.lines() {
        println!("  {line}");
    }
}

/// 渲染调度产出：降级提示行 + 结果
pub fn render(outcome: &TranslateOutcome) {
    if let Some(from) = &outcome.fallback_from {
        println!("⚠️  {from} 未找到结果，已自动切换到 {} 翻译", outcome.result.engine);
        println!();
    }
    let result = &outcome.result;
    if result.parts.is_empty() && result.examples.is_empty() {
        render_plain(result);
    } else {
        render_dictionary(result);
    }
}

/// --json 模式：稳定契约（成功/失败同构）
pub fn outcome_json(outcome: &TranslateOutcome) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "fallback_from": outcome.fallback_from,
        "result": {
            "engine": outcome.result.engine,
            "from": outcome.result.from,
            "to": outcome.result.to,
            "text": outcome.result.text,
            "translation": outcome.result.translation,
            "phonetic": outcome.result.phonetic,
            "parts": outcome.result.parts,
            "examples": outcome.result.examples,
        },
    })
}

pub fn error_json(message: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use byte_translator_core::translator::{Example, PartOfSpeech};

    #[test]
    fn json_contract_shapes() {
        let result = TranslationResult::dictionary(
            "iciba",
            "en",
            "zh",
            "love",
            vec!["lʌv".into()],
            vec![PartOfSpeech { pos: "n.".into(), mean: "爱".into() }],
            vec![Example { orig: "I love you.".into(), trans: "我爱你。".into() }],
        );
        let outcome = TranslateOutcome { result, fallback_from: Some("iciba".into()) };
        let v = outcome_json(&outcome);
        assert_eq!(v["ok"], true);
        assert_eq!(v["fallback_from"], "iciba");
        assert_eq!(v["result"]["engine"], "iciba");
        assert_eq!(v["result"]["parts"][0]["pos"], "n.");
        let e = error_json("boom");
        assert_eq!(e["ok"], false);
        assert_eq!(e["error"], "boom");
    }
}
