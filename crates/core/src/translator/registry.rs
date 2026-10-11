//! 引擎注册表：全部内置引擎的元信息与按配置装配。
//!
//! Key 配置存 sys_config：`engine.<name>` → JSON（各引擎字段见 descriptor.key_fields）。
//! iciba 沿用独立键 `iciba_api_key`（历史约定）。

use serde_json::Value;

use super::commercial::{BaiduTranslator, DeeplTranslator, TencentTranslator, YoudaoTranslator};
use super::iciba::IcibaTranslator;
use super::web_free::{GoogleTranslator, MyMemoryTranslator};
use super::Translator;
use crate::lang::{detect_zh_en, normalize_lang};

/// 引擎分组（UI 呈现顺序）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineGroup {
    /// 免 Key 即用
    Free,
    /// 需配置 Key
    Keyed,
    /// 词典形态
    Dictionary,
    /// LLM（供应商见设置·AI 供应商）
    Llm,
}

impl EngineGroup {
    pub fn label(&self) -> &'static str {
        match self {
            EngineGroup::Free => "免 Key 即用",
            EngineGroup::Keyed => "需配置",
            EngineGroup::Dictionary => "词典",
            EngineGroup::Llm => "AI",
        }
    }
}

/// 引擎描述：注册项（静态元信息 + 装配器）
pub struct EngineDescriptor {
    pub name: &'static str,
    pub label: &'static str,
    /// 能力一句话（语言范围 / 特点）
    pub capability: &'static str,
    pub group: EngineGroup,
    /// Key 配置字段：(json 字段, 中文标签)
    pub key_fields: &'static [(&'static str, &'static str)],
    /// 按 sys_config 里的 JSON（可为 None）装配引擎；免 Key 引擎忽略配置
    pub build: fn(Option<&Value>) -> Option<Box<dyn Translator>>,
}

/// sys_config 中引擎配置键
pub fn config_key(engine: &str) -> String {
    format!("engine.{engine}")
}

pub fn descriptors() -> &'static [EngineDescriptor] {
    &[
        EngineDescriptor {
            name: "google",
            label: "Google 翻译",
            capability: "任意语言对 · 自动检测 · 社区免 Key 接口",
            group: EngineGroup::Free,
            key_fields: &[],
            build: |_| Some(Box::new(GoogleTranslator::new())),
        },
        EngineDescriptor {
            name: "mymemory",
            label: "MyMemory",
            capability: "常见语言对 · 翻译记忆库聚合 · 免 Key",
            group: EngineGroup::Free,
            key_fields: &[],
            build: |_| Some(Box::new(MyMemoryTranslator::new())),
        },
        EngineDescriptor {
            name: "deepl",
            label: "DeepL",
            capability: "任意语言对 · 高质量 · 免费档需申请 auth_key",
            group: EngineGroup::Keyed,
            key_fields: &[("auth_key", "Auth Key（免费档以 :fx 结尾）")],
            build: |cfg| {
                cfg.and_then(|v| v.get("auth_key")?.as_str().map(|k| {
                    Box::new(DeeplTranslator::new(k.to_string())) as Box<dyn Translator>
                }))
            },
        },
        EngineDescriptor {
            name: "baidu",
            label: "百度翻译",
            capability: "任意语言对 · 标准版每月 100 万字符免费",
            group: EngineGroup::Keyed,
            key_fields: &[("appid", "APP ID"), ("secret", "密钥")],
            build: |cfg| {
                cfg.and_then(|v| {
                    let appid = v.get("appid")?.as_str()?;
                    let secret = v.get("secret")?.as_str()?;
                    Some(Box::new(BaiduTranslator::new(appid.into(), secret.into())) as Box<dyn Translator>)
                })
            },
        },
        EngineDescriptor {
            name: "youdao",
            label: "有道智云",
            capability: "任意语言对 · 免费额度 · 中英质量佳",
            group: EngineGroup::Keyed,
            key_fields: &[("app_key", "应用 Key"), ("app_secret", "应用密钥")],
            build: |cfg| {
                cfg.and_then(|v| {
                    let key = v.get("app_key")?.as_str()?;
                    let secret = v.get("app_secret")?.as_str()?;
                    Some(Box::new(YoudaoTranslator::new(key.into(), secret.into())) as Box<dyn Translator>)
                })
            },
        },
        EngineDescriptor {
            name: "tencent",
            label: "腾讯云翻译",
            capability: "任意语言对 · 每月 500 万字符免费",
            group: EngineGroup::Keyed,
            key_fields: &[("secret_id", "SecretId"), ("secret_key", "SecretKey")],
            build: |cfg| {
                cfg.and_then(|v| {
                    let id = v.get("secret_id")?.as_str()?;
                    let key = v.get("secret_key")?.as_str()?;
                    Some(Box::new(TencentTranslator::new(id.into(), key.into())) as Box<dyn Translator>)
                })
            },
        },
        EngineDescriptor {
            name: "iciba",
            label: "iciba 词典",
            capability: "音标 / 词性 / 例句 · 仅中英 · 结果本地缓存",
            group: EngineGroup::Dictionary,
            // iciba 的 key 走独立配置键（见 service::config），此处字段仅作 UI 提示
            key_fields: &[("key", "词典 Key（open.iciba.com 免费申请）")],
            build: |cfg| {
                let key = cfg
                    .and_then(|v| v.get("key")?.as_str())
                    .filter(|k| !k.trim().is_empty())
                    .map(String::from);
                Some(Box::new(IcibaTranslator::new(key)) as Box<dyn Translator>)
            },
        },
        EngineDescriptor {
            name: "llm",
            label: "LLM",
            capability: "任意语言对 · 句子长文 · 走 AI 供应商（见设置）",
            group: EngineGroup::Llm,
            key_fields: &[],
            build: |_| None, // LLM 由调度层按启用供应商装配（非注册表构建）
        },
    ]
}

