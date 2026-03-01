#!/usr/bin/env python3

# 读取 autopilot 的 report.json,输出一段短摘要,便于在终端快速判断发生了什么。
#
# 设计原则:
# - 输出要短,但要包含 "退出码/原因/失败断言/证据入口" 这些关键信息。
# - 不在这里重新做判定逻辑,只做展示与索引。

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


def _usage() -> None:
    print("Usage:", file=sys.stderr)
    print("  python3 scripts/summarize_report.py <out-dir|report.json>", file=sys.stderr)


def _load_report(path: Path) -> dict:
    if path.is_dir():
        report_path = path / "report.json"
    else:
        report_path = path

    if not report_path.exists():
        raise FileNotFoundError(f"report.json not found: {report_path}")

    with report_path.open("r", encoding="utf-8") as f:
        return json.load(f)


def _as_bool(v) -> bool | None:
    if isinstance(v, bool):
        return v
    return None


def _summarize_parallel_runner_concurrency(stdout_path: Path) -> dict | None:
    # 说明:
    # - autopilot 的 record-session JSONL 是“主证据源”,但并发度(有多少 runner 同时 running)
    #   更容易从 stdout 的状态机日志中提取.
    # - 这里不做 hard verdict,只输出指标,方便人类快速判断并行是否真的发生.
    state_line = re.compile(r"^\[(experiment_runner#\d+):state\]\s+(\w+)\s*$")

    states: dict[str, str] = {}
    entered_running: set[str] = set()
    max_running = 0

    try:
        with stdout_path.open("r", encoding="utf-8", errors="replace") as f:
            for raw in f:
                m = state_line.match(raw)
                if not m:
                    continue
                instance_id, state = m.group(1), m.group(2)
                states[instance_id] = state
                if state == "running":
                    entered_running.add(instance_id)

                running = sum(1 for s in states.values() if s == "running")
                if running > max_running:
                    max_running = running
    except FileNotFoundError:
        return None

    if not states:
        return None

    return {
        "unique_runner_instances_seen": len(states),
        "runner_instances_entered_running": len(entered_running),
        "max_concurrent_running": max_running,
        "entered_running": sorted(entered_running),
    }


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] in {"-h", "--help"}:
        _usage()
        return 2

    try:
        report = _load_report(Path(argv[1]))
    except Exception as e:
        print(f"[summarize_report] Failed: {e}", file=sys.stderr)
        return 2

    exit_code = report.get("exit_code")
    exit_semantic = report.get("exit_code_semantic")
    exit_reason = report.get("exit_reason")

    hard = report.get("hard_verdict", {}) or {}
    hard_passed = _as_bool(hard.get("passed"))
    assertions = hard.get("assertions", []) or []
    failed_assertions = [a for a in assertions if a and a.get("passed") is False]

    agent = report.get("agent", {}) or {}
    agent_status = agent.get("status")
    agent_reason = agent.get("reason")
    agent_output = agent.get("output") or {}
    agent_verdict = agent_output.get("verdict")
    quality_score = agent_output.get("quality_score")

    out_dir = report.get("out_dir")
    record_session = report.get("record_session")

    print("")
    print("Autopilot 摘要")
    print(f"- exit_code: {exit_code} ({exit_semantic})")
    print(f"- exit_reason: {exit_reason}")
    print(f"- hard_passed: {hard_passed}")
    print(f"- agent_status: {agent_status}")

    if agent_verdict is not None:
        print(f"- agent_verdict: {agent_verdict} (quality_score={quality_score})")
    if agent_reason is not None:
        print(f"- agent_reason: {agent_reason}")

    # 并行度摘要(可选): 只有 parallel 示例/配置的 stdout 才会包含该状态机日志.
    if out_dir is not None:
        metrics = _summarize_parallel_runner_concurrency(Path(out_dir) / "stdout.txt")
        if metrics is not None:
            print("")
            print("并行度(从 stdout 状态机推断,experiment_runner 维度):")
            print(
                f"- unique_runner_instances_seen: {metrics['unique_runner_instances_seen']}"
            )
            print(
                f"- runner_instances_entered_running: {metrics['runner_instances_entered_running']}"
            )
            print(f"- max_concurrent_running: {metrics['max_concurrent_running']}")
            print(f"- entered_running: {metrics['entered_running']}")

    if failed_assertions:
        print("")
        print("失败的硬断言(按顺序):")
        for a in failed_assertions[:10]:
            name = a.get("name")
            expected = a.get("expected")
            actual = a.get("actual")
            print(f"- {name}")
            print(f"  expected: {expected}")
            print(f"  actual: {actual}")
        if len(failed_assertions) > 10:
            print(f"- ... and {len(failed_assertions) - 10} more")

    print("")
    print("证据入口(建议打开):")
    if out_dir is not None:
        print(f"- {out_dir}/report.md")
        print(f"- {out_dir}/report.json")
        print(f"- {out_dir}/analysis_input.json")
        print(f"- {out_dir}/analysis_output.json")
        print(f"- {out_dir}/stdout.txt")
        print(f"- {out_dir}/stderr.txt")
    if record_session is not None:
        print(f"- record_session: {record_session}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
