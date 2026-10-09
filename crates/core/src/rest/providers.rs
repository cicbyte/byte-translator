//! GET/PUT /api/v1/providers：供应商查看（脱敏）与配置。
//!
//! GET 返回列表（Key 只给 api_key_set 布尔）；
//! PUT 新增/更新（Key 留空 = 保持原值）+ enable 字段可同时切换启用。

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;

use super::ApiState;
use crate::service::provider::ProviderInput;

pub async fn list(State(state): State<ApiState>) -> Response {
    match state.svc.providers.list() {
        Ok(list) => Json(serde_json::json!({ "list": list })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{e:#}") })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ProviderBody {
    pub name: String,
    pub provider_key: String,
    #[serde(default)]
    pub base_url: String,
    /// 留空表示保持已有 Key（更新时）
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub models: Option<Vec<String>>,
    /// 同时设置启用状态（true 时独占启用位）
    #[serde(default)]
    pub enable: Option<bool>,
}

fn default_max_tokens() -> i64 {
    4096
}
fn default_temperature() -> f64 {
    0.3
}

pub async fn upsert(State(state): State<ApiState>, Json(body): Json<ProviderBody>) -> Response {
    let key_for_enable = body.provider_key.clone();
    let input = ProviderInput {
        name: body.name,
        provider_key: body.provider_key,
        base_url: body.base_url,
        api_key: body.api_key,
        model: body.model,
        max_tokens: body.max_tokens,
        temperature: body.temperature,
        models: body.models.unwrap_or_default(),
    };
    let result = state
        .svc
        .providers
        .upsert(&input)
        .and_then(|_| match body.enable {
            Some(true) => state.svc.providers.set_enabled(&key_for_enable, true),
            Some(false) => state.svc.providers.set_enabled(&key_for_enable, false),
            None => Ok(()),
        });
    match result {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{e:#}") })),
        )
            .into_response(),
    }
}
