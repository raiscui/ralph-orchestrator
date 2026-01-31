# WORKLOG

> 说明：历史 WORKLOG 已按“超过 1000 行自动归档”的规则拆分为时间戳文件。
> - `archive/WORKLOG_2026-01-29_1908.md`
> - `archive/WORKLOG_2026-01-29_2022.md`

## 2026-01-30 01:16 +0800｜Tier8 E2E：覆盖 `parallel-trigger-routing` example（按 hat 统计 job_runs）+ completion 收敛护栏

### 你要解决的问题
1) `examples/parallel-trigger-routing` 在并行跑 demo 时：
   - 你观察到 `ralph#1` 输出 `LOOP_COMPLETE` 后，其他进程仍持续创建/运行 job（不收敛）。
2) 你希望 E2E **直接覆盖 example 配置**，并断言每个 hat 的“应跑次数”：
   - `spec_writer == 2`
   - `spec_reviewer == 2`
   - `spec_logger == 3`
   - 并且明确：这里的“次数”应该按 **job_runs** 统计，而不是按 instance 数量。

### 我做了什么
- 新增 Tier8 场景 `parallel-trigger-routing-example`：
  - `crates/ralph-e2e/src/scenarios/parallel_trigger_routing_example.rs`
  - 行为：把 `examples/parallel-trigger-routing/ralph.yml` **原样拷贝**进 E2E workspace 运行（并行 headless）。
  - 断言口径：
    - 解析并行 stdout 的 `[instance:out|err:job=<id>]` 前缀；
    - `job_id` 去重；
    - 再按 hat 名聚合（`spec_writer#*` → `spec_writer`）得到 `job_runs`。
  - 断言内容：
    - `spec_writer job_runs == 2`
    - `spec_reviewer job_runs == 2`
    - `spec_logger job_runs == 3`
    - `LOOP_COMPLETE` 后不得出现新的 `job_id`（防止 completion 后仍派工）。

- 为了做到“真的跑 example prompt”，E2E executor 支持不覆盖 `event_loop.prompt`：
  - `crates/ralph-e2e/src/executor.rs`：新增 `PromptSource::Config`（不传 `-p`，使用 `ralph.yml` 内置 prompt）。

### 关键结论（针对你问的“event 是否会标记用过”）
- Hat 输出事件不是从文件读的，它们走的是 supervisor ↔ hat instance 的 channel。
- `.ralph/events.jsonl` 主要是**历史日志/回放/排障锚点**，不是“消费队列”。
- 外部注入事件（`ralph emit`）才会写入 `.ralph/current-events` 指向的 JSONL，并由 reader 以 file position 增量读取。

### 关于你看到的 “spec_writer 跑 3 次”
- 我在本机统计了 `examples/parallel-trigger-routing/.ralph/events.jsonl`：
  - `spec.start` 出现了 3 次。
  - 更符合“你连续跑了多次 demo 导致 append-only 历史日志叠加”，而不是“单次 run 内必然重复触发”。
- E2E keep-workspace 的单次 run（`.e2e-tests/parallel-trigger-routing-example/.ralph/events.jsonl`）显示闭环为：
  - `spec.start → spec.ready → spec.rejected → spec.ready → spec.approved`（符合预期）。

### 验证
- `cargo fmt --check` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅
- `cargo run -p ralph-e2e -- codex --filter parallel-trigger-routing-example --keep-workspace --verbose --skip-analysis` ✅

## 2026-01-30 01:51 +0800｜问答：`ralph.yml` 里 hat 的 `description` 会不会注入到 prompt？
- 会注入，但主要是注入到“协调者”的 prompt 里，作为 `## HATS` / `## HATS TOPOLOGY (CONFIGURED)` 的描述列。
- 不会自动注入到每个 hat 的 job prompt；如果你希望某个 hat 自己也“读到这段描述”，把它写进该 hat 的 `instructions` 更可靠。

## 2026-01-30 03:56 +0800｜并行 Supervisor Output：接近 mdfried（Big Headers + stderr 前缀列）

### 目标回顾
- 终端：Warp
- 作用范围：仅并行 Supervisor 的 Output 面板
- 视觉目标：接近 `mdfried` 的 Big Headers（未来再做 `![]()` 图片内联）

### 我做了什么（落地改动）
- 引入图片渲染的安全开关与依赖：
  - 新增配置：`tui.images.enabled`、`tui.images.inline_remote`（默认都为 false）
  - 接入 `ratatui-image`（协议探测/渲染）、`cosmic-text`（字体栅格化）、`image`（RGBA 图像）
