# 笔记续档

- 旧文件超过1000行,已续档为 `notes_2026-02-11_204839.md`.

## 2026-02-11 21:12 +0800 | 并行 E2E 复验诊断结论

### 现场结论

- 命令: `bash scripts/run-parallel-hat-instances-codex.sh`
- 结果: `parallel-hat-instances` 与 `parallel-hat-instances-zh` 仍失败(各约 120s).
- 共性失败:
  - `routing.escalate` 缺失
  - `completion_seen=false`(未观察到 `LOOP_COMPLETE`)
  - 计数断言波动(不同 run 的 `writer#1/#2`、`collector#1/#2` 分配不同)

### 关键发现

- 该脚本此前只执行 `cargo build`(dev),但 harness 使用 `target/release/ralph` 运行。
- 这会导致“验证目标二进制”与“最新代码”可能不一致。
- 另外脚本原先只清 `events.jsonl/current-events`,未清整目录,存在 workspace 复用污染风险。

### 已落地修复

- `scripts/run-parallel-hat-instances-codex.sh`
  - 预构建改为 `cargo build --release -p ralph-cli --bin ralph`.
  - 运行前改为删除整个 `.e2e-tests/parallel-hat-instances` workspace 再重建,提升确定性.

### 仍待跟进

- 场景本身在真实 Codex 下存在行为漂移(尤其 `routing.escalate` 与 completion 触发),需要单独调整 scenario 约束或断言稳定性策略.

## 2026-02-11 22:48 +0800 | parallel-hat-instances 稳定性修复(通过)

### 根因归纳

- 原场景依赖模型行为较强:
  - `writer` 长输出 + `tester` 二次派发 + `collector` 条件分支,在真实 Codex 下容易发生时序漂移。
- 漂移后果:
  - 第二任务不稳定落到 `writer#2`.
  - `collector` 触发 `routing.escalate` 条件经常错过。
  - 最终 `LOOP_COMPLETE` 缺失并触发超时。

### 修复策略

- 把“可能漂移的实例选择”改为“确定性实例直达”:
  - `tester` 第二任务固定 `target_instance=writer#2`.
- 把“手工发 completion candidate”改为“运行时严格校验自动生成”:
  - `collector` 仅发 `build.task(target=ghost_hat)`,由 Supervisor 产出 `routing.escalate`.
- 断言从“固定实例精确计数”改为“语义稳定断言”:
  - 按 hat 汇总次数 + `writer#2` 出现校验.
- 适度提高场景 runtime 护栏:
  - `max_runtime_seconds: 180`.

### 验证结果

- `bash scripts/run-parallel-hat-instances-codex.sh`:
  - `parallel-hat-instances` ✅ (105.4s)
  - `parallel-hat-instances-zh` ✅ (121.6s)
- `cargo test -p ralph-e2e` 全通过。

## 2026-02-11 23:18 +0800 | all_hat 从运行时读取改为编译期内嵌

### 需求理解

- 用户要求 `config/all_hat.md` 作为编译时配置项,并在编译时注入到所有 hat prompt(包含 `ralph`).
- 语义重点: 运行时不再依赖 workspace 下的 `config/all_hat.md` 文件.

### 代码现状核查

- 注入链路覆盖已存在:
  - 串行: `EventLoop`.
  - 并行: `ParallelSupervisor` -> `HatInstanceActor` + dispatch decider.
- 但配置源是运行时读取:
  - `prompt_overlay::load_all_hat_prompt(&workspace_root)`.

### 改造结论

- `crates/ralph-core/src/prompt_overlay.rs`
  - 使用 `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/all_hat.md"))` 编译期内嵌.
  - `load_all_hat_prompt()` 改为无参,只返回内嵌内容(去首尾空白后).
- 调用点统一切换到无参加载:
  - `crates/ralph-core/src/event_loop/mod.rs`
  - `crates/ralph-core/src/parallel/supervisor.rs`
- 测试同步:
  - 单元测试新增 `load_all_hat_prompt_reads_compiled_overlay`.
  - `routing_tests` 从“临时写 workspace config”改为“断言内嵌内容锚点”.
  - `event_loop/tests.rs` 新增 ralph + custom hat 双路径注入回归测试.
  - 集成测试 `event_loop_ralph` 更新为编译期断言并重命名:
    - `test_ralph_prompt_includes_all_hat_overlay_from_compiled_config`.

### 验证结果

- `cargo fmt && cargo fmt --check` ✅
- `cargo test -p ralph-core --test event_loop_ralph test_ralph_prompt_includes_all_hat_overlay_from_compiled_config` ✅
- `cargo test` ✅

### 风险与影响

- 影响: 修改 `config/all_hat.md` 后需要重新编译才能生效.
- 收益: prompt 注入语义稳定,不受运行目录/文件缺失影响.

