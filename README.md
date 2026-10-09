<div align="center">

# byte-translator

**本地优先、AI 原生的开源翻译工具**

一套 Rust 领域核心，四种使用形态：**CLI · 桌面端 · REST API · MCP 服务**

[![Rust](https://img.shields.io/badge/rust-stable-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

## 它解决什么

日常翻译需求散落在割裂的工具里：网页词典与终端脱节、查词与翻句没有统一入口与历史、翻译能力无法被脚本和 AI 客户端直接调用。

byte-translator 用一个本地工具统一解决：**查词（词典形态）+ 翻句（LLM 形态）+ 历史与生词本，全部数据在本地，随处可调。**

## 特性

- 🔀 **智能调度**——单词自动查词典（音标/词性/释义/例句），句子自动走 LLM；词典未命中自动降级，也可显式指定引擎
- 🧩 **插件式多引擎**——内置金山词霸（iciba）+ LLM（OpenAI 兼容 / DeepSeek / 智谱 / Anthropic / Gemini / Ollama…），引擎接口开放扩展
- 🌍 **完整多语言**——默认自动检测中英互译；`--from/--to` 显式指定任意语言对（LLM 引擎天然多语言）
- 📚 **历史与生词本**——每次翻译自动留存本地 SQLite，可按关键词/引擎/语言检索；生词本幂等收藏加备注
- ⚡ **四种形态同一核心**——`btr` 命令行（`--json` 供脚本）、REST API（SSE 流式 + Swagger）、MCP 服务（stdio / HTTP，Claude Desktop / Cursor 直连）；桌面端规划中
- 🔐 **本地优先**——数据全在 `~/.cicbyte/byte-translator/`（`CICBYTE_HOME` 可覆盖）；API Key 仅存本地不回显；REST/MCP 双独立 Key + 防 DNS Rebinding
- 🗄️ **词典缓存**——词典结果本地缓存，LLM 结果不缓存

## 安装

需前置：Rust 1.75+（推荐 stable）。

```bash
git clone https://github.com/cicbyte/byte-translator.git
cd byte-translator
cargo build --release
# 可执行文件：target/release/btr（Windows 为 btr.exe）
```

## 快速开始

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

## 四种形态

### CLI（`--json` 供 AI 与脚本）

```bash
btr --json hello                  # 结构化输出（稳定契约）
echo "long text" | btr            # stdin 读入
btr -- history                    # 翻译保留字本身
```

### REST API

```bash
btr serve                         # 默认 http://127.0.0.1:8321
# 首次自动生成 API Key（<data>/config/api.json）

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

## 文档

- [CLI 手册](docs/cli.md)
- [REST API](docs/rest.md)
- [MCP 接入指南](docs/mcp.md)

## 开发

```bash
cargo test --workspace                 # 单测（含 REST/MCP feature）
cd tests && python -m pytest           # CLI 端到端（pytest + YAML 用例）
```

结构：

```
crates/core      领域层：引擎/调度/数据/服务（CLI 与桌面端共用，无 UI 依赖）
apps/cli         btr 命令行（翻译/管理/serve/mcp）
tests/           端到端测试（YAML 用例 + REST/MCP 专项）
```

## License

[MIT](LICENSE)
