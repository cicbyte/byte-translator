//! 词典缓存装饰器：命中本地缓存直接返回，穿透后回写。
//!
//! 只应包装结果稳定的词典引擎；LLM 结果不缓存（每次现调）。
//! 缓存键为 (归一化词条, 语言方向)；写库失败仅记日志不阻断翻译。

use anyhow::Result;
use async_trait::async_trait;

use super::{
    EngineKind, TranslateRequest, TranslationResult, Translator,
};
use crate::db::Database;

/// 包装词典引擎的缓存层
pub struct CachedTranslator {
    inner: Box<dyn Translator>,
    db: std::sync::Arc<Database>,
}

impl CachedTranslator {
    pub fn new(inner: Box<dyn Translator>, db: std::sync::Arc<Database>) -> Self {
        Self { inner, db }
    }

    fn cache_key(req: &TranslateRequest) -> (String, String) {
        let (from, to) = match (&req.from, &req.to) {
            (Some(f), Some(t)) => (crate::lang::normalize_lang(f), crate::lang::normalize_lang(t)),
            _ => {
                let (f, t) = crate::lang::detect_zh_en(&req.text);
                (f.to_string(), t.to_string())
            }
        };
        (req.text.trim().to_lowercase(), format!("{from}->{to}"))
    }

    fn load(&self, word: &str, lang: &str) -> Option<TranslationResult> {
        let conn = self.db.lock().ok()?;
        let json: String = conn
            .query_row(
                "SELECT result_json FROM dict_cache WHERE word=?1 AND lang=?2",
                rusqlite::params![word, lang],
                |r| r.get(0),
            )
            .ok()?;
        serde_json::from_str(&json).ok()
    }

    fn store(&self, word: &str, lang: &str, result: &TranslationResult) {
        let Ok(json) = serde_json::to_string(result) else { return };
        let Ok(conn) = self.db.lock() else { return };
        let _ = conn.execute(
            "INSERT INTO dict_cache (word, lang, result_json) VALUES (?1, ?2, ?3) \
             ON CONFLICT (word, lang) DO UPDATE SET result_json=excluded.result_json",
            rusqlite::params![word, lang, json],
        );
    }
}

#[async_trait]
impl Translator for CachedTranslator {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn kind(&self) -> EngineKind {
        self.inner.kind()
    }

    fn supports(&self, from: &str, to: &str) -> bool {
        self.inner.supports(from, to)
    }

    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let (word, lang) = Self::cache_key(req);
        if let Some(hit) = self.load(&word, &lang) {
            tracing::debug!(engine = self.inner.name(), word, "词典缓存命中");
            return Ok(hit);
        }
        let result = self.inner.translate(req).await?;
        self.store(&word, &lang, &result);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::{PartOfSpeech, TranslateRequest};
    use std::sync::Arc;

    /// 计数桩引擎：共享计数器记录真实调用次数，返回固定词典结果
    struct CountingEngine {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait]
    impl Translator for CountingEngine {
        fn name(&self) -> &'static str {
            "stub"
        }
        fn kind(&self) -> EngineKind {
            EngineKind::Dictionary
        }
        fn supports(&self, _from: &str, _to: &str) -> bool {
            true
        }
        async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(TranslationResult::dictionary(
                "stub",
                "en",
                "zh",
                &req.text,
                vec![],
                vec![PartOfSpeech { pos: "n.".into(), mean: "测试".into() }],
                vec![],
            ))
        }
    }

    fn db() -> Arc<Database> {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cache-test.db");
        // TempDir 生命周期只到函数尾：把目录 drop 延迟到进程退出（Windows 上
        // SQLite 连接持有文件句柄，测试期间目录仍有效）
        std::mem::forget(tmp);
        Arc::new(Database::open(&path).unwrap())
    }

    fn counting() -> (std::sync::Arc<std::sync::atomic::AtomicUsize>, CountingEngine) {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        (calls.clone(), CountingEngine { calls })
    }

    #[tokio::test]
    async fn second_call_hits_cache() {
        let (calls, engine) = counting();
        let cached = CachedTranslator::new(Box::new(engine), db());
        let req = TranslateRequest {
            text: "love".into(),
            from: Some("en".into()),
            to: Some("zh".into()),
        };
        let r1 = cached.translate(&req).await.unwrap();
        assert_eq!(r1.engine, "stub");
        let r2 = cached.translate(&req).await.unwrap();
        assert_eq!(r2.parts, r1.parts, "命中缓存返回相同结果");
        assert_eq!(count_rows(&cached), 1, "穿透后只写一条缓存");
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "第二次应命中缓存，不打桩"
        );
    }

    #[tokio::test]
    async fn different_lang_direction_misses() {
        let (calls, engine) = counting();
        let cached = CachedTranslator::new(Box::new(engine), db());
        let en_zh = TranslateRequest {
            text: "love".into(),
            from: Some("en".into()),
            to: Some("zh".into()),
        };
        let zh_en = TranslateRequest {
            text: "love".into(),
            from: Some("zh".into()),
            to: Some("en".into()),
        };
        cached.translate(&en_zh).await.unwrap();
        cached.translate(&zh_en).await.unwrap();
        assert_eq!(count_rows(&cached), 2, "两个方向各写一条");
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "不同语言方向应各穿透一次"
        );
    }

    fn count_rows(c: &CachedTranslator) -> i64 {
        let conn = c.db.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM dict_cache", [], |r| r.get(0)).unwrap()
    }
}
