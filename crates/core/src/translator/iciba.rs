//! 金山词霸词典引擎：dict-co.iciba.com XML 接口。
//!
//! 词典形态（音标/词性/释义/例句），仅支持中英互译。
//! 免费公开接口有限流风险，外层建议套缓存装饰器。

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use super::{
    EngineKind, Example, PartOfSpeech, TranslateRequest, TranslationResult, Translator,
};
use crate::lang::detect_zh_en;

/// 历史公开 Key（缺省值）。该 Key 已被接口方停用，仅作向后兼容保留；
/// 实际使用需到 open.iciba.com 免费申请自有 Key 并通过配置传入
pub const DEFAULT_PUBLIC_KEY: &str = "D191EBD014295E913574E1EAF8E06666";

const API_URL: &str = "https://dict-co.iciba.com/api/dictionary.php";

/// 词典引擎
pub struct IcibaTranslator {
    api_key: String,
    client: reqwest::Client,
}

impl IcibaTranslator {
    pub fn new(api_key: Option<String>) -> Self {
        let key = api_key.filter(|k| !k.trim().is_empty()).unwrap_or_else(|| DEFAULT_PUBLIC_KEY.to_string());
        Self {
            api_key: key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }
}

#[async_trait]
impl Translator for IcibaTranslator {
    fn name(&self) -> &'static str {
        "iciba"
    }

    fn kind(&self) -> EngineKind {
        EngineKind::Dictionary
    }

    fn supports(&self, from: &str, to: &str) -> bool {
        let zh_en = |a: &str, b: &str| (a == "zh" && b == "en") || (a == "en" && b == "zh");
        zh_en(from, to)
    }

    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let (from, to) = match (&req.from, &req.to) {
            (Some(f), Some(t)) => (f.clone(), t.clone()),
            _ => {
                let (f, t) = detect_zh_en(&req.text);
                (f.to_string(), t.to_string())
            }
        };
        if !self.supports(&from, &to) {
            return Err(anyhow!("iciba 仅支持中英互译（收到 {from}->{to}）"));
        }

        let url = format!("{}?key={}&w={}", API_URL, self.api_key, urlencode(&req.text));
        let resp = self.client.get(&url).send().await.map_err(|e| anyhow!("请求词典接口失败: {e}"))?;
        if !resp.status().is_success() {
            return Err(anyhow!("词典接口返回 {}", resp.status()));
        }
        let body = resp.text().await.map_err(|e| anyhow!("读取词典响应失败: {e}"))?;

        let entry = parse_iciba_xml(&body).ok_or_else(|| anyhow!("未找到词条: {}", req.text))?;
        Ok(TranslationResult::dictionary(
            self.name(),
            &from,
            &to,
            &entry.word,
            entry.phonetic,
            entry.parts,
            entry.examples,
        ))
    }
}

/// URL 编码（百分号 + 大写十六进制，等价 Go 的 url.QueryEscape）
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 解析后的词条
#[derive(Debug, Default, PartialEq)]
pub struct DictEntry {
    pub word: String,
    pub phonetic: Vec<String>,
    pub parts: Vec<PartOfSpeech>,
    pub examples: Vec<Example>,
}

/// 解析 iciba XML 响应；空 dict / 无释义无例句返回 None（未命中）
pub fn parse_iciba_xml(xml: &str) -> Option<DictEntry> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    let root = doc.root_element();
    if root.tag_name().name() != "dict" {
        return None;
    }

    let mut phonetic = Vec::new();
    let mut pos_list = Vec::new();
    let mut mean_list = Vec::new();
    let mut examples = Vec::new();

    for node in root.children() {
        let tag = node.tag_name().name();
        let text = || node.text().unwrap_or("").trim().to_string();
        match tag {
            "key" => {}
            "ps" => phonetic.push(text()),
            "pos" => pos_list.push(text()),
            "acceptation" => mean_list.push(text()),
            "sent" => {
                let mut orig = String::new();
                let mut trans = String::new();
                for child in node.children() {
                    match child.tag_name().name() {
                        "orig" => orig = child.text().unwrap_or("").trim().to_string(),
                        "trans" => trans = child.text().unwrap_or("").trim().to_string(),
                        _ => {}
                    }
                }
                if !orig.is_empty() || !trans.is_empty() {
                    examples.push(Example { orig, trans });
                }
            }
            _ => {}
        }
    }

    let word = root
        .children()
        .find(|n| n.tag_name().name() == "key")
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_string();

    if word.is_empty() && pos_list.is_empty() && examples.is_empty() {
        return None;
    }

    // pos 与 acceptation 按出现顺序一一对应（空 pos 跳过）
    let parts = pos_list
        .into_iter()
        .zip(mean_list)
        .filter(|(pos, mean)| !pos.is_empty() && !mean.is_empty())
        .map(|(pos, mean)| PartOfSpeech { pos, mean })
        .collect::<Vec<_>>();
    // 释义与例句全空视为未命中（接口对句子/生僻词返回空 dict）
    if parts.is_empty() && examples.is_empty() {
        return None;
    }

    Some(DictEntry {
        word,
        phonetic,
        parts,
        examples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<dict>
    <key>love</key>
    <ps>lʌv</ps>
    <ps>lʌv</ps>
    <pos>n.</pos>
    <acceptation>恋爱；亲爱的；酷爱；喜爱的事物</acceptation>
    <pos>v.</pos>
    <acceptation>爱，热爱；爱戴；赞美</acceptation>
    <sent><orig>Love is the radical of lovely.</orig><trans>Love是lovely的词根。</trans></sent>
    <sent><orig>I love you.</orig><trans>我爱你。</trans></sent>
</dict>"#;

    #[test]
    fn parses_full_entry() {
        let e = parse_iciba_xml(SAMPLE).expect("应解析成功");
        assert_eq!(e.word, "love");
        assert_eq!(e.phonetic, vec!["lʌv", "lʌv"]);
        assert_eq!(e.parts.len(), 2);
        assert_eq!(e.parts[0].pos, "n.");
        assert_eq!(e.parts[0].mean, "恋爱；亲爱的；酷爱；喜爱的事物");
        assert_eq!(e.examples.len(), 2);
        assert_eq!(e.examples[1].orig, "I love you.");
    }

    #[test]
    fn empty_dict_is_miss() {
        assert!(parse_iciba_xml("<dict></dict>").is_none());
        assert!(parse_iciba_xml("").is_none());
        assert!(parse_iciba_xml("not xml at all").is_none());
    }

    #[test]
    fn word_without_meanings_is_miss() {
        // 只有 key 无释义无例句 → 未命中（触发上层降级）
        let xml = "<dict><key>xyzabc</key></dict>";
        assert!(parse_iciba_xml(xml).is_none());
    }

    #[test]
    fn urlencode_keeps_safe_chars() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("你好"), "%E4%BD%A0%E5%A5%BD");
        assert_eq!(urlencode("a-b_c~d.e"), "a-b_c~d.e");
    }

    #[test]
    fn supports_only_zh_en() {
        let t = IcibaTranslator::new(None);
        assert!(t.supports("en", "zh"));
        assert!(t.supports("zh", "en"));
        assert!(!t.supports("ja", "en"));
        assert!(!t.supports("en", "ja"));
    }
}
