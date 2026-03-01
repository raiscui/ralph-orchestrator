# ERRORFIX

> 说明: 历史 ERRORFIX 已归档到 `archive/ERRORFIX_2026-02-18_102134.md`(文件超过 1000 行自动轮转).


## 2026-02-18 10:21 +0800 | Fix: autopilot agent analysis 在 custom backend 下默认失败

### 现象

- 运行 `ralph autopilot analyze` 时:
  - hard verdict PASS.
  - agent analysis 失败,report.json/analysis_output.json 出现类似错误:
    - "Custom backend requires a command - set 'cli.command' in config".

### 根因

- `run_agent_analysis()` 之前只从主配置读取 `cli.backend` 字符串.
- 当 `cli.backend=custom` 时,生成的 `analysis_ralph.yml` 丢失 `cli.command/cli.args`.
- 子进程 `ralph run --config analysis_ralph.yml` 在 config validate 阶段直接退出.

### 修复

- `crates/ralph-cli/src/autopilot.rs`:
  - agent analysis 默认继承主配置的完整 `cli`(custom 时包含 command/args/prompt_mode/prompt_flag).
  - `--analysis-backend` 仅覆盖 backend 字段.
  - 在 analysis config 注入严格 `event_loop.ralph_prompt`,并收紧护栏:
    - `max_iterations=3`
    - `max_runtime_seconds=300`
  - 子进程若因护栏退出(exit code=2),但 stdout 已产出可解析的 `analyze.complete` JSON,则继续判定为分析成功.

### 验证

- `cargo fmt` ✅
- `cargo test -p ralph-cli` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

## 2026-02-18 14:07 +0800 | Fix: 并行模式动态 ralph#2 prompt 缺失导致 topic 漂移,进而触发 autopilot hard fail

### 现象

- 执行 `$parallel-engine-autopilot` 的并行 example 时,autopilot hard verdict 失败:
  - report.json 里 `required_topic:integration.applied` 缺失.
  - record-session JSONL 里出现了 `integration.done`(非协议 topic).

### 根因

- 并行 Supervisor 会在 `ralph#1` 忙时按需创建 `ralph#2` 作为备用协调实例.
- 之前 `ralph#2` 没有拿到与 `ralph#1` 等价的 coordinator instructions:
  - 缺少官方语义段落与 `event_loop.ralph_prompt` 注入.
  - 因此更容易 prompt 漂移,发布不在协议内的 topic(例如 `integration.done`),导致 autopilot/CI 硬断言失败.

### 修复

- 让所有 ralph 实例(包括动态创建的 `ralph#2`)都使用同一套 coordinator instructions 生成逻辑.
- 把 config 的 `event_loop.ralph_prompt` 同等注入到 `ralph#2`.
- 增加单元测试锁死行为:
  - `busy_ralph_secondary_includes_coordinator_instructions_and_config_prompt`

### 验证

- 实跑 autopilot 通过:
  - out_dir: `/tmp/ralph-autopilot-out-parallel-20260218-133409`
  - record-session: `/tmp/ralph-autopilot-out-parallel-20260218-133409/session.jsonl`
  - exit_code: 0
- `cargo fmt` ✅
- `cargo test -p ralph-core smoke_runner` ✅
- `cargo test` ✅

## 2026-02-23 18:28 +0800 | Fix: mock-cli allowlist 被空 `RALPH_MOCK_ALLOW` 意外覆盖

### 现象

- 运行 mock-mode 时,terminal 里能看到 `[E2E_CMD] ...`，但 allowlist 命令未执行:
  - `ralph emit ...` 没有被执行,因此 `spawn_instance` 不会发生.
  - 场景会卡到 `max_runtime_seconds`，最终 exit code=2 或直接失败断言.

### 根因

- `ralph-e2e` 的 mock-cli 子命令 allowlist 解析逻辑:
  - 环境变量 `RALPH_MOCK_ALLOW` 优先于 CLI `--allow`。
  - 但当该环境变量存在且为空字符串时,会把 allowlist 覆盖为空.
- allowlist 为空会导致 `execute_whitelisted_commands()` 直接 early return,命令完全不执行.

### 修复

- `crates/ralph-e2e/src/main.rs`:
  - 对 `RALPH_MOCK_ALLOW` 做 `trim()` 并过滤空字符串.
  - 空字符串视为“未提供”,不再覆盖 CLI allowlist.

