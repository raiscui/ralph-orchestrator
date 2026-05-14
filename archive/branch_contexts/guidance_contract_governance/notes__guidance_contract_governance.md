
## [2026-05-12 10:03:00] [Session ID: omx-1778475786175-ogndry] 笔记: ralph-state-cli-adapter 初始勘察

## 来源

### 来源1: `crates/ralph-cli/src/main.rs`

- 要点:
  - CLI 顶层 `Commands` enum 位于 `crates/ralph-cli/src/main.rs` 附近。
  - `Verify(VerifyArgs)`、`RuntimeGraph(RuntimeGraphArgs)` 都使用 command group 模式。
  - 主分发 match 位于 `main()` 中,已有 `verify_command` / `runtime_graph_command` 这种 helper。
  - `verify_agent_guidance_command` 可作为输出格式和 `ColorMode` 处理参考。

### 来源2: `crates/ralph-cli/tests/integration_verify.rs`

- 要点:
  - CLI integration tests 使用 `assert_cmd::Command::cargo_bin("ralph")`。
  - 测试临时 repo 使用 `tempfile::TempDir`。
  - stdout/stderr 断言保留了完整失败输出,适合作为新 CLI adapter 的测试风格。

### 来源3: `crates/ralph-core/src/state_operations.rs`

- 要点:
  - `StateOperationStore` 已提供 `state_read`、`state_write`、`state_clear`、`state_list_active`、`state_get_status`。
  - `StateMode` 已实现 `FromStr` 和 `Display`,CLI 应直接解析为 core 类型。
  - session scope 和 global scope 的路径解析已由 core 负责,CLI 不应重复拼 JSON path。

## 综合发现

- CLI adapter 的最小合理形态是新增 `ralph state` command group。
- v1 适合先开放:
  - `ralph state status [--mode <mode>] [--session-id <id>] [--json]`
  - `ralph state read <mode> [--session-id <id>] [--json]`
  - `ralph state clear <mode> [--session-id <id>] [--all-sessions]`
- 不建议 v1 暴露 `state write`,因为它容易变成手工篡改 lifecycle state 的入口。后续需要时应另开 change。
