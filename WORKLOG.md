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
