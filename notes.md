# 笔记：mdfried 风格渲染差异与技术要点

## 现象（你截图里看到的差别）
- 你看到的内容大量来自 `stderr`（每行都有 `"[stderr]"` 前缀）。
- Ralph 并行 TUI 对 `stderr` 默认走 Plain 模式，不做 Markdown 渲染，并且还会把行整体弱化成灰色。
- 所以看起来像“纯文本”，标题/列表/引用不会被渲染成结构化样式。

## 关键本质：mdfried ≠ mdfrier
- `mdfried` 是完整的 Markdown viewer：
  - 大标题（Big Headers™）
  - 图片内联（多图形协议）
- `mdfrier` 是一个“解析 + 语义换行 + 输出 span/style 信息”的库：
  - 它本身不负责把标题变大、也不负责图片协议。
- 结论：要达到 `mdfried` 的视觉效果，必须引入 **图片/图形协议渲染层**（例如 `ratatui-image`）。

## 技术组件候选（方案 A）
- `ratatui-image`
  - 负责探测终端协议（Kitty / iTerm2 / Sixel）与字体像素尺寸
  - 提供 Image/StatefulImage widgets
  - fallback：halfblocks /（可选）Chafa
- Header 放大策略（可能二选一或混合）：
  - Kitty text sizing protocol（若可用）：直接缩放文字
  - 否则：把标题渲染成图片（需要字体栅格化）并用图形协议绘制

## 设计警告（对 Ralph 的影响）
- 这会把“输出视图”从纯文本渲染，升级成“文本 + 图片块”的混合渲染。
- 会影响：
  - 软换行/滚动（图片块高度不是 1 行）
  - 框选复制（图片不可复制，需要定义复制语义）
  - 性能（StatefulImage 的 resize/encode 可能阻塞，需要后台线程/缓存）

## 2026-01-30 03:40 +0800 进展总结（本次实现后的结论）
- 现在并行 Supervisor 的 Output 面板已经支持“Text + Image”的统一滚动模型。
- `stderr` 不再被强制当作 Plain，也不再把流标识拼到正文里。
  - 流标识改为 UI 前缀列渲染，因此 Markdown 行首语义（`#`/`>`/`-`）能正常工作。
- Big Headers 的实现路径与 `mdfried` 一致：
  - 用 `cosmic-text` 把 H1/H2/H3 栅格化为 RGBA，再用 `ratatui-image` 编码成协议图像。
  - 同宽度/同文本会命中缓存，避免重复 encode。
- 还没做的部分：`![]()` 图片内联（已留好开关与数据结构，等下一轮实现）。

## 2026-01-30 12:47 +0800｜决策：取消 mdfried/mdfrier，回退 termimad

- 你决定：不再使用 `mdfrier`（参考 `mdfried`）来渲染 Markdown。
- 我做的回退（最小回退，优先恢复原本渲染器）：
  - stdout（`PrettyStreamHandler`）：`termimad` 直接把 Markdown 渲染成 ANSI 字符串并写入 stdout。
  - TUI（`render_text_to_lines`）：`termimad` 先输出 ANSI，再用 `ansi-to-tui` 解析回 `ratatui::Line`。
- 依赖变化：
  - workspace：移除 `mdfrier`，新增 `termimad = 0.34.1`（其内部依赖 `minimad`）。
- 影响提醒：
  - 既然不再依赖 `mdfrier`，仓库是否还需要保持 `GPL-3.0-or-later` 可以另开任务再决定。

## 2026-01-30 13:34 +0800｜执行：彻底回退 Big Headers/图片渲染 + 许可证回退 + 移除左侧红色 E

- 你选择了“彻底回退”（方案 A）：
  - 移除了 Big Headers / 图片块等 `mdfried` 相关渲染特性
  - 并行 Output 面板不再渲染左侧红色 `E` 前缀列（stderr 仅用灰色弱化区分）
- 许可证结论：
  - 已把仓库许可证从 `GPL-3.0-or-later` 回退到 `MIT`

## 2026-01-30 22:17 +0800｜termimad H1 对齐：默认居中 → 改为左对齐

- `termimad` 的默认皮肤在 `impl Default for MadSkin` 里显式设置了：
  - `skin.headers[0].align = Alignment::Center;`
  - 这会导致 H1（`# Title`）被居中渲染，并在左侧填充空格。
- `termimad` 已对外 re-export 了 `Alignment`：
  - 可以直接使用 `termimad::Alignment::Left`，无需直接依赖 `minimad`。
- 预期落点（本仓库）：
  - `crates/ralph-adapters/src/stream_handler.rs`
  - 将两处 `MadSkin::default()` 统一替换为“自定义 skin builder”，并只改 H1 的 `align`。

## 2026-01-30 22:54 +0800｜四文件摘要（用于决定是否提取 skill）
- 任务目标（task_plan.md）：
  - `termimad` 渲染 Markdown 时，H1（`# Title`）从“居中”改为“靠左对齐”，且 stdout/TUI 两条路径一致。
- 关键决定（task_plan.md）：
  - 不改上游 `termimad`，在本仓库统一封装 `default_markdown_skin()` 覆盖默认 H1 对齐。
- 关键发现（notes.md）：
  - `MadSkin::default()` 会把 `headers[0].align` 设为 `Alignment::Center`，因此 H1 左侧会被填充空格。
- 实际变更（WORKLOG.md）：
  - 在 `crates/ralph-adapters/src/stream_handler.rs` 增加 skin builder，并在 stdout/TUI 渲染链路统一使用；补回归测试锁定行为。
- 错误与根因（ERRORFIX.md，如有）：
  - 本次无新增错误；根因属于 `termimad` 默认样式选择（H1 居中）。
- 可复用点候选（1-3 条）：
  1. `termimad`：如果你需要标题不居中，优先用“自定义 MadSkin”覆写 `headers[0].align`，而不是在渲染后做字符串裁剪。
  2. 渲染一致性：stdout 与 TUI 必须复用同一套 skin builder，避免“一个左对齐一个仍居中”的体验分裂。
- 是否需要固化到 docs/specs：否（属于局部 UI 行为改良，已有回归测试锁定）。
- 是否提取/更新 skill：是（提取 `self-learning.termimad-h1-left-align`，作为 termimad 的常见坑/默认行为备忘）。

