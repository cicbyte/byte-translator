//! 翻译调度：输入特征与语言对路由、词典优先 + 失败降级 LLM、显式引擎指定。
//!
//! 路由规则（默认 Auto）：
//! 1. 语言对非中英 → LLM（词典引擎只支持中英）；
//! 2. 中英 + 单词特征（无空格且 ≤ 20 字符）→ 词典优先，未命中自动降级 LLM；
//! 3. 中英 + 句子特征 → LLM。
//!
//! 显式指定引擎（Named）不做自动降级——失败直接报错，由用户决定换引擎。
//! 两类引擎都是「每次调用现构造」的工厂：供应商切换即时生效，测试可注入计数桩。

use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::db::Database;
use crate::lang::{detect_zh_en, normalize_lang};
use crate::service::provider::ProviderService;
use crate::translator::llm::LlmTranslator;
use crate::translator::registry;
use crate::translator::{cache::CachedTranslator, iciba::IcibaTranslator, TranslateRequest, TranslationResult, Translator};

/// 引擎选择：自动调度 / 显式指定
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineChoice {
    Auto,
    /// 引擎名（iciba / llm）
    Named(String),
}

/// 调度产出：统一结果 + 降级标注（渲染层据此提示）
#[derive(Debug, Clone)]
pub struct TranslateOutcome {
    pub result: TranslationResult,
    /// 自动降级时记录「原引擎未命中」，如 Some("iciba")
    pub fallback_from: Option<String>,
}

/// 输入特征判定：无空格且 ≤ 20 字符视为单词（词典可查），否则是句子
pub fn is_word_like(text: &str) -> bool {
    !text.contains(' ') && text.chars().count() <= 20
}

type EngineFactory = Arc<dyn Fn() -> Box<dyn Translator> + Send + Sync>;
type LlmFactory = Arc<dyn Fn() -> Result<Box<dyn Translator>> + Send + Sync>;

/// 调度器：词典工厂 + LLM 工厂 + 引擎配置库（注册表引擎装配用）
pub struct TranslateService {
    dictionary_factory: Option<EngineFactory>,
    llm_factory: LlmFactory,
    config_db: Option<Arc<Database>>,
}

impl TranslateService {
    /// 生产装配：词典 = 本地缓存包装的 iciba（Key 取 ICIBA_API_KEY 环境变量，
    /// 未设置时用内置公共 Key——该 Key 可能失效，命中不了会自动降级 LLM）；
    /// LLM = 启用中的供应商（未启用时翻译报错并给配置指引）
    pub fn production(db: Arc<Database>) -> Self {
        let providers = ProviderService::new(db.clone());
        let config_db = db.clone();
        let dict_db = db.clone();
        Self {
            // Key 在工厂内现读环境变量：运行期可更新（桌面设置页改 Key 即时生效）
            dictionary_factory: Some(Arc::new(move || {
                let key = std::env::var("ICIBA_API_KEY").ok().filter(|k| !k.trim().is_empty());
                Box::new(CachedTranslator::new(
                    Box::new(IcibaTranslator::new(key)),
                    dict_db.clone(),
                ))
            })),
            llm_factory: Arc::new(move || {
                let cfg = providers.enabled_config()?;
                Ok(Box::new(LlmTranslator::new(cfg)))
            }),
            config_db: Some(config_db),
        }
    }

    /// 测试注入构造器
    pub fn with_engines(dictionary_factory: Option<EngineFactory>, llm_factory: LlmFactory) -> Self {
        Self { dictionary_factory, llm_factory, config_db: None }
    }

    /// 执行翻译
    pub async fn translate(
        &self,
        req: &TranslateRequest,
        choice: &EngineChoice,
    ) -> Result<TranslateOutcome> {
        // 归一化语言对：显式给齐用显式，否则自动检测（默认中英互译）
        let (from, to) = match (&req.from, &req.to) {
            (Some(f), Some(t)) if !f.is_empty() && !t.is_empty() => {
                (normalize_lang(f), normalize_lang(t))
            }
            _ => {
                let (f, t) = detect_zh_en(&req.text);
                (f.to_string(), t.to_string())
            }
        };
        let req = TranslateRequest {
            from: Some(from.clone()),
            to: Some(to.clone()),
            text: req.text.clone(),
        };

        match choice {
            EngineChoice::Auto => self.translate_auto(&req, &from, &to).await,
            EngineChoice::Named(name) => {
                let result = match name.as_str() {
                    "iciba" => self.translate_iciba(&req, &from, &to).await?,
                    "llm" => self.translate_llm(&req).await?,
                    other => self.translate_registry(other, &req).await?,
                };
                Ok(TranslateOutcome { result, fallback_from: None })
            }
        }
    }

