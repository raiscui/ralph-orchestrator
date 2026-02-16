# 任务计划: Event.id 切换为 nanoid + reply 协作链路验证(2026-02-14 22:47 +0800)

## 目标
- 让每条 event 都具备可引用的稳定 `id`.
- `id` 的默认生成规则改为 nanoid(随机,短,URL-safe).
- `reply` 保持单值(in-reply-to),并确保串行/并行/E2E 不回退.

## 我正在做什么 & 为什么
- 我正在把 Event 的默认 id 生成,从"确定性序号"切换为"nanoid".
- 因为你希望 ralph 与多个 hat 之间能更顺畅地"引用并回复"某条事件.
- 用 nanoid 可以避免把 instance 信息编码进 id(例如 `writer#1:7`),同时降低冲突风险.
- 这个改动会影响测试的确定性.
  所以我会同步调整单测与 e2e 断言口径,避免随机 id 导致回归不稳定.

## 方案方向(两条路)
- 方案A(不惜代价,最佳方案,本次采用):
  - 复用既有 `Event.id` 字段.
  - 当 `id` 缺失时,由 runtime 自动补齐 nanoid.
  - 只做"默认生成"替换,不强制覆盖 hat 手动设置的 `id`.
- 方案B(先能用,后面再优雅):
  - 保留确定性 id.
  - 额外新增一个 `nanoid` 字段专用于引用.
  - 缺点是协议膨胀,也更容易出现两套 id 口径不一致.

## 决策
- 采用方案A.
  - 原因: "改良胜过新增".
  - 目标是让 `Event.id` 本身就是可引用 id.

## 阶段
- [ ] 阶段1: 盘点所有 id 生成点与断言
- [ ] 阶段2: 引入 nanoid 依赖并替换默认生成
- [ ] 阶段3: 更新单测/文案 + 调整 e2e 解析/断言
- [ ] 阶段4: 运行 fmt + smoke + e2e + 全量测试
- [ ] 阶段5: 追加 WORKLOG.md 收尾,必要时补 LATER_PLANS.md

## 关键问题
1. 哪些地方依赖了确定性 id 文本(例如 `eventbus:0`)? 需要改为"存在即可"或"匹配格式".
2. 随机 id 是否会影响 replay/诊断? 需要确认 events.jsonl 落盘包含 id,且不被截断.
3. reply 的空字符串语义是否保持一致(空字符串 => None)?

## 状态
**目前在阶段1** - 我正在盘点 id 生成点与相关测试断言.

---

## 阶段进展更新(2026-02-14 23:02 +0800)

- [x] 阶段1: 盘点所有 id 生成点与断言
- [x] 阶段2: 引入 nanoid 依赖并替换默认生成
- [x] 阶段3: 更新单测/文案 + 调整 e2e 解析/断言
- [x] 阶段4: 运行 fmt + smoke + e2e + 全量测试
- [x] 阶段5: 追加 WORKLOG.md 收尾,必要时补 LATER_PLANS.md

### 验证

