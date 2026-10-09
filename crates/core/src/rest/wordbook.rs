//! GET/POST /api/v1/wordbook、DELETE /api/v1/wordbook/{id}：生词本管理。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;

use super::ApiState;

#[derive(Debug, Deserialize)]
pub struct WordbookParams {
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub size: Option<i64>,
}

pub async fn list(
    State(state): State<ApiState>,
    Query(p): Query<WordbookParams>,
) -> Response {
    match state
        .svc
        .wordbook
        .list(p.keyword.as_deref().unwrap_or(""), p.page.unwrap_or(1), p.size.unwrap_or(20))
    {
        Ok((list, total)) => Json(serde_json::json!({ "total": total, "list": list })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{e:#}") })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct AddBody {
    pub text: String,
    #[serde(default)]
    pub note: String,
}

pub async fn add(State(state): State<ApiState>, Json(body): Json<AddBody>) -> Response {
    match state.svc.wordbook.add(&body.text, &body.note) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{e:#}") })),
        )
            .into_response(),
    }
}

pub async fn remove(State(state): State<ApiState>, Path(id): Path<i64>) -> Response {
    match state.svc.wordbook.remove(id) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("{e:#}") })),
        )
            .into_response(),
    }
}
