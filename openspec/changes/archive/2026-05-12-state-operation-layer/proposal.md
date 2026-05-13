## Why

Ralph 现在已经有多种磁盘状态面: `.agent/memories.md` / `.agent/tasks.jsonl`、`.ralph/events*.jsonl`、`--record-session` JSONL、diagnostics、parallel/TUI runtime snapshots。但它们目前没有一个统一的 runtime workflow state operation contract,后续要做 question obligation、runtime capability、team lifecycle 或 mode 状态时,很容易再次出现“每个入口自己读写一份状态”的漂移。

`oh-my-codex` 值得借鉴的部分不是 `.omx/state` 这个目录名,而是统一的 `state_read` / `state_write` / `state_clear` / `state_list_active` / `state_get_status` 操作层、原子写和 path-level 写入串行化。Ralph 需要先把这层规格钉稳,再决定是否实现 CLI/MCP/runtime 接入。

## What Changes

- 新增 runtime workflow state operation contract。
- 定义五个标准操作:
  - `state_read`
  - `state_write`
  - `state_clear`
  - `state_list_active`
  - `state_get_status`
- 定义状态记录的最小字段:
  - `mode`
  - `active`
  - `current_phase`
  - `updated_at`
  - `run_outcome`
  - `lifecycle_outcome`
  - `session_id`
  - `state`
- 定义 v1 支持的状态 mode 范围:
  - `ralph`
  - `ralplan`
  - `team`
  - `deep-interview`
  - `capability-invocation`
- 定义状态存储必须使用原子写,并且同一路径写入必须串行化。
- 定义 state operation layer 与现有真相源的边界:
  - 不替代 `.agent/memories.md`。
  - 不替代 `.agent/tasks.jsonl`。
  - 不替代 `.ralph/events*.jsonl` 或 `--record-session` evidence stream。
  - 不直接改变 event routing、TUI reducer 或 supervisor topology。
- 实现 `ralph-core` 的 core state operation 模块,但不新增 CLI/MCP/runtime adapter。

## Capabilities

### New Capabilities

- `state-operation-layer`: 统一 Ralph runtime workflow state 的读写、清理、活跃模式列表和状态摘要契约。

### Modified Capabilities

- None.

## Impact

- 受影响代码区域:
  - `crates/ralph-core`: state operation 数据结构、路径解析、原子写、状态 merge、校验和单元测试。
  - `crates/ralph-cli`: 后续可能新增 `ralph state ...` 或内部 debug/doctor 命令。
  - `crates/ralph-cli/src/parallel_runner.rs`: 后续可能把 parallel/team lifecycle 写入 state operation layer,但不能在本 change 内直接改。
  - docs / AGENTS / OpenSpec: 记录 state operation 与 memories/tasks/events/record-session 的边界。
- 不做的事情:
  - 不新增 MCP server。
  - 不改 `.agent/memories.md` / `.agent/tasks.jsonl` 的格式。
  - 不改 `.ralph/events*.jsonl` 或 `--record-session` JSONL contract。
  - 不把 state operation layer 混入 `agent-guidance-catalog-cli` 或 `prompt-contract-runtime-alignment`。
