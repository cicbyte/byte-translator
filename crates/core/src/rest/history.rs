//! GET /api/v1/history：历史检索（关键词/引擎/语言对过滤 + 分页）。

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Json};
use serde::Deserialize;

use super::ApiState;
use crate::service::history::HistoryQuery;

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub size: Option<i64>,
}

pub async fn list(
    State(state): State<ApiState>,
    Query(p): Query<HistoryParams>,
) -> impl IntoResponse {
    let q = HistoryQuery {
        keyword: p.keyword.unwrap_or_default(),
        engine: p.engine.unwrap_or_default(),
        from: p.from.unwrap_or_default(),
        to: p.to.unwrap_or_default(),
        page: p.page.unwrap_or(1),
        size: p.size.unwrap_or(20),
    };
    match state.svc.history.search(&q) {
        Ok((list, total)) => Json(serde_json::json!({ "total": total, "list": list })),
        Err(e) => Json(serde_json::json!({ "error": format!("{e:#}") })),
    }
}