    async fn translate_auto(
        &self,
        req: &TranslateRequest,
        from: &str,
        to: &str,
    ) -> Result<TranslateOutcome> {
        let zh_en = (from == "zh" && to == "en") || (from == "en" && to == "zh");
        let word_like = is_word_like(&req.text);

        if zh_en && word_like {
            // 词典优先；未命中降级 LLM（无供应商时合并双方错误给指引）
            match self.translate_iciba(req, from, to).await {
                Ok(result) => Ok(TranslateOutcome { result, fallback_from: None }),
                Err(dict_err) => {
                    tracing::debug!(error = %dict_err, "词典未命中，自动降级 LLM");
                    match self.translate_llm(req).await {
                        Ok(result) => Ok(TranslateOutcome {
                            result,
                            fallback_from: Some("iciba".to_string()),
                        }),
                        Err(llm_err) => Err(anyhow!(
                            "词典未命中（{dict_err}），LLM 翻译失败（{llm_err}）"
                        )),
                    }
                }
            }
        } else {
            // 句子或非中英语言对 → LLM
            let result = self.translate_llm(req).await?;
            Ok(TranslateOutcome { result, fallback_from: None })
        }
    }

    /// 注册表引擎（google / mymemory / deepl / baidu / youdao / tencent）
    async fn translate_registry(
        &self,
        name: &str,
        req: &TranslateRequest,
    ) -> Result<TranslationResult> {
        let desc = registry::descriptor(name)
            .ok_or_else(|| anyhow!("不支持的引擎: {name}（可用: iciba / llm / {}", registry::descriptor_names().join(" / ")))?;
        let cfg = self
            .config_db
            .as_ref()
            .and_then(|db| {
                let svc = crate::service::config::ConfigService::new(db.clone());
                svc.get(&registry::config_key(name)).ok().flatten()
            })
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let engine = (desc.build)(cfg.as_ref())
            .ok_or_else(|| anyhow!("引擎 {name} 未配置（设置 → 翻译引擎）"))?;
        engine.translate(req).await
    }

    async fn translate_iciba(
        &self,
        req: &TranslateRequest,
        from: &str,
        to: &str,
    ) -> Result<TranslationResult> {
        let zh_en = (from == "zh" && to == "en") || (from == "en" && to == "zh");
        if !zh_en {
            return Err(anyhow!("iciba 仅支持中英互译（收到 {from}->{to}），请使用 llm"));
        }
        let factory = self
            .dictionary_factory
            .as_ref()
            .ok_or_else(|| anyhow!("词典引擎未装配"))?;
        factory().translate(req).await
    }

    async fn translate_llm(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let engine = (self.llm_factory)()?;
        engine.translate(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::EngineKind;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 可控桩：fail 控制是否报错；共享计数器记录真实调用次数
    struct StubEngine {
        name: &'static str,
        fail: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Translator for StubEngine {
        fn name(&self) -> &'static str {
            self.name
        }
        fn kind(&self) -> EngineKind {
            EngineKind::Text
        }
        fn supports(&self, _from: &str, _to: &str) -> bool {
            true
        }
        async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(anyhow!("桩失败:{}", self.name));
            }
            Ok(TranslationResult::plain(self.name, "en", "zh", &req.text, "桩译文"))
        }
    }

    fn stub_factory(
        name: &'static str,
        fail: bool,
        calls: Arc<AtomicUsize>,
    ) -> Arc<dyn Fn() -> Box<dyn Translator> + Send + Sync> {
        Arc::new(move || Box::new(StubEngine { name, fail, calls: calls.clone() }))
    }

    fn llm_ok(calls: Arc<AtomicUsize>) -> LlmFactory {
        Arc::new(move || Ok(stub_factory("llm", false, calls.clone())()))
    }

    fn req(text: &str) -> TranslateRequest {
        TranslateRequest { text: text.into(), from: None, to: None }
    }

    fn req_lang(text: &str, from: &str, to: &str) -> TranslateRequest {
        TranslateRequest { text: text.into(), from: Some(from.into()), to: Some(to.into()) }
    }