---

# 笔记：多提交理性整合（backend args / hats 拓扑图 / presets / per-hat backend args / scratchpad）

## 2026-01-31 01:30 +0800｜提交盘点（按你给的 hash）

### 988541883f328b897b034cbb0f8dbc8bc6046a9c（feat(cli): ralph run 支持自定义 backend args）
- 价值点：
  - `ralph run -b <backend> -- <custom args...>` 这种“按次覆盖参数”的能力，很实用。
  - 逻辑简单：把 `custom_args` 追加到 `CliBackend.args` 后再创建 executor。
- 风险/注意：
  - 该改动与后续 `887ea99`（per-hat backend args）会发生“功能重叠”，需要避免重复实现/重复字段。
- 我的整合策略：
  - 保留该行为，但最终以“统一后的 backend args 合成逻辑”为准（run 级别 custom_args + hat/backend 级别 args + config 默认 args 的优先级/拼接顺序要可测）。

### 26f2364566fbe1d35880d889b836e5b55d343301（feat: ralph hats CLI + topology 可视化）
- 价值点：
  - 增加 `ralph hats` 命令：list/show/validate/graph，属于高价值的可观测性工具。
  - 提供 Mermaid 拓扑输出（`--format mermaid`）便于外部渲染。
- 明显不合适的点（需要改写）：
  - `ascii/unicode/compact` 竟然通过“调用 AI backend”生成 ASCII 图，并且测试被 `#[ignore]`（需要 live backend）。
  - 这会导致：不可复现、CI 无法稳定测试、离线不可用。
- 你的明确要求：
  - Mermaid ASCII 绘制必须改用 `/Users/cuiluming/local_doc/l_dev/my/rust/beautiful-mermaid-rs`。
- 我的整合策略：
  - 保留 Mermaid 生成（deterministic），再用 `beautiful_mermaid_rs::render_mermaid_ascii()` 做 ascii/unicode/compact。
  - `compact` 先用更小 padding 的渲染参数实现（至少可用 + 可测）。
  - 删除/避免 “AI 画图” 逻辑与 `#[ignore]` 测试（让测试可在 CI 稳定跑）。

### ec58e14bb6f95aa8b705f478881a9d754315219e（feat(presets): 新预设 + 工作流改良）
- 价值点：
  - 增补 `bugfix` / `code-assist` / `pdd-to-code-assist` 等预设；并同步脚本 `sync-embedded-files.sh`。
  - 属于“内容型改良”，对 CLI 用户体验提升大。
- 风险/注意：
  - 变更量大但主要是 YAML/文档；冲突多半来自我们本地 README/docs 改动，需要小心合并。

### 887ea9972c9877f72e20f3e60a821d32b5a249c7（feat(config): per-hat backend 支持 args）
- 价值点：
  - `HatBackend` 支持对象形式 `type + args`，并保持字符串形式兼容。
  - KiroAgent 支持 `args`；新增 `NamedWithArgs` 以支持 `claude/gemini/...` 这类命名 backend 的额外参数。
- 风险/注意：
  - 该提交同时改了 `.agent/memories.md`、`.agent/tasks.jsonl`（属于“状态类内容”，不一定要跟上游同步）。
- 我的整合策略：
  - 重点合入 `HatBackend` 的结构与解析测试、`CliBackend` 的构造支持。
  - 对 `.agent/*` 的改动：默认不引入（除非它们对行为/测试有刚性依赖）。

### 70f224b4f61bfa6e6862236ce5ccb7b006765886（fix: 真正 honor hat-level backend 配置 + starting_event bug）
- 价值点（强烈建议采用）：
  - 修复“hat-level backend 配置完全不生效”的关键 bug，尤其是 PTY 模式下需要能动态切换 backend（`PtyExecutor::set_backend()`）。
  - 修复 `event_loop.starting_event` 被忽略的问题（initialize 硬编码 `task.start`）。
- 噪音/不建议原样引入：
  - `.reviews/**`、`BUG_ANALYSIS.md`、`BUG_EXAMPLE.md` 这类大文档会显著污染仓库根目录（除非你明确要保留）。
- 我的整合策略：
  - 合入代码层面的 fix（loop_runner / event_loop / pty_executor）。
  - 文档类产物先不合入；把关键结论（根因/修复点）在本仓库 `ERRORFIX.md` 里记录即可。

### eb1f7e0e4ea585bbefd895b70c2a0959bcc0c02d（fix(events): JSONL 原子写）
- 价值点：
  - 多进程并发 append 时避免 JSONL 行破损，属于“正确性修复”。
- 我的整合策略：
  - 直接合入（低风险高价值）。

### 413dae5675a91fa7b3cdf5479accc9f747480c75（fix(loop): fresh run 清理 scratchpad）
- 价值点：
  - `ralph run` 新目标启动时清掉旧 scratchpad，避免 stale state。
- 注意：
  - 与 `e1727dc` 的“scratchpad 自动注入 prompt”是强相关的一组行为，需要一起校验。

### 0fc152cf6a8ec53e4f0f25d3259905ae36d94d29（feat(hats): Event Publishing Guide + hat active 时跳过 topology）
- 初步判断：有价值，建议采用。
  - Event Publishing Guide 能用更少 token 解释“publish 会触发谁”，减少 hats 对 Mermaid 图的依赖。
  - hat active 时跳过全 topology，能显著省 token（并且更聚焦）。
- 需要确认：
  - 与我们本地 prompt 改动（尤其是 scratchpad 章节）是否有冲突；以“结构清晰 + 测试覆盖”作为合并标准。

### e1727dcb39c4f389d2137bb11694665a6487aaac（feat: scratchpad 内容自动注入 prompt）
- 价值点：
  - 每轮少一次 tool call，减少 agent 自行读取 scratchpad 的重复动作。
  - 有预算截断逻辑，且保留 tail（最近内容），符合“状态应以最新为准”的直觉。
- 风险/注意：
  - 如果配合 `fresh run 清理 scratchpad`，要保证：
    - `run` 清理后 prompt 不会插入空 scratchpad section
    - `resume` 不清理，并能正确注入已有 scratchpad

## 2026-01-31 02:35 +0800｜整合落地结果（最终）