- 并行 Output 面板重构为“富块渲染”（Text + Image）：
  - 新增 `OutputBlock/OutputRow/ParallelOutputBuffer` 统一滚动模型
  - 新增 `ParallelOutputPane`：支持 stream 前缀列 + 图片协议渲染/占位渲染
  - 框选复制改为“所见即所得”：copy 模式不渲染图片协议，并把前缀列置空（避免破坏 Markdown 行首语义）
- Big Headers（H1/H2/H3）：
  - Rendered 模式 + 启用图片渲染 + picker 可用时，把标题渲染为图片块（2 行高）
  - 参考 mdfried：`cosmic-text` → RGBA → `ratatui-image` 协议图像
  - 加入缓存：同宽度/同文本/同协议类型不重复 encode；宽度变化会清空缓存
- stderr 前缀列与正文分离：
  - 不再把 `"[stderr]"` 拼进正文
  - stdout/stderr 的区分改由 UI 的前缀列呈现，因此 stderr 的 `#`/`>`/`-` 等行首语义不会被破坏

### 回归测试与验证
- 新增单测：
  - stderr 的 Markdown 渲染输出应与 markdown 渲染器一致（不应注入前缀）
  - Big Headers 在启用图片渲染（halfblocks）时应占用多行，并验证缓存复用
- 验证命令：
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
  - `cargo test -p ralph-core --test smoke_runner` ✅

## 2026-01-30 12:47 +0800｜回退 Markdown 渲染器：恢复 termimad（撤销 mdfrier）

### 目标回顾
- 取消使用 `mdfried/mdfrier` 的渲染链路。
- 恢复项目原本使用 `termimad` 渲染 Markdown 的方式。

### 我做了什么
- 渲染入口回退到 `termimad`：
  - `crates/ralph-adapters/src/stream_handler.rs`：
    - `Rendered` 模式下改用 `termimad::MadSkin` 渲染 Markdown。
    - stdout（Pretty）直接输出 termimad 的 ANSI，避免“ratatui::Line → ANSI”的二次转换。
    - TUI 路径保持为：termimad 输出 ANSI → `ansi-to-tui` 解析回 `ratatui::Line`。
- 清理依赖：
  - `Cargo.toml`：移除 `mdfrier`，新增 `termimad = "0.34.1"`。
  - `crates/ralph-adapters/Cargo.toml`：依赖从 `mdfrier.workspace` 切回 `termimad.workspace`。
  - `Cargo.lock`：已更新，不再包含 `mdfrier`。
- 注释同步：
  - `crates/ralph-tui/src/state/parallel.rs`：将语义换行说明从 `mdfrier` 更新为 `termimad`。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-31 17:08 +0800｜并行 TUI：输出缓冲默认 10000 行 + ralph.yml 可配置

### 目标回顾
- 并行 instance/job 输出回看窗口不要太快丢历史：
  - 默认上限从 5000 行提升到 10000 行
- 把 `max_buffer_lines` 做成 `ralph.yml` 配置项（无需改代码即可调整）

### 我做了什么
- 默认值提升：
  - `crates/ralph-tui/src/state/parallel.rs`：`ParallelTuiState::default().max_buffer_lines = 10_000`
- 配置项落地：
  - `crates/ralph-core/src/config.rs`：新增 `tui.max_buffer_lines`（默认 10_000）
- 配置注入到并行 TUI：
  - `crates/ralph-tui/src/lib.rs`：新增 `Tui::with_parallel_max_buffer_lines(...)`
  - `crates/ralph-cli/src/parallel_runner.rs`：创建并行 TUI 时注入 `config.tui.max_buffer_lines`
- `ralph init` 生成模板补充示例：
  - `crates/ralph-cli/src/init.rs`：在注释区新增 `tui.max_buffer_lines` 示例
- 顺手做了一个边界值修正：
  - `max_buffer_lines == 0` 时不再累积 `raw_lines`，避免“UI 不保留输出但内存仍无限增长”的反直觉情况。

### 验证
- `cargo fmt --check` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-31 15:40 +0800｜澄清：starting_event 语义 + 解释 parallel instance 显示 `failed`

### 你问的点
1. 你认为 `starting_event` 被忽略（指向 `crates/ralph-core/src/event_loop/mod.rs:270`）。
2. 你在 `examples/parallel-trigger-routing` 跑 parallel demo 时看到 instance 状态为 `failed`，想确认含义。

