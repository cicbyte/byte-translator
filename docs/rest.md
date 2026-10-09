# REST API

`btr serve [--host H] [--port P] [--api-key K] [--allow-no-key]`

- 默认监听 `http://127.0.0.1:8321`；交互式文档 `http://<host>:<port>/api/v1/docs`（Swagger UI），规范 `GET /api/v1/openapi.json`
- 首次启动自动生成 API Key 并存 `<data>/config/api.json`（REST 与 MCP 各一把，互不影响）

## 鉴权

- 请求头 `X-API-Key: <KEY>` 或 `Authorization: Bearer <KEY>`（常量时间比较）
- 绑定非回环地址（如 `--host 0.0.0.0`）时**必须**配置 Key，空 Key 拒绝启动
- `--allow-no-key` 仅放行回环连接（本机调试），非回环始终要求 Key
- Host 头白名单（localhost / 127.0.0.1 / ::1 / 绑定地址），其余 403（防 DNS Rebinding）

无 CORS——面向非浏览器客户端（CLI / 脚本 / AI 客户端）。

## 端点

### POST /api/v1/translate

```json
{
  "text": "我想回家了",
  "from": "zh",           // 可选，缺省自动检测
  "to": "en",             // 可选，缺省中英互译
  "engine": "llm",        // 可选：iciba / llm，缺省自动调度
  "stream": false         // true 走 SSE
}
```

非流式响应：

```json
{
  "ok": true,
  "fallback_from": null,
  "result": {
    "engine": "llm", "from": "zh", "to": "en",
    "text": "我想回家了", "translation": "I want to go home.",
    "phonetic": [], "parts": [], "examples": []
  }
}
```

`fallback_from` 非空表示词典未命中自动降级 LLM（值为原引擎名）。

SSE 流式（`stream: true`）：`data: {"delta": "..."}` 逐段推送，结束帧 `{"done": true, "result": {...}}` 或 `{"error": "..."}`；词典命中时等价于单段完整推送。

### GET /api/v1/history

查询参数：`keyword`（LIKE 字面量）、`engine`、`from`、`to`、`page`（默认 1）、`size`（默认 20，上限 100）。响应 `{"total": N, "list": [...]}`（倒序）。

### GET/POST /api/v1/wordbook

- `GET ?keyword=&page=&size=` → `{"total": N, "list": [{id, text, note, created_at}]}`
- `POST {"text": "...", "note": "..."}`（幂等：重复收藏更新备注）

### DELETE /api/v1/wordbook/{id}

不存在返回 404。

### GET/PUT /api/v1/providers

- `GET` → `{"list": [...]}`（Key 不回显，仅 `api_key_set` 布尔）
- `PUT`：新增/更新（`api_key` 留空 = 保持原值）；`enable: true` 同时设为启用（单选）

```json
{
  "name": "DeepSeek", "provider_key": "deepseek",
  "base_url": "https://api.deepseek.com", "api_key": "sk-xxx",
  "model": "deepseek-chat", "max_tokens": 4096, "temperature": 0.3,
  "enable": true
}
```

### GET /health

免鉴权探活：`{"ok": true}`。

## 多机单一数据源

主力机 `btr serve --host 0.0.0.0`；其它机器 `btr --remote http://主力机:8321 --api-key <KEY> <任意查询命令>`（CLI 远程模式与本 API 同源，输出 `--json` 契约一致）。
