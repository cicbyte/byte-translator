//! btr——byte-translator 命令行。
//!
//! 人类可读输出走 stdout（`--json` 时为结构化 JSON），错误走 stderr。
//! `--remote` 直连远端 REST（远程模式免本地数据）。

mod backend;
mod render;

use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use backend::Backend;
use byte_translator_core::db::Database;
use byte_translator_core::service::history::HistoryQuery;
use byte_translator_core::service::provider::ProviderInput;
use byte_translator_core::service::translate::EngineChoice;
use byte_translator_core::translator::TranslateRequest;

/// 本地优先的翻译命令行：词查词典、句走 LLM，历史与生词本在本地
#[derive(Parser)]
#[command(
    name = "btr",
    version,
    about,
    help_template = "{about-with-newline}\n{usage-heading} {usage}\n\n{options}{after-help}",
    after_help = "示例
  btr hello                     翻译单词（自动路由词典）
  btr 我想回家了                翻译句子（自动路由 LLM）
  btr -e llm love               强制 LLM
  btr --from ja --to en こんにちは   指定语言对
  echo 'long text' | btr        从 stdin 读入
  btr -- history                翻译保留字本身（-- 后按文本处理）

管理
  btr history list --keyword love --engine llm    历史检索
  btr wordbook add love --note 'n. 爱'           收藏生词
  btr provider set deepseek --base-url https://api.deepseek.com --api-key sk-xxx --model deepseek-chat --enable
  btr provider test deepseek                     连通测试（不耗 token）

服务
  btr serve                     REST API（默认 127.0.0.1:8321）
  btr mcp                       MCP stdio（Claude Desktop/Cursor 直连）
  btr mcp --transport http      MCP HTTP（默认 :8324/mcp）

多机（单一数据源）
  主力机      btr serve --host 0.0.0.0（自动生成 API Key，见 <data>/config/api.json）
  其它机器    btr --remote http://主力机IP:8321 --api-key <KEY> hello
  （BTR_REMOTE / BTR_API_KEY 环境变量亦可；供应商管理在主力机本地执行）

提示
  --json        结构化输出（AI/脚本消费的稳定契约）
  -e/--engine   iciba 或 llm；缺省自动调度（词典优先、失败降级 LLM）
  --app-root    覆盖数据目录（默认 ~/.cicbyte/byte-translator）
