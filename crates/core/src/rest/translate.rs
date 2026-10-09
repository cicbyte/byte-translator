//! POST /api/v1/translate：非流式 JSON 响应与 SSE 流式两种形态。

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use std::convert::Infallible;

use super::ApiState;
use crate::lang::normalize_lang;
use crate::service::translate::{EngineChoice, TranslateOutcome};
use crate::translator::llm::translate_stream;
use crate::translator::TranslateRequest;

#[derive(Debug, Deserialize)]
pub struct TranslateBody {
    /// 要翻译的文本
    pub text: String,
    /// 源语言（ISO 639，缺省自动检测）
    #[serde(default)]
    pub from: Option<String>,
    /// 目标语言（缺省自动检测：中英互译）
    #[serde(default)]
    pub to: Option<String>,
    /// 引擎（iciba / llm，缺省自动调度）
    #[serde(default)]
    pub engine: Option<String>,
    /// true 走 SSE 流式（LLM 逐段推送；词典引擎等价于单段）
    #[serde(default)]
    pub stream: bool,
}

pub async fn translate(
    State(state): State<ApiState>,
    Json(body): Json<TranslateBody>,
) -> Response {
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "text 不能为空");
    }
    let req = TranslateRequest {
        from: body.from.as_deref().map(normalize_lang),
        to: body.to.as_deref().map(normalize_lang),
        text,
    };
    let choice = match &body.engine {
        Some(e) => EngineChoice::Named(e.clone()),
        None => EngineChoice::Auto,
    };

    if body.stream {
        return sse_translate(state, req, choice).await;
    }

    match state.svc.translate.translate(&req, &choice).await {
        Ok(outcome) => {
            record_history(&state, &outcome.result);
            Json(outcome_json(&outcome)).into_response()
        }
        Err(e) => error_response(StatusCode::BAD_GATEWAY, &format!("{e:#}")),
    }
}

/// SSE 流式：data: {"delta": "..."} 增量段，结束帧 {"done": true, result} 或 {"error": ...}
async fn sse_translate(
    state: ApiState,
    req: TranslateRequest,
    choice: EngineChoice,
) -> Response {
    // 词典可命中时：一次性推送完整结果（两帧）
    let use_dict = match &choice {
        EngineChoice::Named(name) => name == "iciba",
        EngineChoice::Auto => crate::service::translate::is_word_like(&req.text),
    };
    if use_dict {
        if let Ok(outcome) = state.svc.translate.translate(&req, &choice).await {
            record_history(&state, &outcome.result);
            let payload = serde_json::to_string(&serde_json::json!({
                "delta": outcome.result.translation,
                "result": outcome_json(&outcome)["result"],
            }))
            .unwrap_or_default();
            let stream = futures::stream::iter(vec![
                Ok::<Event, Infallible>(Event::default().data(payload)),
                Ok(Event::default().data(r#"{"done": true}"#)),
            ]);
            return Sse::new(stream).keep_alive(KeepAlive::default()).into_response();
        }
    }

    // LLM 流式路径
    let provider = match state.svc.providers.enabled_config() {
        Ok(p) => p,
        Err(_) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "未配置启用的 AI 供应商（流式仅 LLM 路径可用）",
            )
        }
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let svc = state.svc.clone();
    tokio::spawn(async move {
        let outcome = translate_stream(&provider, &req, |delta| {
            let _ = tx.send(serde_json::json!({ "delta": delta }).to_string());
        }, || false)
        .await;
        let final_frame = match outcome {
            Ok(result) => {
                if let Err(e) = svc.history.record(&result) {
                    tracing::warn!(error = %e, "历史留存失败（不影响翻译结果）");
                }
                serde_json::json!({ "done": true, "result": {
                    "engine": result.engine, "from": result.from, "to": result.to,
                    "text": result.text, "translation": result.translation,
                    "phonetic": result.phonetic, "parts": result.parts, "examples": result.examples,
                }})
                .to_string()
            }
            Err(e) => serde_json::json!({ "error": format!("{e:#}") }).to_string(),
        };
        let _ = tx.send(final_frame);
    });

    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
        .map(|data| Ok::<Event, Infallible>(Event::default().data(data)));
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

fn record_history(state: &ApiState, result: &crate::translator::TranslationResult) {
    if let Err(e) = state.svc.history.record(result) {
        tracing::warn!(error = %e, "历史留存失败（不影响翻译结果）");
    }
}

pub(crate) fn outcome_json(outcome: &TranslateOutcome) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "fallback_from": outcome.fallback_from,
        "result": {
            "engine": outcome.result.engine,
            "from": outcome.result.from,
            "to": outcome.result.to,
            "text": outcome.result.text,
            "translation": outcome.result.translation,
            "phonetic": outcome.result.phonetic,
            "parts": outcome.result.parts,
            "examples": outcome.result.examples,
        },
    })
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    (status, Json(serde_json::json!({ "ok": false, "error": msg }))).into_response()
}