### 我做了什么（最小改良，避免改核心行为）
- 澄清并固化 `starting_event` 的“可选语义”（避免误读为“初始化事件”）：
  - `crates/ralph-core/src/event_loop/mod.rs`：更新注释，明确 “starting_event 有/无配置” 两种分支语义。
  - `crates/ralph-cli/src/loop_runner.rs`：同步更新注释，避免 CLI 层再出现相反描述。
- 给 parallel 协调者（`ralph#1`）增加更明确的 prompt 语义锚点（减少模型漂移导致的误解）：
  - `crates/ralph-core/src/parallel/supervisor.rs`：在 `KEY SEMANTICS` 里显式写出：
    - starting_event set → MUST publish it
    - starting_event unset → MUST decide entry (prefer derived candidates)

### 说明：instance 状态 `failed` 的代码口径
- `failed` 对应 `HatInstanceState::Failed`，含义是：该实例“最近一次 job 执行失败”（exit code 非 0 / timeout / cancel）。
- 它不等价于“整个 run 必然失败”，但通常意味着该实例需要排查后端/超时/中断原因。

### 我做了一个最接近你现场的复现
- 在 `examples/parallel-trigger-routing` 目录下跑了一次：
  - `../../target/release/ralph run -c ralph.yml --no-tui --plain --verbose`
- 观察到最终输出：
  - `[supervisor] final states: ... done`（未出现 failed）

### 更正（避免误导未来阅读）
- 之前 WORKLOG 中“fresh run 使用 starting_event（默认 task.start）”这句容易被理解成“starting_event 是初始化事件”。
- 更准确的表述是：
  - 初始化握手 topic 固定 `task.start`/`task.resume`
  - starting_event 是协调后的 workflow entry（可选；未配置时由 `ralph#1` 推测/决定）

### 验证（证据）
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅
- mock E2E ✅：
  - `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose`

## 2026-01-31 13:40 +0800｜重构：拆分 `ralph-e2e` Tier8 `parallel.rs`（模块目录化，降低维护成本）

### 目标回顾
- `crates/ralph-e2e/src/scenarios/parallel.rs` 行数过长（>1000 行），后续继续扩展会加剧冲突与维护成本。
- 目标是“纯重构”：不改变任何场景语义、断言口径与对外 API。

### 我做了什么
- 将 `parallel` 改为目录模块，并拆分为 4 个文件：
  - `crates/ralph-e2e/src/scenarios/parallel/mod.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/starting_event_inference.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/job_run_counts.rs`
- 维持原有导出路径：
  - `ParallelHatInstancesScenario` / `ParallelStartingEventInferenceScenario` 仍由 `scenarios::parallel` 导出
  - `JobRunCounts` / `parse_parallel_job_line` 仍可被 `parallel_trigger_routing_example` 通过 `super::parallel::{...}` 复用（可见性限制在 `crate::scenarios`）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅
- mock E2E ✅：
  - `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose`

## 2026-01-31 13:25 +0800｜新增 E2E 变体：starting_event 推测（多入口候选）

### 目标回顾
- 为 `starting_event` 未配置（由 `ralph#1` 自行推测入口事件）的语义，再补一个更贴近真实的 E2E 变体：
  - 拓扑里存在多个 derived entry candidates（例如 `spec.start` 与 `docs.start`）
  - prompt 给出明确 workflow 顺序（Planner 必须先跑），因此入口选择变得可判定、可做强断言

### 我做了什么
- 扩展 `ParallelStartingEventInferenceScenario` 为“多变体”：
  - 现有场景（单入口候选）：`parallel-starting-event-inference`
  - 新增变体（多入口候选）：`parallel-starting-event-inference-multi-candidate`
- 变体拓扑引入 `docs` 干扰 hat：
  - `docs.start → docs.done`（不在 `complete_publishes` 内）
  - 期望 `ralph#1` 仍选择触发 Planner 的入口（`spec.start`），并完成 `spec.start → build.task → build.done` 闭环
- 录制并登记 mock cassette：
  - `cassettes/e2e/parallel-starting-event-inference-multi-candidate-codex.jsonl`

### 变更文件
- `crates/ralph-e2e/src/scenarios/parallel.rs`：新增 `MultiCandidate` 变体与断言（含 `docs.*` 未使用断言）
- `crates/ralph-e2e/src/main.rs`：注册新变体场景
- `specs/e2e-starting-event-inference.spec.md`：补充变体需求与 cassette 约定
- `crates/ralph-e2e/README.md`：Tier8 场景列表补充变体说明
- `cassettes/e2e/README.md`：登记新 cassette

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅
- live E2E（Codex）✅：
  - `cargo run -p ralph-e2e -- codex --filter parallel-starting-event-inference-multi-candidate --skip-analysis --keep-workspace --verbose`
