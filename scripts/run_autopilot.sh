#!/usr/bin/env bash
set -euo pipefail

# 标准化 autopilot 运行入口.
#
# 目标:
# - 把一条可复用的无人值守命令固化下来,避免每次手工拼参数.
# - 产物统一落盘到 out-dir,方便 CI 收集 artifact 与排障.

usage() {
  cat <<'EOF'
Usage:
  bash scripts/run_autopilot.sh --repo-dir <git-repo> [options]

Options:
  --repo-dir <DIR>          目标 Git 仓库目录(必须是已初始化的 Git repo) (Required)
  --config <FILE>           配置文件路径(相对路径会按 repo 根目录解析) (Default: ralph.yml)
  --out-dir <DIR>           产物输出目录 (Default: <repo>/.ralph/autopilot/<timestamp>)
  --record-session <FILE>   record-session JSONL 输出路径 (Default: <out-dir>/session.jsonl)
  --isolate                 在“临时 clone repo”中运行,避免污染 --repo-dir 指向的原仓库.
                            默认会保留 git 历史(git clone --local),并用 rsync 覆盖工作树(包含 untracked/未提交改动).
                            注意: isolate 模式下若未显式指定 --out-dir,默认改为 /tmp/ralph-autopilot-out-<timestamp>,
                            以避免清理临时 repo 时误删证据.
  --keep-temp-repo          isolate 模式下,即使 autopilot PASS 也保留临时 repo(用于排障/复现).
                            默认行为: PASS 会清理临时 repo; FAIL 会保留临时 repo 并打印路径.
  --no-snapshot-commit      isolate 模式下不创建快照 commit(默认会创建 1 个 snapshot commit,保证 worktree 干净).
  --child-parallel-max-running-jobs <N>
                            (测试用) 覆盖子进程 `ralph run` 的并行并发上限(parallel.autoscale.max_running_jobs).
                            仅影响 autopilot 启动的 child run,不影响用户直接执行 `ralph run` 的默认并发语义.
  --skip-agent-analysis     跳过 agent 分析(只做硬断言)
  --analysis-backend <NAME> agent 分析使用的 backend(可选. 默认跟随 --config 的 cli.backend 或 auto-detect)
  -h, --help                打印帮助

Notes:
  - 默认会在 hard verdict PASS 后执行 agent 分析.
    如需关闭,传 --skip-agent-analysis.
  - --analysis-backend 仅用于覆盖 agent 分析这一步使用的后端.
    它不是开启 agent 分析的必需参数.

Exit codes:
  0 pass, 1 hard-fail, 2 agent-fail, 3 analysis-error
EOF
}

repo_dir=""
config="ralph.yml"
out_dir=""
record_session=""
isolate="false"
keep_temp_repo="false"
snapshot_commit="true"
child_parallel_max_running_jobs=""
skip_agent_analysis="false"
analysis_backend=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-dir)
      repo_dir="$2"
      shift 2
      ;;
    --config)
      config="$2"
      shift 2
      ;;
    --out-dir)
      out_dir="$2"
      shift 2
      ;;
    --record-session)
      record_session="$2"
      shift 2
      ;;
    --isolate)
      isolate="true"
      shift
      ;;
    --keep-temp-repo)
      keep_temp_repo="true"
      shift
      ;;
    --no-snapshot-commit)
      snapshot_commit="false"
      shift
      ;;
    --child-parallel-max-running-jobs)
      child_parallel_max_running_jobs="$2"
      shift 2
      ;;
    --skip-agent-analysis)
      skip_agent_analysis="true"
      shift
      ;;
    --analysis-backend)
      analysis_backend="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$repo_dir" ]]; then
  echo "Missing required: --repo-dir" >&2
  usage >&2
  exit 2
fi

repo_dir="$(cd "$repo_dir" && pwd -P)"

if ! git -C "$repo_dir" rev-parse --git-dir >/dev/null 2>&1; then
  echo "Invalid --repo-dir (not a git repo): $repo_dir" >&2
  exit 2
fi

tmp_repo=""
effective_repo_dir="$repo_dir"

