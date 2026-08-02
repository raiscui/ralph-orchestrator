## [2026-05-28 14:19:18] [Session ID: codex-20260528-135644] 笔记: 项目演进机会证据汇总

## 来源

### 来源1: README / docs / Cargo workspace

- `README.md` 描述两种运行模式: Traditional 与 Hat-Based。
- `README.md` Features 中列出 multi-backend,hat,event,backpressure,TUI,memories,tasks,session recording。
- `README.md` Architecture 与 `docs/system/architecture.md` 都确认七个 crate: `ralph-proto`,`ralph-core`,`ralph-adapters`,`ralph-tui`,`ralph-cli`,`ralph-e2e`,`ralph-bench`。
- `docs/system/architecture.md` 明确当前薄编排边界: CLI 加载配置,core 拥有 event loop/config/parser/store,adapters 执行外部 CLI,TUI 只观察不成为真相源。
- `Cargo.toml` workspace package 使用 Rust 2024 edition,同时 README 的旧安装/quick-start 文档里仍有 Python v1 命令。

### 来源2: OpenSpec 当前状态

- `openspec list --json` 显示 active changes:
  - `agent-cli-recoverable-failure-retry`: 17/34,in-progress。
  - `tui-mdfried-viewer`: 13/15,in-progress。
- recoverable retry 的 1.x-3.x 已完成,剩余 4.x manual continue,5.x human-facing observability,6.x integration guardrails。
- TUI mdfried viewer 已完成依赖、能力探测、富块、Big Headers,剩余 `![]()` 默认占位与显式下载/缓存/超时回退。

### 来源3: CodeGraph / 代码结构

- CodeGraph 状态: 386 files,8726 nodes,25974 edges。
- `ralph-core/src/parallel/instance.rs` 的 `HatInstanceActor` 同时负责 job timeout、permission gate、workspace/worktree、recoverable retry、session strategy、completion freeze、prompt 构建、event 解析、worktree hooks 和执行错误收敛。
- `ralph-core/src/parallel/supervisor.rs` 的 `ParallelSupervisor` 同时维护 instances、agents snapshot、dynamic instances、queue decisions、request-reply origins、gates、capability runtime、topology spawn、recoverable failures、TUI observers 和 runtime lifecycle/delivery logging。
- `ralph-cli/src/record_session.rs` 同时包含 strict parse、aggregate、Evidence Inspect render、agents snapshot render、capability/result topics render、pointer helper 和 tests。
- `ralph-tui/src/app.rs` 同时包含 layout、diagram/radar、mouse hit testing、clipboard、action dispatch、App run loop 和大量 UI tests。

### 来源4: 文件大小与文档治理

- Rust 大文件统计中超过 1000 行的高风险文件包括:
  - `crates/ralph-core/src/parallel/instance.rs`: 3464 行。
  - `crates/ralph-core/src/parallel/supervisor.rs`: 1934 行。
  - `crates/ralph-tui/src/app.rs`: 3943 行。
  - `crates/ralph-tui/src/state.rs`: 2919 行。
  - `crates/ralph-cli/src/main.rs`: 2865 行。
  - `crates/ralph-cli/src/record_session.rs`: 1404 行。
  - `crates/ralph-core/src/config.rs`: 3001 行。
- `mkdocs.yml` 使用新的 MkDocs information architecture,并显式 exclude 旧 docs tree。
- 旧 docs tree 里仍可搜索到大量 `python ralph_orchestrator.py`、v1.0.0、QCHAT 环境变量等旧叙事。发布站点不会包含它们,但仓库搜索和 agent 阅读会被污染。

## 综合发现

### 已验证现象

- 项目已经从单一 loop CLI 演进为多 crate、多证据源、多运行态的 Rust workspace。
- OpenSpec 仍有两条 active 工作线,而且都接近收口但还没完全闭环。
- 运行态 evidence 已经形成明确方向: record-session、events、agents snapshot、evidence-index 分工清晰,不能合成第二套真相源。
- 多个核心文件承担过多职责,并且 workspace lint 中 `too_many_lines` 当前是 allow。
- 文档发布面与仓库保留面已经分层,但旧文档仍在搜索面暴露。

### 推断

- 当前最值得演进的主线不是再新增大功能,而是把已开始的 runtime/evidence/TUI 工作收束成更稳定的产品面。
- `agent-cli-recoverable-failure-retry` 的剩余 4.x/5.x/6.x 是可靠性闭环的最高收益项,因为它直接解决用户遇到 rate limit / retry limit 时无法可见恢复的问题。
- `record_session.rs`、`ParallelSupervisor`、`HatInstanceActor`、TUI app/state 适合做"保持 public API 不变的边界拆分",否则后续继续加 observability / manual continue / image blocks 时会继续堆复杂度。
- 文档治理应优先做索引与归档边界,不是急着删除旧文档。当前 `mkdocs.yml` 已经说明保留旧树是为了不丢本地编辑。

### 未确认

- 没有运行全量测试,所以本轮不声明任何功能当前通过或失败。
- 当前工作树有大量既有修改,本轮分析只基于当前文件系统状态,不把 git diff 当作干净 release baseline。
- CodeGraph 可能滞后于刚写入文件,但本轮查询主要读既有代码结构,不依赖刚创建的支线 notes。

## [2026-05-28 14:26:47] [Session ID: codex-20260528-135644] 笔记: TUI mdfried OpenSpec 与当前实现存在状态漂移

### 现象

- `openspec/changes/tui-mdfried-viewer/tasks.md` 标记 1.1 已引入 `ratatui-image`,3.1 已引入 `OutputBlock::{Text, Image}`,4.1/4.2 已实现 Big Headers 和图片渲染。
- 当前 `crates/ralph-tui/Cargo.toml` 的依赖列表没有 `ratatui-image`、`image` 或 `cosmic-text`。
- 当前 `crates/ralph-tui/src/state/parallel/output.rs` 的模块注释明确写着只存 `ratatui::text::Line`,不引入 Big Headers / 图片块等额外渲染结构。

### 结论

- 这不是"只剩图片 inline 5.x"那么简单。
- 当前更可靠的判断是: `tui-mdfried-viewer` 需要先做 OpenSpec tasks 与当前实现的对账。
- 在没有重新验证前,不能把已勾选的 Big Headers / ratatui-image 任务当成已实现事实。

### 后续建议

- 将 TUI mdfried 演进项从"继续 5.1/5.2"调整为"先做 spec-code reconciliation"。
- 如果实现确实被回退,应恢复 tasks 状态或写一个新的 correction change。
- 如果功能迁移到了别处,应补 docs/index 和测试证据,说明当前真实入口。
