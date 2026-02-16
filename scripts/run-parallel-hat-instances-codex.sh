#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# 一键运行：parallel-hat-instances（Codex 后端）
#
# 目的：
# - 把“并行 E2E 场景”的跑法固定成一个脚本，避免每次手敲长命令。
# - 尽量做基础自检（cargo / codex 是否存在），并把输出落盘方便排障。
#
# 说明：
# - 该脚本会调用 `cargo run -p ralph-e2e`，只跑 `parallel-hat-instances` 场景。
# - 会保留 workspace（--keep-workspace），方便你在 `.e2e-tests/` 下查看现场。
# =============================================================================

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "[parallel-e2e] 工作目录: ${ROOT_DIR}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[parallel-e2e] 错误：未找到 cargo，请先安装 Rust toolchain。" >&2
  exit 1
fi

if ! command -v codex >/dev/null 2>&1; then
  echo "[parallel-e2e] 错误：未找到 codex CLI，请先安装并确保在 PATH 中可用。" >&2
  exit 1
fi

echo "[parallel-e2e] codex 版本：$(codex --version || true)"

# 关键：确保 ralph-e2e 能解析到“本仓库构建”的 ralph 二进制，
# 避免误用 PATH 上的旧版本。
echo "[parallel-e2e] 预构建 ralph 二进制（release，确保与 harness 选择的 target/release/ralph 一致）..."
(cd "${ROOT_DIR}" && cargo build --release -p ralph-cli --bin ralph)

mkdir -p "${ROOT_DIR}/.e2e-tests"
LOG_PATH="${ROOT_DIR}/.e2e-tests/parallel-hat-instances-codex.log"

# 关键：E2E harness 在断言时会读取 workspace 内产物。
# 若复用 workspace（--keep-workspace），旧数据/旧 .ralph 状态可能污染断言（误判通过/误判失败）。
WORKSPACE_DIR="${ROOT_DIR}/.e2e-tests/parallel-hat-instances"
if [[ -d "${WORKSPACE_DIR}" ]]; then
  echo "[parallel-e2e] 清理上次残留 workspace（${WORKSPACE_DIR}）..."
  rm -rf "${WORKSPACE_DIR}"
fi

echo "[parallel-e2e] 开始运行 E2E（只跑 parallel-hat-instances / backend=codex）..."
echo "[parallel-e2e] 日志输出: ${LOG_PATH}"

set -o pipefail
(cd "${ROOT_DIR}" && \
  cargo run -p ralph-e2e -- \
    codex \
    --filter parallel-hat-instances \
    --keep-workspace \
    --verbose \
    --skip-analysis \
    --report both \
  2>&1 | tee "${LOG_PATH}")

echo
echo "[parallel-e2e] 运行结束。产物位置："
echo "  - 场景 workspace: ${ROOT_DIR}/.e2e-tests/parallel-hat-instances/"
echo "  - 报告（Markdown）: ${ROOT_DIR}/.e2e-tests/report.md"
echo "  - 报告（JSON）: ${ROOT_DIR}/.e2e-tests/report.json"
echo "  - 日志: ${LOG_PATH}"
