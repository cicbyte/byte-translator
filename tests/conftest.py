"""
btr CLI 端到端测试公共设施。

- btr 二进制定位：target/debug/btr.exe（每次会话增量构建，保证测到最新代码）
- app_root 隔离：每个 flow 使用独立临时数据目录，不触碰用户真实数据
- 运行辅助：run_btr(...) 统一 subprocess 调用（UTF-8、超时、stdin）
"""

import json
import os
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
BTR_PATH = REPO_ROOT / "target" / "debug" / ("btr.exe" if os.name == "nt" else "btr")


def _ensure_btr_binary() -> Path:
    """构建 btr 二进制。每次测试会话都跑增量构建——跳过构建会拿旧二进制测旧代码。"""
    subprocess.run(
        ["cargo", "build", "-p", "byte-translator-cli"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
    )
    assert BTR_PATH.exists(), f"构建后仍未找到 btr：{BTR_PATH}"
    return BTR_PATH


@pytest.fixture(scope="session")
def btr_bin() -> Path:
    return _ensure_btr_binary()


@pytest.fixture
def app_root(tmp_path: Path) -> Path:
    """每个 flow 独立的临时数据目录（含全新 DB 与迁移）。"""
    return tmp_path / "btr-root"


def run_btr(
    btr_bin: Path,
    app_root: Path,
    args: list,
    stdin: str = None,
    timeout: int = 60,
    env_extra: dict = None,
) -> subprocess.CompletedProcess:
    """执行一条 btr 命令。args 不含 --app-root（自动注入）。

    stdin 以字节传入（Windows 管道的文本模式编码不可靠）。
    """
    env = os.environ.copy()
    env.pop("ICIBA_API_KEY", None)
    env.pop("BTR_REMOTE", None)
    env.pop("BTR_API_KEY", None)
    if env_extra:
        env.update(env_extra)
    proc = subprocess.run(
        [str(btr_bin), "--app-root", str(app_root), *[str(a) for a in args]],
        input=stdin.encode("utf-8") if stdin is not None else None,
        capture_output=True,
        timeout=timeout,
        env=env,
        cwd=str(REPO_ROOT),
    )
    proc.stdout_text = proc.stdout.decode("utf-8", errors="replace")
    proc.stderr_text = proc.stderr.decode("utf-8", errors="replace")
    return proc


# ===== YAML runner 辅助 =====

def parse_json(text: str):
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None


def get_path(obj, dotted: str):
    """按点分路径取值：a.b.0.c"""
    cur = obj
    for part in dotted.split("."):
        if part == "":
            continue
        if isinstance(cur, list):
            cur = cur[int(part)]
        elif isinstance(cur, dict):
            cur = cur.get(part)
        else:
            return None
    return cur


def substitute(value, variables: dict):
    """${var} 替换；整串恰为 ${var} 时保留原类型。"""
    if isinstance(value, str):
        if value == "${stdin_data}":
            return value
        if value.startswith("${") and value.endswith("}") and value.count("${") == 1:
            return variables.get(value[2:-1], value)
        for k, v in variables.items():
            token = "${" + k + "}"
            if token in value:
                value = value.replace(token, str(v))
        return value
    if isinstance(value, list):
        return [substitute(v, variables) for v in value]
    if isinstance(value, dict):
        return {k: substitute(v, variables) for k, v in value.items()}
    return value


def substitute_deep(value, variables: dict):
    return substitute(value, variables)


def subset_match(actual, expected) -> bool:
    """递归子集断言：dict 子集 / list 逐项（长度相等）/ 标量相等。"""
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return False
        return all(k in actual and subset_match(actual[k], v) for k, v in expected.items())
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            return False
        return all(subset_match(a, e) for a, e in zip(actual, expected))
    return actual == expected
