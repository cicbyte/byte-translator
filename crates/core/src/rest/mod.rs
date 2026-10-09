//! REST API 服务（axum，feature = "server"）。
//!
//! 鉴权规则：
//! - X-API-Key 请求头（或 Authorization: Bearer <key>），常量时间比较；
//! - 默认绑定 127.0.0.1；绑定非回环地址时必须持有 API Key（空 key 拒绝启动）；
//! - 回环连接在显式允许时可免 key（allow_local_no_key，本机调试用）；
//! - 无 CORS——面向非浏览器客户端（CLI / 脚本），浏览器端由桌面端 IPC 负责。
//!
//! REST 与 MCP 是两把独立 Key（暴露面不同，轮换互不影响），
//! 存于 <data>/config/api.json（原子写，支持热轮换）。

pub mod docs;
pub mod history;
pub mod providers;
pub mod translate;
pub mod wordbook;

use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get, post};
use axum::Router;

use crate::db::Database;
use crate::service::history::HistoryService;
use crate::service::provider::ProviderService;
use crate::service::translate::TranslateService;
use crate::service::wordbook::WordbookService;

/// REST 默认端口（与生态内其它本地服务错开）
pub const DEFAULT_PORT: u16 = 8321;

/// 运行中服务的 Key 句柄：轮换/自定义时热更新（每请求读取）
pub type SharedKey = Arc<std::sync::RwLock<String>>;

/// 各领域服务的聚合句柄（各端点共享；SQLite WAL + busy_timeout 承担并发）
pub struct RestServices {
    pub translate: TranslateService,
    pub history: HistoryService,
    pub wordbook: WordbookService,
    pub providers: ProviderService,
}

impl RestServices {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            translate: TranslateService::production(db.clone()),
            history: HistoryService::new(db.clone()),
            wordbook: WordbookService::new(db.clone()),
            providers: ProviderService::new(db),
        }
    }
}

/// 共享状态（Clone 廉价：Arc + 标量）
#[derive(Clone)]
pub struct ApiState {
    pub svc: Arc<RestServices>,
    pub api_key: SharedKey,
    /// 回环连接免 key（仅本机调试场景；非回环始终要求 key）
    pub allow_local_no_key: bool,
    /// 绑定地址（Host 头校验用，防 DNS Rebinding——见 host_ok）
    pub bind_host: String,
}

/// 服务启动配置（btr serve 的参数形状）
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub allow_local_no_key: bool,
}

impl ServeConfig {
    /// 校验绑定安全性：非回环地址必须带非空 key（空 key 拒绝）。返回绑定地址。
    pub fn validated_bind_addr(&self) -> Result<SocketAddr> {
        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .with_context(|| format!("非法绑定地址 {}:{}", self.host, self.port))?;
        if !addr.ip().is_loopback() && self.api_key.trim().is_empty() {
            return Err(anyhow!(
                "绑定非回环地址（{}）必须配置 API Key——空 key 对局域网裸奔",
                addr.ip()
            ));
        }
        Ok(addr)
    }
}

/// 组装路由（不含监听；测试与 serve 共用）
pub fn build_router(state: ApiState) -> Router {
    let api = Router::new()
        .route("/translate", post(translate::translate))
        .route("/history", get(history::list))
        .route("/wordbook", get(wordbook::list).post(wordbook::add))
        .route("/wordbook/{id}", delete(wordbook::remove))
        .route("/providers", get(providers::list).put(providers::upsert))
        .layer(middleware::from_fn_with_state(state.clone(), auth));
    let api = api
        .route("/docs", get(docs::swagger_ui))
        .route("/openapi.json", get(docs::openapi_spec));
    Router::new()
        .route("/health", get(health))
        // /api/v1 前缀整体挂 auth 中间件；docs/openapi 与 /health 免认证
        .nest("/api/v1", api)
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

/// 鉴权中间件：规则见模块头注释。
/// 远端地址取自 request extensions（into_make_service_with_connect_info 注入；
/// 缺失时按非回环处理——要求 key，安全侧取向）。
async fn auth(
    State(state): State<ApiState>,
    req: Request,
    next: Next,
) -> Response {
    let provided = extract_key(&req);
    // Host 头校验（防 DNS Rebinding）：浏览器把 URL 的域名写进 Host——
    // 攻击页面把自有域名重绑定到 127.0.0.1 后请求来源是回环、但 Host 是
    // 攻击者域名；只放行本机字面量与配置的绑定地址，其余一律 403。
    let host = req
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    if !host_ok(&host, &state.bind_host) {
        return (
            StatusCode::FORBIDDEN,
            "非法 Host（仅允许 localhost / 127.0.0.1 / 绑定地址）",
        )
            .into_response();
    }
    let key_now = state.api_key.read().map(|k| k.clone()).unwrap_or_default();
    let remote_ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or_else(|| "192.0.2.1".parse::<IpAddr>().expect("TEST-NET-1 地址"));
    if !auth_ok(provided.as_deref(), remote_ip, &key_now, state.allow_local_no_key) {
        return (
            StatusCode::UNAUTHORIZED,
            "缺少或错误的 API Key（X-API-Key 或 Authorization: Bearer）",
        )
            .into_response();
    }
    next.run(req).await
}

fn extract_key(req: &Request) -> Option<String> {
    req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            req.headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(str::to_owned)
        })
}