## 2026-02-12 23:55 +0800 | 现状核对: parallel 下 ralph(MCP) vs hat(exec)

### 关键代码位置

- `crates/ralph-cli/src/parallel_runner.rs`
  - `CliHatJobExecutor::should_use_codex_mcp()` 当前硬编码只对 `ralph#1/ralph#2` 生效.
  - 其他 hat 一律走 `codex exec --full-auto` 一次性 invocation.
- `crates/ralph-cli/src/codex_mcp_session.rs`
  - `CodexMcpRuntime` 已经支持 "按 instance_id" 维护 MCP session.
  - `sessions: HashMap<String, Arc<Mutex<CodexMcpSession>>>` 的 key 就是 instance_id 字符串.
  - 也就是说,技术上不止 ralph,writer#1 等 hat instance 也能复用同一 thread.

### 结论(对后续设计的约束)

- "Hat 是否 persistent" 目前是 executor 层策略,不影响 ralph-core 的调度/路由语义.
- 因此如果要做混合模式(C),最小改动点就是:
  - 让 executor 根据配置决定哪些 instance_id/hat_id 走 CodexMcpRuntime.

## 2026-02-13 01:09 +0800 | 已落地: 动态会话策略(session_strategy) + sticky(只升级)

### 代码要点

- `crates/ralph-proto/src/event.rs`
  - 新增 `SessionStrategy`.
  - `Event` 新增 `session_strategy: Option<SessionStrategy>`.
- `crates/ralph-core/src/event_parser.rs`
  - 解析 `<event ... session_strategy="exec|mcp">`.
- `crates/ralph-core/src/parallel/instance.rs`
  - 合并规则: pending events 任意请求 mcp -> job.mcp.
  - sticky 规则: instance 一旦进入 mcp -> 永久保持 mcp.
- `crates/ralph-cli/src/parallel_runner.rs`
  - ralph#1/#2 固定走 MCP.
  - 其他 hat 按 job.session_strategy 决定是否走 MCP.

### 文档同步

- `specs/parallel-hat-instances.spec.md`: 增加 session override 说明与示例.
- `config/all_hat.md`: 增加 session_strategy 使用说明(给 ralph/hats).
- `docs/plans/2026-02-13-parallel-session-strategy-design.md`: 本次设计文档.

### 验证

- `cargo fmt` ✅
- `cargo test` ✅
- `cargo test -p ralph-core smoke_runner` ✅

---

## 2026-02-14 08:47 +0800 | Codex App Server: turn/steer 关键语义摘录(用于 Ralph 设计)

### 结论先行

- Codex App Server 原生支持 `turn/steer` 与 `turn/interrupt`.
- 这意味着它具备 "同一 in-flight turn 内追加输入" 的协议能力.
- 相比之下,`codex mcp-server` 更像是 "下一轮继续",不具备 turn 内追加语义.

### 协议形态(实现侧会踩坑的点)

- 传输: JSONL over stdio 是默认(也可 ws).
- 协议: JSON-RPC 2.0,但报文里不带 `"jsonrpc":"2.0"` 字段.
- 生命周期: `initialize` -> `initialized` -> `thread/start` -> `turn/start` -> (可选) `turn/steer` / `turn/interrupt`.

### `turn/steer` 的硬约束(决定我们怎么接入)

- 必须带 `expectedTurnId`.
  - 这要求 executor 在 `turn/start` 后缓存 "当前活跃 turnId".
- `turn/steer` 会在同一 turn 内追加输入,不会创建新的 turn.
- 如果 thread 没有 in-flight turn,`turn/steer` 会失败.
- `turn/steer` 不支持 turn overrides(例如 model/cwd/sandboxPolicy 等).

### 对 Ralph 的直接映射建议

- `Event.session_strategy` 扩展为: `exec|mcp|app_server`.
- 继续沿用 "显式信号 + sticky(只升级)".
  - 这样 replay 才能复现真实选择.
- 增加 turn 级动作字段(建议叫 `turn_action` 或类似概念):
  - `start`(默认): 新开 turn.
  - `steer`: 对 in-flight turn 做 `turn/steer`.
  - `interrupt`: 对 in-flight turn 做 `turn/interrupt`(或等价 cancel).

### 参考

- 官方协议文档: `https://developers.openai.com/codex/app-server`

---

## 2026-02-14 08:55 +0800 | Event 需要 "可回复" 的最小协议: id(nanoid) + reply

### 用户新增需求

- 每个 event 需要一个可引用的 nanoid(或等价的稳定 id).
- event 需要新增属性 `reply=""`,用于表达 "我在回复哪个 event".
- 目标是让其他 hat 在回复某条 event 时,可以把该 event 的 nanoid 带入 reply,实现协作链路关联.

