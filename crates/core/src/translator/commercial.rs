//! 需 Key 的商业翻译引擎：DeepL / 百度 / 有道 / 腾讯云。
//!
//! 各家签名与码表差异收敛在此层；Key 配置由引擎注册表从 sys_config 装配。

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{EngineKind, TranslateRequest, TranslationResult, Translator};

// ============ DeepL（官方 API 免费档，auth_key） ============

pub struct DeeplTranslator {
    auth_key: String,
    client: reqwest::Client,
}

impl DeeplTranslator {
    pub fn new(auth_key: String) -> Self {
        Self {
            auth_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }
}

#[async_trait]
impl Translator for DeeplTranslator {
    fn name(&self) -> &'static str {
        "deepl"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }
    fn supports(&self, _from: &str, _to: &str) -> bool {
        true
    }
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let from = req.from.as_deref().unwrap_or("auto");
        let to = req.to.as_deref().ok_or_else(|| anyhow!("deepl 需要目标语言"))?;
        // DeepL 免费档 host 以 key 后缀 :fx 区分；官方建议统一 api-free 入口
        let host = if self.auth_key.trim_end().ends_with(":fx") {
            "https://api-free.deepl.com"
        } else {
            "https://api.deepl.com"
        };
        let mut body = json!({ "text": [req.text], "target_lang": to.to_uppercase() });
        if from != "auto" {
            body["source_lang"] = json!(from.to_uppercase());
        }
        let resp: Value = self
            .client
            .post(format!("{host}/v2/translate"))
            .header("Authorization", format!("DeepL-Auth-Key {}", self.auth_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("deepl 请求失败: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("deepl 响应解析失败: {e}"))?;
        let text = resp
            .get("translations")
            .and_then(|t| t.get(0))
            .and_then(|t| t.get("text"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("deepl 未返回译文"))?;
        let detected = resp
            .pointer("/translations/0/detected_source_language")
            .and_then(|v| v.as_str())
            .unwrap_or(from)
            .to_lowercase();
        Ok(TranslationResult::plain(self.name(), &detected, to, &req.text, text))
    }
}

// ============ 百度翻译开放平台（appid + 密钥，MD5 签名） ============

pub struct BaiduTranslator {
    appid: String,
    secret: String,
    client: reqwest::Client,
}

/// 百度语言码表（与 ISO 639 的差异映射）
fn baidu_lang(code: &str) -> &'static str {
    match code {
        "zh" => "zh",
        "en" => "en",
        "ja" => "jp",
        "ko" => "kor",
        "fr" => "fra",
        "de" => "de",
        "es" => "spa",
        "ru" => "ru",
        "pt" => "pt",
        "it" => "it",
        "th" => "th",
        "vi" => "vie",
        "auto" => "auto",
        _ => "en",
    }
}

impl BaiduTranslator {
    pub fn new(appid: String, secret: String) -> Self {
        Self {
            appid,
            secret,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }
}

fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[async_trait]
impl Translator for BaiduTranslator {
    fn name(&self) -> &'static str {
        "baidu"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }
    fn supports(&self, _from: &str, _to: &str) -> bool {
        true
    }
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let from = baidu_lang(req.from.as_deref().unwrap_or("auto"));
        let to = baidu_lang(req.to.as_deref().ok_or_else(|| anyhow!("baidu 需要目标语言"))?);
        let salt = chrono::Utc::now().timestamp_millis().to_string();
        let sign = md5_hex(&format!("{}{}{}{}", self.appid, req.text, salt, self.secret));
        let resp: Value = self
            .client
            .post("https://fanyi-api.baidu.com/api/trans/vip/translate")
            .form(&[
                ("q", req.text.as_str()),
                ("from", from),
                ("to", to),
                ("appid", self.appid.as_str()),
                ("salt", salt.as_str()),
                ("sign", sign.as_str()),
            ])
            .send()
            .await
            .map_err(|e| anyhow!("baidu 请求失败: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("baidu 响应解析失败: {e}"))?;
        if let Some(err) = resp.get("error_code").and_then(|v| v.as_str()) {
            return Err(anyhow!("baidu 错误 {err}: {}", resp.get("error_msg").and_then(|v| v.as_str()).unwrap_or("")));
        }
        let mut out = String::new();
        if let Some(rows) = resp.get("trans_result").and_then(|v| v.as_array()) {
            for row in rows {
                if let Some(dst) = row.get("dst").and_then(|v| v.as_str()) {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(dst);
                }
            }
        }
        if out.is_empty() {
            return Err(anyhow!("baidu 未返回译文"));
        }
        let detected = resp.get("from").and_then(|v| v.as_str()).unwrap_or("auto").to_string();
        Ok(TranslationResult::plain(self.name(), &detected, to, &req.text, &out))
    }
}

// ============ 有道智云（appKey + appSecret，SHA256 签名） ============

pub struct YoudaoTranslator {
    app_key: String,
    app_secret: String,
    client: reqwest::Client,
}

/// 有道语言码表（中文为 zh-CHS）
fn youdao_lang(code: &str) -> String {
    match code {
        "zh" => "zh-CHS".into(),
        other => other.to_string(),
    }
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

impl YoudaoTranslator {
    pub fn new(app_key: String, app_secret: String) -> Self {
        Self {
            app_key,
            app_secret,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }

    /// 有道 input 规则：q ≤ 20 字符取原文；超长取前 10 + 长度 + 后 10
    fn input_rule(q: &str) -> String {
        let chars: Vec<char> = q.chars().collect();
        if chars.len() <= 20 {
            q.to_string()
        } else {
            let head: String = chars[..10].iter().collect();
            let tail: String = chars[chars.len() - 10..].iter().collect();
            format!("{head}{}{tail}", chars.len())
        }
    }
}

#[async_trait]
impl Translator for YoudaoTranslator {
    fn name(&self) -> &'static str {
        "youdao"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }
    fn supports(&self, _from: &str, _to: &str) -> bool {
        true
    }
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let from = youdao_lang(req.from.as_deref().unwrap_or("auto"));
        let to = youdao_lang(req.to.as_deref().ok_or_else(|| anyhow!("youdao 需要目标语言"))?);
        let salt = chrono::Utc::now().timestamp_millis().to_string();
        let curtime = chrono::Utc::now().timestamp().to_string();
        let input = Self::input_rule(&req.text);
        let sign = sha256_hex(&format!("{}{}{}{}{}", self.app_key, input, salt, curtime, self.app_secret));
        let resp: Value = self
            .client
            .post("https://openapi.youdao.com/api")
            .form(&[
                ("q", req.text.as_str()),
                ("from", from.as_str()),
                ("to", to.as_str()),
                ("appKey", self.app_key.as_str()),
                ("salt", salt.as_str()),
                ("sign", sign.as_str()),
                ("signType", "v3"),
                ("curtime", curtime.as_str()),
            ])
            .send()
            .await
            .map_err(|e| anyhow!("youdao 请求失败: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("youdao 响应解析失败: {e}"))?;
        if let Some(code) = resp.get("errorCode").and_then(|v| v.as_str()) {
            if code != "0" {
                return Err(anyhow!("youdao 错误码 {code}"));
            }
        }
        let text = resp
            .get("translation")
            .and_then(|t| t.get(0))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("youdao 未返回译文"))?;
        Ok(TranslationResult::plain(self.name(), &from, &to, &req.text, text))
    }
}

// ============ 腾讯云机器翻译（TC3-HMAC-SHA256 签名） ============

pub struct TencentTranslator {
    secret_id: String,
    secret_key: String,
    client: reqwest::Client,
}

impl TencentTranslator {
    pub fn new(secret_id: String, secret_key: String) -> Self {
        Self {
            secret_id,
            secret_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("构建 HTTP 客户端失败"),
        }
    }
}

fn hmac_sha256(key: &[u8], msg: &str) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC key 长度任意");
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// 腾讯语言码与 ISO 639 一致（zh/en/ja/ko/fr/de/es/ru/pt/it…），auto 检测
fn tencent_lang(code: &str) -> &str {
    if code.is_empty() { "auto" } else { code }
}

#[async_trait]
impl Translator for TencentTranslator {
    fn name(&self) -> &'static str {
        "tencent"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Text
    }
    fn supports(&self, _from: &str, _to: &str) -> bool {
        true
    }
    async fn translate(&self, req: &TranslateRequest) -> Result<TranslationResult> {
        let from = tencent_lang(req.from.as_deref().unwrap_or("auto"));
        let to = tencent_lang(req.to.as_deref().ok_or_else(|| anyhow!("tencent 需要目标语言"))?);
        let host = "tmt.tencentcloudapi.com";
        let service = "tmt";
        let action = "TextTranslate";
        let version = "2018-03-21";
        let region = "ap-guangzhou";

        let payload = json!({
            "SourceText": req.text,
            "Source": from,
            "Target": to,
            "ProjectId": 0,
        })
        .to_string();

        // TC3 签名
        let now = chrono::Utc::now();
        let date = now.format("%Y-%m-%d").to_string();
        let timestamp = now.timestamp().to_string();
        let hashed_payload = sha256_hex(&payload);
        let canonical = format!(
            "POST\n/\n\ncontent-type:application/json; charset=utf-8\nhost:{host}\n\ncontent-type;host\n{hashed_payload}"
        );
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{timestamp}\n{date}/{service}/tc3_request\n{}",
            sha256_hex(&canonical)
        );
        let secret_date = hmac_sha256(format!("TC3{}", self.secret_key).as_bytes(), &date);
        let secret_service = hmac_sha256(&secret_date, service);
        let secret_signing = hmac_sha256(&secret_service, "tc3_request");
        let signature = hex::encode(hmac_sha256(&secret_signing, &string_to_sign));
        let authorization = format!(
            "TC3-HMAC-SHA256 Credential={}/{date}/{service}/tc3_request, SignedHeaders=content-type;host, Signature={signature}",
            self.secret_id
        );

        let resp: Value = self
            .client
            .post(format!("https://{host}/"))
            .header("Authorization", authorization)
            .header("X-TC-Action", action)
            .header("X-TC-Version", version)
            .header("X-TC-Region", region)
            .header("X-TC-Timestamp", &timestamp)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(payload)
            .send()
            .await
            .map_err(|e| anyhow!("tencent 请求失败: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("tencent 响应解析失败: {e}"))?;

        if let Some(err) = resp.get("Response").and_then(|r| r.get("Error")) {
            return Err(anyhow!(
                "tencent 错误 {}: {}",
                err.get("Code").and_then(|v| v.as_str()).unwrap_or("?"),
                err.get("Message").and_then(|v| v.as_str()).unwrap_or("")
            ));
        }
        let text = resp
            .pointer("/Response/TargetText")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("tencent 未返回译文"))?;
        Ok(TranslationResult::plain(self.name(), from, to, &req.text, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baidu_lang_table() {
        assert_eq!(baidu_lang("ja"), "jp");
        assert_eq!(baidu_lang("ko"), "kor");
        assert_eq!(baidu_lang("fr"), "fra");
        assert_eq!(baidu_lang("es"), "spa");
        assert_eq!(baidu_lang("vi"), "vie");
        assert_eq!(baidu_lang("zh"), "zh");
        assert_eq!(baidu_lang("xx"), "en", "未知码回落 en");
    }

    #[test]
    fn youdao_lang_table() {
        assert_eq!(youdao_lang("zh"), "zh-CHS");
        assert_eq!(youdao_lang("ja"), "ja");
    }

    #[test]
    fn youdao_input_rule() {
        assert_eq!(YoudaoTranslator::input_rule("hello"), "hello");
        let long = "a".repeat(30);
        let rule = YoudaoTranslator::input_rule(&long);
        assert!(rule.starts_with("aaaaaaaaaa") && rule.contains("30"));
    }

    #[test]
    fn md5_known_vector() {
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn tc3_hmac_chain_shape() {
        // 签名链各步输出长度固定（32 字节 hex 64 字符）
        let k = hmac_sha256(b"key", "msg");
        assert_eq!(k.len(), 32);
    }
}