### 已落地的价值点（按主题归类）

- backend args（`9885418` + `887ea99` + `70f224b`）：
  - `ralph run -- <BACKEND_ARGS...>`：支持“按次追加”backend 参数。
  - per-hat backend 支持 args：`NamedWithArgs` / `KiroAgent.args` / `Custom.args`。
  - 串行 PTY 模式下每轮可切换 backend（避免首轮锁死）。
  - `starting_event` 现在会被 honor（fresh run），resume 固定 `task.resume`。

- hats 拓扑可视化（`26f2364`，按你要求改写）：
  - `ralph hats graph` 的 ascii/unicode/compact 现在是“Mermaid → beautiful-mermaid-rs”的确定性渲染。
  - 不再依赖 AI backend 画图（CI 可测、离线可用）。

- presets（`ec58e14`）：
  - `scripts/sync-embedded-files.sh` 已支持把 `/presets/**` 镜像到 `crates/ralph-cli/presets/**`。
  - 新增预设（例如 `bugfix.yml` / `code-assist.yml` / `pdd-to-code-assist.yml`）已同步进 `crates/ralph-cli/presets/`。

- events（`eb1f7e0`）：
  - events.jsonl 写入改为“整行 JSON + 换行一次性追加”，降低半行 JSON 的概率。

- scratchpad（`413dae5` + `e1727dc`）：
  - fresh run 会清理旧 scratchpad 内容（truncate 为空，而不是删除文件）。
  - scratchpad 内容会自动注入到 prompt（带预算截断 + tail 保留）。

- hatless prompt 省 token（采纳 `0fc152c` 的价值点）：
  - 当存在 active hat 时，输出 `## ACTIVE HAT` + `### Event Publishing Guide`，跳过 `## HATS` 全量拓扑与 Mermaid。

### 已同步文档
- README：更新 `ralph hats graph` 示例（移除 `--backend`），并补充 `ralph run -- <BACKEND_ARGS...>`。

### 已知风险/待决策
- `beautiful-mermaid-rs` 当前以本机绝对路径依赖接入：
  - 优点：你本机立刻可用、实现简单。
  - 风险：CI/他人环境无法编译。
  - 后续如果要“团队/CI 可编译”，建议把它改为：git 依赖 / submodule / 或 vendoring 到本仓库 workspace。

## 2026-01-31 03:02 +0800｜starting_event 语义澄清与回退（按你的要求）

- 你指出我之前把 `starting_event` 当作“初始化事件 topic”的改动不符合你的设计。
- 我重新对齐了项目文档/注释语义（`EventLoopConfig.starting_event` 的注释本身就写了：未设置时由 ralph 决定）。

### 最终语义（我已按此实现）
- fresh run：
  - 初始化事件固定为 `task.start`（用于把用户目标作为 top-level prompt 注入上下文）
  - `starting_event` 仅用于提示 ralph#1 “协调后优先发布哪个工作流入口事件”
- `starting_event` 未设置：
  - 明确由 ralph#1 自行决定第一次 delegation 的入口事件
  - prompt 中增加了明确指引（并提供启发式候选入口事件列表，帮助快速决策）

## 2026-01-31 11:27 +0800｜四文件摘要（用于 continuous-learning）

- 任务目标（task_plan.md）：
  - 理性整合多提交价值点（backend args / hats graph / presets / events / scratchpad），并强制 Mermaid ASCII 使用 `beautiful-mermaid-rs`。
  - 按你的反馈回退 starting_event 语义：starting_event 未设置时由 ralph#1 决策；不能把 starting_event 当初始化事件。
- 关键决定（task_plan.md）：
  - “价值整合而非代码搬运”，并用全量测试背压（fmt/clippy/test/smoke）。
  - 初始化事件语义固定：fresh run 永远 `task.start`；starting_event 只是协调后入口提示。
- 关键发现（notes.md）：
  - 任何依赖 AI 生成的“图/输出”都会带来不可复现与 CI 不稳定；必须改成确定性渲染。
  - fresh run 如果删除 scratchpad 文件，会直接破坏 `run --continue`（尤其在 mock backend/无 agent 产物时）。
  - starting_event 语义最容易被误读：它不是 first event，而是 workflow entry event after coordination。
- 实际变更（WORKLOG.md）：
  - 增强 hats 可视化与确定性渲染；per-hat backend args；events JSONL 原子写；scratchpad 自动注入与清理策略；starting_event 语义回退 + prompt 指引 + README 同步。
- 错误与根因（ERRORFIX.md，如有）：
  - “starting_event 被当作初始化事件”的语义错误（概念混淆）。
  - “fresh run 删除 scratchpad 导致 continue 失败”的行为错误（清理策略不当）。
- 可复用点候选（1-3 条）：
  1. **starting_event 语义**：fresh run 初始化必须是 `task.start`；starting_event 仅是协调后入口提示；未设置时由 ralph#1 从拓扑/目标自行选择入口事件。
  2. **scratchpad 清理策略**：fresh run 要“清空内容而不是删除文件”，否则会破坏 continue/resume 流程与测试稳定性。
  3. **确定性渲染优先**：CLI 工具输出（尤其 diagram）必须 deterministic，避免引入“需要 live backend 才能跑”的测试。
- 是否需要固化到 docs/specs：否（本次已把最关键的语义纠偏写进 README；更完整的语义已有 `docs/concepts/hats-and-events.md`）
- 是否提取/更新 skill：是（项目级 skills，避免以后再次误改语义/重复踩坑）

---

## 2026-01-31 12:20 +0800｜E2E：starting_event 未配置时的入口推测（parallel）+ mock 回放分段

### 目标回顾

- `event_loop.starting_event` 未设置时，不应该由程序“替 LLM 选入口事件”。
- 应该由 `ralph#1` 基于 hats 拓扑推测入口事件并发布，从而启动 workflow。

### 本次落地内容（可复用点）

1. 新增 parallel E2E 场景（Codex）
   - id：`parallel-starting-event-inference`
   - 核心断言：`task.start` 之后，`ralph#1` 的第一个 workflow entry event 必须是 `spec.start`（候选集退化为单元素，稳定可测）。
2. 录制 cassette 并验证 mock-mode
   - cassette：`cassettes/e2e/parallel-starting-event-inference-codex.jsonl`
   - mock：`cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference`
