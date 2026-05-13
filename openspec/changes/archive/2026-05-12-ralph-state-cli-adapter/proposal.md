## Why

`state-operation-layer` 已经在 `ralph-core` 落地,但现在只有 Rust core API。维护者要检查或清理 `.ralph/state` 时,仍需要写临时代码或手工读 JSON,这会绕开刚建立的单一 state operation contract。

本 change 要把 core state operation layer 接到 CLI 上,提供最小、可验证、日常可用的 `ralph state ...` 入口,同时避免把 CLI 变成第二套 JSON 实现。

## What Changes

- 新增 `ralph state` command group。
- 新增 `ralph state status`:
  - 支持查看全部 mode 或单个 mode 的状态摘要。
  - 支持 `--session-id <id>` 查看 session scope。
  - 支持 `--json` 输出机器可读 JSON。
- 新增 `ralph state read <mode>`:
  - 读取指定 mode 的 runtime workflow state。
  - 支持 `--session-id <id>`。
  - 支持 `--json` 输出机器可读 JSON。
- 新增 `ralph state clear <mode>`:
  - 清理指定 mode 的 global 或 session scope state。
  - 支持 `--session-id <id>`。
  - 支持 `--all-sessions` 清理 global + all session scopes。
- CLI adapter 必须调用 `ralph-core::StateOperationStore`,不能重新实现 JSON 路径解析、merge、clear 或 atomic write。
- v1 不新增 `ralph state write`,避免用户手工篡改 workflow lifecycle state。

## Capabilities

### New Capabilities

- `state-cli-adapter`: Ralph state operation layer 的 CLI adapter,覆盖 status/read/clear 命令、JSON 输出和 core API 复用边界。

### Modified Capabilities

- None.

## Impact

- 受影响区域:
  - `crates/ralph-cli/src/main.rs`: 新增 `state` command group、参数结构和 command handlers。
  - `crates/ralph-cli/tests/`: 新增 state CLI integration tests。
  - `agent-guidance-manifest.toml`: 登记新的 OpenSpec change proposal。
  - `openspec/changes/ralph-state-cli-adapter/`: 新增 proposal、design、spec 和 tasks。
- 不做的事情:
  - 不新增 MCP state tools。
  - 不接 runtime question obligation。
  - 不接 capability invocation runtime 写入路径。
  - 不修改 `.agent/memories.md`、`.agent/tasks.jsonl`、`.ralph/events*.jsonl`、record-session 或 diagnostics 的职责。
  - 不新增第二套 state 文件格式。