- `cargo fmt` ✅
- `cargo test -p ralph-proto` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-e2e` ✅
- `cargo test` ✅
- `cargo clippy --all-targets --all-features -- -D warnings` ✅

### 状态

**已完成** - Event.id 已切换为 nanoid,并确保 reply 协作链路与测试验证全部通过.

---

# 任务计划: 并行 TUI 录制能力核对(2026-02-15 10:55 +0800)

## 目标

- 明确 "parallel TUI" 下 "录制" 的两种含义(录 cassette vs 录屏).
- 给出当前版本的结论与用法.
- 盘点哪些历史变更/使用方式会让人感觉 "无法录制".

## 我正在做什么 & 为什么

- 我正在在代码里核对 `--record-session` 在串行/并行 runner 的 wiring.
- 同时对照 `specs/parallel-record-session.spec.md` 的验收口径.
- 这样可以避免凭印象回答,并且能定位到具体文件与行为差异.

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案): 内建 cassette 录制(推荐,可回放)
  - 用 `--record-session` 生成 JSONL "cassette".
  - 用于 replay smoke tests / `ralph-e2e --mock` 的 `mock-cli` 回放.
  - 重点: 录制的是 `bus.publish` + stdout-only 的 `ux.terminal.write`,不是 TUI 的逐帧画面.
- 方案B(先能用,后面再优雅): 外部录屏/截屏(录 UI 观感证据)
  - 用 asciinema 等工具录制终端屏幕过程,或用 tmux + freeze 做可复现截图.
  - 适合做 UI 回归证据或演示.
  - 缺点: 不能直接驱动 "事件回放" 这类测试用途.

## 阶段

- [ ] 阶段1: 需求口径澄清(录 cassette 还是录屏)
- [ ] 阶段2: 代码与 spec 核对(串行/并行/TUI)
- [ ] 阶段3: 输出结论 + 排查清单
- [ ] 阶段4: 追加 notes/WORKLOG,必要时补 LATER_PLANS

## 状态

**目前在阶段2** - 我已完成代码与 spec 核对,正在整理结论与排查点.

---

## 阶段进展更新(2026-02-15 10:58 +0800)

- [x] 阶段1: 需求口径澄清(录 cassette 还是录屏)
- [x] 阶段2: 代码与 spec 核对(串行/并行/TUI)
- [x] 阶段3: 输出结论 + 排查清单
- [x] 阶段4: 追加 notes/WORKLOG,必要时补 LATER_PLANS

### 产出

- 规格: `specs/parallel-record-session.spec.md` 明确 non-goal: 不录 TUI frame.
- 代码:
  - `crates/ralph-cli/src/parallel_runner.rs`: parallel(`parallel-tui`/`parallel-cli`) 的 cassette 写入.
  - `crates/ralph-cli/src/loop_runner.rs`: 串行 cassette 写入.
- 文档: `README.md` 修正了 "no parallel TUI yet" 的过期描述.

### 状态

**已完成** - 已确认 parallel TUI 支持 `--record-session` 录制 cassette,并整理了历史差异与常见误区.

---

# 任务计划: ralph_hat_instance_id 置顶,避免 all-hat 示例歧义(2026-02-15 11:56 +0800)

## 目标

- 让每个 hat 的 prompt 第一行永远是 `ralph_hat_instance_id:"..."`.
- 避免该行落在 `config/all_hat.md` 的示例段落之后,被模型误当成示例的一部分.
- 为该行为补一条回归测试,避免以后改动 prompt 组装逻辑时回退.

## 我正在做什么 & 为什么

- 我正在沿着 prompt 拼接链路,定位 `ralph_hat_instance_id` 与 all-hat overlay 的相对顺序.
- 因为当前实现是先注入 `ralph_hat_instance_id`,再把 all-hat overlay "整体前置".
- 这会导致最终 prompt 的开头是 overlay,而 `ralph_hat_instance_id` 出现在 overlay 末尾.
- 当 overlay 里刚好有 `<event ...>` 的示例块时,模型更容易把紧随其后的 `ralph_hat_instance_id` 误解成示例续行.

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案,本次采用): 从根上修正 overlay 注入算法
  - 改 `crates/ralph-core/src/prompt_overlay.rs`:
    - 如果 prompt 以 `ralph_hat_instance_id:"..."` 开头,则把 overlay 插入到该行之后.
    - 这样所有调用点都不需要改顺序,且保证 `ralph_hat_instance_id` 真正置顶.
- 方案B(先能用,后面再优雅): 只改文案,用分隔符缓解歧义
  - 在 `config/all_hat.md` 的示例后加醒目分隔符(例如 "示例结束").
  - 缺点: 只是缓解,不能保证未来不再出现类似歧义.

## 阶段

- [ ] 阶段1: 复现/定位注入顺序与调用点
- [ ] 阶段2: 修改 overlay 注入逻辑,确保 id 真正置顶
- [ ] 阶段3: 补回归测试(锁死顺序)
- [ ] 阶段4: `cargo fmt` + `cargo test`
- [ ] 阶段5: 追加 WORKLOG.md,必要时追加 ERRORFIX.md

## 状态

**目前在阶段1** - 我正在定位 all-hat overlay 与 `ralph_hat_instance_id` 的拼接顺序,并选择最小侵入修法.

---

## 阶段进展更新(2026-02-15 12:00 +0800)

- [x] 阶段1: 复现/定位注入顺序与调用点
- [x] 阶段2: 修改 overlay 注入逻辑,确保 id 真正置顶
- [x] 阶段3: 补回归测试(锁死顺序)
- [x] 阶段4: `cargo fmt` + `cargo test`
- [x] 阶段5: 追加 WORKLOG.md,必要时追加 ERRORFIX.md

### 实施

- `crates/ralph-core/src/prompt_overlay.rs`
  - `inject_all_hat_prompt()` 现在会识别第一行的 `ralph_hat_instance_id:"..."`,
    并把 overlay 插入到该行之后.
  - 新增回归测试锁死 "runtime id 永远是第一行".

### 验证

- `cargo fmt` ✅
- `cargo test -p ralph-core` ✅
- `cargo test` ✅

### 状态

**已完成** - `ralph_hat_instance_id` 已稳定置顶,不再落在 all-hat 示例块之后造成歧义.

---

# 任务计划: `ux.terminal.write` 录制可读文本,提升诊断可读性(2026-02-15 12:21 +0800)

## 目标

- 让 `--record-session` 生成的 JSONL cassette 在不做 base64 解码的情况下也能直接读.
- 不牺牲回放的字节级保真(仍保留原始 bytes),避免引入新的不确定性.

## 我正在做什么 & 为什么

- 我正在复核 `ux.terminal.write` 的 schema 与读写路径.
- 因为当前 `bytes` 是 base64,人肉排障很痛苦.
- 我们需要一个面向人类的字段,同时保留面向机器回放的字段.

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案,本次采用): 保留 `bytes`(base64) + 新增 `text`(UTF-8 lossy) 辅助诊断
  - 优点: 回放仍以 bytes 为准,完全不影响 replay 的保真与现有 fixture.
  - 缺点: cassette 体积会变大(同一份内容保存两份表示).
- 方案B(先能用,后面再优雅): 直接把 `bytes` 改为明文 `text`,不再存 base64
  - 优点: JSONL 更小,也更直观.
  - 缺点: 不能表达非 UTF-8 输出,且需要批量更新 fixtures/测试,风险更高.

## 阶段

- [ ] 阶段1: 明确字段语义与落点(保证回放不变)
- [ ] 阶段2: 修改 `ralph-proto::TerminalWrite` schema + 单测
- [ ] 阶段3: 更新录制路径,写入 `text` 字段
- [ ] 阶段4: 更新 fixtures 文档 + 运行 fmt/test/smoke
- [ ] 阶段5: 追加 notes/WORKLOG,必要时补 LATER_PLANS

## 关键问题

1. `text` 是否必须始终存在? 还是允许 None(旧 cassette 兼容)?
2. `text` 的编码口径: 使用 `String::from_utf8_lossy` 是否足够?
3. 诊断时默认看 `text`,回放时永远以 `bytes` 为准,是否需要在文档里强制强调?

## 状态

**目前在阶段1** - 我正在把 schema/实现/测试的改动范围盘清楚,避免破坏现有回放链路.

---

## 阶段进展更新(2026-02-15 12:26 +0800)

- [x] 阶段1: 明确字段语义与落点(保证回放不变)
- [x] 阶段2: 修改 `ralph-proto::TerminalWrite` schema + 单测
- [x] 阶段3: 更新录制路径,写入 `text` 字段
- [x] 阶段4: 更新 fixtures 文档 + 运行 fmt/test/smoke
- [x] 阶段5: 追加 notes/WORKLOG,必要时补 LATER_PLANS

### 实施

- `crates/ralph-proto/src/ux_event.rs`
  - `TerminalWrite` 新增 `text: Option<String>`(UTF-8 lossy,用于诊断).
  - `TerminalWrite::new()` 自动填充 `text`,确保新录制的 cassette 可直接阅读.
  - 新增单测:
    - `test_terminal_write_backward_compat_without_text` 锁死旧 cassette 仍可解析.
- `crates/ralph-core/src/session_recorder.rs`
  - 单测补充断言: 输出 JSONL 里包含 `"text":"Hello"`,确保诊断字段确实落盘.
- 文档与规格同步:
  - `crates/ralph-core/tests/fixtures/README.md`
  - `crates/ralph-core/tests/fixtures/kiro/README.md`
  - `specs/parallel-record-session.spec.md`

### 验证

- `cargo fmt` ✅
- `cargo test -p ralph-proto` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test -p ralph-e2e` ✅
- `cargo test` ✅

