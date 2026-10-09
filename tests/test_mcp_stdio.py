"""MCP stdio 冒烟：JSON-RPC 握手 / 工具列表 / 工具调用。"""

import json
import subprocess

from conftest import BTR_PATH, REPO_ROOT


def _rpc(app_root, messages, timeout=30):
    """向 btr mcp 喂一组 JSON-RPC 行，收集响应。"""
    lines = ["\n".join(json.dumps(m) for m in messages), ""]
    proc = subprocess.run(
        [str(BTR_PATH), "--app-root", str(app_root), "mcp"],
        input="\n".join(lines).encode("utf-8"),
        capture_output=True,
        timeout=timeout,
        cwd=str(REPO_ROOT),
    )
    out = []
    for line in proc.stdout.decode("utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


def test_mcp_stdio_handshake_tools_call(tmp_path):
    app_root = tmp_path / "mcp-root"
    # 第一段：握手 + 工具列表 + 收藏（MCP 请求可能并发处理，读操作放第二段避免竞态）
    responses = _rpc(
        app_root,
        [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize",
             "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                        "clientInfo": {"name": "e2e", "version": "0"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
            {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
             "params": {"name": "wordbook_add",
                        "arguments": {"text": "ephemeral", "note": "mcp 收藏"}}},
        ],
    )
    by_id = {r.get("id"): r for r in responses if "id" in r}

    init = by_id.get(1)
    assert init and init["result"]["serverInfo"]["name"] == "byte-translator"

    tools = by_id.get(2)
    names = sorted(t["name"] for t in tools["result"]["tools"])
    assert names == ["define", "history_search", "translate", "wordbook_add", "wordbook_list"]

    add = by_id.get(3)
    assert "已收藏" in add["result"]["content"][0]["text"]

    # 第二段：新进程读同一数据目录（验证持久化与跨会话读取）
    responses2 = _rpc(
        app_root,
        [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize",
             "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                        "clientInfo": {"name": "e2e", "version": "0"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 4, "method": "tools/call",
             "params": {"name": "wordbook_list", "arguments": {}}},
        ],
    )
    by_id2 = {r.get("id"): r for r in responses2 if "id" in r}
    text = by_id2[4]["result"]["content"][0]["text"]
    assert "ephemeral" in text
    assert "mcp 收藏" in text