### 验证

- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- --mock --filter parallel-emit-spawn-instance` ✅
- `cargo test` ✅

## 2026-02-23 21:12 +0800 | Fix: busy ralph 改投破坏 turn/steer 与 turn/interrupt

### 现象

- 并行模式下,当 `ralph#1` 处于 Running 时,外部显式投递到 `ralph#1` 的 in-flight 控制信号有概率被改投到 `ralph#2`:
  - `ralph emit ... --turn-action steer --target-instance ralph#1`
  - `ralph emit ... --turn-action interrupt --target-instance ralph#1`
- 结果:
  - `ralph#1` 的 in-flight turn 收不到 steer/interrupt,导致 turn 无法按预期被影响(例如 E2E 稳定卡死到 MaxRuntime)。

### 根因

- `crates/ralph-core/src/parallel/supervisor/routing.rs` 的 `rewrite_target_for_busy_ralph()` 会在 `ralph#1` Running 时把显式 `target_instance=ralph#1` 的事件改投到 `ralph#2`。
- 该策略对“普通协调面事件”有价值,但对 `turn_action=Steer|Interrupt` 属于语义破坏:
  - 这类事件必须直达正在运行的目标实例,否则无法影响 in-flight turn。

### 修复

- 对 `event.turn_action == Steer|Interrupt` 直接跳过改投逻辑,保持直达目标实例(例如 `ralph#1`)。
- 增加单元测试锁死:
  - `busy_ralph_primary_explicit_target_is_not_redirected_for_turn_steer`
  - `busy_ralph_primary_explicit_target_is_not_redirected_for_turn_interrupt`

### 验证

- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-23 23:12 +0800 | Codex app-server steer 回执缺失/过早 steer 失败,导致 live E2E 不可信

### 问题

- `parallel-app-server-steer-multi-turn-live`(真实 codex app-server)中:
  - human-log 看不到 `turn/steer` 的 response 回执.
  - 即使有回执,也可能是 error response,但 trace 不显示.
  - 部分 run 中 `turn/started` 后立刻发 steer,会收到 error: "no active turn to steer".
  - 部分 real 版本不稳定出现 `turn/completed`,导致 job 完成判定不可靠.

### 原因

- `CodexAppServerSession::trace_recv` 只记录 `{id,result}` response,忽略 `{id,error}` response.
  - 导致 e2e/human-log 里看不到“runner 回复了错误回执”,误以为没回复.
- `turn/started` 并不等价于“turn 已进入可 steer 的 active 状态”.
  - real codex 更可靠的门槛是 `codex/event/task_started`.
- completion 判定只等 `turn/completed`,在部分 real 版本下不稳定.

### 修复

- `crates/ralph-cli/src/codex_app_server_session.rs`
  - trace: 增加对 error response 的日志输出(包含 error_code/message 截断).
  - steer: 等 `codex/event/task_started` 后再 flush `pending_steers`,并提供 2s 兜底.
  - completion: 兼容 `codex/event/task_complete|task_completed` 作为完成信号.
  - 可观测性: 增大 stderr broadcast buffer,降低 trace 丢失概率.

- `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn_live.rs`
  - 断言: 要求 steer 成功 response>=2(排除 error_code),确保“能力可用”而非“只是收到了错误”.
  - human-log: 增加精选握手/回执证据段落.

### 验证

- `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn-live` PASS(看到 steer send+成功 response).

## 2026-02-24 09:56 +0800 | Fix: fake steer 计算结果为 <unknown> + steer 不触发(缺 task_started)

### 现象

- 场景: `parallel-app-server-steer-multi-turn`(fake codex app-server)
- 失败表现(两个阶段):
  1) fake app-server 不触发 `turn/steer`:
     - stdout 只看到 `WAITING_FOR_STEER`,随后卡到 `max_runtime_seconds`。
  2) 修复 flush 后,仍出现:
     - `TASK_FEEDBACK[*]: answer: <unknown>`
     - 断言找不到 `answer: 164`/`answer: 15`。

### 根因

1) steer flush 门槛升级后,fake app-server 与真实协议不一致:
- `CodexAppServerRuntime` 会优先等 `codex/event/task_started` 再认为“可安全 steer”。
- fake app-server 如果在 `turn/started` 后进入静默(不再推送 notify),
  并且 steer 发生在前 2s 内,会导致 pending steer 长时间无法被触发 flush。