3. 修复 mock-cli 的“多轮调用回放”能力（关键）
   - 问题：parallel 下 `ralph#1` 会有多个 job；旧 mock-cli 会把同一 instance 的全部输出一次性回放，导致 `LOOP_COMPLETE` 提前出现，workflow 中断。
   - 修复：mock-cli 引入“按调用次数分段回放”
     - 顺序模式：按 `_meta.iteration` 分段
     - 并行模式：按 `bus.publish.source_instance==instance` 的经验边界分段
   - 状态存储：workspace 内 `.ralph/mock-cli/*.count`（每个 instance 单独计数）

---

## 2026-01-31 13:25 +0800｜E2E 变体：starting_event 推测（多入口候选，仍可判定）

### 为什么要做这个变体
- 之前的 `parallel-starting-event-inference` 把 derived entry candidates 退化为单元素（`spec.start`），非常稳定，但也偏“理想化”。
- 真实项目里经常会存在：
  - 多个入口候选（多个 trigger 未被任何 hat publish）
  - 以及一些“不是本次 workflow 需要”的 hat
- 这个变体的目标是：**让入口选择更贴近真实，但仍然保持可判定、可做强断言**。

### 变体设计（核心点）
- 新增一个 `docs` 干扰 hat：
  - `docs.start → docs.done`
  - 它不是 `complete_publishes`，因此不应成为本次 workflow 的入口
- prompt 明确 workflow 顺序：
  - Planner 必须先跑，再跑 Builder
  - 因此 `ralph#1` 必须选择能触发 Planner 的入口（也就是 `spec.start`）

### 录制 cassette 的实用流程（本次采用）
1. 先跑 live E2E，生成并保留 workspace（便于复用 `ralph.yml`）：
   - `cargo run -p ralph-e2e -- codex --filter parallel-starting-event-inference-multi-candidate --skip-analysis --keep-workspace --verbose`
2. 在该 workspace 下写一个 `prompt.md`（避免命令行转义换行）
3. 用 `ralph run --record-session` 直接录制到 `cassettes/e2e/`：
   - `../../target/release/ralph run -c ralph.yml --no-tui --max-iterations 20 --record-session ../../cassettes/e2e/parallel-starting-event-inference-multi-candidate-codex.jsonl -p @prompt.md`

### 小提醒：filter 是子串匹配
- `ralph-e2e --filter parallel-starting-event-inference` 会同时匹配：
  - `parallel-starting-event-inference`
  - `parallel-starting-event-inference-multi-candidate`
  这在一次性复核两个场景时很方便，但如果你只想跑其中一个，要把 filter 写得更精确。

---

## 2026-01-31 13:40 +0800｜重构记录：Rust 模块拆分时的可见性与 re-export 约束

### 现象
- 我们把 `crates/ralph-e2e/src/scenarios/parallel.rs` 拆成了目录模块（`parallel/mod.rs + 子模块`）。
- 拆分后仍希望保持：
  - 场景对外导出路径不变（`scenarios::parallel::ParallelHatInstancesScenario` 等）
  - `parallel_trigger_routing_example.rs` 继续能 `use super::parallel::{JobRunCounts, parse_parallel_job_line};`

### 关键点（容易踩坑）
- 当 helper 被移动到 `parallel/job_run_counts.rs` 这类“子模块”后：
  - 原先的 `pub(super)` 语义会发生变化（`super` 变成 `parallel`，而不是 `scenarios`）。
- 为了避免 helper 可见性泄露到整个 crate，同时又能让 `scenarios` 内部复用：
  - 使用 `pub(in crate::scenarios)` 是一个很稳的折中。
  - 然后在 `parallel/mod.rs` 里用同样的可见性做 re-export，保持原 import 路径不变。

### 本次结论
- 这类“纯重构”优先保证：
  - import 路径不变（减少改动扩散）
  - 可见性不扩大（避免把内部 helper 意外变成公共 API）

---

## 2026-01-31 15:40 +0800｜语义对齐：starting_event + parallel instance=failed

### starting_event：对齐后的语义（用于排除“被忽略”的误读）

- 控制面握手事件（runtime handshake）固定：
  - fresh run：`task.start`
  - resume run：`task.resume`
- `event_loop.starting_event` 是 **可选** 的 “协调后 workflow entry event”：
  - **配置了 starting_event**：协调者（parallel 时为 `ralph#1`）必须优先发布该 topic 作为 workflow entry
  - **未配置 starting_event**：由协调者（parallel 时为 `ralph#1`）基于目标与 hats 拓扑自行决定 workflow entry

### parallel instance 显示 `failed`：代码口径

- `failed` 是 `HatInstanceState::Failed`（协议层：`crates/ralph-proto/src/hat.rs`）。
- 触发条件是“该 instance 最近一次 job 执行失败”，常见原因包括：
  - 后端 CLI 进程 exit code 非 0
  - job 超时（timed_out=true）
  - job 被取消（canceled=true）
- 映射逻辑在 `crates/ralph-core/src/parallel/instance.rs`：
  - `HatJobResult.success == true` → state=Idle
  - `HatJobResult.success == false` → state=Failed
  - 说明：`failed` 不是“整个 run 已失败”的同义词，而是“该实例上一轮 job 失败”的状态标签。

### 本机复现（用于你问的 demo）

- 我在 `examples/parallel-trigger-routing` 下跑了一次 demo：
  - `../../target/release/ralph run -c ralph.yml --no-tui --plain --verbose`
  - 该次 run 最终输出了 `[supervisor] final states: ... done`（未出现 failed）
  - 结论：如果你看到 `failed`，更像是“某次 job 实际失败（exit/timeout/cancel）”，而不是 starting_event 语义导致的必现问题

### 是否需要提取 skill（continuous-learning 决策）

- 暂不提取新的 `self-learning.*`：
  - 这次属于“语义澄清 + prompt 文案更明确”，可复用点较少且不够“踩坑级别”。
  - 但我们已把关键信息追加到四文件，后续如果同类问题频繁出现，再考虑提炼为 skill 或补充到 docs。

---

## 2026-01-31 17:08 +0800｜并行 TUI：max_buffer_lines（输出回看窗口）默认值与配置注入点

