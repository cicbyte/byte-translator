<div align="center">

# byte-translator

**本地优先的开源翻译工具**

一套 Rust 领域核心，四种使用形态：**CLI · 桌面端 · REST API · MCP 服务**

**English**: [README_EN.md](README_EN.md)

[![Rust](https://img.shields.io/badge/rust-stable-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

## 它解决什么

日常翻译需求散落在割裂的工具里：网页词典与终端脱节、查词与翻句没有统一入口与历史、翻译能力无法被脚本和 AI 客户端直接调用。

byte-translator 用一个本地工具统一解决：**查词（词典形态）+ 翻句（LLM 形态）+ 历史与生词本，全部数据在本地，随处可调。**

## 目录

- [功能特性](#-功能特性)
- [安装](#-安装)
- [快速开始](#-快速开始)
- [使用方法](#-使用方法)
- [配置](#-配置)
- [开发](#-开发)
- [文档](#-文档)
- [参与贡献](#-参与贡献)
- [开源许可证](#-开源许可证)

## ✨ 功能特性

- 🔀 **智能调度**——单词自动查词典（音标/词性/释义/例句），句子自动走 LLM；词典未命中自动降级，也可显式指定引擎
- 🧩 **八大翻译引擎**——免 Key 即用（Google / MyMemory）、商用主流（DeepL / 百度 / 有道 / 腾讯）、词典（金山词霸）、LLM（OpenAI 兼容 / DeepSeek / 智谱 / Anthropic / Gemini / Ollama…）；支持多组凭据自动轮换
- 🖱️ **划词翻译**——系统级快捷键选中即译（`Ctrl+Alt+D`），译文小窗跟随鼠标弹出
- 🌍 **完整多语言**——默认自动检测中英互译；`--from/--to` 显式指定任意语言对（LLM 引擎天然多语言）
- 📚 **历史与生词本**——每次翻译自动留存本地 SQLite，可按关键词/引擎/语言检索；生词本幂等收藏加备注
- ⚡ **四种形态同一核心**——`btr` 命令行（`--json` 供脚本）、桌面端（Tauri）、REST API（SSE 流式 + Swagger）、MCP 服务（stdio / HTTP，Claude Desktop / Cursor 直连）
- 🔐 **本地优先**——数据全在 `~/.cicbyte/apps/byte-translator/`（`CICBYTE_HOME` 可覆盖）；API Key 仅存本地、加密落盘；REST/MCP 双独立 Key + 防 DNS Rebinding
- 🗄️ **词典缓存**——词典结果本地缓存，LLM 结果不缓存

## 📦 安装

前置：Rust stable。桌面端构建另需 Node.js 20+ 与 pnpm。

```bash
# 源码构建（全部形态）
git clone https://github.com/cicbyte/byte-translator.git
cd byte-translator
cargo build --release
# 可执行文件：target/release/btr（Windows 为 btr.exe）

# 或仅安装 CLI
cargo install --path apps/cli
```

预编译产物（各平台 CLI 压缩包、Windows 桌面安装包）将随 [GitHub Releases](https://github.com/cicbyte/byte-translator/releases) 发布。

## 🚀 快速开始

```bash
# 翻译：单词自动查词典，句子自动走 LLM
btr love
btr 我想回家了

# 首次使用 LLM 前配置供应商（以 DeepSeek 为例）
btr provider set deepseek --base-url https://api.deepseek.com \
    --api-key sk-xxx --model deepseek-chat --enable

# 显式控制
btr -e llm love                  # 强制 LLM
btr --from ja --to en こんにちは    # 指定语言对

# 历史与生词本
btr history list --keyword love --engine llm
btr wordbook add serendipity --note "n. 意外发现美好事物的能力"
```

## 📖 使用方法

### CLI

```bash
btr --json hello                  # 结构化输出（稳定契约，供 AI 与脚本）
echo "long text" | btr            # stdin 读入
btr -- history                    # 翻译保留字本身
btr engine                        # 查看全部翻译引擎
```

| 命令 | 说明 |
| --- | --- |
| `btr <文本>` | 翻译（自动调度，`-e` 指定引擎、`--from/--to` 指定语言对） |
| `btr engine` | 翻译引擎清单（`--json` 结构化） |
| `btr history list / recent` | 翻译历史检索 |
| `btr wordbook add / list / remove` | 生词本管理 |
| `btr provider set / list / test / …` | AI 供应商管理 |
| `btr serve` | 启动 REST API 服务 |
| `btr mcp` | 启动 MCP 服务（stdio / http） |
| `--json` / `--remote` / `--app-root` | 全局选项：结构化输出 / 远程模式 / 覆盖数据目录 |

完整参数见 [CLI 手册](docs/cli.md) 或 `btr --help`。

### 划词翻译

桌面端运行时全局可用：选中任意文本按 `Ctrl+Alt+D`，译文小窗跟随鼠标弹出；`Alt+T` 呼出/隐藏主窗口，Esc 或点击别处关闭小窗。

### 桌面端

翻译（流式输出、可中断）、历史、生词本、翻译引擎与 AI 供应商设置、REST/MCP 服务启停与密钥管理、主题外观，另带系统级划词翻译。

```bash
cd apps/desktop
pnpm install
pnpm tauri dev      # 开发运行；打包用 pnpm tauri build
```

### REST API

```bash
btr serve                         # 默认 http://127.0.0.1:8321
# 首次自动生成 API Key（启动时打印；加密存 <data>/config/api.json）

curl -X POST http://127.0.0.1:8321/api/v1/translate \
  -H "X-API-Key: <KEY>" -H "Content-Type: application/json" \
  -d '{"text": "hello", "stream": true}'   # stream:true 走 SSE

# 交互式文档
open http://127.0.0.1:8321/api/v1/docs
```

### MCP（AI 客户端直连）

Claude Desktop / Cursor 配置（stdio）：

```json
{
  "mcpServers": {
    "btr": { "command": "C:/path/to/btr.exe", "args": ["mcp"] }
  }
}
```

HTTP 模式（多客户端）：`btr mcp --transport http` → `http://127.0.0.1:8324/mcp`

工具集：`translate` / `define` / `history_search` / `wordbook_add` / `wordbook_list`

### 多机（单一数据源）

```bash
# 主力机
btr serve --host 0.0.0.0          # 非回环绑定强制 API Key

# 其它机器（免本地数据直连）
btr --remote http://主力机IP:8321 --api-key <KEY> hello
# BTR_REMOTE / BTR_API_KEY 环境变量亦可
```

## ⚙️ 配置

数据根目录：`~/.cicbyte/apps/byte-translator/`（可用 `CICBYTE_HOME` 或 `--app-root` 覆盖）。

| 路径 | 内容 |
| --- | --- |
| `data/db/` | 翻译历史、生词本、词典缓存（SQLite） |
| `data/config/api.json` | REST / MCP 双独立 API Key（加密落盘） |

| 环境变量 | 说明 | 默认值 |
| --- | --- | --- |
| `CICBYTE_HOME` | 覆盖数据根目录 | `~/.cicbyte/apps/byte-translator/` |
| `ICIBA_API_KEY` | 金山词霸接口 Key | 内置公共 Key |
| `BTR_REMOTE` | 远程模式 REST 地址 | — |
| `BTR_API_KEY` | 远程模式 API Key | — |

## 🧪 开发

```bash
cargo test --workspace                 # 单测（含 REST/MCP feature）
cd tests && python -m pytest           # CLI 端到端（pytest + YAML 用例）
```

```
crates/core        领域层：引擎/调度/数据/服务（CLI 与桌面端共用，无 UI 依赖）
apps/cli           btr 命令行（翻译/管理/serve/mcp）
apps/desktop       桌面端（Tauri + Vue）
tests/             端到端测试（YAML 用例 + REST/MCP 专项）
```

## 📚 文档

- [CLI 手册](docs/cli.md)
- [REST API](docs/rest.md)
- [MCP 接入指南](docs/mcp.md)

## 🤝 参与贡献

欢迎 Issue 与 Pull Request。提交遵循 conventional commits；合入前请本地跑通单测与端到端用例。

## 📄 开源许可证

[MIT](LICENSE) © 2026 cicbyte