### 现状核对(代码事实)

- `ralph-proto::Event` 已经有 `id: Option<String>`,注释是 "Optional stable identifier for this event.".
- 并行模式下,实例侧会在 event 没有 id 时补齐:
  - 当前实现是 `"{instance_id}:{seq}"`(稳定可回放).
- Supervisor 自己生成的 event 也会补齐 id:
  - 当前实现是 `"supervisor:{seq}"`.
- 但是,并行模式下 hat 的 prompt 里 "Incoming Events" 目前只注入了 `topic + payload`,没有注入 `id`.
  - 这会导致: hat 看不到 event.id,因此无法在输出里写 `reply="<id>"`.

### 设计要点(最小改动,改良胜过新增)

- 优先复用现有字段 `Event.id` 作为 "nanoid/事件主键",避免再新增 `Event.nanoid` 造成双主键.
- 新增 `Event.reply: Option<String>`(或 `reply_to`),用于存储被回复的事件 id.
  - `<event ... reply=\"<event_id>\">...</event>` 解析进该字段.
- Prompt 必须把 incoming events 的 `id` 暴露给 hat.
  - 这样 writer/reviewer 才能把它原样带到 reply 字段里.

### 未决项(需要你确认口径)

1. "nanoid" 是不是就是 `Event.id` 的生成格式? 还是你坚持新字段名必须叫 `nanoid`?
2. `reply` 是单值(只回复一个 event)还是允许多值(例如逗号分隔或 JSON 数组)?

### 已确认口径(2026-02-14)

- 每条 event 都必须有可引用 id.
  - 这里的 id 复用 `Event.id`(不新增 `nanoid` 字段).
  - 允许保持现有稳定生成格式(例如 `{instance_id}:{seq}`),只要保证 "总会有".
- `reply` 是单值.
  - 一条 event 最多回复一个 event.id.
  - 多父依赖需要拆成多条回复事件,或选择一个主回复对象(由 ralph 协调约定).

---

## 2026-02-15 10:55 +0800 | parallel TUI: `--record-session` 录制口径核对

### 结论

- 并行 runtime 不再忽略 `--record-session`,会生成可回放的 JSONL cassette.
- parallel + TUI 时,`_meta.loop_start.ux_mode` 会写入 `"parallel-tui"`;不开 TUI 则是 `"parallel-cli"`.
- 录制内容以 "回放/解析" 为目标:
  - `bus.publish`: 每条业务事件一条.
  - `ux.terminal.write`: stdout-only,并带 `instance_id`(例如 `writer#1`)用于分流回放.
- 不录制 TUI 逐帧画面.
  - spec 明确写了 non-goal: 不要求录制 `ux.tui.frame`.

### 关键代码位置

- `specs/parallel-record-session.spec.md`: 目标/非目标/验收口径.
- `crates/ralph-cli/src/parallel_runner.rs`: 并行模式下 record-session wiring,以及 stdout-only + `instance_id` 归因.
- `crates/ralph-cli/src/loop_runner.rs`: 串行模式下 stdout-only 迭代输出写入 cassette.
- `crates/ralph-core/src/session_recorder.rs`: Record JSONL 格式,以及 `ux.terminal.write`/`bus.publish` 的序列化结构.

### 易踩坑

- 如果你用 `| tee out.txt` 这类管道,stdout 不是 TTY,并行会自动降级为 log 模式(看起来像 "TUI 没启动").
- cassette 默认只录 stdout:
  - 后端把主要输出打到 stderr 时,会看起来 "录不到内容".
  - 这是为了避免 stderr 里的 `<event ...>` 示例块造成假事件污染(并行语义要求).

---

## 2026-02-15 12:26 +0800 | `ux.terminal.write`: 增加 `text` 字段,便于诊断阅读

- 背景: 目前 cassette 里 `data.bytes` 是 base64,肉眼排障需要额外解码,非常不便.
- 方案: 保留 `bytes`(base64,回放/保真) + 新增 `text`(UTF-8 lossy,诊断可读).
- 关键约束: 回放与事件解析永远以 `bytes` 为准,`text` 只用于人类阅读 JSONL.
- 兼容性: `text` 是 `Option<String>`,旧 cassette 不带该字段也能正常解析.

---

## 2026-02-15 14:01 +0800 | 并行: stderr 可观测/可录制,但事件解析保持 stdout-only

### 现象

- 你给的 `parallel-experimental-dev-engine.jsonl` 只有 `stdout=true` 的 `ux.terminal.write`.
- 并行模式下 `ralph#1` 默认走 codex app-server.
  - 旧实现会后台消费 app-server stderr,但只打 tracing warn,不进入并行输出流.
  - 结果就是: "灰色 stderr 思考/诊断" 看不到,录制 cassette 也缺证据.

