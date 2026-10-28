<div align="center">

# byte-translator

**A local-first open-source translation toolkit**

One Rust domain core, four ways to use it: **CLI · Desktop · REST API · MCP**

**中文**: [README.md](README.md)

[![Rust](https://img.shields.io/badge/rust-stable-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>

## What It Solves

Everyday translation needs live in scattered tools: web dictionaries are disconnected from your terminal, word lookup and sentence translation have no shared entry point or history, and translation capability cannot be called directly from scripts or AI clients.

byte-translator brings it together in one local tool: **word lookup (dictionary) + sentence translation (LLM) + history & wordbook — all data stays local and is callable anywhere.**

## Table of Contents

- [Features](#-features)
- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Usage](#-usage)
- [Configuration](#-configuration)
- [Development](#-development)
- [Documentation](#-documentation)
- [Contributing](#-contributing)
- [License](#-license)

## ✨ Features

- 🔀 **Smart dispatch** — single words automatically go to the dictionary (phonetics / part-of-speech / definitions / examples), sentences to the LLM; dictionary misses fall back automatically, or pick an engine explicitly
- 🧩 **Eight translation engines** — keyless (Google / MyMemory), major commercial APIs (DeepL / Baidu / Youdao / Tencent), dictionary (iCIBA), and LLM (OpenAI-compatible / DeepSeek / Zhipu / Anthropic / Gemini / Ollama…); multi-credential rotation supported
- 🖱️ **Selection translation** — system-wide hotkey: select text and translate (`Ctrl+Alt+D`), with the result popup following your cursor
- 🌍 **Full multilingual support** — auto-detects and translates between Chinese and English by default; `--from/--to` for any language pair (LLM engines are natively multilingual)
- 📚 **History & wordbook** — every translation is recorded in local SQLite, searchable by keyword / engine / language; wordbook supports idempotent bookmarking with notes
- ⚡ **Four forms, one core** — the `btr` CLI (`--json` for scripts), desktop app (Tauri), REST API (SSE streaming + Swagger), and MCP server (stdio / HTTP for Claude Desktop / Cursor)
- 🔐 **Local-first** — all data under `~/.cicbyte/apps/byte-translator/` (`CICBYTE_HOME` overridable); API keys stay local and are encrypted at rest; independent REST/MCP keys + DNS rebinding protection
- 🗄️ **Dictionary cache** — dictionary results are cached locally; LLM results are never cached

## 📦 Installation

Prerequisites: Rust stable. Building the desktop app additionally requires Node.js 20+ and pnpm.

```bash
# Build from source (all forms)
git clone https://github.com/cicbyte/byte-translator.git
cd byte-translator
cargo build --release
# Binary: target/release/btr (btr.exe on Windows)

# Or install the CLI only
cargo install --path apps/cli
```

Prebuilt artifacts (CLI archives per platform, Windows desktop installers) will be published on [GitHub Releases](https://github.com/cicbyte/byte-translator/releases).

## 🚀 Quick Start

```bash
# Translate: words go to the dictionary, sentences to the LLM
btr love
btr 我想回家了

# Configure an LLM provider before first use (DeepSeek as an example)
btr provider set deepseek --base-url https://api.deepseek.com \
    --api-key sk-xxx --model deepseek-chat --enable

# Explicit control
btr -e llm love                  # force the LLM engine
btr --from ja --to en こんにちは    # explicit language pair

# History & wordbook
btr history list --keyword love --engine llm
btr wordbook add serendipity --note "n. 意外发现美好事物的能力"
```

## 📖 Usage

### CLI

```bash
btr --json hello                  # structured output (stable contract for AI & scripts)
echo "long text" | btr            # read from stdin
btr -- history                    # translate a reserved word literally
btr engine                        # list all translation engines
```

| Command | Description |
| --- | --- |
| `btr <text>` | Translate (auto dispatch; `-e` picks an engine, `--from/--to` a language pair) |
| `btr engine` | Translation engine list (`--json` for structured output) |
| `btr history list / recent` | Search translation history |
| `btr wordbook add / list / remove` | Wordbook management |
| `btr provider set / list / test / …` | AI provider management |
| `btr serve` | Start the REST API server |
| `btr mcp` | Start the MCP server (stdio / http) |
| `--json` / `--remote` / `--app-root` | Global options: structured output / remote mode / data directory override |

See the [CLI manual](docs/cli.md) (Chinese) or `btr --help` for full options.

### Selection Translate

Available system-wide while the desktop app is running: select any text and press `Ctrl+Alt+D` to get a translation popup next to your cursor. `Alt+T` shows/hides the main window; press Esc or click elsewhere to dismiss the popup.

### Desktop App

Streaming translation (interruptible), history, wordbook, translation engine & AI provider settings, REST/MCP service management with key controls, themes — plus system-wide selection translation.

```bash
cd apps/desktop
pnpm install
pnpm tauri dev      # run in development; use pnpm tauri build to package
```

### REST API

```bash
btr serve                         # default http://127.0.0.1:8321
# An API key is generated on first run (printed at startup; stored encrypted in <data>/config/api.json)

curl -X POST http://127.0.0.1:8321/api/v1/translate \
  -H "X-API-Key: <KEY>" -H "Content-Type: application/json" \
  -d '{"text": "hello", "stream": true}'   # stream:true for SSE

# Interactive docs
open http://127.0.0.1:8321/api/v1/docs
```

### MCP (direct AI client integration)

Claude Desktop / Cursor configuration (stdio):

```json
{
  "mcpServers": {
    "btr": { "command": "C:/path/to/btr.exe", "args": ["mcp"] }
  }
}
```

HTTP mode (multiple clients): `btr mcp --transport http` → `http://127.0.0.1:8324/mcp`

Tool set: `translate` / `define` / `history_search` / `wordbook_add` / `wordbook_list`

### Multi-Machine (Single Data Source)

```bash
# Main machine
btr serve --host 0.0.0.0          # non-loopback binding requires an API key

# Other machines (connect without local data)
btr --remote http://HOST-IP:8321 --api-key <KEY> hello
# BTR_REMOTE / BTR_API_KEY environment variables work too
```

## ⚙️ Configuration

Data root: `~/.cicbyte/apps/byte-translator/` (overridable via `CICBYTE_HOME` or `--app-root`).

| Path | Contents |
| --- | --- |
| `data/db/` | Translation history, wordbook, dictionary cache (SQLite) |
| `data/config/api.json` | Independent REST / MCP API keys (encrypted at rest) |

| Environment variable | Description | Default |
| --- | --- | --- |
| `CICBYTE_HOME` | Override the data root | `~/.cicbyte/apps/byte-translator/` |
| `ICIBA_API_KEY` | iCIBA dictionary API key | built-in public key |
| `BTR_REMOTE` | REST endpoint for remote mode | — |
| `BTR_API_KEY` | API key for remote mode | — |

## 🧪 Development

```bash
cargo test --workspace                 # unit tests (incl. REST/MCP features)
cd tests && python -m pytest           # CLI end-to-end tests (pytest + YAML flows)
```

```
crates/core        domain layer: engines / dispatch / data / services (shared by CLI & desktop, no UI deps)
apps/cli           the btr command line (translate / manage / serve / mcp)
apps/desktop       desktop app (Tauri + Vue)
tests/             end-to-end tests (YAML flows + REST/MCP suites)
```

## 📚 Documentation

- [CLI manual](docs/cli.md) (Chinese)
- [REST API](docs/rest.md) (Chinese)
- [MCP integration guide](docs/mcp.md) (Chinese)

## 🤝 Contributing

Issues and pull requests are welcome. Commits follow Conventional Commits; please run the unit and end-to-end tests locally before submitting.

## 📄 License

[MIT](LICENSE) © 2026 cicbyte