2) Python regex 在 Rust raw string 中误用双反斜杠:
- 把 `\d` 写成 `\\d` 会让 regex 实际匹配字面量 `\d`,导致永远匹配不到数字。

### 修复

- `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn.rs`
  - fake app-server 在 `turn/started` 后补发 `codex/event/task_started`,
    对齐真实 app-server 的语义门槛。
  - 修正加法 regex: `r"(\d+)\s*\+\s*(\d+)\s*=\?"`(单反斜杠)。
  - human-log 摘录补齐 `TASK_FEEDBACK`/`answer:` 行,覆盖“任务反馈”证据。

### 验证

- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- codex --filter "multiple steers"` ✅

## 2026-02-24 20:02 +0800 | Fix: live-reply 场景“有 steer ACK 但看起来无回复”(补齐可见 answer 闭环)

### 问题

- 场景: `parallel-app-server-steer-live-reply-multi-turn`(真实 codex app-server)
- 失败表现(旧版本):
  - `turn/steer` 已 send/recv,但 stdout 里看不到 `TASK_FEEDBACK`/`answer`/`LOOP_COMPLETE`。
  - human-log 只有 RPC trace,难以判断“到底是模型没回复,还是输出没进我们可见通道”。

### 原因

- 真实 app-server 下,steer 输入进入 thread 历史,但不保证能在同一轮 in-flight turn 里立刻产生“可见回复”。
- 如果 prompt 让模型“等 steer 才输出”,模型可能长时间停留在 reasoning/summary 的输出节奏里,导致从 stdout 观测像“无回复”。

### 修复

- 新增/强化 live-reply 场景,改为“两轮 turn/iteration”闭环:
  1) 第 1 轮(`[task.start]`): 输出 30 行 `STEER_WINDOW_OPEN`,只负责开 steer 窗口,不结束 loop。
  2) 外部注入 2 次 `turn/steer`(含具体任务 payload)。
  3) 第 2 轮(emit `e2e.reply.step2`): 明确要求从 thread 历史读取两条输入并输出:
     - `TASK_REQUEST[n]`
     - `TASK_FEEDBACK[n]: answer: ...`
     - 最后 `LOOP_COMPLETE`
- human-log 补齐 hat runner stdout/state 摘录:
  - 直接摘录 `answer: 164/15` 与 `LOOP_COMPLETE`,让“是否真的有回复”一眼可读。

### 额外踩坑(已规避)

- 真实 codex app-server 不支持 `type=inputText`:
  - error: `-32600 unknown variant inputText, expected text/image/localImage/skill/mention`
  - 因此继续使用 `type=text`。

### 验证

- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-live-reply-multi-turn` ✅

### 证据

- `.e2e-tests/artifacts/parallel-app-server-steer-live-reply-multi-turn/human-log.md`
- `.e2e-tests/artifacts/parallel-app-server-steer-live-reply-multi-turn/stdout.txt`

## 2026-02-25 19:22 +0800 | 修复: ralph-e2e 误选旧 release ralph 导致 `--idle-start` 报 "unexpected argument"

### 问题

- 现象:
  - 运行新场景 `parallel-app-server-idle-start` 时,stderr 报:
    - `error: unexpected argument '--idle-start' found`
  - 导致 E2E stdout 为空,injector 等不到 `.ralph/agents.json`，最终以超时/无回复失败。

### 原因

- `crates/ralph-e2e/src/executor.rs` 的 `resolve_ralph_binary()` 之前无脑优先 `target/release/ralph`。
- 在开发期常见状态是:
  - 旧的 `target/release/ralph` 仍存在
  - 新的改动只编译到了 `target/debug/ralph`(例如你刚跑了 `cargo test`)
- 结果: E2E 实际跑的是旧 release,因此缺少新 flag(例如 `--idle-start`)。

### 修复

- `resolve_ralph_binary()` 调整为:
  - release/debug 都存在时,按文件 mtime 选择更新的那个。
  - 避免 E2E 被旧二进制污染,减少“假失败”。

### 验证

- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- codex --filter "fake codex shim"` ✅

## 2026-02-25 23:53 +0800 | Fix: example 子目录运行时报 "Prompt file ... not found"

### 现象

- 在 `examples/parallel-experimental-dev-engine/` 目录执行 `ralph run`(或 `cargo run --bin ralph -- run`)时,报错:
  - `Prompt file 'examples/parallel-experimental-dev-engine/PROMPT.md' not found...`

### 原因

- `examples/parallel-experimental-dev-engine/ralph.yml` 的 `event_loop.prompt_file` 使用了仓库根目录相对路径.
- 当用户在 example 子目录内直接运行时,该路径不再成立,导致找不到 prompt 文件.

### 修复

- 让 example 目录自包含:
  - `examples/parallel-experimental-dev-engine/ralph.yml`: `prompt_file` 改为 `PROMPT.md`(同目录).
  - `examples/parallel-experimental-dev-engine/README.md`: 同步更新运行方式说明.
- 增加回归测试:
  - `crates/ralph-cli/tests/integration_examples.rs`: 断言 example config 包含 `prompt_file: "PROMPT.md"` 且不再写死仓库根路径.

### 验证

- 在 example 目录 `--dry-run` 显示 `Prompt file: PROMPT.md`.
- `cargo test -p ralph-cli` ✅

## 2026-02-26 10:08 +0800 | Fix: 并行 TUI idle chat 下 ralph#1 自己回复自己(human.message loop)

### 现象

- 在并行 TUI 的 chat idle 模式下,给 `ralph#1` 发一条 `human.message` 后:
  - ralph#1 会发布 `human.message` 作为“回复”。
  - 该回复又会被 Supervisor 再次路由回 ralph,触发 ralph#1 继续回复自己的 `human.message`,形成循环。
- 复现证据:
  - `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-talk-example.jsonl`

### 原因

- `human.message` 的协议语义是“外部输入”(human -> hats)。
- 但 parallel Supervisor 的默认路由会把 hat 产出的任何事件(包括 `human.message`)继续参与路由。
- 当 ralph#1(兜底协调者,订阅 "*")反向发布 `human.message` 时,该事件会被再次投递给 ralph,从而形成自我对话回路。

### 修复

- 在 `ParallelSupervisor::route_event()` 增加护栏:
  - 若 `topic=="human.message"` 且事件带 `source` 或 `source_instance`:
    - 仍推送给 TUI event_observer 做展示。
    - 但不再参与后续 routing/delivery(直接返回),从机制上打断回路。
- 对应文件:
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`

### 验证

- 回归测试:
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`:
    - `parallel_does_not_route_hat_sourced_human_message_to_prevent_self_chat_loop`
- `cargo test -p ralph-core` ✅

## 2026-02-26 12:21 +0800 | Fix: ralph 回复 topic 改为 reply.human.message + 忽略该 topic 防循环; app-server trace 打印 turn/start prompt

### 现象

1) 回复 topic 不够干净:
- ralph#1 在回复 human 时会输出:
  - `<event topic="human.message" reply="...">...`
- 这会让 `human.message` 同时像“输入”和“输出”,理解成本高,也更容易再次引入自问自答回路。

2) app-server trace 看不到启动注入的 prompt:
- 即使设置:
  - `RALPH_CODEX_APP_SERVER_TRACE=1`
  - `RALPH_CODEX_APP_SERVER_TRACE_STEER_INPUT=1`
- 仍然看不到 `turn/start` 注入的 prompt(你反馈以前 `codex exec` 能从 stderr 看到)。

### 原因

- `human.message` 没有区分输入/回复输出语义。
- app-server trace 之前只对 `turn/steer` 打印 input 预览,`turn/start` 只打印 method/id,所以看不到启动 prompt。

### 修复

1) 语义拆分:
- 引入 `reply.human.message` 作为“回复输出 topic”。
- `ParallelSupervisor::route_event()` 对 `reply.human.message` 做 UI-only early-return:
  - 允许 TUI 展示/录制。
  - 但不参与路由,避免 ralph(订阅 "*")再次收到导致循环。
- ralph#1 内置协调器指令明确:
  - human-facing reply 必须用 `reply.human.message`。

2) turn/start trace:
- 在 `turn/start` 的 send trace 中输出:
  - `input_len`
  - 开启 `RALPH_CODEX_APP_SERVER_TRACE_STEER_INPUT=1` 时输出 `input_preview`(截断)。

### 验证

- 回归测试:
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`:
    - `parallel_does_not_route_reply_human_message_topic`
- `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅

## 2026-02-26 10:57 +0800 | Fix: app-server 默认显示注入 prompt(像 codex exec stderr) + 保留 ANSI 色彩