### 结论（我最终采用的语义）
- `max_buffer_lines` 是 **并行 Supervisor TUI** 的“每个 job 输出回看窗口上限（按逻辑行）”。
- 它只影响 TUI 内存窗口（回看/搜索），不会影响：
  - `.ralph/events*.jsonl`（事件日志）
  - `--record-session`（cassette / 回放录制文件）

### 默认值在哪里
- `crates/ralph-tui/src/state/parallel.rs`：
  - `ParallelTuiState::default().max_buffer_lines = 10_000`

### 配置项在哪里（ralph.yml）
- `crates/ralph-core/src/config.rs`：
  - `tui.max_buffer_lines`（`TuiConfig::max_buffer_lines`）
  - 默认值：`10_000`

示例（ralph.yml）：
```yaml
tui:
  max_buffer_lines: 10000
```

### 配置如何注入到 TUI（关键链路）
- `crates/ralph-cli/src/parallel_runner.rs`：
  - 创建 `Tui::new_parallel()` 时调用：`with_parallel_max_buffer_lines(config.tui.max_buffer_lines)`
- `crates/ralph-tui/src/lib.rs`：
  - 新增 `Tui::with_parallel_max_buffer_lines(...)`，把值写入 `state.parallel.max_buffer_lines`

### 边界值（0）的安全性
- 由于该字段变成可配置项，理论上用户可能设置 `0`。
- 我在 `crates/ralph-tui/src/state/parallel.rs` 里补了保护：
  - `max_buffer_lines == 0` 时不再累积 `raw_lines`，避免“看起来不保留输出，但实际 raw_lines 无限增长”的反直觉内存占用。

---

## 2026-01-31 22:18 +0800｜合并 `for_marge` 分支：合并前侦察（preflight）

### 当前分支与工作区
- 当前在 `main`，工作区干净（无未提交修改）。

### `for_marge` 分支状态
- `for_marge` 是本地分支，并且被另一个 worktree 检出：
  - `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-orchestrator-for_marge`

### 分支分叉点与提交概览
- `merge-base(main, for_marge) = 005e1230...`
- `for_marge` 领先的提交只有 2 个（但其中 1 个提交体量很大）：
  - `68ccc0d`：`ui 调整`（包含 TUI 代码、以及大量四文件/档案文件改动）
  - `3ccf9eb`：`fix(tui): Alacritty 边框高亮 + 入场动画错峰`

### 风险点（需要特别留意）
- `68ccc0d` 修改了 `task_plan.md / notes.md / WORKLOG.md / ERRORFIX.md` 等“会话记录类文件”。
- `main` 这两天也在同一批文件上频繁追加内容。
- 因此合并时大概率会在这些文件上产生冲突，需要我手动做一次“合并日志”整理。

---

## 2026-01-31 22:45 +0800｜合并 `for_marge` 分支：结果与验证

### 合并结果（落地状态）
- 已完成 `for_marge` → `main` 合并，并产生 merge commit：`5f8f58c`（`Merge branch 'for_marge'`）。
- 会话记录类文件的冲突处理采用“保留 main，新增文件照收”的策略：
  - 这样避免把两边的日志内容强行揉在一起，导致阅读体验下降。
  - `for_marge` 带来的历史文件（例如 `notes_2026-01-30_1623.md` / `task_plan_2026-01-30_2147.md` / `WORKLOG_2026-01-30_1525.md`）仍保留在仓库中。

### 代码层面的关键整合点（我认为最重要的）
- `ralph-tui` 引入并落地 `TuiTheme`（Catppuccin Mocha）与 exabind 风格面板：
  - 统一 `panel_block(...)` 构造面板外观。
  - 通过 `patch_exabind_panel_border_bg(...)` 修正边框块元素在部分终端（如 Alacritty/Warp）下的外圈背景细节。
- `ContentPane` 升级为显式接收 `TuiTheme`：
  - 重点是“先铺底，再渲染”，避免渲染逻辑把 panel 的底色刷回 `Reset`，从而在透明背景/动画下出现闪烁或外圈被污染。
- 并行输出仍使用 `ParallelOutputPane`（未切换为 `IterationBuffer`）：
  - 原因是 `main` 的并行输出 buffer 类型与 `for_marge` 不同（`ParallelOutputBuffer` + raw_lines 重渲染链路）。
- 为了让旧调用点先不炸，我在主题模块里补了兼容常量：
  - `crates/ralph-tui/src/theme.rs`：`pub const MUTED_FG: Color = ...`（建议后续逐步替换为 `theme.muted()`）。

### 验证（背压）
- 全量验证已通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`

---

## 2026-02-01 00:34 +0800｜Markdown 内部配色：Monokai Pro（termimad）

### 结论：配色落点应该在 `default_markdown_skin()`
- Ralph 的 Markdown 渲染（stdout 与 TUI）都复用 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()`：
  - stdout：`PrettyStreamHandler::flush_text_buffer()` → `skin.text(...).to_string()`
  - TUI：`render_markdown_to_lines()` → `skin.text(...).to_string()` → `ansi_to_tui`
- 因此把主题收敛到 `MadSkin` 是“最小改动且两端一致”的实现路径。

### termimad 可配置项（关键引用/要点）
- `MadSkin` 里直接暴露了常用 Markdown 元素样式：
  - `paragraph` / `inline_code` / `code_block` / `headers` / `bullet` / `quote_mark` / `horizontal_rule` / `table` 等。
- 常用 API：
  - `set_fg(color)`、`set_bg(color)`、`set_fgbg(fg, bg)`
  - `StyledChar::set_fg(color)`（用于 bullet/quote_mark/hr）

### 实现中的“坑”：同时存在两个版本的 crossterm
- workspace 里直接依赖的是 `crossterm 0.28`，但 `termimad 0.34.1` 依赖的是 `crossterm 0.29`。
- 一旦对 `MadSkin` 调 `set_fg/set_fgbg` 传入 `crossterm::style::Color`，会触发 E0308 类型不匹配。
- 解决：palette 常量使用 `termimad::crossterm::style::Color`（与 termimad 内部类型一致）。

