//! MCP server：rmcp 工具定义 + 双 transport（胶水层，业务在 service）。
//!
//! 工具集（Q-103 定稿，含生词本）：
//! - translate：翻译（自动调度/显式引擎）
//! - define：查词（词典形态，强制 iciba）
//! - history_search：历史检索
//! - wordbook_add / wordbook_list：生词本收藏与列表
//!
//! stdio 模式 stdout 是协议信道——一切提示与日志走 stderr。

pub mod server;
pub mod tools;