### 状态

**已完成** - cassette 仍保留 `bytes`(base64) 用于回放,同时新增 `text` 让 JSONL 可直接读用于诊断.

---

# 任务计划: parallel 模式 stderr 可观测 + cassette 录制补齐 + 示例PROMPT并行自检(2026-02-15 13:44 +0800)

## 目标

- parallel 模式下,能看见来自 codex app-server 的 stderr(灰色).
- `--record-session` 录制的 cassette 同时包含 stdout 与 stderr(用 stdout=true/false 区分),且仍保持"事件解析 stdout-only".
- example 的 PROMPT 模板新增"并行自检"条目,用户不写实验也更容易触发 >=2 个 experiment.task.
- 同步更新外部目录 `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine` 的 `PROMPT.md`/`ralph.yml`,避免两边行为不一致.

## 我正在做什么 & 为什么

- 我正在把"看起来没并行/无回应/没灰色思考"拆成可验证的三个层面:
  1) 输出可见性: stderr 是否被吞掉,是否被录制.
  2) 交互路径: human.message/!steer 是否真正进入并行 Supervisor.
  3) 并行激励: PROMPT 是否强制首批派发 >=2 个实验,避免表面并行但实际只跑 1 个任务.
- 这样我们能用 cassette(JSONL) 与事件链路作为证据,而不是只靠体感判断.

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案,本次采用):
  - app-server stderr 也作为 HatJobOutputChunk 流式上屏与录制,但不进入事件解析 output.
  - cassette 记录 stdout+stderr,用 `ux.terminal.write.data.stdout` 区分.
  - example PROMPT 提供可直接跑的并行自检实验,保证肉眼可见交错输出.
