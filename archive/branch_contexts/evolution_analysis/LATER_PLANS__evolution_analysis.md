## [2026-05-28 14:22:13] [Session ID: codex-20260528-135644] 后续计划: 项目演进候选路线

### 建议优先级
- P0: 继续 `agent-cli-recoverable-failure-retry` 4.x/5.x/6.x,完成 manual continue、human-facing evidence、integration guardrails 和最终 full gate。
- P1: 以 OpenSpec 或 code task 方式拆分大文件:
  - `crates/ralph-cli/src/record_session.rs`: aggregate / evidence_render / pointer。
  - `crates/ralph-core/src/parallel/instance.rs`: retry runtime / workspace lifecycle / prompt build / result handling。
  - `crates/ralph-core/src/parallel/supervisor.rs`: agents snapshot / completion gate / recoverable map / topology/capability runtime。
  - `crates/ralph-tui/src/app.rs`: layout / hit-test / clipboard / action dispatch / run loop。
- P2: 先对账 `tui-mdfried-viewer` 的 OpenSpec tasks 与当前实现,再决定是恢复 Big Headers / `ratatui-image`,还是修正 tasks 状态。当前 tasks 写已完成,但 `ralph-tui` 依赖和 output buffer 注释显示当前实现仍是纯文本行模型。
- P3: 给旧 docs tree 加 legacy/archived 入口或迁移索引,降低 agent 搜索时读到 Python v1/QCHAT 旧命令的概率。
- P4: 把 runtime/evidence release-fast gate 固化成脚本或 task runner,让 OpenSpec validate、focused Rust tests、replay smoke、record-session evidence 检查可一键执行。

### 暂不立即实施原因
- 用户本轮只要求分析项目演进方向。
- 当前 worktree 有大量既有修改和 active OpenSpec 工作线,直接实现会污染边界。