"
)]
struct Cli {
    /// 输出 JSON（结构化，供 AI / 脚本消费）
    #[arg(long, global = true)]
    json: bool,
    /// 应用数据根目录（默认 ~/.cicbyte/byte-translator）
    #[arg(long, global = true, value_name = "DIR")]
    app_root: Option<PathBuf>,
    /// 远程模式：直连 btr serve 挂起的 REST 端点（环境变量 BTR_REMOTE）
    #[arg(long, global = true, value_name = "URL", env = "BTR_REMOTE")]
    remote: Option<String>,
    /// 远程模式的 API Key（环境变量 BTR_API_KEY）
    #[arg(long, global = true, value_name = "KEY", env = "BTR_API_KEY")]
    api_key: Option<String>,
    /// 源语言（ISO 639 代码，缺省自动检测）
    #[arg(long = "from")]
    from: Option<String>,
    /// 目标语言（缺省自动检测：中英互译）
    #[arg(long = "to")]
    to: Option<String>,
    /// 指定引擎（iciba / llm；缺省自动调度）
    #[arg(short = 'e', long)]
    engine: Option<String>,
    /// 要翻译的文本（省略则读 stdin）
    text: Option<String>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// 翻译历史（自动留存，可检索）
    History {
        #[command(subcommand)]
        cmd: HistoryCmd,
    },
    /// 生词本（收藏/备注/列表/删除）
    Wordbook {
        #[command(subcommand)]
        cmd: WordbookCmd,
    },
    /// AI 供应商管理（仅本地）
    Provider {
        #[command(subcommand)]
        cmd: ProviderCmd,
    },
    /// 引擎列表
    Engine,
    /// 启动 REST API 服务（默认 127.0.0.1:8321；非回环绑定强制 API Key）
    Serve {
        /// 绑定地址（默认 127.0.0.1；供局域网访问填 0.0.0.0——非回环强制要求 API Key）
        #[arg(long)]
        host: Option<String>,
        /// 端口（默认 8321）
        #[arg(long, default_value_t = 8321)]
        port: u16,
        /// 显式指定 API Key（优先于配置；缺省读取/自动生成 <data>/config/api.json）
        #[arg(long)]
        api_key: Option<String>,
        /// 回环连接免 key（仅本机调试；非回环连接始终要求 key）
        #[arg(long, default_value_t = false)]
        allow_no_key: bool,
    },
    /// 启动 MCP 服务器（stdio 供 Claude Desktop/Cursor 等接入；http 供多客户端）
    Mcp {
        /// 传输方式：stdio（默认，AI 客户端直接拉起）/ http（/mcp 端点）
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// http 模式绑定地址（默认 127.0.0.1）
        #[arg(long)]
        host: Option<String>,
        /// http 模式端口（默认 8324；与 REST 的 8321 错开）
        #[arg(long, default_value_t = 8324)]
        port: u16,
        /// http 模式 API Key（缺省读取/自动生成 <data>/config/api.json，与 REST 各自独立）
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand)]
enum HistoryCmd {
    /// 检索历史（条件可组合）
    List {
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long)]
        engine: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: i64,
        #[arg(long, default_value_t = 20)]
        size: i64,
    },
    /// 最近 N 条
    Recent {
        #[arg(long, default_value_t = 10)]
        limit: i64,
    },
}

#[derive(Subcommand)]
enum WordbookCmd {
    /// 收藏生词（幂等：重复收藏更新备注）
    Add {
        text: String,
        #[arg(long)]
        note: Option<String>,
    },
    /// 生词本列表
    List {
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long, default_value_t = 1)]
        page: i64,
        #[arg(long, default_value_t = 20)]
        size: i64,
    },
    /// 删除生词（按 id，列表中查看）
    Remove { id: i64 },
}

#[derive(Subcommand)]
enum ProviderCmd {
    /// 供应商列表（Key 不回显）
    List,
    /// 新增/更新供应商（api_key 留空=保持原值；--enable 同时启用）
    Set {
        /// 供应商标识（openai/deepseek/zhipu/anthropic/gemini/ollama…，决定接入协议）
        key: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_name = "URL")]
        base_url: Option<String>,
        #[arg(long, value_name = "KEY")]
        api_key: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        enable: bool,
    },
    /// 启用供应商（单选：其余自动停用）
    Enable { key: String },
    /// 停用全部供应商
    Disable,
    /// 连通测试（轻量探测，不耗 token）
    Test { key: String },
    /// 删除供应商
    Delete { key: String },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        if cli.json {
            println!("{}", render::error_json(&format!("{e:#}")));
        } else {
            eprintln!("错误：{e:#}");
        }
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    byte_translator_core::init_tracing();

    let root = match &cli.app_root {
        Some(p) => p.clone(),
        None => byte_translator_core::resolve_app_root()?,
    };

    // 服务命令不需要 Backend
    match &cli.cmd {
        Some(Cmd::Serve { host, port, api_key, allow_no_key }) => {
            return serve_cmd(&root, host.as_deref(), *port, api_key.as_deref(), *allow_no_key);
        }
        Some(Cmd::Mcp { transport, host, port, api_key }) => {
            return mcp_cmd(&root, transport, host.as_deref(), *port, api_key.as_deref());
        }
        _ => {}
    }

    let be = build_backend(cli, &root)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("构建异步运行时失败")?;
    rt.block_on(async_run(cli, &be))
}