- 方案B(先能用,后面再优雅):
  - 只改 PROMPT/README 强制发两条 experiment.task.
  - 不改 runtime/录制,继续 stderr-only 诊断靠 tracing.
  - 缺点: 根因仍在,回放与排障依然困难.

## 阶段

- [ ] 阶段1: 复盘现状与回归风险点(录制/解析/交互)
- [ ] 阶段2: app-server stderr 流式转发(不参与事件解析)
- [ ] 阶段3: cassette 录制补齐 stderr(stdout=false)
- [ ] 阶段4: example PROMPT/README 改进 + 外部目录同步
- [ ] 阶段5: cargo test + smoke + 追加 WORKLOG/notes,必要时补 LATER_PLANS

## 关键问题

1. app-server stderr 是否可能包含 `<event ...>` 片段? 如果是,必须确保它永远不进入事件解析 output.
2. cassette 同时录 stderr 后,回放是否需要调整? (应无需,因为回放按 stdout 标志过滤/分流即可)
3. `--no-tui` 的 completion 行为会退出进程,这类模式下"发消息无回应"如何在文档里解释清楚?

## 状态

**目前在阶段1** - 我正在把改动点与验收口径写清楚,然后开始逐项落地.

---

## 阶段进展更新(2026-02-15 14:00 +0800)

- [x] 阶段1: 复盘现状与回归风险点(录制/解析/交互)
- [x] 阶段2: app-server stderr 流式转发(不参与事件解析)
- [x] 阶段3: cassette 录制补齐 stderr(stdout=false)
- [x] 阶段4: example PROMPT/README 改进 + 外部目录同步
- [x] 阶段5: cargo test + smoke + 追加 WORKLOG/notes,必要时补 LATER_PLANS

### 实施

- app-server stderr 可观测:
  - `crates/ralph-cli/src/codex_app_server_session.rs`
    - 为每个 session 增加 stderr broadcast 通道.
    - 将 codex app-server stderr 行转成 `HatJobOutputChunk{stream=Stderr}` 流式输出.
    - 保持"事件解析 stdout-only": stderr 不进入 `HatJobResult.output`.
