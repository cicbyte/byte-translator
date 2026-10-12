/**
 * MCP 工具清单（静态，与 crates/core/src/mcp/server.rs 的工具定义保持同步；
 * server.rs 增删工具时更新此表）。
 */

export interface ToolParam {
  name: string;
  type: "string" | "integer";
  required: boolean;
  desc: string;
}

export interface ToolDef {
  name: string;
  desc: string;
  params: ToolParam[];
}

export const readTools: ToolDef[] = [
  {
    name: "translate",
    desc: "翻译：词查词典、句走 LLM，词典未命中自动降级；可指定语言对与引擎",
    params: [
      { name: "text", type: "string", required: true, desc: "要翻译的文本（词或句子）" },
      { name: "from", type: "string", required: false, desc: "源语言（ISO 639，缺省自动检测）" },
      { name: "to", type: "string", required: false, desc: "目标语言（缺省中英互译）" },
      { name: "engine", type: "string", required: false, desc: "引擎（iciba / llm / google / baidu…，缺省自动调度）" },
    ],
  },
  {
    name: "define",
    desc: "查单词词典释义：音标 / 词性 / 例句（仅中英）",
    params: [{ name: "word", type: "string", required: true, desc: "要查的单词" }],
  },
  {
    name: "history_search",
    desc: "检索本地翻译历史（关键词匹配原文）",
    params: [
      { name: "keyword", type: "string", required: false, desc: "检索关键词" },
      { name: "limit", type: "integer", required: false, desc: "返回条数上限（默认 10）" },
    ],
  },
  {
    name: "wordbook_list",
    desc: "生词本列表（可选关键词过滤）",
    params: [
      { name: "keyword", type: "string", required: false, desc: "过滤关键词" },
      { name: "limit", type: "integer", required: false, desc: "返回条数上限（默认 20）" },
    ],
  },
];

export const writeTools: ToolDef[] = [
  {
    name: "wordbook_add",
    desc: "收藏生词到生词本（幂等：重复收藏更新备注）",
    params: [
      { name: "text", type: "string", required: true, desc: "词条或句子" },
      { name: "note", type: "string", required: false, desc: "备注" },
    ],
  },
];