/// Host 头白名单（纯函数，测试直接覆盖）：
/// 允许 localhost / 回环 IP 字面量 / 配置的绑定地址（含带端口形态）
pub(crate) fn host_ok(host_header: &str, bind_host: &str) -> bool {
    let strip_port = |h: &str| -> String {
        if let Some(rest) = h.strip_prefix('[') {
            if let Some(i) = rest.find(']') {
                return rest[..i].to_owned();
            }
        }
        h.split(':').next().unwrap_or(h).to_owned()
    };
    let host = strip_port(host_header);
    if host.is_empty() {
        return false;
    }
    let bind = strip_port(bind_host);
    let allowed = ["localhost", "127.0.0.1", "::1"];
    allowed.contains(&host.as_str()) || (bind != "0.0.0.0" && host == bind)
}

/// 鉴权判定（纯函数，测试直接覆盖）：
/// 免 key 通道 = 回环 + 显式允许（本机调试），与是否配置 key 无关；
/// 其余一律要求 key 匹配；空 key 只能走免 key 通道
fn auth_ok(provided: Option<&str>, remote: IpAddr, api_key: &str, allow_local_no_key: bool) -> bool {
    if remote.is_loopback() && allow_local_no_key {
        return true;
    }
    if api_key.trim().is_empty() {
        return false;
    }
    let Some(p) = provided else { return false };
    constant_time_eq(p.trim(), api_key.trim())
}

/// 常量时间字符串比较（长度不等直接 false——key 为定长 32 字符）
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// 阻塞式启动（调用方提供运行时与关停信号）
pub async fn serve(
    cfg: &ServeConfig,
    svc: Arc<RestServices>,
    key: SharedKey,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let bind = cfg.validated_bind_addr()?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("绑定 {bind} 失败（端口被占用？）"))?;
    tracing::info!(%bind, "REST 服务已绑定");
    let state = ApiState {
        svc,
        api_key: key,
        allow_local_no_key: cfg.allow_local_no_key,
        bind_host: cfg.host.clone(),
    };
    let app = build_router(state).into_make_service_with_connect_info::<SocketAddr>();
    // 优雅停机带上限：SSE 长连接不能无限拖住 serve future
    let shutdown = async {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), shutdown).await;
    };
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .context("REST 服务运行失败")?;
    Ok(())
}

// ===== API Key 的生成与持久化 =====

/// 两把 Key（REST / MCP 独立）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyFile {
    #[serde(default, alias = "api_key")]
    pub rest_key: String,
    #[serde(default)]
    pub mcp_key: String,
}

#[derive(Debug, Clone)]
pub struct ApiKeys {
    pub rest: String,
    pub mcp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Rest,
    Mcp,
}