### 测试中的“坑”：`NO_COLOR=1` 会让 ANSI 颜色被抑制
- 当前环境变量：`NO_COLOR=1`。
- crossterm 在这种情况下会把颜色参数输出为空，导致渲染结果里出现 `\x1b[m` 但没有 `38;2;...` / `48;2;...`。
- 因此“基于渲染后的 ANSI 再解析 Span 样式”的断言不稳定。
- 解决：回归测试直接断言 `default_markdown_skin()` 内部配置（fg/bg），与环境无关。

---

## 2026-02-01 00:59 +0800｜Markdown 内部配色：sublime-monokai-extended（Monokai Extended）

### 为什么改
- 你反馈 Monokai Pro 的配色“不好看”。
- 目标改为使用 `jonschlinkert/sublime-monokai-extended` 的 **Monokai Extended** 配色（更接近经典 Monokai + Sublime 的手感）。

### 关键色提取（来自 `Monokai Extended.tmTheme` 的 Markdown/markup scope）
- Heading：`#fd971f`
- Quote：`#66d9ef`
- Bold：`#f92672`
- Italic：`#e42e70`
- Strike：`#cc4273`
- Raw inline：`#ec3533`
- Base foreground：`#f8f8f2`
- lineHighlight：`#333333`
- selection：`#444444`
- dimmed：`#636050` / `#565656`
- list punctuation：`#777777`

### 映射到 termimad 的落点
- 仍然只改 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()`：
  - stdout 与 TUI 两条路径自动同步（复用同一个 `MadSkin`）。
- code block / inline code 的背景色选择：
  - code block：用 `lineHighlight (#333333)` 做背景，让代码区域明确但不刺眼；
  - inline code：用 `selection (#444444)` 做背景，让行内代码更突出。

### 额外取舍（终端体验）
- table 不再强制设置背景色：
  - 避免在不同终端/不同面板底色下出现“边框块状底色不一致”的视觉噪音。

---

## 2026-02-01 01:10 +0800｜Markdown 配色微调：代码块取消背景 + 标题改为 #ffd866

### 需求
- 代码块（fenced code block）取消背景色（不要铺底）。
- 标题颜色统一改为 `#ffd866`。

### 实现要点
- 修改点仍然只落在 `default_markdown_skin()`：
  - stdout/TUI 两条渲染路径复用同一套 `termimad::MadSkin`，改动范围最小且一致性最好。
- code block 取消背景的关键实现：
  - `skin.code_block.set_fg(...)` 保留前景色
  - `skin.code_block.compound_style.object_style.background_color = None` 直接清掉背景（覆盖 termimad 默认铺底）
- heading 改色：
  - `sublime_monokai_extended::HEADING` 改为 `#ffd866`（RGB: 255, 216, 102）

### 测试策略
- 由于 `NO_COLOR=1` 会抑制 ANSI 色彩输出，测试继续采用“断言 skin 内部配置”：
  - code block bg 必须为 `None`
  - heading fg 必须为 `#ffd866`

---

## 2026-02-01 11:23 +0800｜Markdown 配色微调：inline code 取消背景 + 红色改为 #ff6188

### 需求
- inline code（行内代码）取消背景色。
- Markdown 红色改为 `#ff6188`。

### 落地策略（保持“单入口”一致性）
- 仍然只改 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()`：
  - stdout/TUI 两条渲染路径复用同一个 `termimad::MadSkin`，因此一处修改两端一致。

### 关键实现点
- inline code：
  - 保留前景色（改为 `#ff6188`）
  - 清空背景：`skin.inline_code.object_style.background_color = None`
- “红色系”统一：
  - 目前把 `RAW_INLINE` 与 `BOLD` 的前景色都统一为 `#ff6188`（减少主题里红/粉的分裂感）。

### 测试策略（延续 NO_COLOR 兼容）
- 继续直接断言 `MadSkin` 内部配置：
  - inline code fg 必须为 `#ff6188`
  - inline code bg 必须为 `None`

---

## 2026-02-01｜并行 TUI：`LOOP_COMPLETE` 暂停语义 + 禁用实例回收

### 现象

- 并行 TUI 里，hat instance 可能显示为 `done`。
- 默认 human message 会定向到“当前选中实例”（避免 broadcast）。
  - 若选中实例已经 `done`（或已从 Supervisor 的 registry 移除），消息虽然能写入外部 JSONL，但实际投递会失败，导致“看起来发出但没有响应”。

### 关键代码点（定位实现落点）

1) `HatInstanceState::Done` 的含义（协议层）
- 定义：`done = 已完成（不再接收新任务，等待回收/归档）`
- 位置：`crates/ralph-proto/src/hat.rs`

2) completion promise（默认 `LOOP_COMPLETE`）当前会让 Supervisor “收敛并退出”
- 在 `HatInstanceEvent::JobCompleted` 检测到 `LOOP_COMPLETE` 后设置 termination，并进入 drain + shutdown
- tick 分支在 completion 后停止接收/派发新事件（包括 external / gate.timeout）
- 位置：`crates/ralph-core/src/parallel/supervisor.rs`

3) completion 之后“禁止继续路由新事件”的护栏必须保留
- 现有回归测试覆盖：completion 后 writer 延迟发出的 `build.done` 不应再触发 collector（避免假活跃/无限派生）
- 位置：`crates/ralph-core/src/parallel/supervisor/routing_tests.rs`

4) “动态实例 idle 回收”是 `done` 的主要来源之一
- 动态实例在 truly-idle 超过 TTL 后 self-shutdown 并进入 `Done`
- TTL 来自 `config.parallel.autoscale.dynamic_idle_ttl_secs`（默认 30s）
- 位置：
  - TTL 配置：`crates/ralph-core/src/config.rs`
  - 回收判定：`crates/ralph-core/src/parallel/instance.rs`
  - spawn 传参：`crates/ralph-core/src/parallel/supervisor.rs`、`crates/ralph-core/src/parallel/supervisor/routing.rs`

### 结论（方案 A 的实现原则）

- 在 **TUI 模式**：把 completion promise 从“退出信号”改成“进入暂停态（仍消费 external events）”
- 同时保留 completion 的“收敛护栏”：暂停态下不继续路由内部延迟事件派生新 job
- 在 **TUI 模式**：禁用动态实例 idle 回收，避免 `done`，保证可继续对话

---