- mock E2E（cassette）✅：
  - `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference-multi-candidate --verbose`

## 2026-01-31 12:20 +0800｜新增 E2E：starting_event 未配置时 ralph#1 入口推测（parallel）

### 目标回顾
- 当 `event_loop.starting_event` 未设置时，应由 `ralph#1` 基于 hats 拓扑推测并发布 workflow entry event。
- 需要一个端到端回归场景，覆盖“入口推测 + 触发链路 + 收敛到 LOOP_COMPLETE”。

### 我做了什么
- 新增 spec：`specs/e2e-starting-event-inference.spec.md`（定义可测口径与验收标准）。
- 新增 ralph-e2e 场景：
  - `ParallelStartingEventInferenceScenario`（id：`parallel-starting-event-inference`，Codex only）
  - 断言点：
    - `task.start` 后 `ralph#1` 的第一个 workflow entry event 必须是 `spec.start`
    - 事件链路包含 `spec.start` → `build.task` → `build.done`
    - 检测到 `LOOP_COMPLETE`
- 录制 cassette + 打通 mock-mode：
  - 新增 `cassettes/e2e/parallel-starting-event-inference-codex.jsonl`
  - 修复 `ralph-e2e mock-cli`：支持“按调用次数分段回放”（否则 parallel 下 `ralph#1` 多 job 会导致 `LOOP_COMPLETE` 提前回放、workflow 中断）。

### 验证（证据）
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- live E2E（Codex）✅：
  - `cargo run -p ralph-e2e -- codex --filter parallel-starting-event-inference --skip-analysis --verbose --keep-workspace`
- mock E2E（cassette）✅：
  - `cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose`

## 2026-01-31 03:02 +0800｜按你的反馈回退 starting_event 语义：初始化固定 task.start，入口事件由 ralph#1 决策

### 你指出的问题
- 我之前把 `event_loop.starting_event` 当成了 fresh run 的“初始化事件 topic”（并在 `EventLoop::initialize()` 里使用它）。
- 你明确要求：`starting_event` 未设置时，就由 `ralph#1` 自行决定；而不是默认替你选 `task.start` 或把它当作“第一事件”。

### 我做了什么
- 语义回退（按设计对齐）：
  - `EventLoop::initialize()` fresh run 始终发布 `task.start`（`starting_event` 不再影响初始化事件 topic）。
  - `loop_runner` 的 debug event logger 同步修正：fresh run 记录的初始事件也固定为 `task.start`。
- prompt 增强（让 ralph#1 更清楚如何处理）：
  - `starting_event` 已设置：提示 ralph#1 “协调后优先发布该入口事件启动 workflow”。
  - `starting_event` 未设置：提示 ralph#1 “必须自行决定第一次 delegation 的入口事件”，并给出启发式候选入口事件列表（订阅但未被任何 hat 发布的事件）。
- README 同步：
  - 把 `starting_event` 从“First event published”改为“协调后入口事件（不是 first event）”，并修正示例配置。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-30 23:03 +0800｜continuous-learning：四文件摘要 + 归档清理（保持工作区干净）

### 我做了什么
- 在 `notes.md` 追加“四文件摘要（用于决定是否提取 skill）”，并确认 termimad 的 H1 默认居中属于可复用踩坑点。
- 新增一个全局可复用 skill：
  - `self-learning.termimad-h1-left-align`
  - 位置：`/Users/cuiluming/.codex/skills/self-learning.termimad-h1-left-align/SKILL.md`
- 归档历史文件，减少根目录噪音：
  - `notes_*.md` / `task_plan_*.md` → `archive/`
  - `WORKLOG_2026-01-29_1908.md` / `WORKLOG_2026-01-29_2022.md` → `archive/`（并同步更新 `WORKLOG.md` 引用路径）
- 删除重复的未跟踪 example：`examples/parallel-trigger-routing2/`

### 提交
- `f4de8c5`：`chore: archive session notes and plans`

## 2026-01-30 22:22 +0800｜termimad：H1 标题改为左对齐（取消居中）

### 你要的效果
- `termimad` 渲染 Markdown 时，H1（`# Title`）不再居中，改为靠左对齐。

### 我做了什么
- `crates/ralph-adapters/src/stream_handler.rs`：
  - 新增 `default_markdown_skin()`：基于 `MadSkin::default()`，把 `headers[0].align` 从 `Center` 改为 `Left`。
  - stdout（Pretty 输出）与 TUI（`ratatui::Line`）两条渲染路径统一使用该 skin，避免“一个左对齐一个仍居中”的分裂体验。
  - 新增回归测试：`markdown_h1_is_left_aligned_in_rendered_mode`，防止未来升级 termimad 或重构时把 H1 又改回居中。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅

## 2026-01-30 13:34 +0800｜彻底回退 mdfried 相关功能：移除 Big Headers/图片渲染 + 移除左侧红色 E + 许可证回退 MIT

### 目标回顾
- 彻底移除 Big Headers/图片渲染等 `mdfried` 相关特性（回到纯文本 Output）。
- 并行 Output 面板不再显示左侧红色 `E`（stderr 用灰色弱化区分即可）。
- 许可证从 `GPL-3.0-or-later` 回退到 `MIT`，并同步更新文档与元数据。

### 我做了什么
- 移除 Big Headers/图片渲染：
  - 删除并行输出的 Image 相关结构与渲染逻辑，输出 buffer 回到“纯文本行”模型。
  - 移除 `ratatui-image` / `cosmic-text` / `image` 依赖与相关代码。
  - 移除 `tui.images.*` 配置项与 CLI/TUI 传递链路。
- 移除 Output 左侧红色 `E`：
  - `ParallelOutputPane` 不再渲染任何左侧前缀列，stderr 仅通过 `MUTED_FG`（灰色）弱化呈现。
- 许可证回退：
  - `Cargo.toml`：`workspace.package.license = "MIT"`
  - 根目录 `LICENSE`：替换为 MIT License 文本
  - README/docs：许可证 badge 与说明同步改为 MIT

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

## 2026-01-31 02:35 +0800｜理性整合上游提交：backend args / hats graph / presets / events / scratchpad（Mermaid ASCII 改用 beautiful-mermaid-rs）

### 目标回顾
- 将你指定的多个 commit 的“高价值行为”整合进当前主线，避免原样搬运上游实现细节。
- `ralph hats graph` 的 ascii/unicode/compact 必须使用 `/Users/cuiluming/local_doc/l_dev/my/rust/beautiful-mermaid-rs` 做确定性渲染。
- 通过全量验证（fmt/clippy/test + replay smoke tests），用测试做背压门。

### 我做了什么（按能力分组）

- backend args（run 级别 + per-hat 级别）：
  - `ralph run -- <BACKEND_ARGS...>`：支持把 trailing args 追加到 backend 命令行。
  - `HatBackend` 扩展为支持 args：
    - `NamedWithArgs { backend_type, args }`
    - `KiroAgent { type, agent, args }`
    - `Custom { command, args }`
  - 串行 PTY 模式支持“每轮切换 backend”（避免 backend 在首轮锁死）。
  - 统一 hat-level backend 生效优先级：优先使用 hat backend，失败回退全局 backend，并保持 timeout 配置按 backend 名生效。

- starting_event 修复：
  - `event_loop.starting_event` 不再被忽略：
    - fresh run：使用 `starting_event`（默认 `task.start`）
    - resume：固定为 `task.resume`

- events JSONL 正确性改良：
  - `events.jsonl` 写入改为“整行 JSON + 换行一次性追加写入”，降低半行 JSON 的概率。

- scratchpad 行为组（清理 + 自动注入）：
  - fresh run 会清理旧 scratchpad 内容（truncate 为空，而不是删除文件），避免 stale state 误导本轮目标。
  - scratchpad 内容会自动注入 prompt（带字符预算 + tail 保留），减少 agent 每轮自行读取 scratchpad 的重复动作。

- hats 可视化（确定性、可测试、离线可用）：
  - 新增/完善 `ralph hats`：list/show/validate/graph。
  - `graph --format ascii/unicode/compact`：从 Mermaid 文本生成，再用 `beautiful-mermaid-rs` 渲染（去掉 AI backend 画图逻辑）。

- presets 镜像同步：
  - `scripts/sync-embedded-files.sh` 支持把 `/presets/**` 镜像到 `crates/ralph-cli/presets/**`，保证 `cargo install` 也能拿到相同 presets。

- prompt 省 token（active hat 场景）：
  - 当存在 active hat 时，Ralph prompt 输出 `## ACTIVE HAT` + `### Event Publishing Guide`，跳过 `## HATS` 全量拓扑与 Mermaid（更聚焦、更省 token）。

### 文档同步
- `README.md`：
  - 更新 `ralph hats graph` 示例：移除 `--backend`（现在不需要 backend 也能渲染 ascii/unicode）。
  - 补充 `ralph run -- <BACKEND_ARGS...>` 的说明。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅
