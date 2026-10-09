//! byte-translator 领域层：翻译引擎、调度与本地数据。
//!
//! CLI 与桌面端共用本 crate。日志一律写 stderr——stdout 保留给
//! CLI 数据输出与 MCP stdio 协议信道，任何模块不得向 stdout 打日志。

use std::path::PathBuf;

use anyhow::Result;

pub mod ai;
pub mod db;
pub mod lang;
pub mod service;
pub mod translator;

#[cfg(feature = "server")]
pub mod rest;

#[cfg(feature = "mcp")]
pub mod mcp;

/// 数据根目录覆盖用的环境变量名
pub const HOME_ENV: &str = "CICBYTE_HOME";

/// 解析应用数据根目录：优先环境变量覆盖，否则 `~/.cicbyte/apps/byte-translator`
pub fn resolve_app_root() -> Result<PathBuf> {
    resolve_root_from(std::env::var(HOME_ENV).ok(), dirs::home_dir())
}

fn resolve_root_from(env_override: Option<String>, home: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(custom) = env_override {
        let custom = custom.trim();
        if !custom.is_empty() {
            return Ok(PathBuf::from(custom));
        }
    }
    let home = home.ok_or_else(|| anyhow::anyhow!("无法定位用户主目录"))?;
    Ok(home.join(".cicbyte").join("apps").join("byte-translator"))
}

/// 初始化日志：stderr 输出，级别由 RUST_LOG 过滤（缺省 info）。
/// 重复调用是 no-op（全局订阅者只装一次）。
pub fn init_tracing() {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_target(false));
    let _ = subscriber.try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> Option<PathBuf> {
        Some(PathBuf::from("/home tester"))
    }

    #[test]
    fn env_override_wins_when_nonempty() {
        let root = resolve_root_from(Some(" /data/custom ".into()), home()).unwrap();
        assert_eq!(root, PathBuf::from("/data/custom"));
    }

    #[test]
    fn blank_env_falls_back_to_home() {
        let root = resolve_root_from(Some("  ".into()), home()).unwrap();
        assert_eq!(root, PathBuf::from("/home tester/.cicbyte/apps/byte-translator"));
    }

    #[test]
    fn default_root_under_cicbyte_apps() {
        let root = resolve_root_from(None, home()).unwrap();
        assert_eq!(root, PathBuf::from("/home tester/.cicbyte/apps/byte-translator"));
    }

    #[test]
    fn missing_home_is_error() {
        assert!(resolve_root_from(None, None).is_err());
    }

}
