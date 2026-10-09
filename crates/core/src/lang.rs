//! 语言工具：ISO 639 代码约定与文本语言检测。
//!
//! 自动检测采用保守的字符集启发式：含 CJK 表意字符判为 `zh`，
//! 其余判为 `en`——仅用于默认中英互译的路由决策，显式指定的
//! 语言对永远优先于检测结果。

/// 判断文本是否含 CJK 表意字符（含扩展 A 区）
pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c)
    })
}

/// 默认中英互译：检测源语言并返回 (from, to)
pub fn detect_zh_en(text: &str) -> (&'static str, &'static str) {
    if contains_cjk(text) {
        ("zh", "en")
    } else {
        ("en", "zh")
    }
}

/// 归一化语言代码：小写、去空白；`zh-CN`/`zh_CN` → `zh`
pub fn normalize_lang(code: &str) -> String {
    let base = code.trim().to_lowercase();
    match base.split(['-', '_']).next() {
        Some(b) if !b.is_empty() => b.to_string(),
        _ => base,
    }
}

/// 语言代码 → 英文名称（prompt 用）；未知代码原样返回
pub fn lang_name(code: &str) -> String {
    let c = normalize_lang(code);
    match c.as_str() {
        "zh" => "Chinese".into(),
        "en" => "English".into(),
        "ja" => "Japanese".into(),
        "ko" => "Korean".into(),
        "fr" => "French".into(),
        "de" => "German".into(),
        "es" => "Spanish".into(),
        "pt" => "Portuguese".into(),
        "it" => "Italian".into(),
        "ru" => "Russian".into(),
        "ar" => "Arabic".into(),
        "hi" => "Hindi".into(),
        "th" => "Thai".into(),
        "vi" => "Vietnamese".into(),
        "id" => "Indonesian".into(),
        "tr" => "Turkish".into(),
        "nl" => "Dutch".into(),
        "pl" => "Polish".into(),
        "uk" => "Ukrainian".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_detection() {
        assert!(contains_cjk("你好"));
        assert!(contains_cjk("hello 世界"));
        assert!(!contains_cjk("hello world"));
        assert!(!contains_cjk("café"));
    }

    #[test]
    fn detect_pairs() {
        assert_eq!(detect_zh_en("你好"), ("zh", "en"));
        assert_eq!(detect_zh_en("hello"), ("en", "zh"));
        assert_eq!(detect_zh_en("mix 中文 text"), ("zh", "en"));
    }

    #[test]
    fn lang_normalization() {
        assert_eq!(normalize_lang(" zh-CN "), "zh");
        assert_eq!(normalize_lang("EN_us"), "en");
        assert_eq!(normalize_lang("ja"), "ja");
    }

    #[test]
    fn lang_names() {
        assert_eq!(lang_name("zh-CN"), "Chinese");
        assert_eq!(lang_name("JA"), "Japanese");
        assert_eq!(lang_name("xx"), "xx");
    }
}
