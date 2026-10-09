//! MCP 工具实现：service 调用 + 结果文本化。

use crate::db::Database;
use crate::service::history::{HistoryQuery, HistoryService};
use crate::service::translate::{EngineChoice, TranslateService};
use crate::service::wordbook::WordbookService;
use crate::translator::TranslateRequest;
use std::sync::Arc;

/// 工具上下文：每会话共享
pub struct ToolCtx {
    pub translate: TranslateService,
    pub history: HistoryService,
    pub wordbook: WordbookService,
}

impl ToolCtx {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            translate: TranslateService::production(db.clone()),
            history: HistoryService::new(db.clone()),
            wordbook: WordbookService::new(db),
        }
    }
}

fn outcome_text(outcome: &crate::service::translate::TranslateOutcome) -> String {
    let r = &outcome.result;
    let mut out = String::new();
    if let Some(from) = &outcome.fallback_from {
        out.push_str(&format!("（{from} 未命中，已降级）\n"));
    }
    if r.parts.is_empty() && r.examples.is_empty() {
        out.push_str(&r.translation);
        return out;
    }
    // 词典形态
    if !r.phonetic.is_empty() {
        out.push_str(&format!("[{}] ", r.phonetic.join("] [")));
    }
    out.push_str(&format!("{}\n", r.text));
    for part in &r.parts {
        out.push_str(&format!("- {} {}\n", part.pos, part.mean));
    }
    for (i, ex) in r.examples.iter().enumerate() {
        out.push_str(&format!("{}. {}\n   {}\n", i + 1, ex.orig, ex.trans));
    }
    out
}

pub async fn translate(
    ctx: &ToolCtx,
    text: &str,
    from: Option<String>,
    to: Option<String>,
    engine: Option<String>,
) -> String {
    let choice = match engine.as_deref() {
        Some("iciba") | Some("llm") => EngineChoice::Named(engine.unwrap()),
        None => EngineChoice::Auto,
        Some(other) => return format!("错误：不支持的引擎 {other}（可用: iciba / llm）"),
    };
    let req = TranslateRequest {
        text: text.to_string(),
        from: from.filter(|s| !s.is_empty()),
        to: to.filter(|s| !s.is_empty()),
    };
    match ctx.translate.translate(&req, &choice).await {
        Ok(outcome) => {
            let _ = ctx.history.record(&outcome.result);
            outcome_text(&outcome)
        }
        Err(e) => format!("错误：{e:#}"),
    }
}

pub async fn define(ctx: &ToolCtx, word: &str) -> String {
    let req = TranslateRequest { text: word.to_string(), from: None, to: None };
    match ctx
        .translate
        .translate(&req, &EngineChoice::Named("iciba".into()))
        .await
    {
        Ok(outcome) => {
            let _ = ctx.history.record(&outcome.result);
            outcome_text(&outcome)
        }
        Err(e) => format!("错误：{e:#}（可尝试 translate 走 LLM）"),
    }
}

pub fn history_search(ctx: &ToolCtx, keyword: &str, limit: i64) -> String {
    let q = HistoryQuery {
        keyword: keyword.to_string(),
        page: 1,
        size: limit,
        ..Default::default()
    };
    match ctx.history.search(&q) {
        Ok((list, total)) if list.is_empty() => format!("（无匹配，共 {total} 条历史）"),
        Ok((list, total)) => {
            let mut out = format!("共 {total} 条，前 {} 条：\n", list.len());
            for h in &list {
                out.push_str(&format!(
                    "[{}] {} → {}（{}，{}）\n",
                    h.id, h.text, h.to_lang, h.engine, h.created_at
                ));
            }
            out
        }
        Err(e) => format!("错误：{e:#}"),
    }
}

pub fn wordbook_add(ctx: &ToolCtx, text: &str, note: &str) -> String {
    match ctx.wordbook.add(text, note) {
        Ok(()) => format!("已收藏：{text}"),
        Err(e) => format!("错误：{e:#}"),
    }
}

pub fn wordbook_list(ctx: &ToolCtx, keyword: &str, limit: i64) -> String {
    match ctx.wordbook.list(keyword, 1, limit) {
        Ok((list, _)) if list.is_empty() => "（生词本为空）".to_string(),
        Ok((list, total)) => {
            let mut out = format!("共 {total} 词：\n");
            for w in &list {
                if w.note.is_empty() {
                    out.push_str(&format!("- {}\n", w.text));
                } else {
                    out.push_str(&format!("- {}：{}\n", w.text, w.note));
                }
            }
            out
        }
        Err(e) => format!("错误：{e:#}"),
    }
}