## 2026-02-01｜并行 TUI：`LOOP_COMPLETE` 后重置并暂停 max_runtime，直到 `Running` 才重新计时

### 需求要点（你提出的精确语义）

- `LOOP_COMPLETE` 之后，`event_loop.max_runtime_seconds` **重置**。
- 在暂停态期间 **不计时**。
- 直到“任何 hat instance 又开始运行”（`HatInstanceState::Running`）才重新开始计时。

### 关键实现点（落点）

- 计时状态机在 `ParallelSupervisor::run()` 内实现（不新增 YAML 配置字段）：
  - `max_runtime_started_at`：当前计时窗口的起点
  - `max_runtime_counting`：是否正在计时（暂停态=false）
  - `max_runtime_waiting_for_running`：是否在等待下一次 Running 来启动计时
- 进入暂停态（TUI + completion promise）时：
  - `max_runtime_started_at = now`（重置）
  - `max_runtime_counting = false`（暂停计时）
  - `max_runtime_waiting_for_running = true`（等 Running）
- 收到 `HatInstanceEvent::StateChanged(Running)` 且 waiting=true 时：
  - `max_runtime_started_at = now`
  - `max_runtime_counting = true`
  - `max_runtime_waiting_for_running = false`
- 收到 external events 解锁暂停态时：
  - 不直接开始计时，仍等 Running（符合你的口径）
  - 但加了一个兜底：如果此刻已经存在 Running，则直接开始计时，避免极端竞态导致“永久暂停”。

### 测试覆盖

- 新增回归测试：暂停态下等待超过 max_runtime 不应退出；恢复并进入 Running 后应能触发 MaxRuntime 终止。

---

## 2026-02-01 12:02 +0800｜Markdown 配色微调：bold（强调/标签类）改为 #a9dc76

### 现象
- 你反馈类似 `"1. 初 始 化 ： 入 口 命 令 启 动"` 这种 Markdown：
  - “初始化”作为强调/标签类文本（通常用 `**bold**` 表达）
  - 当前渲染颜色偏红，观感不符合你希望的“可执行步骤/标签”语义。

### 目标
- 把 `**bold**` 的前景色从红色系改为 `#a9dc76`（绿色）。

### 落地位置（单入口）
- 仍然只改 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()`：
  - stdout/TUI 两条渲染路径复用同一个 `termimad::MadSkin`，因此一处修改两端一致。

### 关键实现点
- `sublime_monokai_extended::BOLD` 改为 `#a9dc76`（RGB: 169, 220, 118）。
- 新增回归测试：直接断言 `MadSkin` 内部 `bold` 的 fg 配置（避免 `NO_COLOR=1` 抑制 ANSI 导致不稳定）。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-02 02:39 +0800｜TUI：右上角 Hat Graph Radar（ASCII Mermaid），按键 `p` 放大/还原

### 关键发现

- `ralph-cli` 已经具备 “hats graph → Mermaid → ASCII/Unicode” 的渲染链路：
  - Mermaid 生成：`crates/ralph-cli/src/hats.rs` 的 `generate_mermaid_string(...)`
  - ASCII 渲染：`beautiful_mermaid_rs::render_mermaid_ascii(...)`
- `ralph-tui` 的渲染入口在 `crates/ralph-tui/src/app.rs` 的 `terminal.draw(...)`：
  - 在 content 渲染后、footer 前追加 overlay，能做到“覆盖层”效果
  - Warp 的 bg=Reset 模式下，exabind 边框需要在 effects 之后再 patch 一次 bg

### 落地策略（方案 B）

- 生成与渲染放在 `ralph-cli`（启动 TUI 时 best-effort）：
  - 产出 `(ascii_compact, ascii_full)` 两份缓存字符串
  - 注入到 `ralph-tui`：`Tui::with_hat_graph_radar(ascii_compact, ascii_full)`
- `ralph-tui` 只负责：
  - 缓存 + 渲染 overlay（右上角）
  - `p` 切换 zoom（串行/并行都可用；并行 Chat 聚焦时不抢键）

### 验证

- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-02 12:21 +0800｜Bug：TUI 启动前卡很久（Hat Graph Radar 渲染阻塞）

### 现象

- 合并 `a15bced` 后，`ralph run --tui` 在进入 TUI（alternate screen）前会卡很久。

### 证据（量化）

- `beautiful-mermaid-rs` 的 Mermaid→ASCII 渲染非常慢：
  - release：`target/release/ralph hats graph --format ascii -c presets/pdd-to-code-assist.yml` 约 22 秒
  - debug：`cargo run --bin ralph -- hats graph --format ascii -c presets/pdd-to-code-assist.yml` 约 87 秒

### 根因

- Radar 生成复用了 `beautiful-mermaid-rs` 的 Mermaid→ASCII（QuickJS + eval 大 bundle）。
- 且是在 **启动 TUI 之前** 同步执行，导致用户看到“长时间无 UI”。

### 修复策略

- Radar 不再做 Mermaid→ASCII（QuickJS）渲染。
- 改为直接展示 Mermaid 源码文本：
  - compact：只显示关键连线（edges-only）
  - full：完整 Mermaid（含节点 label）
- 这样既保留“拓扑结构信息”，又避免把启动路径拉成十几秒/几十秒。

### 验证

- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-01 23:32 +0800｜`ralph hats graph` Mermaid 逻辑视图：隐藏 Ralph、Hat→Hat 实线、可选 starting_event 入口

### 现象
- `ralph hats graph --format mermaid` 输出包含 `Ralph` 中心节点。
- 同时会出现：
  - `Ralph -> Hat`（订阅）
  - `Hat -> Ralph`（发布）
  - `Hat -.-> Hat`（逻辑连线，但用虚线）
- 当 hat 数量一多时，视觉上接近“全连接”，很难读。

### 期望（用户口径）
- 图上 **不要出现** `Ralph`（调度员在背后即可）。
- 不要出现 `Hat -> Ralph` / `Ralph -> Hat` 这种“内部调度边”。
- Hat 与 Hat 的逻辑关系要用 **实线** `-->`（不要 `-.->`）。