/// 组装后端：--remote 走 REST，否则本地打开数据库
fn build_backend(cli: &Cli, root: &std::path::Path) -> Result<Backend> {
    if let Some(url) = &cli.remote {
        let key = cli.api_key.clone().unwrap_or_default();
        if key.is_empty() {
            return Err(anyhow!("远程模式需要 --api-key（或环境变量 BTR_API_KEY）"));
        }
        return Ok(Backend::remote(url, &key));
    }
    let db_path = root.join("data").join("db").join("byte-translator.db");
    let db = Database::open(&db_path)
        .with_context(|| format!("打开数据库失败（{}）", db_path.display()))?;
    Ok(Backend::local(Arc::new(db)))
}

async fn async_run(cli: &Cli, be: &Backend) -> Result<()> {
    match &cli.cmd {
        None => translate_cmd(cli, be).await,
        Some(Cmd::History { cmd }) => history_cmd(cli, be, cmd).await,
        Some(Cmd::Wordbook { cmd }) => wordbook_cmd(cli, be, cmd).await,
        Some(Cmd::Provider { cmd }) => provider_cmd(cli, be, cmd).await,
        Some(Cmd::Engine) => {
            let engines = serde_json::json!([
                { "engine": "iciba", "kind": "词典", "langs": "zh<->en",
                  "desc": "音标/词性/释义/例句；单词自动优先，未命中自动降级" },
                { "engine": "llm", "kind": "文本", "langs": "任意",
                  "desc": "多供应商 LLM；句子与非中英语言对自动路由" },
            ]);
            emit_list(
                cli,
                vec![
                    "iciba\t词典\tzh<->en\t音标/词性/释义/例句；单词自动优先".to_string(),
                    "llm\t文本\t任意\t多供应商 LLM；句子与非中英自动路由".to_string(),
                ],
                engines,
            );
            Ok(())
        }
        _ => unreachable!("serve/mcp 已在前置分支处理"),
    }
}

// ===== 翻译主命令 =====

async fn translate_cmd(cli: &Cli, be: &Backend) -> Result<()> {
    let text = match &cli.text {
        Some(t) if !t.trim().is_empty() => t.clone(),
        Some(_) => return Err(anyhow!("文本不能为空")),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("读取 stdin 失败")?;
            let buf = buf.trim().to_string();
            if buf.is_empty() {
                return Err(anyhow!("未提供文本（参数或 stdin）"));
            }
            buf
        }
    };
    let choice = match &cli.engine {
        Some(e) => EngineChoice::Named(e.clone()),
        None => EngineChoice::Auto,
    };
    let req = TranslateRequest {
        from: cli.from.as_deref().map(byte_translator_core::lang::normalize_lang),
        to: cli.to.as_deref().map(byte_translator_core::lang::normalize_lang),
        text,
    };
    let outcome = be.translate(&req, &choice).await?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&render::outcome_json(&outcome))?);
    } else {
        render::render(&outcome);
    }
    Ok(())
}

// ===== 历史 =====

