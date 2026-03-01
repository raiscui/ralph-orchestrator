---
name: parallel-engine-autopilot
description: |
  在已初始化的 Git 仓库内,无人值守运行 `ralph autopilot run` 或离线 `ralph autopilot analyze`,
  以 record-session JSONL 为主证据源生成 report.json/report.md,并用稳定退出码表达 verdict。
  适用场景:
  1) "帮我验证 parallel-experimental-dev-engine 是否闭环"
  2) "我有一份 --record-session.jsonl,帮我自动判定 PASS/FAIL"
  3) "把 report.json 快速总结给我"
author: Codex CLI (GPT-5.2)
version: 1.0.2
date: 2026-02-19
---

# parallel-engine-autopilot

## 这是什么

`ralph autopilot` 是一个无人值守包装层:

- `autopilot run`: 在指定 Git 仓库目录内,以子进程方式执行一次真实的 `ralph run --no-tui --record-session ...`.
  然后基于 JSONL 做硬断言判定,并生成报告.
- `autopilot analyze`: 不重新运行工作流.
  只分析一份已存在的 record-session JSONL,生成报告与稳定退出码.

核心原则:

- record-session JSONL 是最终成果,也是判定的主证据源.
- stdout/stderr 只是辅助证据,用于快速排障.

## 快速开始

### 1) 一键运行(推荐)

用脚本封装标准命令:

```bash
bash scripts/run_autopilot.sh \
  --repo-dir /path/to/git-repo \
  --config ralph.yml \
  --out-dir /tmp/ralph-autopilot-out \
  --child-parallel-max-running-jobs 2 \
  --skip-agent-analysis
```

说明:

- `--skip-agent-analysis` 只做硬断言,不需要 AI 后端,适合 CI 先接入.
- `--child-parallel-max-running-jobs` 是一个**测试用**并发上限覆盖:
  - 仅影响 autopilot 启动的子进程 `ralph run`(通过派生 config 实现强隔离).
  - 不会改变用户在自己项目目录里直接执行 `ralph run` 的默认并发语义.
- 要开启 agent 分析: 只需要不传 `--skip-agent-analysis`.
  autopilot 会在 hard verdict 通过后自动触发 agent 分析.
- `--analysis-backend` 不是开启 agent 分析的必需参数.
  它只是可选覆盖,用于你想强制指定 agent 分析这一步用哪个后端.
  不传时会默认跟随 `--config` 的 `cli.backend`(若为 `auto`,则按 Ralph 的 auto-detect 逻辑选择后端).

### 1.1) 在未提交改动上隔离运行(最推荐)

如果你要验证“当前工作区未提交改进”的效果,但不希望 autopilot 的运行态文件/提交污染原仓库,
直接在脚本里加 `--isolate`:

```bash
bash scripts/run_autopilot.sh \
  --repo-dir /path/to/git-repo \
  --config ralph.yml \
  --isolate \
  --child-parallel-max-running-jobs 2 \
  --skip-agent-analysis
```

说明:

- `--isolate` 会:
  - `git clone --local` 保留 git 历史(避免 integration.task 的 `commit_lookup_cmd` 漂移).
  - 用 `rsync --delete` 覆盖工作树(包含 untracked/未提交改动),并排除 `.git/target/.ralph/.e2e-tests` 等运行态目录.
  - 默认创建 1 个 snapshot commit,让临时 repo worktree 干净(可用 `--no-snapshot-commit` 关闭).
- isolate 模式下若未显式指定 `--out-dir`,默认会用 `/tmp/ralph-autopilot-out-<timestamp>`,
  避免清理临时 repo 时误删证据.
- 临时 repo 的清理策略:
  - PASS: 默认清理
  - FAIL: 默认保留(便于排障)
  - 可用 `--keep-temp-repo` 强制保留

### 2) 直接运行 autopilot run

```bash
cargo run -q --bin ralph -- autopilot run \
  --repo-dir /path/to/git-repo \
  --config ralph.yml \
  --record-session /tmp/ralph-autopilot-out/session.jsonl \
  --out-dir /tmp/ralph-autopilot-out \
  --child-parallel-max-running-jobs 2 \
  --skip-agent-analysis
```

### 3) 离线分析(只读 JSONL)

```bash
cargo run -q --bin ralph -- autopilot analyze \
  --repo-dir /path/to/git-repo \
  --record-session /tmp/ralph-autopilot-out/session.jsonl \
  --out-dir /tmp/ralph-autopilot-out \
  --skip-agent-analysis
```

## 退出码语义(稳定合同)

- `0`: 硬断言 PASS 且 agent verdict PASS(或显式跳过 agent 分析)
- `1`: 硬断言 FAIL(闭环未达成/出现禁止 topic/termination 非 CompletionPromise/JSONL 不可解析等)
- `2`: 硬断言 PASS 但 agent verdict FAIL 或 `quality_score=suboptimal`
- `3`: 硬断言 PASS,但 agent 分析运行/解析失败

## 失败时先看哪些证据

`--out-dir` 下至少会有:

- `report.md`: 人类可读总览与排障入口
- `report.json`: 机器可读 verdict(退出码/硬断言/agent 摘要)
- `analysis_input.json`: 证据包(带体积预算)
- `analysis_output.json`: agent 分析结果(或 skipped/error 占位)
- `stdout.txt`: 子进程 `ralph run` 的 stdout(辅助证据)
- `stderr.txt`: 子进程 `ralph run` 的 stderr(辅助证据)

## 快速总结 report.json

```bash
python3 scripts/summarize_report.py /tmp/ralph-autopilot-out
```

## 在未提交改动上隔离运行(推荐)

如果你要验证“当前工作区未提交改进”的效果,又不想让 autopilot 的运行态文件/提交污染当前仓库,建议先构造一个临时 repo.

关键点:

- 需要保留 git 历史(否则 integration 阶段常用的 `commit_lookup_cmd` 可能 miss,增加漂移与耗时).
- 需要尽量包含未提交改动(含 untracked 文件).
- 临时 repo 跑完即可删除,只保留 `/tmp/...` out_dir 作为证据.

### 方案A(更稳): 本地 clone + rsync 覆盖工作树(保留历史 + 包含 untracked)

```bash
# 1) 先本地 clone,保留完整历史
tmp_repo="/private/tmp/ralph-autopilot-repo-$(date +%Y%m%d-%H%M%S)"
git clone --local . "$tmp_repo"

# 2) 用 rsync 覆盖当前工作树(包含未提交改动与 untracked 文件),但保留 tmp_repo 的 .git 历史
#    说明: 这里刻意排除运行态目录,避免把旧状态复制过去
rsync -a \
  --exclude '.git/' \
  --exclude 'target/' \
  --exclude '.ralph/' \
  --exclude '.e2e-tests/' \
  ./ "$tmp_repo"/

# 3) (可选) 在临时 repo 提交一个快照 commit,便于审计/回滚
cd "$tmp_repo"
git add -A
git -c user.name="ralph" -c user.email="ralph@local" commit -m "snapshot: include uncommitted changes"

# 4) 跑 autopilot
bash scripts/run_autopilot.sh \
  --repo-dir "$tmp_repo" \
  --config examples/parallel-experimental-dev-engine/ralph.yml \
  --out-dir "/tmp/ralph-autopilot-out-parallel-$(date +%Y%m%d-%H%M%S)" \
  --child-parallel-max-running-jobs 2 \
  --skip-agent-analysis
```

### 方案B(更轻): 本地 clone + git diff patch(仅覆盖 tracked 改动)

如果你确定本轮未提交改动都在 tracked 文件里(没有新增 untracked 文件),可以用 patch 方式更轻量:

```bash
tmp_repo="/private/tmp/ralph-autopilot-repo-$(date +%Y%m%d-%H%M%S)"
git clone --local . "$tmp_repo"

# 导出未提交改动(unstaged + staged)
git diff > /tmp/ralph-autopilot.patch
git diff --cached >> /tmp/ralph-autopilot.patch

cd "$tmp_repo"
git apply /tmp/ralph-autopilot.patch

# (可选) 提交快照 commit
git add -A
git -c user.name="ralph" -c user.email="ralph@local" commit -m "snapshot: include uncommitted changes"
```

然后同样用 `--repo-dir "$tmp_repo"` 跑 autopilot 即可.