/// 写 key 文件：tmp 名含 pid+原子序号——CLI 与桌面并发写时不互相同名截断；
/// 先 fsync 再 rename，掉电不留半截文件
fn write_key_file(dir: &Path, file: &Path, f: &ApiKeyFile) -> Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    std::fs::create_dir_all(dir).context("创建配置目录失败")?;
    let tmp = dir.join(format!(
        ".api.json.{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let payload = serde_json::to_string_pretty(f)?;
    {
        use std::io::Write as _;
        let mut fh = std::fs::File::create(&tmp).context("写入 API Key 失败")?;
        fh.write_all(payload.as_bytes()).context("写入 API Key 失败")?;
        fh.sync_all().context("刷盘 API Key 失败")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
    }
    std::fs::rename(&tmp, file).context("落盘 API Key 失败")?;
    Ok(())
}

/// 读取或生成 API Keys（<data>/config/api.json）。
/// 首次生成返回 Some(ApiKeyFile)（调用方提示用户保存），已存在返回 None。
pub fn load_or_create_api_keys(data_root: &Path) -> Result<(ApiKeys, Option<ApiKeyFile>)> {
    let dir = data_root.join("config");
    let file = dir.join("api.json");
    if file.exists() {
        // 并发写方刚 rename 的瞬间可能 NotFound/半空——小睡重试一次
        let read = || -> Result<String> {
            for attempt in 0..3 {
                match std::fs::read_to_string(&file) {
                    Ok(t) if !t.trim().is_empty() => return Ok(t),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound && attempt < 2 => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(e) => return Err(anyhow::Error::new(e)),
                    Ok(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                }
            }
            std::fs::read_to_string(&file).context("读取 API Key 配置失败")
        };
        let txt = read()?;
        let mut f: ApiKeyFile = serde_json::from_str(&txt).context("解析 API Key 配置失败")?;
        let mut created = None;
        if f.mcp_key.is_empty() {
            f.mcp_key = new_key();
            write_key_file(&dir, &file, &f)?;
            created = Some(f.clone());
        }
        if f.rest_key.is_empty() {
            f.rest_key = new_key();
            write_key_file(&dir, &file, &f)?;
            created = Some(f.clone());
        }
        return Ok((ApiKeys { rest: f.rest_key, mcp: f.mcp_key }, created));
    }
    let f = ApiKeyFile {
        rest_key: new_key(),
        mcp_key: new_key(),
    };
    write_key_file(&dir, &file, &f)?;
    Ok((
        ApiKeys {
            rest: f.rest_key.clone(),
            mcp: f.mcp_key.clone(),
        },
        Some(f),
    ))
}

fn new_key() -> String {
    // uuid v4 无连字符形式恰为 32 个十六进制字符
    uuid::Uuid::new_v4().simple().to_string()
}

/// 覆写指定通道的 Key（轮换），另一把不动
pub fn rotate_api_key(data_root: &Path, kind: KeyKind) -> Result<String> {
    let dir = data_root.join("config");
    let file = dir.join("api.json");
    let mut f: ApiKeyFile = if file.exists() {
        let txt = std::fs::read_to_string(&file).context("读取 API Key 配置失败")?;
        serde_json::from_str(&txt).context("解析 API Key 配置失败")?
    } else {
        ApiKeyFile {
            rest_key: String::new(),
            mcp_key: String::new(),
        }
    };
    let new = new_key();
    match kind {
        KeyKind::Rest => f.rest_key = new.clone(),
        KeyKind::Mcp => f.mcp_key = new.clone(),
    }
    write_key_file(&dir, &file, &f)?;
    Ok(new)
}

/// 设置指定通道的自定义 Key（校验非空）
pub fn set_api_key(data_root: &Path, kind: KeyKind, key: &str) -> Result<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(anyhow!("Key 不能为空"));
    }
    let dir = data_root.join("config");
    let file = dir.join("api.json");
    let mut f: ApiKeyFile = if file.exists() {
        let txt = std::fs::read_to_string(&file).context("读取 API Key 配置失败")?;
        serde_json::from_str(&txt).context("解析 API Key 配置失败")?
    } else {
        ApiKeyFile {
            rest_key: String::new(),
            mcp_key: String::new(),
        }
    };
    match kind {
        KeyKind::Rest => f.rest_key = key.to_string(),
        KeyKind::Mcp => f.mcp_key = key.to_string(),
    }
    write_key_file(&dir, &file, &f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_header_blocks_rebinding() {
        // DNS Rebinding：攻击者域名（即便解析到 127.0.0.1）不得通过
        assert!(!host_ok("evil.com:8321", "127.0.0.1"));
        assert!(!host_ok("evil.com", "0.0.0.0"));
        assert!(!host_ok("", "127.0.0.1"));
        // 本机字面量与带端口形态放行
        assert!(host_ok("127.0.0.1:8321", "127.0.0.1"));
        assert!(host_ok("localhost:8321", "0.0.0.0"));
        assert!(host_ok("[::1]:8321", "127.0.0.1"));
        // 配置了具体绑定地址时该地址放行；通配不放行
        assert!(host_ok("192.168.1.5:8321", "192.168.1.5"));
        assert!(!host_ok("192.168.1.5:8321", "0.0.0.0"));
    }

    #[test]
    fn auth_rules() {
        let lo = "127.0.0.1".parse::<IpAddr>().unwrap();
        let lan = "192.168.1.5".parse::<IpAddr>().unwrap();
        // 非回环：key 必须匹配
        assert!(!auth_ok(None, lan, "k32", false));
        assert!(auth_ok(Some("k32"), lan, "k32", false));
        assert!(!auth_ok(Some("k33"), lan, "k32", false));
        // 回环：同样要 key（allow_local_no_key=false）
        assert!(!auth_ok(None, lo, "k32", false));
        // 免 key 模式：仅回环放行，局域网仍要 key
        assert!(auth_ok(None, lo, "k32", true));
        assert!(!auth_ok(None, lan, "k32", true));
    }

    #[test]
    fn non_loopback_empty_key_rejected_at_config() {
        let cfg = ServeConfig {
            host: "0.0.0.0".into(),
            port: DEFAULT_PORT,
            api_key: String::new(),
            allow_local_no_key: true,
        };
        assert!(cfg.validated_bind_addr().is_err());
        let ok = ServeConfig {
            host: "127.0.0.1".into(),
            port: DEFAULT_PORT,
            api_key: String::new(),
            allow_local_no_key: true,
        };
        assert!(ok.validated_bind_addr().is_ok());
    }

    #[test]
    fn key_file_roundtrip_and_rotate_independence() {
        let tmp = tempfile::tempdir().unwrap();
        let (k1, created) = load_or_create_api_keys(tmp.path()).unwrap();
        assert_eq!(k1.rest.len(), 32);
        assert_eq!(k1.mcp.len(), 32);
        assert_ne!(k1.rest, k1.mcp, "两把 Key 应独立生成");
        assert!(created.is_some(), "首次应返回新 key");
        let (k2, created2) = load_or_create_api_keys(tmp.path()).unwrap();
        assert_eq!(k1.rest, k2.rest);
        assert_eq!(k1.mcp, k2.mcp);
        assert!(created2.is_none(), "二次读取不再新建");
        // 轮换 REST 不影响 MCP
        let new_rest = rotate_api_key(tmp.path(), KeyKind::Rest).unwrap();
        assert_ne!(new_rest, k1.rest);
        let (k3, _) = load_or_create_api_keys(tmp.path()).unwrap();
        assert_eq!(k3.rest, new_rest);
        assert_eq!(k3.mcp, k1.mcp, "轮换 REST 不得影响 MCP");
        // 自定义 MCP
        set_api_key(tmp.path(), KeyKind::Mcp, "custom-key-32-chars-aaaaaaaaaaa").unwrap();
        let (k4, _) = load_or_create_api_keys(tmp.path()).unwrap();
        assert_eq!(k4.mcp, "custom-key-32-chars-aaaaaaaaaaa");
    }

    // ===== 端点集成（oneshot，不监听端口）=====

    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_state(tmp: &tempfile::TempDir) -> ApiState {
        let db = Database::open(&tmp.path().join("rest-test.db")).unwrap();
        let key: SharedKey = Arc::new(std::sync::RwLock::new("test-key-32-chars-aaaaaaaaaaa".into()));
        ApiState {
            svc: Arc::new(RestServices::new(Arc::new(db))),
            api_key: key,
            allow_local_no_key: false,
            bind_host: "127.0.0.1".into(),
        }
    }

    async fn body_json(body: axum::body::Body) -> serde_json::Value {
        let bytes = body.into_data_stream().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_is_free() {
        let tmp = tempfile::tempdir().unwrap();
        let app = build_router(test_state(&tmp));
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn business_routes_require_key() {
        let tmp = tempfile::tempdir().unwrap();
        let app = build_router(test_state(&tmp));
        // 无 key：401
        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/translate")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"text":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        // 非法 Host：403
        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/translate")
                    .header("content-type", "application/json")
                    .header("host", "evil.com")
                    .header("x-api-key", "test-key-32-chars-aaaaaaaaaaa")
                    .body(Body::from(r#"{"text":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn wordbook_flow_with_key() {
        let tmp = tempfile::tempdir().unwrap();
        let app = build_router(test_state(&tmp));
        let key = "test-key-32-chars-aaaaaaaaaaa";

        // 收藏
        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/wordbook")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .header("x-api-key", key)
                    .body(Body::from(r#"{"text":"love","note":"n. 爱"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 列表
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/wordbook?keyword=love")
                    .header("host", "127.0.0.1")
                    .header("authorization", format!("Bearer {key}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp.into_body()).await;
        assert_eq!(v["total"], 1);
        assert_eq!(v["list"][0]["text"], "love");
        assert_eq!(v["list"][0]["note"], "n. 爱");
    }

    #[tokio::test]
    async fn translate_empty_text_is_400() {
        let tmp = tempfile::tempdir().unwrap();
        let app = build_router(test_state(&tmp));
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/translate")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .header("x-api-key", "test-key-32-chars-aaaaaaaaaaa")
                    .body(Body::from(r#"{"text":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