### 落地位置
- `crates/ralph-cli/src/hats.rs`：
  - `graph_hats()`：接入 `RalphConfig`，让图能感知 `event_loop.starting_event`
  - `generate_mermaid_string()`：改为输出“逻辑视图”（Hat→Hat 实线），并在有 starting_event 时补 `Start[task.start]` 入口边

### 关键实现点
- 仍保持 Mermaid “ID/label 分离”：
  - 节点 ID：`Hat_{sanitize(hat.id)}`（ASCII 安全）
  - 节点 label：`hat.name`（允许中文/emoji）
- 入口边（可选）：
  - 当 `config.event_loop.starting_event` 存在时，输出：
    - `Start[task.start]`
    - `Start -->|starting_event| Hat_X`（所有订阅了 starting_event 的 hats）
- Hat→Hat 边：
  - 收集 `(source publishes topic) && (target subscribes topic) && source != target` 的组合
  - 按 `(source_id, topic, target_id)` 排序 + 去重
  - 用 `-->` 输出

### 验证
- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-02 00:35 +0800｜hats graph：把 `complete_publishes` 画成 `Complete[complete]` 终点节点

### 现象
- 在 `examples/parallel-trigger-routing/ralph.yml` 里配置了：
  - `event_loop.complete_publishes: "spec.approved"`
- 但 `ralph hats graph --format mermaid` 的逻辑视图只画 Hat→Hat（需要订阅者），导致：
  - `spec.approved` 因为没有任何 hat 订阅而“消失”
  - 图上看不到工作流怎么结束

### 根因
- 逻辑视图的 Hat→Hat 边推导规则是：
  - `(A publishes T) && (B subscribes T)` 才画 `A -->|T| B`
- `complete_publishes` 本质是“工作流完成候选事件”，不要求被 hat 订阅，因此不能用上述规则推导。

### 修复策略
- 当 `event_loop.complete_publishes` 存在时：
  - 固定输出一个终点节点：`Complete[complete]`
  - 找出所有发布该 topic 的 hats，并画边：`Hat_X -->|complete_publishes| Complete`

### 落地位置
- `crates/ralph-cli/src/hats.rs`：`generate_mermaid_string()`
- `specs/hats-graph-logical-view.spec.md`：新增 `G5`（complete_publishes 终点节点语义）

### 验证
- `cargo fmt` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-01 15:28 +0800｜修复 `ralph hats graph` 在中文/emoji hat 名称下吞节点

### 现象
- 在 `examples/parallel-trigger-routing/ralph.yml` 这类配置里：
  - `ralph hats graph --format mermaid` 输出的 Mermaid 文本包含完整 hats 拓扑。
  - 但 `ralph hats graph --format unicode/ascii` 只显示 task.start→Ralph（hats 节点与边消失）。

### 根因
- 我们之前把 Mermaid 节点 ID 直接用 `hat.name`（中文/emoji）生成。
- `beautiful-mermaid-rs` 在 Mermaid→ASCII/Unicode 渲染链路里，对“Unicode 节点 ID”兼容性不足，会吞边/吞节点，但不会报错。

### 修复策略（Ralph 侧修复，最小改动）
- Mermaid 输出改为“节点 ID / 节点 label 分离”：
  - 节点 ID：用 `hat.id`，并限制为 ASCII `[A-Za-z0-9_]`，统一加 `Hat_` 前缀避免冲突。
  - 节点 label：继续用 `hat.name`（保留中文/emoji）。
- 为了降低渲染布局波动，输出前对 hats 按 `hat.id` 排序（避免 HashMap 迭代顺序随机）。

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-01 12:19 +0800｜Markdown 配色微调：全色混入 3% #4493f8（白色不变）

### 需求
- 你希望 Markdown 的“所有颜色”统一轻微偏蓝：
  - 对所有颜色混入 3% 的 `#4493f8`
  - 但白色（正文）保持不变

### 为什么用“统一混合”而不是逐个改色
- 逐个手调每个颜色会很难保持一致性，而且以后你想改 2%/5% 会很痛苦。
- 用统一混合可以把“偏色”变成一个可控参数，改动更小，也更容易回归测试锁定。

### 落地位置（单入口）
- 仍然只改 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()` 与 palette 常量：
  - stdout/TUI 两条渲染路径复用同一个 `termimad::MadSkin`，因此一处修改两端一致。

### 关键实现点
- 在 `sublime_monokai_extended` palette 内新增 const 混合函数：
  - `new = base * 97% + mix * 3%`（四舍五入）
  - `mix = #4493f8`
- “白色正文不变”：
  - `FOREGROUND` 不做混合，保持 `#f8f8f2`

### 回归测试更新点
- 由于颜色经过混合，测试期望值更新为“混合后的最终 RGB”：
  - inline code / code block：基色 `#78dce8` → 最终约 `#76dae8`
  - heading（H2+）：基色 `#fc9867` → 最终约 `#f6986b`
  - H1：基色 `#ffd866` → 最终约 `#f9d66a`
  - bold：基色 `#a9dc76` → 最终约 `#a6da7a`

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅

---

## 2026-02-01 11:44 +0800｜Markdown 配色微调：H1 #ffd866 + code #78dce8（无背景）+ heading #fc9867 + red #ff6188

### 需求（最新）
- code block：取消背景色；前景 `#78dce8`
- inline code：取消背景色；前景 `#78dce8`
- H1（标题）：`#ffd866`
- H2-H6（heading）：`#fc9867`
- Markdown 红色：`#ff6188`

### 落地位置（单入口）
- 仍然只改 `crates/ralph-adapters/src/stream_handler.rs` 的 `default_markdown_skin()`：
  - stdout/TUI 两条渲染路径复用同一个 `termimad::MadSkin`，因此一处修改两端一致。

### 关键实现点
- palette：
  - 新增 `sublime_monokai_extended::TITLE = #ffd866`
  - `HEADING = #fc9867`（保持）
  - `RAW_INLINE = #78dce8`（保持）
  - `BOLD = #ff6188`（保持）
- header 映射：
  - `skin.headers[0]` 用 `TITLE`
  - `skin.headers[1..]` 用 `HEADING`
- 测试：
  - 新增 `markdown_h1_uses_custom_yellow`
  - `markdown_heading_uses_custom_orange` 调整为断言 `H2`（`headers[1]`）

### 验证
- `cargo fmt --check` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-core kiro` ✅
