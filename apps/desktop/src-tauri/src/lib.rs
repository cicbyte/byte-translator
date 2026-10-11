//! 桌面端应用：commands 薄封装领域服务（业务在 core，UI 依赖之外零逻辑）。
//!
//! 日志必须写 stderr——stdout 是 Tauri IPC 信道。

use std::sync::Arc;

use serde::Serialize;
use tauri::{ipc::Channel, Manager, State};
use tokio_util::sync::CancellationToken;

use byte_translator_core::db::Database;
use byte_translator_core::rest::RestServices;
use byte_translator_core::service::history::HistoryQuery;
use byte_translator_core::service::provider::{ProviderInput, ProviderView};
use byte_translator_core::service::translate::{EngineChoice, TranslateOutcome};
use byte_translator_core::translator::{TranslateRequest, TranslationResult};

/// 应用状态：领域服务聚合（单例）+ 本地服务启停句柄
struct AppState {
    services: Arc<RestServices>,
    db: Arc<Database>,
    data_root: std::path::PathBuf,
    rest: std::sync::Mutex<ServiceHandle>,
    mcp: std::sync::Mutex<ServiceHandle>,
}

#[derive(Default)]
struct ServiceHandle {
    running: bool,
    port: u16,
    cancel: Option<CancellationToken>,
}

#[derive(Serialize)]
struct TranslateWire {
    result: TranslationResult,
    fallback_from: Option<String>,
}

fn wire(outcome: TranslateOutcome) -> TranslateWire {
    TranslateWire {
        result: outcome.result,
        fallback_from: outcome.fallback_from,
    }
}

#[tauri::command]
async fn translate(
    state: State<'_, AppState>,
    text: String,
    engine: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<TranslateWire, String> {
    let choice = match engine.as_deref() {
        Some("iciba") | Some("llm") => EngineChoice::Named(engine.unwrap()),
        _ => EngineChoice::Auto,
    };
    let req = TranslateRequest {
        text,
        from: from.filter(|s| !s.is_empty()),
        to: to.filter(|s| !s.is_empty()),
    };
    let outcome = state
        .services
        .translate
        .translate(&req, &choice)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let _ = state.services.history.record(&outcome.result);
    Ok(wire(outcome))
}

#[tauri::command]
async fn history_list(
    state: State<'_, AppState>,
    keyword: Option<String>,
    engine: Option<String>,
    from: Option<String>,
    to: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let q = HistoryQuery {
        keyword: keyword.unwrap_or_default(),
        engine: engine.unwrap_or_default(),
        from: from.unwrap_or_default(),
        to: to.unwrap_or_default(),
        page: page.unwrap_or(1),
        size: size.unwrap_or(20),
    };
    let (list, total) = state
        .services
        .history
        .search(&q)
        .map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "total": total, "list": list }))
}

#[tauri::command]
async fn wordbook_list(
    state: State<'_, AppState>,
    keyword: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
) -> Result<serde_json::Value, String> {
    let (list, total) = state
        .services
        .wordbook
        .list(keyword.as_deref().unwrap_or(""), page.unwrap_or(1), size.unwrap_or(50))
        .map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "total": total, "list": list }))
}

#[tauri::command]
async fn wordbook_add(
    state: State<'_, AppState>,
    text: String,
    note: Option<String>,
) -> Result<serde_json::Value, String> {
    state
        .services
        .wordbook
        .add(&text, note.as_deref().unwrap_or(""))
        .map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn wordbook_remove(state: State<'_, AppState>, id: i64) -> Result<serde_json::Value, String> {
    state.services.wordbook.remove(id).map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "ok": true }))
}

// ===== 供应商 =====

#[tauri::command]
async fn provider_list(state: State<'_, AppState>) -> Result<Vec<ProviderView>, String> {
    state.services.providers.list().map_err(|e| format!("{e:#}"))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderSetArgs {
    provider_key: String,
    name: Option<String>,
    base_url: Option<String>,
    /// 留空 = 保持已有值
    api_key: Option<String>,
    model: Option<String>,
    max_tokens: Option<i64>,
    temperature: Option<f64>,
    models: Option<Vec<String>>,
    enable: Option<bool>,
}

#[tauri::command]
async fn provider_set(
    state: State<'_, AppState>,
    args: ProviderSetArgs,
) -> Result<serde_json::Value, String> {
    let input = ProviderInput {
        name: args.name.unwrap_or_else(|| args.provider_key.clone()),
        provider_key: args.provider_key.clone(),
        base_url: args.base_url.unwrap_or_default(),
        api_key: args.api_key.unwrap_or_default(),
        model: args.model.unwrap_or_default(),
        max_tokens: args.max_tokens.unwrap_or(4096),
        temperature: args.temperature.unwrap_or(0.3),
        models: args.models.unwrap_or_default(),
    };
    state
        .services
        .providers
        .upsert(&input)
        .map_err(|e| format!("{e:#}"))?;
    if args.enable == Some(true) {
        state
            .services
            .providers
            .set_enabled(&args.provider_key, true)
            .map_err(|e| format!("{e:#}"))?;
    }
    Ok(serde_json::json!({ "ok": true, "provider_key": args.provider_key }))
}

#[tauri::command]
async fn provider_enable(
    state: State<'_, AppState>,
    provider_key: String,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    // 停用全部的约定：provider_key 传空串
    state
        .services
        .providers
        .set_enabled(&provider_key, enabled)
        .map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
async fn provider_test(
    state: State<'_, AppState>,
    provider_key: String,
) -> Result<serde_json::Value, String> {
    state
        .services
        .providers
        .test_connectivity(&provider_key)
        .await
        .map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "ok": true, "provider_key": provider_key }))
}

// ===== 翻译（流式：LLM 逐段经 Channel 推前端）=====

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEventWire {
    Delta { text: String },
    Done { result: TranslationResult },
    Error { message: String },
}

#[tauri::command]
async fn translate_llm_stream(
    state: State<'_, AppState>,
    text: String,
    from: Option<String>,
    to: Option<String>,
    on_event: Channel<StreamEventWire>,
) -> Result<(), String> {
    let req = TranslateRequest {
        text,
        from: from.filter(|s| !s.is_empty()),
        to: to.filter(|s| !s.is_empty()),
    };
    // LLM 供应商就绪才走流式；词典路径让前端回落非流式 command
    let provider = state
        .services
        .providers
        .enabled_config()
        .map_err(|e| format!("{e:#}"))?;
    let history = state.services.history.clone();
    let outcome = byte_translator_core::translator::llm::translate_stream(&provider, &req, |delta| {
        let _ = on_event.send(StreamEventWire::Delta { text: delta.to_string() });
    }, || false)
    .await;
    match outcome {
        Ok(result) => {
            let _ = history.record(&result);
            let _ = on_event.send(StreamEventWire::Done { result });
            Ok(())
        }
        Err(e) => {
            let _ = on_event.send(StreamEventWire::Error { message: format!("{e:#}") });
            Err(format!("{e:#}"))
        }
    }
}

// ===== 本地服务启停（REST / MCP）=====

#[tauri::command]
fn api_keys(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (keys, _) = byte_translator_core::rest::load_or_create_api_keys(&state.data_root)
        .map_err(|e| format!("{e:#}"))?;
    Ok(serde_json::json!({ "rest": keys.rest, "mcp": keys.mcp }))
}

/// 拉取供应商可用模型列表（不耗 token），返回 id 列表
#[tauri::command]
async fn provider_fetch_models(
    state: State<'_, AppState>,
    provider_key: String,
) -> Result<Vec<String>, String> {
    state
        .services
        .providers
        .fetch_models(&provider_key)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// 设置词典引擎 Key：写库（跨端共享）+ 进程内即时生效
#[tauri::command]
fn set_iciba_key(state: State<'_, AppState>, key: String) -> Result<(), String> {
    let k = key.trim().to_string();
    state
        .services
        .config
        .set(byte_translator_core::service::config::KEY_ICIBA_API_KEY, &k)
        .map_err(|e| format!("{e:#}"))?;
    if k.is_empty() {
        std::env::remove_var("ICIBA_API_KEY");
    } else {
        std::env::set_var("ICIBA_API_KEY", k);
    }
    Ok(())
}

/// 引擎列表（注册表全量 + 配置状态 + 已保存字段标记）
#[tauri::command]
fn engines_list(state: State<'_, AppState>) -> serde_json::Value {
    use byte_translator_core::translator::registry;
    let cfg_svc = &state.services.config;
    let engines: Vec<serde_json::Value> = registry::descriptors()
        .iter()
        .map(|d| {
            let cfg_json = cfg_svc
                .get(&registry::config_key(d.name))
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
            let saved: serde_json::Value = match &cfg_json {
                Some(v) => {
                    let mut m = serde_json::Map::new();
                    for (field, _) in d.key_fields {
                        let set = v.get(*field)
                            .and_then(|f| f.as_str())
                            .map(|x| !x.trim().is_empty())
                            .unwrap_or(false);
                        m.insert(field.to_string(), serde_json::json!(set));
                    }
                    serde_json::Value::Object(m)
                }
                None => serde_json::json!({}),
            };
            // iciba 的 key 走独立配置键
            let (configured, saved) = if d.name == "iciba" {
                let set = cfg_svc
                    .get(byte_translator_core::service::config::KEY_ICIBA_API_KEY)
                    .ok()
                    .flatten()
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
                    || std::env::var("ICIBA_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false);
                (set, serde_json::json!({ "key": set }))
            } else if d.name == "llm" {
                let on = state.services.providers.enabled_config().is_ok();
                (on, serde_json::json!({}))
            } else {
                (registry::is_configured(d.name, cfg_json.as_ref()), saved)
            };
            serde_json::json!({
                "name": d.name,
                "label": d.label,
                "capability": d.capability,
                "group": format!("{:?}", d.group),
                "groupLabel": d.group.label(),
                "keyFields": d.key_fields.iter()
                    .map(|(f, l)| serde_json::json!({ "field": f, "label": l }))
                    .collect::<Vec<_>>(),
                "configured": configured,
                "saved": saved,
            })
        })
        .collect();
    serde_json::json!({
        "engines": engines,
        "defaultEngine": state.services.config.default_engine(),
    })
}

/// 保存引擎配置（values: 各字段值；iciba 特判走独立键）
#[tauri::command]
fn engine_set_config(
    state: State<'_, AppState>,
    name: String,
    values: serde_json::Value,
) -> Result<(), String> {
    use byte_translator_core::translator::registry;
    if name == "iciba" {
        let key = values.get("key").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        return state
            .services
            .config
            .set(byte_translator_core::service::config::KEY_ICIBA_API_KEY, &key)
            .map_err(|e| format!("{e:#}"));
    }
    // 仅保留注册的字段；空值字段剔除（留空保持已有值的语义：前端只传修改项）
    let desc = registry::descriptor(&name).ok_or_else(|| format!("未知引擎: {name}"))?;
    if desc.key_fields.is_empty() {
        return Err(format!("引擎 {name} 无需配置"));
    }
    // 合并已有配置（未传字段保留原值）
    let existing = state
        .services
        .config
        .get(&registry::config_key(&name))
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let mut merged = existing;
    for (field, _) in desc.key_fields {
        if let Some(v) = values.get(*field).and_then(|v| v.as_str()) {
            if v.trim().is_empty() {
                // 传空串 = 清除该字段
                if let Some(obj) = merged.as_object_mut() {
                    obj.remove(*field);
                }
            } else {
                merged[*field] = serde_json::json!(v.trim());
            }
        }
    }
    state
        .services
        .config
        .set(&registry::config_key(&name), &merged.to_string())
        .map_err(|e| format!("{e:#}"))
}

/// 引擎连通测试：装配后真查一个常用词
#[tauri::command]
async fn engine_test(state: State<'_, AppState>, name: String) -> Result<serde_json::Value, String> {
    if name == "llm" {
        return Ok(serde_json::json!({ "ok": true, "note": "LLM 走启用中的 AI 供应商" }));
    }
    let req = byte_translator_core::translator::TranslateRequest {
        text: "love".into(),
        from: Some("en".into()),
        to: Some("zh".into()),
    };
    match state
        .services
        .translate
        .translate(&req, &byte_translator_core::service::translate::EngineChoice::Named(name))
        .await
    {
        Ok(outcome) => Ok(serde_json::json!({
            "ok": true,
            "word": outcome.result.text,
            "translation": outcome.result.translation.chars().take(40).collect::<String>(),
        })),
        Err(e) => Err(format!("{e:#}")),
    }
}

/// 设置默认引擎策略（auto / iciba / llm）
#[tauri::command]
fn set_default_engine(state: State<'_, AppState>, engine: String) -> Result<(), String> {
    if !["auto", "iciba", "llm"].contains(&engine.as_str()) {
        return Err(format!("非法引擎策略: {engine}"));
    }
    state
        .services
        .config
        .set(byte_translator_core::service::config::KEY_DEFAULT_ENGINE, &engine)
        .map_err(|e| format!("{e:#}"))
}

/// 应用信息（数据目录等，供设置页展示）
#[tauri::command]
fn app_info(state: State<'_, AppState>) -> serde_json::Value {
    serde_json::json!({
        "dataRoot": state.data_root.display().to_string(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// 轮换指定通道 Key（另一通道不动），返回新 Key
#[tauri::command]
fn api_key_rotate(state: State<'_, AppState>, kind: String) -> Result<String, String> {
    let k = match kind.as_str() {
        "rest" => byte_translator_core::rest::KeyKind::Rest,
        "mcp" => byte_translator_core::rest::KeyKind::Mcp,
        other => return Err(format!("未知通道 {other}")),
    };
    byte_translator_core::rest::rotate_api_key(&state.data_root, k).map_err(|e| format!("{e:#}"))
}

/// 设置指定通道自定义 Key（非空校验）
#[tauri::command]
fn api_key_set(state: State<'_, AppState>, kind: String, key: String) -> Result<(), String> {
    let k = match kind.as_str() {
        "rest" => byte_translator_core::rest::KeyKind::Rest,
        "mcp" => byte_translator_core::rest::KeyKind::Mcp,
        other => return Err(format!("未知通道 {other}")),
    };
    byte_translator_core::rest::set_api_key(&state.data_root, k, &key).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn service_status(state: State<'_, AppState>) -> serde_json::Value {
    let rest = state.rest.lock().unwrap();
    let mcp = state.mcp.lock().unwrap();
    serde_json::json!({
        "rest": { "running": rest.running, "port": rest.port },
        "mcp": { "running": mcp.running, "port": mcp.port },
    })
}

#[tauri::command]
async fn service_toggle(
    state: State<'_, AppState>,
    kind: String,
    on: bool,
    port: Option<u16>,
) -> Result<serde_json::Value, String> {
    let is_rest = match kind.as_str() {
        "rest" => true,
        "mcp" => false,
        other => return Err(format!("未知服务类型 {other}")),
    };
    let slot: &std::sync::Mutex<ServiceHandle> = if is_rest { &state.rest } else { &state.mcp };

    if !on {
        let mut h = slot.lock().unwrap();
        if let Some(cancel) = h.cancel.take() {
            cancel.cancel();
        }
        h.running = false;
        return Ok(serde_json::json!({ "ok": true, "running": false }));
    }

    // 启动：锁内登记运行位；bind 失败立即回滚（不出现假运行）
    let port = port.unwrap_or(if is_rest { 8321 } else { 8324 });
    {
        let mut h = slot.lock().unwrap();
        if h.running {
            return Err("服务已在运行".into());
        }
        h.running = true;
        h.port = port;
    }
    let rollback = || {
        let mut h = slot.lock().unwrap();
        h.running = false;
        h.cancel = None;
    };

    let (keys, _) = byte_translator_core::rest::load_or_create_api_keys(&state.data_root)
        .map_err(|e| {
            rollback();
            format!("{e:#}")
        })?;
    let cancel = CancellationToken::new();
    {
        let mut h = slot.lock().unwrap();
        h.cancel = Some(cancel.clone());
    }
    let shutdown = async move { cancel.cancelled_owned().await };

    if is_rest {
        let cfg = byte_translator_core::rest::ServeConfig {
            host: "127.0.0.1".into(),
            port,
            api_key: keys.rest,
            allow_local_no_key: false,
        };
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .map_err(|e| {
                rollback();
                format!("绑定端口 {port} 失败：{e}")
            })?;
        let key: byte_translator_core::rest::SharedKey =
            std::sync::Arc::new(std::sync::RwLock::new(cfg.api_key.clone()));
        let services = state.services.clone();
        tokio::spawn(async move {
            let _ = byte_translator_core::rest::serve_listener(listener, &cfg, services, key, shutdown).await;
            tracing::info!("REST 服务已停止");
        });
    } else {
        if keys.mcp.trim().is_empty() {
            rollback();
            return Err("MCP Key 未配置".into());
        }
        let key: byte_translator_core::rest::SharedKey =
            std::sync::Arc::new(std::sync::RwLock::new(keys.mcp.clone()));
        let db = state.db.clone();
        tokio::spawn(async move {
            let _ = byte_translator_core::mcp::server::serve_http(db, "127.0.0.1", port, key, shutdown).await;
            tracing::info!("MCP 服务已停止");
        });
    }
    Ok(serde_json::json!({ "ok": true, "running": true, "port": port }))
}

pub fn run() {
    init_tracing();
    if let Err(e) = try_run() {
        eprintln!("启动失败：{e:#}");
    }
}

fn try_run() -> anyhow::Result<()> {
    let root = byte_translator_core::resolve_app_root()?;
    let data_root = root.join("data");
    let db_path = data_root.join("db").join("byte-translator.db");
    let db = Arc::new(Database::open(&db_path)?);
    let services = Arc::new(RestServices::new(db.clone()));

    tauri::Builder::default()
        .setup(move |app| {
            app.manage(AppState {
                services,
                db,
                data_root,
                rest: std::sync::Mutex::new(ServiceHandle::default()),
                mcp: std::sync::Mutex::new(ServiceHandle::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            translate,
            translate_llm_stream,
            history_list,
            wordbook_list,
            wordbook_add,
            wordbook_remove,
            provider_list,
            provider_set,
            provider_enable,
            provider_test,
            provider_fetch_models,
            api_keys,
            api_key_rotate,
            set_iciba_key,
            engines_list,
            engine_set_config,
            set_default_engine,
            engine_test,
            app_info,
            api_key_set,
            service_status,
            service_toggle,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("应用运行失败：{e}"))?;
    Ok(())
}

/// 日志写 stderr（stdout 是 IPC 信道，任何输出都会破坏通信）
fn init_tracing() {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_target(false));
    let _ = subscriber.try_init();
}