/// 全部引擎名（提示文案用）
pub fn descriptor_names() -> Vec<&'static str> {
    descriptors().iter().map(|d| d.name).collect()
}

pub fn descriptor(name: &str) -> Option<&'static EngineDescriptor> {
    descriptors().iter().find(|d| d.name == name)
}

/// 是否已配置（免 Key / 已有配置 JSON 的返回 true；llm 特判）
pub fn is_configured(name: &str, cfg: Option<&Value>) -> bool {
    let d = match descriptor(name) {
        Some(d) => d,
        None => return false,
    };
    if name == "llm" {
        return true;
    }
    if d.key_fields.is_empty() {
        return true;
    }
    cfg.map(|v| {
        d.key_fields
            .iter()
            .all(|(field, _)| v.get(*field).and_then(|f| f.as_str()).map(|s| !s.trim().is_empty()).unwrap_or(false))
    })
    .unwrap_or(false)
}

/// 引擎默认语言对解析（与调度器同一套归一化逻辑，供测试命令复用）
pub fn resolve_pair(text: &str, from: Option<&str>, to: Option<&str>) -> (String, String) {
    match (from, to) {
        (Some(f), Some(t)) if !f.is_empty() && !t.is_empty() => {
            (normalize_lang(f), normalize_lang(t))
        }
        _ => {
            let (f, t) = detect_zh_en(text);
            (f.to_string(), t.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_shape() {
        let ds = descriptors();
        assert_eq!(ds.len(), 8, "8 个内置引擎");
        assert!(ds.iter().any(|d| d.name == "google" && d.group == EngineGroup::Free));
        assert!(ds.iter().any(|d| d.name == "iciba" && d.group == EngineGroup::Dictionary));
    }

    #[test]
    fn build_free_engines_without_config() {
        assert!((descriptor("google").unwrap().build)(None).is_some());
        assert!((descriptor("mymemory").unwrap().build)(None).is_some());
    }

    #[test]
    fn build_keyed_requires_fields() {
        let d = descriptor("baidu").unwrap();
        assert!((d.build)(None).is_none(), "无配置不可装配");
        let partial = serde_json::json!({ "appid": "x" });
        assert!((d.build)(Some(&partial)).is_none(), "缺 secret 不可装配");
        let full = serde_json::json!({ "appid": "x", "secret": "y" });
        assert!((d.build)(Some(&full)).is_some());
    }

    #[test]
    fn configured_check() {
        assert!(is_configured("google", None), "免 Key 恒可用");
        assert!(!is_configured("deepl", None));
        assert!(is_configured("deepl", Some(&serde_json::json!({ "auth_key": "k:fx" }))));
        assert!(!is_configured("deepl", Some(&serde_json::json!({ "auth_key": " " }))));
    }
}