### 现象

- Codex app-server(session_strategy=app_server)下,默认看不到 "turn/start 注入了什么".
  - 你期望像以前 `codex exec` 一样,stderr 会显示 ralph prompt / sys / user 等转录.
- 并行输出展示里 stderr 会被弱化:
  - TUI 的 stderr-muted 会覆盖 fg,导致 ANSI 色彩信息被吞掉.
  - log mode 对 stderr 外层包 GRAY,也会破坏原始色彩语义.

### 原因

- `codex app-server` 通道本身不负责 echo prompt.
- 之前的 ralph 仅在 trace env 下输出 turn/start 的截断预览,默认不可观测.
- TUI/log-mode 的 stderr 弱化策略是 "无条件覆盖",未考虑 "stderr 自带 ANSI" 的情况.

### 修复

1) 默认 prompt transcript
- 在 `CodexAppServerRuntime::execute_job()` 每次 `turn/start` 前,把完整 prompt 以多行 transcript 写入 stderr 流.
- transcript 支持 ANSI 色彩,并受 `--color` 控制.
- 对应文件:
  - `crates/ralph-cli/src/codex_app_server_session.rs`

2) ANSI 保真
- TUI: 当 stderr chunk 含 ANSI 时,不再强制 muted 覆盖 fg.
  - `crates/ralph-tui/src/state/parallel.rs`
- log mode: 当 stderr 行含 ANSI 时,不再外层包 GRAY.
  - `crates/ralph-cli/src/parallel_runner.rs`

### 验证

- 新增回归测试:
  - `crates/ralph-cli/src/codex_app_server_session.rs`: transcript 生成包含 ANSI,并保留尾部空行.
  - `crates/ralph-tui/src/state/parallel.rs`: stderr 含 ANSI 时不被 force muted.
- `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅

### 额外验证

- `cargo test` ✅

## 2026-02-26 14:50 +0800 | 修复: reply.human.message 偶发缺 `</event>` 导致 chat 看起来“没回复”; app-server thinking 不可见

### 现象

- 并行 TUI chat 里,有时能看到 stdout 里输出了 `<event topic="reply.human.message" ...>` 的开头与正文,
  但缺少 `</event>`。
- Supervisor 的 `EventParser` 需要看到闭合标签才能发布 `bus.publish`,
  因此 UI 侧会像“问了但没回复”(或回复不进入事件链)。
- 同时在 `session_strategy=app_server` 路径下,你看不到像 `codex exec` 那样的 thinking 文本
  (真实 app-server 会推送 `item/reasoning/summaryTextDelta`)。

### 根因

- 协议层: `<event ...>` 是严格闭合标签; 模型偶发未按协议输出,会导致事件丢失。
- 运行时: app-server runtime 之前在切到 `item/agentMessage/delta` 后,会忽略后续 summary delta,
  导致 thinking 不可见。
- 解析器: 遇到“未闭合但后面又出现新 `<event ...>`”的输出时,
  简单的 `find("</event>")` 会把后续 event 的 closing tag 误当成当前 event 的 closing,
  进而吞掉后续事件(不稳健)。

### 修复

1) prompt 硬约束(减少模型违约概率)
- `crates/ralph-core/src/parallel/supervisor.rs`
  - 明确要求每个 `<event ...>` 必须闭合 `</event>`。
  - 推荐单行 event,降低跨行/截断导致的解析失败。

2) app-server thinking 回显(不影响事件解析)
- `crates/ralph-cli/src/codex_app_server_session.rs`
  - 当输出源为 `AgentMessageDelta` 时:
    - `item/reasoning/summaryTextDelta` 持续回显到 stderr(更像 `codex exec` 的体验)。
    - 但不进入 `HatJobResult.output`(仍坚持 stdout-only 事件解析边界)。

3) parser 最小容错(只针对 UI-only 回复 topic)
- `crates/ralph-core/src/event_parser.rs`
  - `reply.human.message` 若在 EOF 缺失 `</event>`,允许把 EOF 当作隐式闭合,避免 chat 看起来“没回复”。
  - 若 payload 内出现新的 `<event `,则认为当前 event 未闭合,跳过并继续扫描后续事件,避免吞事件。

### 验证

- `cargo fmt --check` ✅
- `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅

## 2026-02-26 21:29 +0800 | 修复: 并行模式中断退出导致 record-session JSONL 尾部丢失(误判“没回复就 end”)

### 现象

- `--record-session` 生成的 JSONL 里:
  - 只有 `_meta.loop_start` + prompt transcript 等早期输出。
  - 缺少 `reply.human.message` 与 `_meta.termination`。
- 但 `.ralph/events.jsonl` 里能找到同一条 `human.message` 的真实回复,说明“系统确实回复了”。

### 根因

- `crates/ralph-cli/src/parallel_runner.rs` 的 interrupt 分支会早退:
  - 退出前没有写 `_meta.termination`,也没有 `SessionRecorder.flush()`。
- main 对非 0 exit code 可能调用 `std::process::exit(...)`,
  `BufWriter<File>` 的尾部缓冲区无法保证落盘。

### 修复

- 在 parallel runner 的两条早退路径补齐:
  - 写 `_meta.termination`
  - `recorder.flush()`
  - 适用路径:
    1) Ctrl+C/SIGTERM/SIGHUP interrupt
    2) supervisor.run 返回 Err(e)
- 对应文件:
  - `crates/ralph-cli/src/parallel_runner.rs`

### 验证

- `cargo test -p ralph-cli` ✅

## 2026-02-27 12:38 +0800 | 修复: 子目录执行 `ralph emit` 可能写错 events 文件; coordinator prompt 误导“只能发 `<event>`”

### 现象

- 在并行 idle/chat 场景里,你希望:
  - 随时用 `ralph emit ... --target-instance <hat#n>` 向任意 instance 注入消息/steer。
  - 且不必被 prompt 误导为"只能输出 `<event ...>...</event>` 才能发消息"。
- 但当你在子目录(尤其是 `.ralph/worktrees/...`)执行 `ralph emit` 时:
  - 可能读不到 `.ralph/current-events` marker,从而写入错误的 fallback `.ralph/events.jsonl`。
  - 结果表现为"注入了但 run 没反应"或"events 路径不对"。

### 根因

- `ralph emit` 与 `ralph events` 之前只在当前工作目录读取 `.ralph/current-events`:
  - 在子目录执行时,无法命中 workspace root 的 marker。
- marker 内容通常是相对路径(例如 `.ralph/events-<run_id>.jsonl`):
  - 若按当前 cwd 解析,也可能拼出错误路径。

### 修复

1) coordinator prompt 明确双通道 + 单轮多事件
- `crates/ralph-core/src/parallel/supervisor.rs`
  - 允许单轮输出多条 `<event ...>...</event>`。
  - 明确: 当 backend 支持 tool/shell 时,也可以 out-of-band 执行 `ralph emit ...` 注入事件。

2) `ralph emit`/`ralph events` 子目录自动定位 marker
- `crates/ralph-cli/src/main.rs`
  - 向上遍历父目录寻找最近的 `.ralph/current-events`。
  - 解析 marker 相对路径时,以 workspace root(包含 `.ralph/` 的目录)为基准。
  - 并复用到 `ralph events` 保持一致行为。

### 验证

- `cargo fmt --check` ✅
- `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-28 15:26 +0800 | 修复: macOS `cargo test` 报 "You have not agreed to the Xcode license"(exit 69)

### 现象

- 运行 `cargo test`(或某些会触发编译/链接的命令)时,报错:
  - `You have not agreed to the Xcode license agreements...`
  - 并以 exit code 69 退出。

### 根因

- `cargo test` 会触发编译/链接,间接调用 Apple toolchain(例如 clang/ld)。
- 本机 `xcode-select -p` 指向 `/Applications/Xcode.app/Contents/Developer`。
- 当 Xcode.app license 未接受时,`xcrun` 会拒绝执行 clang/ld,从而让 cargo 的构建/链接失败。

### 修复

- 无需 sudo 的修复(推荐本仓库开发/CI 语境): 跑 cargo 时显式指定 Command Line Tools:
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test ...`
- 全局修复(需要管理员权限,不适合在受限环境里做):
  - `sudo xcodebuild -license accept`
  - 或 `sudo xcode-select --switch /Library/Developer/CommandLineTools`

### 验证

- `DEVELOPER_DIR=/Library/Developer/CommandLineTools xcrun --find clang` ✅
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test -p ralph-cli` ✅
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test -p ralph-core smoke_runner` ✅
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test` ✅
