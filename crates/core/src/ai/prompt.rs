//! 翻译提示词构造：两形态——默认中英互译 / 显式语言对。

use crate::lang;

/// 默认形态：自动中英互译（含 CJK 判定由输入决定方向）
const AUTO_PROMPT: &str = "\
You are a professional translator. Translate the input accurately.

Rules:
- Chinese input → translate to English
- English input → translate to Chinese

Output ONLY the translation, nothing else. No explanations, no examples, no extra text.";

/// 显式语言对形态：from/to 为归一化语言代码（如 zh、en、ja）
pub fn directed_prompt(from: &str, to: &str) -> String {
    format!(
        "You are a professional translator. Translate the input from {} to {}.\n\
         Output ONLY the translation, nothing else. No explanations, no examples, no extra text.",
        lang::lang_name(from),
        lang::lang_name(to),
    )
}

/// 按请求构造 system prompt：语言对齐全时走显式形态，否则默认中英互译
pub fn translation_prompt(from: Option<&str>, to: Option<&str>) -> String {
    match (from, to) {
        (Some(f), Some(t)) if !f.is_empty() && !t.is_empty() => directed_prompt(f, t),
        _ => AUTO_PROMPT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_when_missing_pair() {
        let p = translation_prompt(None, None);
        assert!(p.contains("Chinese input → translate to English"));

        let p2 = translation_prompt(Some("zh"), Some(""));
        assert!(p2.contains("Chinese input → translate to English"), "空 to 应回落自动形态");
    }

    #[test]
    fn directed_when_pair_given() {
        let p = translation_prompt(Some("ja"), Some("en"));
        assert!(p.contains("from Japanese to English"));
        assert!(!p.contains("Chinese input"));
    }

    #[test]
    fn directed_unknown_lang_falls_back_to_code() {
        let p = translation_prompt(Some("xx"), Some("zh"));
        assert!(p.contains("from xx to Chinese"), "未知语言代码原样入 prompt");
    }
}
