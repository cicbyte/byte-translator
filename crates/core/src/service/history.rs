//! 翻译历史：自动留存与检索。REST / CLI / MCP 共用。

use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::db::Database;
use crate::translator::TranslationResult;

/// 历史条目（对外视图）
#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub engine: String,
    pub from_lang: String,
    pub to_lang: String,
    pub created_at: String,
    /// 完整翻译结果（JSON 字符串，结构同 TranslationResult）
    pub result_json: String,
}

/// 检索条件：全部条件可组合，均为可选
#[derive(Debug, Default, Clone)]
pub struct HistoryQuery {
    /// 关键词（原文 LIKE 匹配）
    pub keyword: String,
    /// 引擎过滤（iciba / llm）
    pub engine: String,
    /// 源语言过滤（归一化代码）
    pub from: String,
    /// 目标语言过滤
    pub to: String,
    pub page: i64,
    pub size: i64,
}

/// LIKE 关键字转义：\ % _ 前加反斜杠（配合 SQL 的 ESCAPE '\'），
/// 防止用户输入 %/_ 被当通配符放大查询
fn escape_like(kw: &str) -> String {
    let mut out = String::with_capacity(kw.len() + 4);
    for c in kw.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub struct HistoryService {
    db: Arc<Database>,
}

impl HistoryService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 翻译完成后自动留存（失败仅记日志，不阻断翻译主流程）
    pub fn record(&self, result: &TranslationResult) -> Result<()> {
        let json = serde_json::to_string(result)?;
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO history (text, result_json, engine, from_lang, to_lang) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![result.text, json, result.engine, result.from, result.to],
        )?;
        Ok(())
    }

    /// 检索：关键词/引擎/语言对过滤 + 分页，返回 (条目, 总数)
    pub fn search(&self, q: &HistoryQuery) -> Result<(Vec<HistoryEntry>, i64)> {
        let page = q.page.max(1);
        let size = q.size.clamp(1, 100);
        let offset = (page - 1) * size;

        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if !q.keyword.trim().is_empty() {
            where_clauses.push(format!(
                "text LIKE ?{} ESCAPE '\\'",
                params.len() + 1
            ));
            params.push(Box::new(format!("%{}%", escape_like(q.keyword.trim()))));
        }
        if !q.engine.trim().is_empty() {
            where_clauses.push(format!("engine = ?{}", params.len() + 1));
            params.push(Box::new(q.engine.trim().to_string()));
        }
        if !q.from.trim().is_empty() {
            where_clauses.push(format!("from_lang = ?{}", params.len() + 1));
            params.push(Box::new(q.from.trim().to_string()));
        }
        if !q.to.trim().is_empty() {
            where_clauses.push(format!("to_lang = ?{}", params.len() + 1));
            params.push(Box::new(q.to.trim().to_string()));
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;

        let count_sql = format!("SELECT COUNT(*) FROM history {where_sql}");
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |r| r.get(0),
        )?;

        let list_sql = format!(
            "SELECT id, text, engine, from_lang, to_lang, created_at, result_json \
             FROM history {where_sql} ORDER BY id DESC LIMIT {size} OFFSET {offset}"
        );
        let mut stmt = conn.prepare(&list_sql)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |r| {
                    Ok(HistoryEntry {
                        id: r.get(0)?,
                        text: r.get(1)?,
                        engine: r.get(2)?,
                        from_lang: r.get(3)?,
                        to_lang: r.get(4)?,
                        created_at: r.get(5)?,
                        result_json: r.get(6)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok((rows, total))
    }

    /// 最近 N 条（快捷视图）
    pub fn recent(&self, limit: i64) -> Result<Vec<HistoryEntry>> {
        let (list, _) = self.search(&HistoryQuery {
            page: 1,
            size: limit,
            ..Default::default()
        })?;
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> HistoryService {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history-test.db");
        std::mem::forget(tmp);
        HistoryService::new(Arc::new(Database::open(&path).unwrap()))
    }

    fn result(engine: &str, text: &str, from: &str, to: &str) -> TranslationResult {
        TranslationResult::plain(engine, from, to, text, "译文")
    }

    #[test]
    fn record_then_search_roundtrip() {
        let svc = service();
        svc.record(&result("iciba", "love", "en", "zh")).unwrap();
        svc.record(&result("llm", "I want to go home", "en", "zh")).unwrap();
        svc.record(&result("llm", "我想回家了", "zh", "en")).unwrap();

        let (all, total) = svc
            .search(&HistoryQuery { page: 1, size: 10, ..Default::default() })
            .unwrap();
        assert_eq!(total, 3);
        assert_eq!(all[0].text, "我想回家了", "倒序：最新在前");
        // result_json 可反序列化回统一结构
        let parsed: TranslationResult = serde_json::from_str(&all[0].result_json).unwrap();
        assert_eq!(parsed.engine, "llm");
    }

    #[test]
    fn filters_combine() {
        let svc = service();
        svc.record(&result("iciba", "love", "en", "zh")).unwrap();
        svc.record(&result("llm", "love story", "en", "zh")).unwrap();
        svc.record(&result("llm", "我想回家了", "zh", "en")).unwrap();

        // 引擎过滤
        let (list, total) = svc
            .search(&HistoryQuery { engine: "iciba".into(), page: 1, size: 10, ..Default::default() })
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(list[0].engine, "iciba");

        // 关键词 + 引擎组合
        let (_, total) = svc
            .search(&HistoryQuery {
                keyword: "love".into(),
                engine: "llm".into(),
                page: 1,
                size: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(total, 1, "关键词 love + 引擎 llm 只命中 love story");

        // 语言对过滤
        let (_, total) = svc
            .search(&HistoryQuery { from: "zh".into(), to: "en".into(), page: 1, size: 10, ..Default::default() })
            .unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn like_wildcards_escaped() {
        let svc = service();
        svc.record(&result("llm", "100% 完成", "zh", "en")).unwrap();
        svc.record(&result("llm", "普通条目", "zh", "en")).unwrap();
        // "%" 作为字面量搜索：只命中含 % 的条目，不当通配符
        let (_, total) = svc
            .search(&HistoryQuery { keyword: "100%".into(), page: 1, size: 10, ..Default::default() })
            .unwrap();
        assert_eq!(total, 1);
        // "_" 同理：命中含字面下划线的
        svc.record(&result("llm", "snake_case", "en", "zh")).unwrap();
        let (_, total) = svc
            .search(&HistoryQuery { keyword: "snake_case".into(), page: 1, size: 10, ..Default::default() })
            .unwrap();
        assert_eq!(total, 1, "下划线按字面量匹配");
    }

    #[test]
    fn pagination_bounds() {
        let svc = service();
        for i in 0..5 {
            svc.record(&result("llm", &format!("item{i}"), "en", "zh")).unwrap();
        }
        let (page1, total) = svc
            .search(&HistoryQuery { page: 1, size: 2, ..Default::default() })
            .unwrap();
        assert_eq!(total, 5);
        assert_eq!(page1.len(), 2);
        let (page3, _) = svc
            .search(&HistoryQuery { page: 3, size: 2, ..Default::default() })
            .unwrap();
        assert_eq!(page3.len(), 1, "最后一页只剩 1 条");
        // size 越界被钳制
        let (_, _) = svc
            .search(&HistoryQuery { page: 0, size: 0, ..Default::default() })
            .unwrap();
    }
}
