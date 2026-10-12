//! Key 轮换包装器：同一引擎多凭据实例，失败自动切换。
//!
//! 策略：sticky——从上次成功的实例开始尝试，失败（凭据失效/限流）换下一个，
//! 一轮全失败才向上报错。成功后记住索引，后续优先复用同一凭据。

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use super::{EngineKind, TranslateRequest, TranslationResult, Translator};

pub struct KeyRotationTranslator {
    inner: Vec<Box<dyn Translator>>,
    sticky: std::sync::Mutex<usize>,
}

impl KeyRotationTranslator {
    pub fn new(inner: Vec<Box<dyn Translator>>) -> Self {
        let sticky = std::sync::Mutex::new(0);
        Self { inner, sticky }
    }
}

#[async_trait]
impl Translator for KeyRotationTranslator {
    fn name(&self) -> &'static str {
        self.inner.first().map(|t| t.name()).unwrap_or("rotation")
    }

    fn kind(&self) -> EngineKind {
        self.inner.first().map(|t| t.kind()).unwrap_or(EngineKind::Text)
    }

    fn supports(&self, from: &str, to: &str) -> bool {
        self.inner.first().map(|t| t.supports(from, to)).unwrap_or(false)
    }

    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let n = self.inner.len();
        if n == 0 {
            return Err(anyhow!("无可用凭据"));
        }
        let start = *self.sticky.lock().unwrap_or_else(|p| p.into_inner());
        let mut last_err: Option<anyhow::Error> = None;
        for i in 0..n {
            let idx = (start + i) % n;
            match self.inner[idx].translate(req).await {
                Ok(result) => {
                    if let Ok(mut s) = self.sticky.lock() {
                        *s = idx;
                    }
                    return Ok(result);
                }
                Err(e) => {
                    tracing::debug!(engine = self.inner[idx].name(), idx, error = %e, "凭据失败，切换下一个");
                    last_err = Some(e);
                }
            }
        }
        Err(anyhow!(
            "全部 {} 个凭据均失败，最后错误: {}",
            n,
            last_err.map(|e| format!("{e:#}")).unwrap_or_default()
        ))
    }
}

/// 多值切分：逗号 / 中文逗号 / 分号 / 换行分隔，逐段 trim、去空段
pub fn split_multi(raw: &str) -> Vec<String> {
    raw.split([',', '，', ';', '；', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::PartOfSpeech;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct StubEngine {
        name: &'static str,
        fail_times: usize,
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
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fail_times {
                return Err(anyhow!("第 {} 次失败", n + 1));
            }
            Ok(TranslationResult::dictionary(
                self.name, "en", "zh", &req.text, vec![], vec![PartOfSpeech { pos: "n.".into(), mean: "ok".into() }], vec![],
            ))
        }
    }

    fn req() -> TranslateRequest {
        TranslateRequest { text: "love".into(), from: Some("en".into()), to: Some("zh".into()) }
    }

    #[tokio::test]
    async fn falls_through_to_second_key() {
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));
        let rot = KeyRotationTranslator::new(vec![
            Box::new(StubEngine { name: "a", fail_times: 10, calls: c1.clone() }),
            Box::new(StubEngine { name: "b", fail_times: 0, calls: c2.clone() }),
        ]);
        let out = rot.translate(&req()).await.unwrap();
        assert_eq!(out.engine, "b");
        assert_eq!(c1.load(Ordering::SeqCst), 1, "第一个凭据试过一次");
        assert_eq!(c2.load(Ordering::SeqCst), 1);

        // sticky：第二次直接从 b 开始，a 不再被调用
        rot.translate(&req()).await.unwrap();
        assert_eq!(c1.load(Ordering::SeqCst), 1, "sticky 后不再打 a");
        assert_eq!(c2.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn all_failures_report_last_error() {
        let rot = KeyRotationTranslator::new(vec![
            Box::new(StubEngine { name: "a", fail_times: 10, calls: Arc::new(AtomicUsize::new(0)) }),
            Box::new(StubEngine { name: "b", fail_times: 10, calls: Arc::new(AtomicUsize::new(0)) }),
        ]);
        let err = rot.translate(&req()).await.unwrap_err();
        assert!(err.to_string().contains("全部 2 个凭据均失败"));
    }

    #[test]
    fn multi_value_split() {
        assert_eq!(split_multi("k1, k2，k3；k4\nk5"), vec!["k1", "k2", "k3", "k4", "k5"]);
        assert_eq!(split_multi("  "), Vec::<String>::new());
        assert_eq!(split_multi("only"), vec!["only"]);
    }
}