- cassette 录制补齐 stderr:
  - `crates/ralph-cli/src/parallel_runner.rs`
    - `--record-session` 现在会录制 stdout+stderr(`ux.terminal.write.data.stdout` 区分).
    - `--hide-stderr` 只影响显示,不影响录制(避免排障时录不到证据).
  - `crates/ralph-core/src/testing/replay_backend.rs`
    - smoke tests 回放仍保持 stdout-only(过滤 stdout=false 的 write),避免 stderr 假事件污染解析.
    - 新增单测 `test_filters_stderr_terminal_writes`.
- 示例与文档:
  - `examples/parallel-experimental-dev-engine/ralph.yml`: Auto-Plan 增强 MUST 约束(至少 2 实验 + 首批派发>=2).
  - `examples/parallel-experimental-dev-engine/PROMPT.md`: 增加两条可直接跑的并行自检实验(exp-par-001/002).
  - `examples/parallel-experimental-dev-engine/README.md`: 补齐 TUI chat/`ralph emit` 的交互说明,并修正 worktree_backend 文案与配置一致.
  - `specs/parallel-record-session.spec.md`: 更新为"cassette 可录 stdout+stderr,但 event parsing/replay 默认 stdout-only".
- 外部目录同步(不覆盖你的 objective 内容,只补齐协议提示与 MUST 约束):
  - `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/ralph.yml`
  - `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/PROMPT.md`

### 验证

- `cargo fmt --check` ✅
- `cargo test -p ralph-cli` ✅
- `cargo test -p ralph-core` ✅(含 smoke_runner)
- `cargo test` ✅

### 状态

**已完成** - parallel 模式下 stderr 可见且可录制,并补齐了示例 PROMPT/README 来更稳定地触发可感知的并行输出与交互注入路径.

---

# 任务计划: 并行示例补齐 stderr 自检 + 交互注入路径文档化 + 外部目录同步复核(2026-02-16 17:22 +0800)

## 目标

- 让 example 的“并行自检”不仅能看到 stdout 交错,也能稳定产出 stderr(灰色)交错,从而验证:
  - TUI/日志模式都能看到 stderr 行.
  - `--record-session` 的 JSONL 里会出现 `ux.terminal.write` 且 `stdout=false` 且带 `instance_id`.
- 文档明确说明并行交互的两条路径:
  - TUI chat: `human.message` / `!steer` / `!interrupt`
  - 另开终端: 通过 `.ralph/current-events` 指向的 JSONL 注入事件(或 `ralph emit`),并解释“无回应”的常见原因.
- 复核并同步外部目录 `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/` 的示例文件,避免两边行为不一致.

## 我正在做什么 & 为什么

- 我正在对照你给的验收标准,逐条核对当前代码与示例文件。
- 虽然核心能力(记录 stderr / app-server stderr 转发)已经落地,
  但 example 的自检实验仍然只输出 stdout,导致你很难用“肉眼 + cassette”快速确认 stderr 路径是否真的贯通。
- 同时 example README 目前只写了 `ralph emit` 的用法,但没有把 `.ralph/current-events` 的 marker 机制与手工注入格式说清楚,容易出现“发了消息但看起来没反应”的体感问题。

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案,本次采用):
  - 补齐 example 自检实验: stdout+stderr 同时持续输出,让并行交错与 stderr 录制一眼可见。
  - README 补齐 `.ralph/current-events` 的解释与手工注入示例,并把 `--no-tui` 的交互限制说清楚。
  - 同步外部目录示例文件。
- 方案B(先能用,后面再优雅):
  - 只写一句“建议用 TUI”,不补自检与注入细节。
  - 缺点: 仍然难以复现/验证 stderr 与交互链路,排障成本高。

## 阶段

- [ ] 阶段1: 复核现状与缺口(示例/文档/外部目录)
- [ ] 阶段2: 补齐示例自检(stderr 输出) + README 交互注入说明
- [ ] 阶段3: 同步外部目录文件(ralph.yml/PROMPT.md/README.md 如存在)
- [ ] 阶段4: 运行定向测试 + smoke,并确认无回归
- [ ] 阶段5: 追加 notes/WORKLOG,必要时补 LATER_PLANS

