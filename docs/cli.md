# CLI 手册（btr）

全局参数（所有子命令可用）：

| 参数 | 说明 |
|------|------|
| `--json` | 结构化输出（AI/脚本消费的稳定契约；错误输出 `{"ok":false,"error":...}` 且退出码 1） |
| `--app-root <DIR>` | 覆盖数据目录（默认 `~/.cicbyte/apps/byte-translator`；`CICBYTE_HOME` 环境变量等价） |
| `--remote <URL>` | 远程模式：直连 `btr serve` 端点（`BTR_REMOTE` 环境变量） |
| `--api-key <KEY>` | 远程模式 Key（`BTR_API_KEY` 环境变量） |

## 翻译（主命令）

```bash
btr [OPTIONS] [TEXT]
```

| 参数 | 说明 |
|------|------|
| `-e, --engine <iciba\|llm>` | 显式指定引擎（显式指定不自动降级）；缺省自动调度 |
| `--from <LANG>` / `--to <LANG>` | ISO 639 语言代码（`zh-CN` 等会归一化为 `zh`）；缺省自动检测、中英互译 |
| `TEXT` 位置参数 | 省略则读 stdin；`btr -- history` 翻译保留字本身 |

调度规则（缺省 Auto）：

1. 语言对非中英 → LLM；
2. 中英 + 单词特征（无空格且 ≤ 20 字符）→ 词典优先，未命中自动降级 LLM（输出降级提示行 / JSON `fallback_from` 字段）；
3. 中英 + 句子 → LLM。

## history

```bash
btr history list [--keyword KW] [--engine iciba|llm] [--from en] [--to zh] [--page 1] [--size 20]
btr history recent [--limit 10]
```

失败/取消的翻译不留存；关键词按字面量匹配（`%`/`_` 不作通配符）。

## wordbook

```bash
btr wordbook add <TEXT> [--note NOTE]    # 幂等：重复收藏更新备注
btr wordbook list [--keyword KW] [--page N] [--size N]
btr wordbook remove <ID>                 # id 见 list 输出
```

## provider（仅本地；远程模式会指引到数据所在机器执行）

```bash
btr provider list
btr provider set <KEY> [--name NAME] [--base-url URL] [--api-key KEY] [--model M] [--enable]
btr provider enable <KEY>                # 单选：其余自动停用
btr provider disable
btr provider test <KEY>                  # 连通测试（轻量探测，不耗 token）
btr provider delete <KEY>
```

`<KEY>` 决定接入协议：`openai`/`deepseek`/`zhipu` 等 → OpenAI 兼容；`anthropic`、`gemini`、`ollama` → 各自原生协议。更新时 `--api-key` 留空表示保持原值；Key 不回显（列表只见 `api_key_set`）。

## engine

```bash
btr engine [--json]
```

## serve / mcp

见 [REST API](rest.md) 与 [MCP 接入指南](mcp.md)。

## 数据布局

```
~/.cicbyte/apps/byte-translator/
├── data/
│   ├── db/byte-translator.db    # SQLite（WAL）：历史/生词本/供应商/词典缓存
│   └── config/api.json          # REST 与 MCP 两把独立 Key（原子写、可轮换）
```
