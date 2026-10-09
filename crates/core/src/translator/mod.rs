//! 翻译引擎抽象：统一请求/结果结构与引擎 trait。
//!
//! 所有引擎（词典类 / LLM 类 / 第三方扩展）实现 [`Translator`]，
//! 结果归一到 [`TranslationResult`] 后进入渲染/序列化/存储管线。
//! 字段即对外 JSON 契约（`--json` / REST / MCP 共用）：
//!
//! ```json
//! {
//!   "engine": "iciba",
//!   "from": "en",
//!   "to": "zh",
//!   "text": "love",
//!   "translation": "",
//!   "phonetic": ["lʌv"],
//!   "parts": [{"pos": "n.", "mean": "爱"}],
//!   "examples": [{"orig": "...", "trans": "..."}]
//! }
//! ```

pub mod cache;
pub mod iciba;
pub mod llm;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 翻译请求：text 必填；from/to 为 None 表示自动检测（默认中英互译）
#[derive(Debug, Clone)]
pub struct TranslateRequest {
    pub text: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// 词性与释义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartOfSpeech {
    pub pos: String,
    pub mean: String,
}

/// 例句
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Example {
    pub orig: String,
    pub trans: String,
}

/// 统一翻译结果：词典引擎填 phonetic/parts/examples，LLM 引擎填 translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    /// 产出引擎名（iciba / llm / …）
    pub engine: String,
    pub from: String,
    pub to: String,
    /// 原文
    pub text: String,
    /// 译文（LLM/自由文本形态；词典形态可为空）
    pub translation: String,
    /// 音标（英/美，词典形态）
    pub phonetic: Vec<String>,
    /// 词性与释义（词典形态）
    pub parts: Vec<PartOfSpeech>,
    /// 例句（词典形态）
    pub examples: Vec<Example>,
}

impl TranslationResult {
    /// 词典形态结果构造器
    pub fn dictionary(
        engine: &str,
        from: &str,
        to: &str,
        text: &str,
        phonetic: Vec<String>,
        parts: Vec<PartOfSpeech>,
        examples: Vec<Example>,
    ) -> Self {
        Self {
            engine: engine.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            text: text.to_string(),
            translation: String::new(),
            phonetic,
            parts,
            examples,
        }
    }

    /// 文本形态结果构造器
    pub fn plain(engine: &str, from: &str, to: &str, text: &str, translation: &str) -> Self {
        Self {
            engine: engine.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            text: text.to_string(),
            translation: translation.to_string(),
            phonetic: Vec::new(),
            parts: Vec::new(),
            examples: Vec::new(),
        }
    }
}

/// 引擎能力声明：决定调度路由
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// 词典形态：查词（音标/词性/例句），仅中英
    Dictionary,
    /// 文本形态：任意语言对翻译
    Text,
}

/// 翻译引擎接口：插件式扩展点，新增引擎实现本 trait 即可被调度
#[async_trait]
pub trait Translator: Send + Sync {
    /// 引擎名（唯一标识，用于配置与结果标注）
    fn name(&self) -> &'static str;

    /// 引擎形态（词典 / 文本）
    fn kind(&self) -> EngineKind;

    /// 该引擎是否支持给定语言对（from/to 为归一化后的代码）
    fn supports(&self, from: &str, to: &str) -> bool;

    /// 执行翻译；无结果/不支持时返回 Err（调度器决定降级路径）
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult>;
}