if [[ "$isolate" == "true" ]]; then
  # 说明:
  # - 这里选择 clone + rsync 的方式,目的是:
  #   1) 保留完整 git 历史(避免 integration.task 的 commit_lookup_cmd miss).
  #   2) 覆盖当前工作树(包含 untracked/未提交改动),以验证“本地未提交改进”的真实效果.
  # - 默认会额外做一次 snapshot commit,让临时 repo worktree 干净,减少 hat 在审计时误判。
  ts="$(date +%Y%m%d-%H%M%S)"
  tmp_repo="$(mktemp -d "/private/tmp/ralph-autopilot-repo-${ts}-XXXXXX")"

  echo "Isolate enabled. Temp repo: $tmp_repo" >&2
  echo "Cloning (preserve history): git clone --local \"$repo_dir\" \"$tmp_repo\"" >&2
  git clone --local "$repo_dir" "$tmp_repo" >/dev/null

  echo "Sync working tree (include untracked, exclude run artifacts)..." >&2
  rsync -a --delete \
    --exclude '.git/' \
    --exclude 'target/' \
    --exclude '.ralph/' \
    --exclude '.e2e-tests/' \
    "$repo_dir"/ "$tmp_repo"/

  if [[ "$snapshot_commit" == "true" ]]; then
    echo "Creating snapshot commit (best-effort)..." >&2
    (
      cd "$tmp_repo"
      git add -A
      # 若没有任何变更,commit 会失败;这里用 diff --cached --quiet 判定避免报错中断脚本.
      if git diff --cached --quiet; then
        echo "No changes to snapshot-commit (clean)." >&2
      else
        git -c user.name="ralph" -c user.email="ralph@local" commit -m "snapshot: include uncommitted changes" >/dev/null
      fi
    )
  else
    echo "Skipping snapshot commit (--no-snapshot-commit)." >&2
  fi

  effective_repo_dir="$tmp_repo"
fi

# 默认 out-dir:
# - 非 isolate: 放到原 repo 下的 `.ralph/autopilot/<timestamp>`.
# - isolate: 放到 /tmp 下,避免清理临时 repo 时误删证据.
if [[ -z "$out_dir" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  if [[ "$isolate" == "true" ]]; then
    out_dir="/tmp/ralph-autopilot-out-${ts}"
  else
    out_dir="${repo_dir%/}/.ralph/autopilot/${ts}"
  fi
fi

# 默认 record-session: 放到 out-dir 下.
if [[ -z "$record_session" ]]; then
  record_session="${out_dir%/}/session.jsonl"
fi

cmd=(
  cargo run -q --bin ralph --
  -c "$config"
  autopilot run
  --repo-dir "$effective_repo_dir"
  --record-session "$record_session"
  --out-dir "$out_dir"
)

if [[ -n "$child_parallel_max_running_jobs" ]]; then
  cmd+=( --child-parallel-max-running-jobs "$child_parallel_max_running_jobs" )
fi

if [[ "$skip_agent_analysis" == "true" ]]; then
  cmd+=( --skip-agent-analysis )
fi

if [[ -n "$analysis_backend" ]]; then
  cmd+=( --analysis-backend "$analysis_backend" )
fi

echo "Running: ${cmd[*]}" >&2
set +e
"${cmd[@]}"
code=$?
set -e

echo "" >&2
echo "Autopilot out-dir: $out_dir" >&2
echo "Autopilot record-session: $record_session" >&2
echo "Autopilot exit code: $code" >&2

# 尽量给出一行摘要(不改变退出码).
python3 scripts/summarize_report.py "$out_dir" || true

if [[ "$isolate" == "true" ]]; then
  echo "" >&2
  echo "Isolate temp repo: $tmp_repo" >&2

  # PASS: 默认清理临时 repo; FAIL: 默认保留临时 repo 便于排障.
  if [[ "$keep_temp_repo" == "true" ]]; then
    echo "keep-temp-repo enabled. Temp repo kept." >&2
  else
    if [[ "$code" != "0" ]]; then
      echo "Autopilot failed. Keeping temp repo for debugging." >&2
    else
      if [[ -n "$tmp_repo" && "$tmp_repo" == /private/tmp/ralph-autopilot-repo-* ]]; then
        echo "Cleaning temp repo..." >&2
        rm -r "$tmp_repo"
      else
        echo "Refusing to remove unexpected temp repo path: $tmp_repo" >&2
      fi
    fi
  fi
fi

exit "$code"
