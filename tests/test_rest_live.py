"""REST 服务在线测试：起真实 serve 进程，urllib 探测鉴权与端点。"""

import json
import socket
import subprocess
import time
import urllib.request

import pytest

from conftest import BTR_PATH, REPO_ROOT, run_btr


def _free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _request(port: int, path: str, method="GET", key=None, body=None, host="127.0.0.1"):
    url = f"http://127.0.0.1:{port}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    req.add_header("Host", host)
    if key:
        req.add_header("X-API-Key", key)
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return resp.status, json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        try:
            payload = json.loads(e.read().decode())
        except Exception:
            payload = None
        return e.code, payload


@pytest.fixture
def serve(btr_bin, tmp_path):
    """起 btr serve（随机端口 + 显式 Key），yield (port, key)，测试完关停。"""
    app_root = tmp_path / "rest-root"
    port = _free_port()
    # 显式 --api-key：api.json 落盘为加密态，测试不经文件取 Key
    key = "e2e-rest-key-32-chars-aaaaaaaaaa"
    proc = subprocess.Popen(
        [str(BTR_PATH), "--app-root", str(app_root), "serve", "--port", str(port), "--api-key", key],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(REPO_ROOT),
    )
    try:
        # 等 /health 可用；serve 提前退出或超时都显式失败
        for _ in range(60):
            try:
                status, _ = _request(port, "/health")
                if status == 200:
                    break
            except Exception:
                pass
            if proc.poll() is not None:
                proc.kill()
                raise RuntimeError(f"serve 启动即退出（端口 {port} 可能被占用）")
            time.sleep(0.5)
        else:
            proc.kill()
            raise RuntimeError("serve /health 就绪超时")
        yield port, key
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


def test_health_free(serve):
    port, _ = serve
    status, body = _request(port, "/health")
    assert status == 200
    assert body == {"ok": True}


def test_business_requires_key(serve):
    port, key = serve
    status, _ = _request(port, "/api/v1/history")
    assert status == 401
    status, body = _request(port, "/api/v1/history", key=key)
    assert status == 200
    assert body["total"] == 0


def test_rebinding_host_blocked(serve):
    port, key = serve
    # urllib 会自管 Host 头，用 http.client 直连以注入攻击者域名
    import http.client

    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
    conn.request("GET", "/api/v1/history", headers={"Host": "evil.com", "X-API-Key": key})
    resp = conn.getresponse()
    assert resp.status == 403
    conn.close()


def test_wordbook_roundtrip_and_remote_consistency(serve, btr_bin, tmp_path):
    port, key = serve
    # REST 收藏
    status, body = _request(
        port, "/api/v1/wordbook", method="POST", key=key,
        body={"text": "serendipity", "note": "rest 收藏"},
    )
    assert status == 200
    # CLI 远程模式读取同一数据
    proc = run_btr(
        btr_bin, tmp_path / "remote-root",
        ["--remote", f"http://127.0.0.1:{port}", "--api-key", key, "--json", "wordbook", "list"],
    )
    assert proc.returncode == 0
    payload = json.loads(proc.stdout_text)
    assert payload["total"] == 1
    assert payload["list"][0]["text"] == "serendipity"
    assert payload["list"][0]["note"] == "rest 收藏"


def test_translate_empty_text_400(serve):
    port, key = serve
    status, body = _request(
        port, "/api/v1/translate", method="POST", key=key, body={"text": "  "},
    )
    assert status == 400
    assert body["ok"] is False


def test_openapi_docs_free(serve):
    port, _ = serve
    status, body = _request(port, "/api/v1/openapi.json")
    assert status == 200
    assert body["openapi"].startswith("3.0")
