//! MCP 服务器：rmcp 工具注册 + 双 transport（stdio / StreamableHTTP）。

use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters, model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;

use super::tools::{self, ToolCtx};
use crate::db::Database;

// ============ 工具入参 ============

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TranslateInput {
    /// 要翻译的文本（词或句子）
    pub text: String,
    /// 源语言（ISO 639，缺省自动检测）
    #[serde(default)]
    pub from: Option<String>,
    /// 目标语言（缺省自动检测：中英互译）
    #[serde(default)]
    pub to: Option<String>,
    /// 引擎：iciba / llm（缺省自动调度——词查词典、句走 LLM、词典未命中降级）
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DefineInput {
    /// 要查的单词
    pub word: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HistorySearchInput {
    /// 检索关键词（原文 LIKE 匹配）
    #[serde(default)]
    pub keyword: String,
    /// 返回条数上限（1-50，默认 10）
    #[serde(default = "default_history_limit")]
    pub limit: i64,
}

fn default_history_limit() -> i64 {
    10
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WordbookAddInput {
    /// 要收藏的词条或句子
    pub text: String,
    /// 备注（可选）
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WordbookListInput {
    /// 过滤关键词（可选）
    #[serde(default)]
    pub keyword: String,
    /// 返回条数上限（默认 20）
    #[serde(default = "default_wordbook_limit")]
    pub limit: i64,
}

fn default_wordbook_limit() -> i64 {
    20
}

/// MCP 服务器：每个会话共享同一个 ToolCtx
#[derive(Clone)]
pub struct BtrMcpServer {
    ctx: Arc<ToolCtx>,
    #[allow(dead_code)]
    tool_router: ToolRouter<BtrMcpServer>,
}

impl BtrMcpServer {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            ctx: Arc::new(ToolCtx::new(db)),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl BtrMcpServer {
    #[tool(description = "翻译文本：词自动查词典（音标/词性/例句），句走 LLM；\
                          词典未命中自动降级 LLM。可指定 from/to 语言与 engine。")]
    async fn translate(
        &self,
        Parameters(TranslateInput { text, from, to, engine }): Parameters<TranslateInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::translate(&self.ctx, &text, from, to, engine).await))
    }

    #[tool(description = "查单词词典释义（音标/词性/例句；仅中英）。\
                          未命中时提示改用 translate。")]
    async fn define(
        &self,
        Parameters(DefineInput { word }): Parameters<DefineInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::define(&self.ctx, &word).await))
    }

    #[tool(description = "检索翻译历史（关键词匹配原文），返回最近条目。")]
    async fn history_search(
        &self,
        Parameters(HistorySearchInput { keyword, limit }): Parameters<HistorySearchInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::history_search(&self.ctx, &keyword, limit)))
    }

    #[tool(description = "收藏生词到生词本（幂等：重复收藏更新备注）。")]
    async fn wordbook_add(
        &self,
        Parameters(WordbookAddInput { text, note }): Parameters<WordbookAddInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::wordbook_add(&self.ctx, &text, &note)))
    }

    #[tool(description = "列出生词本（可选关键词过滤）。")]
    async fn wordbook_list(
        &self,
        Parameters(WordbookListInput { keyword, limit }): Parameters<WordbookListInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::wordbook_list(&self.ctx, &keyword, limit)))
    }
}

#[tool_handler]
impl ServerHandler for BtrMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("byte-translator", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "byte-translator 本地翻译服务。5 个工具：\n\
                 translate(text, from?, to?, engine?) — 翻译（自动调度）\n\
                 define(word) — 查词典（音标/词性/例句）\n\
                 history_search(keyword, limit?) — 检索翻译历史\n\
                 wordbook_add(text, note?) — 收藏生词\n\
                 wordbook_list(keyword?, limit?) — 生词本列表"
                    .to_string(),
            )
    }
}

fn text_result(text: String) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}

// ============ transport ============

/// stdio 模式（阻塞；stdout 是协议信道，日志走 stderr）
pub async fn serve_stdio(db: Arc<Database>) -> Result<()> {
    let server = BtrMcpServer::new(db);
    tracing::info!(target: "stderr", "MCP stdio server starting");
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("stdio serve 失败：{e:?}"))?;
    service.waiting().await?;
    Ok(())
}

/// http 模式（StreamableHTTP via axum，/mcp 端点；MCP Key 独立于 REST）
pub async fn serve_http(
    db: Arc<Database>,
    host: &str,
    port: u16,
    key: crate::rest::SharedKey,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("MCP HTTP bind {addr} 失败：{e}"))?;
    tracing::info!(target: "stderr", "MCP HTTP bound on http://{addr}/mcp");

    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };

    let db_for_sessions = db.clone();
    let service = StreamableHttpService::new(
        move || Ok(BtrMcpServer::new(db_for_sessions.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let mut app = axum::Router::new().nest_service("/mcp", service);
    app = app.layer(axum::middleware::from_fn_with_state(
        McpAuthState {
            key: key.clone(),
            bind_host: host.to_string(),
        },
        mcp_auth_middleware,
    ));

    let shutdown = async {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), shutdown).await;
    };
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| anyhow::anyhow!("MCP HTTP serve 失败：{e}"))?;
    Ok(())
}

/// 鉴权中间件状态
#[derive(Clone)]
struct McpAuthState {
    key: crate::rest::SharedKey,
    bind_host: String,
}

/// MCP HTTP API Key 认证（X-API-Key / Bearer，常量时间比较）+ Host 头校验
async fn mcp_auth_middleware(
    axum::extract::State(state): axum::extract::State<McpAuthState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let host_header = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if !crate::rest::host_ok(host_header, &state.bind_host) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error": "Illegal Host"})),
        )
            .into_response();
    }
    let expected_key = state.key.read().map(|k| k.clone()).unwrap_or_default();
    let provided = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            req.headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(str::to_owned)
        });
    let ok = provided
        .as_deref()
        .map(|p| {
            let k = expected_key.trim();
            p.trim().len() == k.len()
                && p.trim().bytes().zip(k.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
        })
        .unwrap_or(false);
    if ok {
        return next.run(req).await;
    }
    (
        axum::http::StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({"error": "Invalid or missing API key"})),
    )
        .into_response()
}