### 修复要点

- app-server stderr:
  - 转成 `HatJobOutputChunk{stream=Stderr}` 流式输出,交给并行 Supervisor/TUI 统一展示(灰色).
  - 仍保持"事件解析 stdout-only": stderr 永远不进入 `HatJobResult.output`.
- cassette:
  - parallel `--record-session` 录制 stdout+stderr,用 `ux.terminal.write.data.stdout` 区分.
  - smoke tests/ReplayBackend 过滤 stdout=false,避免 stderr 里的 `<event ...>` 假事件污染解析.
- `--hide-stderr`:
  - 只影响显示,不影响 cassette 录制(更利于排障回放).

---

## 2026-02-16 17:32 +0800 | follow-up: 自检实验补齐 stderr + `.ralph/current-events` 注入文档化

### 关键发现

- example 的自检实验如果只打印 stdout,很难用“肉眼 + cassette(JSONL)”确认 stderr 链路是否真的贯通。
  - 解决办法是让自检实验主动打印少量 stderr,从而稳定触发 `ux.terminal.write(stdout=false)`.
- 这里还有一个很隐蔽但高频的坑:
  - 在 Markdown 的 list item 里写 here-doc 示例时,如果把 `PY` 终止符缩进了,用户复制执行会直接卡住或报错。
  - 因此示例里必须保证 here-doc 内容与终止符 `PY` 都是行首对齐(不能带空格缩进)。

### 本次落地(只做最小补齐)

- `examples/parallel-experimental-dev-engine/PROMPT.md`
  - exp-par-001/002: stdout+stderr 同时输出,并修正 here-doc 缩进.
- `examples/parallel-experimental-dev-engine/README.md`
  - 明确 `--no-tui` 不适合持续对话(完成后退出).
  - 解释 `.ralph/current-events` marker 的作用,并给出手工 JSONL 注入 steer/interrupt 的示例.
- `crates/ralph-cli/src/codex_app_server_session.rs`
  - 保留 stderr 的兜底日志,但把默认等级降为 debug,避免 warn 刷屏(主要观测面在 parallel 输出流).

### 外部目录同步注意点

- 外部目录 `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/` 的 `PROMPT.md` 可能是用户正在使用的真实任务内容。
  - 为避免覆盖造成信息丢失,同步时先做了备份: `PROMPT_backup_2026-02-16_1731.md`。

---

## 2026-02-16 18:24 +0800 | `ralph emit` 参数化写入 session_strategy/turn_action/workspace_strategy

### 背景问题

- 并行模式下,headless 场景(例如 `--no-tui`)想做 steer/interrupt 时,
  需要在外部事件 JSONL 里写入 `session_strategy` 与 `turn_action`。
- 之前 `ralph emit` 只支持 `topic/payload/ts/target_instance`,
  导致用户不得不手工追加 JSONL,容易:
  - 写错字段名(例如大小写/下划线).
  - 写错文件(不小心写到旧 run 的 events.jsonl).

### 关键约束(对齐点)

- 外部事件 JSONL 的 schema 由 `ralph_core::event_reader::Event` 定义:
  - 字段名必须是:
    - `workspace_strategy`
    - `session_strategy`
    - `turn_action`
  - 取值必须是 snake_case(尤其是 `app_server` 这种带下划线的值)。
- 为了减少人为错误,CLI 层用 clap 的 `ValueEnum` 做值域约束:
  - `#[value(rename_all = "snake_case")]` 让 `AppServer` 对应 `app_server`。

### 验证方式(最小 backpressure)

- 用集成测试保证闭环:
  - `ralph emit` 写入一行 JSONL
  - 直接用 `serde_json::from_str::<ralph_core::Event>()` 解析
  - 断言可选字段的值严格一致

---

## 2026-02-16 18:24 +0800 | 文档同步: `config/all_hat.md` 补齐 app_server 语义

### 背景

- `config/all_hat.md` 之前只描述了 `session_strategy="mcp"` 的 sticky 规则.
- 当你已经引入 `session_strategy="app_server"`(Codex App Server)后,
  文档如果不更新,很容易造成误解:
  - 以为系统仍然只有 mcp.
  - 或以为 "mcp 的 sticky" 表示运行时一定在 mcp.

### 本次处理

- 直接更新 `config/all_hat.md` 的会话策略段落:
  - 增加 `session_strategy="app_server"` 的说明.
  - 明确强弱排序: `exec < mcp < app_server`.
  - 明确不要从强切回弱(避免上下文分裂).
  - 额外提醒: `mcp` 与 `app_server` 是两套常驻实现,升级也可能丢上下文,因此建议一开始就选定策略.
