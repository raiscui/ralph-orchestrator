## [2026-04-30 09:18:00] [Session ID: 019ddbe6-a5ae-7950-8ba0-27b3b9f53219] 笔记: continuous-learning 六文件摘要

## 来源

### 来源1: 六文件候选清单

- 命令:
  - `rg --files -g 'task_plan*.md' -g 'notes*.md' -g 'WORKLOG*.md' -g 'LATER_PLANS*.md' -g 'ERRORFIX*.md' -g 'EPIPHANY_LOG*.md' -g '!archive/**'`
- 发现的上下文集:
  - 默认组: `task_plan.md`, `notes.md`, `WORKLOG.md`, `LATER_PLANS.md`, `ERRORFIX.md`, `EPIPHANY_LOG.md`
  - `__memory_axes`: `task_plan`, `notes`, `WORKLOG`, `LATER_PLANS`, `ERRORFIX`, `EPIPHANY_LOG`
  - `__memory_boundary_fix`: `task_plan`, `notes`, `WORKLOG`, `ERRORFIX`
  - `__tui_chat_missing`: `task_plan`, `notes`, `WORKLOG`
  - `__serial_tui_issues`: `task_plan`
  - `__continuous_learning`: `task_plan`

### 来源2: 活跃度判定

- `__serial_tui_issues`:
  - 最新标准时间戳是 2026-04-30 09:01:24。
  - 阶段2 到阶段6 仍未完成。
  - 判定为真正活跃支线,不归档。
- `__continuous_learning`:
  - 本轮刚创建。
  - 判定为真正活跃支线,不归档。
- `__memory_boundary_fix`:
  - 最新标准时间戳是 2026-04-29 22:13:00。
  - task_plan 标记全部完成。
  - 判定为已完成的未轮转旧支线,本轮总结后归档。
- `__tui_chat_missing`:
  - 最新标准时间戳是 2026-04-29 22:44:00。
  - task_plan 标记全部完成。
  - 判定为已完成的未轮转旧支线,本轮总结后归档。
- `__memory_axes`:
  - 最新标准时间戳是 2026-04-08 08:22:28。
  - 对应 OpenSpec change `rerun-runtime-graphs` 仍是 in-progress,但这套支线六文件本身不是今天活跃上下文。
  - 关键信息已经沉淀在 `openspec/changes/rerun-runtime-graphs/`。
  - 判定为未轮转旧支线,本轮总结后归档。

## 六文件摘要（用于决定如何沉淀知识）

- 涉及的上下文集:
  - 默认组
  - 支线 `__memory_axes`
  - 支线 `__memory_boundary_fix`
  - 支线 `__tui_chat_missing`
  - 活跃支线 `__serial_tui_issues`
  - 本轮支线 `__continuous_learning`

- 任务目标（task_plan.md）:
  - 默认组最近记录的重点是几条支线索引,尤其是 `memory_boundary_fix`, `tui_chat_missing`, `serial_tui_issues`。
  - 当前活跃任务是 `serial_tui_issues`: 查清非 parallel 模式 Codex 线程报错,以及 serial TUI 输出无法选择的问题。
  - 本轮任务是 `$continuous-learning`: 提取经验、同步长期知识文件、归档旧上下文。

- 关键决定（task_plan.md）:
  - `memory_boundary_fix` 被隔离为独立 bugfix 支线,不接管更大的 `memory_axes` 任务。
  - `tui_chat_missing` 使用 read-only / dynamic capture 先确认是否真是渲染 bug。
  - `serial_tui_issues` 使用 systematic-debugging,不把 Codex stderr 与 serial TUI selection 缺失强行归为同因。
  - 本轮持续学习使用 `continuous_learning` 支线上下文,避免污染活跃调试支线。

- 关键发现（notes.md / 支线 notes）:
  - 默认组旧探索确认: “Ralph 自主选择 preset”应放在 bootstrap selector 阶段,而不是正式 run 中途热切换整套拓扑。
  - `memory_boundary_fix`: panic 根因是把字符预算直接当 byte index 切片,中文等多字节 UTF-8 会触发 `byte index ... is not a char boundary`。
  - `tui_chat_missing`: chat pane 没坏; 当前根配置未启用 `parallel.enabled`,所以进入 serial TUI。Chat / Gates 只属于 parallel Supervisor TUI。
  - `memory_axes`: `rerun-runtime-graphs` 已完成 V1 live runtime graph MVP,剩余任务集中在 V2 durable replay graph。

- 实际变更（WORKLOG.md / 支线 WORKLOG）:
  - `memory_boundary_fix` 已修改 `crates/ralph-core/src/text.rs`, `memory_store.rs`, `event_loop/mod.rs`, `event_loop/tests.rs`,并通过聚焦测试、`ralph-core smoke_runner` 和 `cargo test`。
  - `tui_chat_missing` 没有改源码,通过 tmux 捕获证明 parallel TUI 的 Chat / Gates 在多种高度下存在。
  - `memory_axes` 已实现 `--runtime-graph-rrd <FILE>` V1 live runtime graph,增加 runtime graph recorder、delivery observer 和 integration test; OpenSpec 记录为 11/15 完成。

