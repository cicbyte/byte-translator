"""
YAML 用例驱动的 btr CLI 端到端测试。

用例文件放 tests/test_cases/*.yaml，每个文件包含 flows 列表；每个 flow
是一个有序步骤链（后步可引用前步 save 的变量）。

步骤字段：
  run          : 参数列表（不含 btr 本体与 --app-root）；支持 ${var}
  stdin        : 可选，喂给进程的标准输入
  exit         : 期望退出码（默认 0）
  expect_json  : 对 stdout JSON 的递归子集断言
  contains     : stdout 必须包含的子串列表
  not_contains : stdout 不得包含的子串列表
  save         : {变量名: json点分路径}
  save_stdout  : 变量名，把完整 stdout 存入变量
"""

from pathlib import Path

import pytest
import yaml

from conftest import get_path, parse_json, run_btr, subset_match, substitute_deep

CASE_DIR = Path(__file__).resolve().parent / "test_cases"


def _load_flows():
    flows = []
    for yml in sorted(CASE_DIR.glob("*.yaml")):
        data = yaml.safe_load(yml.read_text(encoding="utf-8")) or {}
        for flow in data.get("flows", []):
            flow["_file"] = yml.name
            flows.append(flow)
    return flows


FLOWS = _load_flows()
assert FLOWS, "tests/test_cases 下没有发现任何 YAML 用例"


@pytest.mark.parametrize("flow", FLOWS, ids=[f["name"] for f in FLOWS])
def test_flow(flow, btr_bin, app_root):
    variables: dict = {}
    for i, step in enumerate(flow.get("steps", []), start=1):
        args = substitute_deep(step["run"], variables)
        if (step.get("expect_json") is not None or step.get("save")) and "--json" not in args:
            args = args + ["--json"]
        stdin = step.get("stdin")
        if stdin is not None and stdin != "${stdin_data}":
            stdin = substitute_deep(stdin, variables)
        proc = run_btr(btr_bin, app_root, args, stdin=stdin)

        want_exit = step.get("exit", 0)
        assert proc.returncode == want_exit, (
            f"[{flow['name']}] 步骤 {i} 退出码 {proc.returncode} != {want_exit}\n"
            f"命令：{args}\nstdout：{proc.stdout_text}\nstderr：{proc.stderr_text}"
        )

        if step.get("expect_json") is not None:
            payload = parse_json(proc.stdout_text)
            assert payload is not None, (
                f"[{flow['name']}] 步骤 {i} stdout 不是合法 JSON：\n{proc.stdout_text}"
            )
            expected = substitute_deep(step["expect_json"], variables)
            assert subset_match(payload, expected), (
                f"[{flow['name']}] 步骤 {i} JSON 子集断言失败\n"
                f"期望：{expected}\n实际：{payload}"
            )

        # 人类模式错误走 stderr、--json 模式错误走 stdout——子串断言合并两路
        combined_text = proc.stdout_text + "\n" + proc.stderr_text
        for sub in step.get("contains", []):
            assert sub in combined_text, (
                f"[{flow['name']}] 步骤 {i} 输出缺少子串 {sub!r}\n"
                f"stdout：{proc.stdout_text}\nstderr：{proc.stderr_text}"
            )
        for sub in step.get("not_contains", []):
            assert sub not in combined_text, (
                f"[{flow['name']}] 步骤 {i} 输出不应包含 {sub!r}\n"
                f"stdout：{proc.stdout_text}\nstderr：{proc.stderr_text}"
            )

        for var, dotted in (step.get("save") or {}).items():
            payload = parse_json(proc.stdout_text)
            assert payload is not None, f"[{flow['name']}] 步骤 {i} save 需要 JSON stdout"
            variables[var] = get_path(payload, dotted)
        if step.get("save_stdout"):
            variables[step["save_stdout"]] = proc.stdout_text