async fn history_cmd(cli: &Cli, be: &Backend, cmd: &HistoryCmd) -> Result<()> {
    let q = match cmd {
        HistoryCmd::List { keyword, engine, from, to, page, size } => HistoryQuery {
            keyword: keyword.clone().unwrap_or_default(),
            engine: engine.clone().unwrap_or_default(),
            from: from.clone().unwrap_or_default(),
            to: to.clone().unwrap_or_default(),
            page: *page,
            size: *size,
        },
        HistoryCmd::Recent { limit } => HistoryQuery {
            page: 1,
            size: *limit,
            ..Default::default()
        },
    };
    let v = be.history_list(&q).await?;
    let rows: Vec<String> = v["list"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|h| {
                    format!(
                        "{}\t[{}] {}\t{}->{}\t{}",
                        h["id"], h["engine"], h["text"], h["from_lang"], h["to_lang"],
                        h["created_at"]
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let mut out = serde_json::json!({ "total": v["total"], "list": v["list"] });
    out["page"] = serde_json::json!(q.page);
    out["size"] = serde_json::json!(q.size);
    emit_list(cli, rows, out);
    Ok(())
}

// ===== 生词本 =====

async fn wordbook_cmd(cli: &Cli, be: &Backend, cmd: &WordbookCmd) -> Result<()> {
    match cmd {
        WordbookCmd::Add { text, note } => {
            let v = be.wordbook_add(text, note.as_deref().unwrap_or("")).await?;
            emit(cli, "已收藏".into(), v);
        }
        WordbookCmd::List { keyword, page, size } => {
            let v = be
                .wordbook_list(keyword.as_deref().unwrap_or(""), *page, *size)
                .await?;
            let rows: Vec<String> = v["list"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|w| {
                            let note = w["note"].as_str().unwrap_or("");
                            if note.is_empty() {
                                format!("{}\t{}", w["id"], w["text"])
                            } else {
                                format!("{}\t{}\t{}", w["id"], w["text"], note)
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            emit_list(cli, rows, v);
        }
        WordbookCmd::Remove { id } => {
            let v = be.wordbook_remove(*id).await?;
            emit(cli, format!("已删除 id={id}"), v);
        }
    }
    Ok(())
}

// ===== 供应商 =====

async fn provider_cmd(cli: &Cli, be: &Backend, cmd: &ProviderCmd) -> Result<()> {
    match cmd {
        ProviderCmd::List => {
            let v = be.provider_list().await?;
            let rows: Vec<String> = v["list"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|p| {
                            format!(
                                "{}\t{}{}\t{}\t{}",
                                p["provider_key"],
                                if p["is_enabled"].as_bool() == Some(true) { "*" } else { " " },
                                p["name"].as_str().unwrap_or(""),
                                p["base_url"].as_str().unwrap_or(""),
                                p["model"].as_str().unwrap_or("")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            emit_list(cli, rows, v);
        }
        ProviderCmd::Set { key, name, base_url, api_key, model, enable } => {
            let input = ProviderInput {
                name: name.clone().unwrap_or_else(|| key.clone()),
                provider_key: key.clone(),
                base_url: base_url.clone().unwrap_or_default(),
                api_key: api_key.clone().unwrap_or_default(),
                model: model.clone().unwrap_or_default(),
                max_tokens: 4096,
                temperature: 0.3,
            };
            let v = be.provider_set(&input, Some(*enable)).await?;
            emit(cli, format!("已保存供应商 {key}"), v);
        }
        ProviderCmd::Enable { key } => {
            let v = be.provider_enable(key, true).await?;
            emit(cli, format!("已启用 {key}（其余自动停用）"), v);
        }
        ProviderCmd::Disable => {
            let v = be.provider_enable("", false).await?;
            let _ = v;
            emit(cli, "已停用全部供应商".into(), serde_json::json!({ "ok": true, "enabled": false }));
        }
        ProviderCmd::Test { key } => {
            let v = be.provider_test(key).await?;
            emit(cli, format!("{key} 连通正常"), v);
        }
        ProviderCmd::Delete { key } => {
            let v = be.provider_delete(key).await?;
            emit(cli, format!("已删除 {key}"), v);
        }
    }
    Ok(())
}

// ===== 输出辅助 =====

fn emit(cli: &Cli, human: String, json: serde_json::Value) {
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&json).expect("json 序列化"));
    } else {
        println!("{human}");
    }
}

fn emit_list(cli: &Cli, human_rows: Vec<String>, json: serde_json::Value) {
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&json).expect("json 序列化"));
    } else if human_rows.is_empty() {
        println!("（空）");
    } else {
        println!("{}", human_rows.join("\n"));
    }
}

// ===== REST serve =====

fn serve_cmd(
    root: &std::path::Path,
    host: Option<&str>,
    port: u16,
    api_key: Option<&str>,
    allow_no_key: bool,
) -> Result<()> {
    use std::sync::Arc;

    let host = host.unwrap_or("127.0.0.1");
    let (key, newly_created) = match api_key {
        Some(k) => (k.trim().to_string(), None),
        None => {
            let (keys, created) =
                byte_translator_core::rest::load_or_create_api_keys(&root.join("data"))?;
            let created = created.map(|_| keys.rest.clone());
            (keys.rest, created)
        }
    };
    let cfg = byte_translator_core::rest::ServeConfig {
        host: host.to_string(),
        port,
        api_key: key,
        allow_local_no_key: allow_no_key,
    };
    let bind = cfg.validated_bind_addr()?;
    println!("btr REST 服务");
    println!("  监听      http://{bind}");
    println!("  数据目录  {}", root.display());
    match newly_created {
        Some(k) => {
            println!("  API Key   {k}  （首次自动生成，已存 config/api.json，请妥善保存）");
        }
        None => {
            println!("  API Key   见 {} 的 config/api.json（或 --api-key 覆盖）", root.display());
        }
    }
    println!("  文档      http://{bind}/api/v1/docs");
    println!("  Ctrl+C 停止");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("构建 tokio 运行时失败")?;
    rt.block_on(async {
        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
            println!("\n收到停止信号，正在优雅关闭…");
        };
        let db_path = root.join("data").join("db").join("byte-translator.db");
        let db = Database::open(&db_path)
            .with_context(|| format!("打开数据库失败（{}）", db_path.display()))?;
        let services = Arc::new(byte_translator_core::rest::RestServices::new(Arc::new(db)));
        let key: byte_translator_core::rest::SharedKey =
            Arc::new(std::sync::RwLock::new(cfg.api_key.clone()));
        byte_translator_core::rest::serve(&cfg, services, key, shutdown).await
    })
}

// ===== MCP serve =====

fn mcp_cmd(
    root: &std::path::Path,
    transport: &str,
    host: Option<&str>,
    port: u16,
    api_key: Option<&str>,
) -> Result<()> {
    use std::sync::Arc;

    let transport = transport.to_ascii_lowercase();
    if transport != "stdio" && transport != "http" {
        return Err(anyhow!("--transport 仅支持 stdio 或 http（收到 {transport}）"));
    }
    let is_stdio = transport == "stdio";

    let db_path = root.join("data").join("db").join("byte-translator.db");
    let db = Database::open(&db_path)
        .with_context(|| format!("打开数据库失败（{}）", db_path.display()))?;
    let db = Arc::new(db);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("构建 tokio 运行时失败")?;
    rt.block_on(async {
        if is_stdio {
            eprintln!("btr MCP stdio 服务已启动（stdout 为协议信道，日志见 stderr）");
            return byte_translator_core::mcp::server::serve_stdio(db).await;
        }
        let host = host.unwrap_or("127.0.0.1");
        let (key, newly) = match api_key {
            Some(k) => (k.trim().to_string(), None),
            None => {
                let (keys, created) =
                    byte_translator_core::rest::load_or_create_api_keys(&root.join("data"))?;
                let created = created.map(|_| keys.mcp.clone());
                (keys.mcp, created)
            }
        };
        if key.is_empty() {
            return Err(anyhow!("MCP http 模式必须配置 API Key（--api-key 或 config/api.json）"));
        }
        if let Some(k) = newly {
            eprintln!("MCP API Key 首次生成：{k}（已存 config/api.json，与 REST 各自独立）");
        }
        println!("btr MCP HTTP 服务");
        println!("  端点      http://{host}:{port}/mcp");
        println!("  Ctrl+C 停止");
        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
            println!("\n收到停止信号，正在优雅关闭…");
        };
        let key_handle: byte_translator_core::rest::SharedKey =
            Arc::new(std::sync::RwLock::new(key.clone()));
        byte_translator_core::mcp::server::serve_http(db, host, port, key_handle, shutdown).await
    })
}