## 关键问题

1. 如何让用户不用依赖 codex 自身是否吐 stderr,也能稳定验证 stderr 显示/录制链路? -> 用自检实验主动打印 stderr.
2. `--no-tui` 为何“看起来无法对话”? -> 进程在 completion 后退出,且没有 chat UI;需要 TUI 或外部事件文件注入.
3. 外部注入事件写到哪里? -> `.ralph/current-events` marker 指向的 JSONL(与 Supervisor/EventReader 对齐)。

## 状态

**目前在阶段1** - 我正在把缺口定位为“示例自检缺 stderr 输出 + README 缺 marker/手工注入说明”,然后按最小改动补齐并用测试验证.

---

## 阶段进展更新(2026-02-16 17:32 +0800)

- [x] 阶段1: 复核现状与缺口(示例/文档/外部目录)
- [x] 阶段2: 补齐示例自检(stderr 输出) + README 交互注入说明
- [x] 阶段3: 同步外部目录文件(ralph.yml/PROMPT.md/README.md 如存在)
- [x] 阶段4: 运行定向测试 + smoke,并确认无回归
- [x] 阶段5: 追加 notes/WORKLOG,必要时补 LATER_PLANS

### 实施

- 示例自检改进:
  - `examples/parallel-experimental-dev-engine/PROMPT.md`
    - exp-par-001/002 自检实验改为同时输出 stdout+stderr,并修正 here-doc 缩进,确保复制即可运行。
- 示例协议加固:
  - `examples/parallel-experimental-dev-engine/ralph.yml`
    - Auto-Plan 明确首批窗口必须在同一输出内连续发布 >=2 条 `experiment.task`(当 `P_max >= 2`).
    - 明确“多实验塞进同一 experiment.task”属于协议失败,必须拆分重发。
- 交互文档补齐:
  - `examples/parallel-experimental-dev-engine/README.md`
    - 明确 `--no-tui` 不适合持续对话(完成后退出).
    - 补齐 `.ralph/current-events` marker + 手工 JSONL 注入 steer/interrupt 的示例.
- App-Server stderr 降噪:
  - `crates/ralph-cli/src/codex_app_server_session.rs`
    - codex app-server stderr 行的兜底日志从 warn 调整为 debug,避免默认刷屏(主要观测面在 parallel 输出流).
- 外部目录同步:
  - `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/ralph.yml`
    - 同步示例协议加固内容.
  - `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/PROMPT.md`
    - 先备份为 `PROMPT_backup_2026-02-16_1731.md`,再用仓库示例模板覆盖,避免两边示例漂移.

### 验证

