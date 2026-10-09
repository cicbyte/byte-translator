//! Swagger UI 页面 + OpenAPI 3.0 JSON spec（免鉴权——文档不含数据）。
//! 手写 spec 而非引入 utoipa：端点面小，避免重依赖。

use axum::response::{IntoResponse, Json};

/// Swagger UI HTML 页面（CDN 加载，spec 指向 /api/v1/openapi.json）
pub(super) async fn swagger_ui() -> impl IntoResponse {
    let html = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>btr REST API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  <style>body { margin: 0; }</style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin="anonymous"></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: "/api/v1/openapi.json",
        dom_id: "#swagger-ui",
        deepLinking: true,
        presets: [SwaggerUIBundle.presets.apis],
        layout: "BaseLayout",
      });
    };
  </script>
</body>
</html>"##;
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

pub(super) async fn openapi_spec() -> Json<serde_json::Value> {
    Json(spec())
}

fn result_schema_ref() -> serde_json::Value {
    serde_json::json!({ "$ref": "#/components/schemas/TranslationResult" })
}

fn spec() -> serde_json::Value {
    serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "btr REST API",
            "description": "byte-translator 翻译服务。鉴权：X-API-Key 或 Authorization: Bearer（非回环绑定强制）",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "servers": [{ "url": "http://127.0.0.1:8321" }],
        "paths": {
            "/health": { "get": {
                "summary": "探活（免鉴权）",
                "responses": { "200": { "description": "服务存活" } },
            }},
            "/api/v1/translate": { "post": {
                "summary": "翻译（词查词典、句走 LLM，自动降级）",
                "requestBody": { "required": true, "content": { "application/json": {
                    "schema": { "$ref": "#/components/schemas/TranslateBody" } },
                }},
                "responses": {
                    "200": { "description": "翻译结果", "content": { "application/json": {
                        "schema": { "$ref": "#/components/schemas/TranslateResponse" } },
                    }},
                    "502": { "description": "引擎失败（含错误信息）" },
                },
            }},
            "/api/v1/history": { "get": {
                "summary": "历史检索",
                "parameters": [
                    { "name": "keyword", "in": "query", "schema": { "type": "string" }, "description": "原文关键词（LIKE）" },
                    { "name": "engine", "in": "query", "schema": { "type": "string", "enum": ["iciba", "llm"] } },
                    { "name": "from", "in": "query", "schema": { "type": "string" }, "description": "源语言（ISO 639）" },
                    { "name": "to", "in": "query", "schema": { "type": "string" } },
                    { "name": "page", "in": "query", "schema": { "type": "integer", "default": 1 } },
                    { "name": "size", "in": "query", "schema": { "type": "integer", "default": 20, "maximum": 100 } },
                ],
                "responses": { "200": { "description": "历史列表" } },
            }},
            "/api/v1/wordbook": {
                "get": { "summary": "生词本列表", "parameters": [
                    { "name": "keyword", "in": "query", "schema": { "type": "string" } },
                    { "name": "page", "in": "query", "schema": { "type": "integer", "default": 1 } },
                    { "name": "size", "in": "query", "schema": { "type": "integer", "default": 20 } },
                ], "responses": { "200": { "description": "生词列表" } } },
                "post": { "summary": "收藏生词（幂等：重复收藏更新备注）",
                    "requestBody": { "required": true, "content": { "application/json": {
                        "schema": { "type": "object", "required": ["text"], "properties": {
                            "text": { "type": "string" }, "note": { "type": "string" } } } } } },
                    "responses": { "200": { "description": "已收藏" }, "400": { "description": "词条为空" } } },
            },
            "/api/v1/wordbook/{id}": { "delete": {
                "summary": "删除生词",
                "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "integer" } }],
                "responses": { "200": { "description": "已删除" }, "404": { "description": "不存在" } } },
            },
            "/api/v1/providers": {
                "get": { "summary": "供应商列表（Key 不回显，仅 api_key_set）", "responses": { "200": { "description": "列表" } } },
                "put": { "summary": "新增/更新供应商（api_key 留空=保持原值；enable 可同时切启用）",
                    "requestBody": { "required": true, "content": { "application/json": {
                        "schema": { "$ref": "#/components/schemas/ProviderBody" } } } },
                    "responses": { "200": { "description": "已保存" }, "400": { "description": "参数错误" } } },
            },
        },
        "components": { "schemas": {
            "TranslateBody": { "type": "object", "required": ["text"], "properties": {
                "text": { "type": "string", "description": "要翻译的文本" },
                "from": { "type": "string", "description": "源语言（ISO 639，缺省自动检测）" },
                "to": { "type": "string", "description": "目标语言（缺省中英互译）" },
                "engine": { "type": "string", "enum": ["iciba", "llm"], "description": "缺省自动调度" },
                "stream": { "type": "boolean", "default": false, "description": "true 走 SSE 流式（LLM 逐段）" },
            }},
            "TranslateResponse": { "type": "object", "properties": {
                "ok": { "type": "boolean" },
                "fallback_from": { "type": "string", "nullable": true, "description": "自动降级时的原引擎" },
                "result": result_schema_ref(),
            }},
            "TranslationResult": { "type": "object", "properties": {
                "engine": { "type": "string" },
                "from": { "type": "string" }, "to": { "type": "string" },
                "text": { "type": "string" }, "translation": { "type": "string" },
                "phonetic": { "type": "array", "items": { "type": "string" } },
                "parts": { "type": "array", "items": { "type": "object", "properties": {
                    "pos": { "type": "string" }, "mean": { "type": "string" } } } },
                "examples": { "type": "array", "items": { "type": "object", "properties": {
                    "orig": { "type": "string" }, "trans": { "type": "string" } } } },
            }},
            "ProviderBody": { "type": "object", "required": ["name", "provider_key"], "properties": {
                "name": { "type": "string" },
                "provider_key": { "type": "string", "description": "如 openai / deepseek / zhipu / anthropic / gemini / ollama" },
                "base_url": { "type": "string" }, "api_key": { "type": "string", "description": "更新时留空=保持原值" },
                "model": { "type": "string" },
                "max_tokens": { "type": "integer", "default": 4096 },
                "temperature": { "type": "number", "default": 0.3 },
                "enable": { "type": "boolean", "description": "同时设置启用状态" },
            }},
        }},
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn spec_shape() {
        let s = super::spec();
        assert_eq!(s["openapi"], "3.0.3");
        let paths = s["paths"].as_object().unwrap();
        for p in ["/health", "/api/v1/translate", "/api/v1/history", "/api/v1/wordbook", "/api/v1/providers"] {
            assert!(paths.contains_key(p), "spec 缺少路径 {p}");
        }
    }
}