    // BDD: 单词自动路由词典
    #[tokio::test]
    async fn bdd_word_routes_to_dictionary() {
        let dict = Arc::new(AtomicUsize::new(0));
        let llm = Arc::new(AtomicUsize::new(0));
        let svc = TranslateService::with_engines(
            Some(stub_factory("iciba", false, dict.clone())),
            llm_ok(llm.clone()),
        );
        let out = svc.translate(&req("love"), &EngineChoice::Auto).await.unwrap();
        assert_eq!(out.result.engine, "iciba");
        assert!(out.fallback_from.is_none());
        assert_eq!(dict.load(Ordering::SeqCst), 1);
        assert_eq!(llm.load(Ordering::SeqCst), 0, "单词命中词典不应打 LLM");
    }

    // BDD: 词典未命中自动降级 LLM
    #[tokio::test]
    async fn bdd_dictionary_miss_falls_back_to_llm() {
        let dict = Arc::new(AtomicUsize::new(0));
        let llm = Arc::new(AtomicUsize::new(0));
        let svc = TranslateService::with_engines(
            Some(stub_factory("iciba", true, dict.clone())),
            llm_ok(llm.clone()),
        );
        let out = svc.translate(&req("今天星期几"), &EngineChoice::Auto).await.unwrap();
        assert_eq!(out.result.engine, "llm");
        assert_eq!(out.fallback_from.as_deref(), Some("iciba"), "应标注降级来源");
    }

    // BDD: 非中英语言对路由 LLM（词典不参与）
    #[tokio::test]
    async fn bdd_non_zh_en_routes_to_llm() {
        let dict = Arc::new(AtomicUsize::new(0));
        let llm = Arc::new(AtomicUsize::new(0));
        let svc = TranslateService::with_engines(
            Some(stub_factory("iciba", false, dict.clone())),
            llm_ok(llm.clone()),
        );
        let out = svc
            .translate(&req_lang("こんにちは", "ja", "en"), &EngineChoice::Auto)
            .await
            .unwrap();
        assert_eq!(out.result.engine, "llm");
        assert_eq!(dict.load(Ordering::SeqCst), 0, "非中英不应打词典");
    }

    // BDD: 显式指定 iciba 且不支持时不降级，报错提示
    #[tokio::test]
    async fn bdd_explicit_iciba_no_fallback() {
        let llm = Arc::new(AtomicUsize::new(0));
        let svc = TranslateService::with_engines(None, llm_ok(llm.clone()));
        let err = svc
            .translate(
                &req_lang("こんにちは世界", "ja", "en"),
                &EngineChoice::Named("iciba".into()),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("仅支持中英互译"));
        assert_eq!(llm.load(Ordering::SeqCst), 0, "显式指定不得自动降级");
    }

    // 显式未知引擎报错
    #[tokio::test]
    async fn explicit_unknown_engine_errors() {
        let llm = Arc::new(AtomicUsize::new(0));
        let svc = TranslateService::with_engines(None, llm_ok(llm.clone()));
        let err = svc
            .translate(&req("love"), &EngineChoice::Named("nosuch".into()))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("不支持的引擎"));
    }

    // 句子（含空格）自动路由 LLM
    #[tokio::test]
    async fn sentence_routes_to_llm() {
        let dict = Arc::new(AtomicUsize::new(0));
        let llm = Arc::new(AtomicUsize::new(0));
        let svc = TranslateService::with_engines(
            Some(stub_factory("iciba", false, dict.clone())),
            llm_ok(llm.clone()),
        );
        let out = svc.translate(&req("I want to go home"), &EngineChoice::Auto).await.unwrap();
        assert_eq!(out.result.engine, "llm");
        assert_eq!(dict.load(Ordering::SeqCst), 0);
    }

    // 词典与 LLM 双失败：合并错误
    #[tokio::test]
    async fn both_failures_merged() {
        let dict = Arc::new(AtomicUsize::new(0));
        let llm_fail: LlmFactory = Arc::new(|| Err(anyhow!("未配置供应商")));
        let svc = TranslateService::with_engines(
            Some(stub_factory("iciba", true, dict.clone())),
            llm_fail,
        );
        let err = svc.translate(&req("love"), &EngineChoice::Auto).await.unwrap_err();
        assert!(err.to_string().contains("词典未命中"));
        assert!(err.to_string().contains("未配置供应商"));
    }

    #[test]
    fn word_like_heuristic() {
        assert!(is_word_like("love"));
        assert!(is_word_like("和谐"));
        assert!(!is_word_like("hello world"));
        assert!(!is_word_like("这是一个特别长的中文句子肯定超过二十个字符了"));
    }
}