- `cargo fmt --check` ✅
- `cargo test -p ralph-cli parallel_runner` ✅
- `cargo test -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅

### 状态

**已完成** - example 现在可以稳定产出 stderr(灰色)交错输出,并且 README 已把交互注入路径与“无回应”的排查方式说清楚；外部目录也已同步到同一套协议语义。

---

# 任务计划: `ralph emit` 增强支持 session_strategy/turn_action/workspace_strategy(2026-02-16 18:17 +0800)

## 目标

- `ralph emit` 支持在外部事件 JSONL 中写入这些可选字段(与 `EventReader` schema 对齐):
  - `workspace_strategy: shared|patch|worktree`
  - `session_strategy: exec|mcp|app_server`
  - `turn_action: start|steer|interrupt`
- 这样在 `--no-tui` 或“另开终端注入事件”的场景下:
  - 不需要手工追加 JSONL 也能 steer/interrupt.
  - 不容易写错字段名,也更不容易写进错误的文件。

## 我正在做什么 & 为什么

- 我正在把之前记在 `LATER_PLANS.md` 的建议落地为一个小而确定的 CLI 改良.
- 因为目前 `ralph emit` 只能写 `topic/payload/ts/target_instance`,
  但并行交互的高级用法(steer/interrupt)需要 `session_strategy/turn_action`,
  导致用户不得不手工写 JSONL,既麻烦也容易出错。
- 这类改动的价值在于“把正确用法变成默认路径”,减少排障成本。

## 方案方向(两条路)

- 方案A(不惜代价,最佳方案,本次采用):
  - 为 `ralph emit` 增加 3 个可选 flag,并把字段透传进 JSONL.
  - 加一条集成测试: 断言写出的 JSONL 能被 `ralph_core::event_reader::Event` 正确解析.
  - 更新 example README: 给出等价的 `ralph emit --session-strategy app_server --turn-action steer` 写法(替代手工 JSONL)。
- 方案B(先能用,后面再优雅):
  - 仅在 README 里继续教用户手工写 JSONL.
  - 缺点: 仍然容易写错,也不利于把 steer/interrupt 变成“可操作的日常能力”。

## 阶段

- [ ] 阶段1: 盘点现有 `emit` 写入结构与 `EventReader` 字段对齐点
- [ ] 阶段2: 实现 `--session-strategy/--turn-action/--workspace-strategy` 并写入 JSONL
- [ ] 阶段3: 补集成测试(写入->读取解析)
- [ ] 阶段4: 更新 example README 用法(替代手工 JSONL)
- [ ] 阶段5: cargo fmt + 定向测试 + 追加 notes/WORKLOG,并清理对应 LATER_PLANS 条目

## 关键问题

1. CLI 输出字段名必须与 `EventReader` 一致,否则写入了也读不到(尤其是 snake_case)。
2. steer/interrupt 的语义依赖 `session_strategy=app_server` 与 in-flight turn:
   - CLI 只负责写入正确字段,运行时能否立即生效取决于目标实例状态(这点要在 README 里解释清楚)。
3. `--file` 与 `.ralph/current-events` 的优先级要保持现有行为(避免写错 run 的事件文件)。

## 状态

**目前在阶段1** - 我正在先对齐 schema 与测试路径,再实现 CLI flag,最后用集成测试做 backpressure 证明“写入->读取”闭环成立.

---

## 阶段进展更新(2026-02-16 18:24 +0800)

- [x] 阶段1: 盘点现有 `emit` 写入结构与 `EventReader` 字段对齐点
- [x] 阶段2: 实现 `--session-strategy/--turn-action/--workspace-strategy` 并写入 JSONL
- [x] 阶段3: 补集成测试(写入->读取解析)
- [x] 阶段4: 更新 example README 用法(替代手工 JSONL)
- [x] 阶段5: cargo fmt + 定向测试 + 追加 notes/WORKLOG,并清理对应 LATER_PLANS 条目

### 实施

- CLI: `ralph emit` 新增 3 个可选字段并写入 JSONL:
  - `crates/ralph-cli/src/main.rs`
    - 新增 flags:
      - `--workspace-strategy shared|patch|worktree`
      - `--session-strategy exec|mcp|app_server`
      - `--turn-action start|steer|interrupt`
    - 写入 JSONL 时透传为字段:
      - `workspace_strategy` / `session_strategy` / `turn_action`
    - 这些字段名与取值与 `ralph_core::event_reader::Event` 的 snake_case 约定对齐。
- 测试: 增加“写入->读取解析”闭环断言:
  - `crates/ralph-cli/tests/integration_events_isolation.rs`
    - 新增 `test_ralph_emit_writes_optional_strategy_fields`:
      - 调用 `ralph emit ... --session-strategy app_server --turn-action steer`
      - 读取写入的 JSONL 最后一行
      - `serde_json::from_str::<ralph_core::Event>()` 断言字段值一致。
- 文档: example README 增强:
  - `examples/parallel-experimental-dev-engine/README.md`
    - steer/interrupt 现在优先推荐 `ralph emit` 直接写入控制字段,不需要手工追加 JSONL。

### 验证

- `cargo fmt` ✅
- `cargo test -p ralph-cli --test integration_events_isolation` ✅
- `cargo test -p ralph-cli` ✅

### 状态

**已完成** - `ralph emit` 现在可以用 flag 形式可靠写入 `session_strategy/turn_action/workspace_strategy`,支持 headless 场景的 steer/interrupt,并且已有集成测试锁死该行为。