- 支线组摘要:
  - `__memory_boundary_fix`: 已完成 bugfix。可复用经验是 Rust 字符串截断要先转换到安全 UTF-8 byte boundary。
  - `__tui_chat_missing`: 已完成调查。可复用经验是 TUI 缺块时先确认 runtime mode,不要把配置路径差异误判为渲染回归。
  - `__memory_axes`: 已完成 V1 runtime graph 并留下 V2 OpenSpec 后续。可复用经验是 live graph 和 durable replay graph 必须分层,`ralph hats graph` 与 Rerun runtime graph 不能混为一种图。
  - `__serial_tui_issues`: 今天活跃,只读取不归档。本轮持续学习给它的建议应聚焦下一步动态证据和 serial/parallel TUI selection 差异。

- 支线组活跃度判定:
  - 活跃: `__serial_tui_issues`, `__continuous_learning`
  - 未轮转旧支线: `__memory_axes`, `__memory_boundary_fix`, `__tui_chat_missing`
  - 历史版本: 本轮没有新增发现根目录日期后缀历史六文件; `archive/` 内旧历史默认不参与。

- 暂缓事项 / 后续方向（LATER_PLANS.md）:
  - 默认组仍有资源 catalog / selector preset 系统后续方向。
  - `__memory_axes` 里旧的 example / runtime 工作已有不少关闭项,当前最清晰的继续入口应回到 `openspec/changes/rerun-runtime-graphs/tasks.md` 的 V2 3.1-3.4。

- 错误与根因（ERRORFIX.md）:
  - `memory_boundary_fix`: 字符预算与 byte index 混用导致 panic。
  - `memory_axes`: example/E2E 曾因 AGENTS 污染、YAML payload 判断和 all-hat overlay 噪音出现误判或长尾。
  - 默认 ERRORFIX 中还有 parallel example 事件格式经验: 对 live backend example,事件形态要写成协议约束,必要时使用唯一输出模板。

- 重大风险 / 灾难点 / 重要规律（EPIPHANY_LOG.md）:
  - `reply` 关联语义和“答案回到请求方”不是同一层协议。
  - bootstrap selector 不应变成运行中热切换拓扑。
  - YAML 注释不是稳定 runtime metadata contract。
  - runtime graph 如果不和静态 topology graph 分产品,后续会混淆。
  - V1 runtime graph 的 recipient 边不能只靠 durable log 猜。

- 可复用点候选:
  1. Rust 文本截断: 只要语义是字符预算,实现必须通过 `char_indices` / `is_char_boundary` / helper 找安全 byte index,并用中文或 emoji 回归测试验证。
  2. Ralph TUI 判断: chat 属于 parallel Supervisor TUI,serial TUI 缺 chat 不是自动等于渲染回归。
  3. runtime graph 产品边界: `ralph hats graph` 是静态拓扑,V1 Rerun runtime graph 是 live 观察,V2 durable replay 需要额外 delivery/lifecycle 证据。

- 最适合写到哪里:
  - `EXPERIENCE.md`: 写入项目级可复用经验,覆盖 TUI mode 判断、runtime graph 分层、continuous-learning 归档口径。
  - project-level skill: 新增 Rust UTF-8 安全截断 skill,因为 `byte index ... is not a char boundary` 是跨项目可复用的 Rust bug 模式。
  - `AGENTS.md`: 增加 `EXPERIENCE.md` 和新增 skill 的索引。
  - `archive/branch_contexts/`: 归档已完成或旧的支线六文件。

- 需要同步的现有 docs / specs / plan 文档:
  - `openspec/changes/rerun-runtime-graphs/tasks.md` 已准确记录剩余 3.1-3.4,无需更新。
  - `docs/concepts/memories-and-tasks.md` 已记录 runtime scoped experience 的 lower-case `experience.md` 设计方向; 本轮新增的 `EXPERIENCE.md` 是 agent-maintained 项目经验文件,需要在 `AGENTS.md` 中说明,避免和 runtime project `experience.md` 混淆。

- 是否需要新增或更新 docs / specs / plan 文档:
  - 需要更新 `AGENTS.md` 索引。
  - 不需要修改正式 `docs/` / `specs/`,因为本轮没有改变实现或规范,只是把已验证经验沉淀到项目经验文件。

- 是否提取/更新 skill:
  - 是。新增项目级 `self-learning.rust-utf8-safe-string-truncation`。
  - 理由: 触发条件明确,根因不明显,已在本项目通过中文回归测试验证,且 Rust 官方文档也支持该修复口径。

## 官方资料补充

- Rust Book 的 string 章节说明: Rust 字符串是 UTF-8,字符串索引的含义会在 bytes / scalar values / grapheme clusters 之间产生歧义; 对非 ASCII 文本,byte offset 不一定是合法字符边界。
- Rust Book 同一章节给出与本项目 panic 同类的例子: 切到字符内部会触发 `byte index ... is not a char boundary` 运行时 panic。
- `std::str::is_char_boundary` 用来判断某个 byte index 是否位于 UTF-8 code point 起点或字符串结尾。
- `std::str::char_indices` 返回字符和它们的 byte position,适合把“保留 N 个 char”转换为安全 byte index。

## 参考资料

- Rust Book: https://doc.rust-lang.org/book/ch08-02-strings.html
- Rust `str::is_char_boundary`: https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary
- Rust `str::char_indices`: https://doc.rust-lang.org/std/primitive.str.html#method.char_indices
