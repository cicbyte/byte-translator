# MCP 接入指南

`btr mcp [--transport stdio|http] [--host H] [--port P] [--api-key K]`

## 工具集

| 工具 | 参数 | 说明 |
|------|------|------|
| `translate` | `text`（必填）、`from?`、`to?`、`engine?` | 翻译：词查词典、句走 LLM，未命中自动降级 |
| `define` | `word`（必填） | 词典释义（音标/词性/例句，仅中英） |
| `history_search` | `keyword?`、`limit?`（默认 10） | 检索本地翻译历史 |
| `wordbook_add` | `text`（必填）、`note?` | 收藏生词（幂等） |
| `wordbook_list` | `keyword?`、`limit?`（默认 20） | 生词本列表 |

## Claude Desktop（stdio）

`claude_desktop_config.json`（Windows：`%APPDATA%\Claude\claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "btr": {
      "command": "C:/path/to/btr.exe",
      "args": ["mcp"],
      "env": {
        "CICBYTE_HOME": "D:/data/btr-home",
        "BTR_API_KEY": ""
      }
    }
  }
}
```

说明：

- `args` 里可加 `--app-root <DIR>` 指定数据目录（不配则用 `~/.cicbyte/byte-translator`）
- 需要词典型翻译效果时，可在 `env` 中配 `ICIBA_API_KEY`（到 open.iciba.com 免费申请；不配时词典未命中会自动走 LLM）
- LLM 引擎需要先在任意终端执行一次 `btr provider set ... --enable`（供应商配置存于共享数据目录）

## Cursor（stdio）

`.cursor/mcp.json`：

```json
{
  "mcpServers": {
    "btr": { "command": "C:/path/to/btr.exe", "args": ["mcp"] }
  }
}
```

## HTTP 模式（多客户端共享一个实例）

```bash
btr mcp --transport http              # 默认 http://127.0.0.1:8324/mcp
# 首次自动生成 MCP Key（与 REST 独立），控制台明示一次
```

客户端以 Streamable HTTP 接入 `http://<host>:8324/mcp`，请求头带 `X-API-Key: <MCP_KEY>`（或 `Authorization: Bearer`）。局域网暴露：`--host 0.0.0.0` + `--api-key`（强制）。

## 验证

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' | btr mcp
```

stdout 返回 `serverInfo: byte-translator` 即就绪（stdio 模式下日志走 stderr，stdout 是协议信道）。
