# LATER_PLANS

> 说明: 记录本次不落地,但值得后续跟进的事项. 仅追加到文件末尾.

## 2026-02-11 20:55 +0800 | Ralph Codex MCP 后续增强

- 主题: 跨进程恢复 `ralph#1/ralph#2` 的 Codex MCP `threadId`.
- 当前状态: 仅进程内常驻有效,进程重启后 thread 会重新建立.
- 后续建议:
  - 在 `.ralph/` 下落盘会话元信息(实例 -> threadId).
  - 恢复时做一次有效性探测,失效则自动重建.

## 2026-02-11 21:13 +0800 | parallel-hat-instances 场景稳定性收敛

- 背景: 真实 Codex 下场景存在行为漂移,`routing.escalate` 与 `LOOP_COMPLETE` 不稳定,导致 120s timeout.
- 建议后续动作:
  - 收敛 scenario 指令,减少对“模型必须输出 200 行/严格条件分支”的依赖.
  - 调整断言为“语义等价”而非固定实例计数(允许 `collector#2`/`writer#1` 接管).
  - 评估提升 `max_runtime_seconds` 或更早、确定性的 completion candidate.

## 2026-02-11 22:49 +0800 | 回写: parallel-hat-instances 场景稳定性收敛已完成

- 状态: 已完成并落地。
- 对应实现:
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs` 已收敛为稳定事件链与语义断言。
  - `scripts/run-parallel-hat-instances-codex.sh` 已修复 release 构建与 workspace 清理基线。
- 验证: `parallel-hat-instances` + `parallel-hat-instances-zh` 均通过。

## 2026-02-11 23:20 +0800 | 编译期 all_hat 配置变更提示能力

- 背景:
  - `config/all_hat.md` 已改为编译期内嵌,修改后必须重新编译才能生效.
- 后续建议:
  - 可在 `ralph doctor` 或启动日志中增加提示:
    - 检测工作区 `config/all_hat.md` 修改时间晚于二进制构建时间时,提示用户重编译.

## 2026-02-13 01:12 +0800 | session_strategy 后续增强(可选)

- 显式降级/重置语义:
  - 目前采用方案1(只升级,不降级).
  - 若未来确实需要从 mcp 回到 exec,建议新增 control-plane topic 或显式 reset 事件,并配套 handoff summary.
- MCP 会话资源收敛:
  - 目前 mcp session 会按 instance 创建,并在进程退出时统一 shutdown.
  - 若 hat autoscale 很多动态实例,可考虑按 dynamic idle TTL 同步关闭对应 mcp session.

## 2026-02-14 18:01 +0800 | E2E: Codex 后端降噪/提速(可选)

- 现状:
  - 并行 E2E 在真实 Codex 下,stdout/stderr 里会出现较长的思考/解释输出,并且 completion 的第二轮有时会被 max_runtime 逼近.
  - 这会让 E2E 时长和波动变大.

- 后续建议:
  - 在 `crates/ralph-e2e` 的场景生成 `ralph.yml` 时,对 codex backend 使用 `HatBackend::NamedWithArgs` 追加参数:
    - 指定更快的 `--model`.
    - 或用 `-c` 覆盖 config(例如 `-c model=...`),并把 reasoning 相关选项调低.
  - 为并行 runner 增加一个仅 E2E 使用的开关,允许 ralph#1 不强制走 MCP(改走 exec),以降低第二轮收敛的时延不确定性.

## 2026-02-14 20:41 +0800 | 回写: E2E Codex 后端降噪/提速已完成(按建议落地)

- 状态: 已完成并落地(仅影响 E2E 生成的 `ralph.yml`,不改默认配置).
- 对应实现:
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/starting_event_inference.rs`
  - 生成的 `cli` 配置改为 `backend: custom + command: codex + args: ["exec","--full-auto","-c",...]`.
  - 默认注入:
    - `-c model_reasoning_effort="low"`
    - `-c model_reasoning_summary="none"`
    - `-c rmcp_client=false`
- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `bash scripts/run-parallel-hat-instances-codex.sh` ✅
    - `parallel-hat-instances`: 77.9s
    - `parallel-hat-instances-zh`: 77.6s

## 2026-02-14 23:04 +0800 | 后续增强: event.id 端到端覆盖(可选)

- E2E 覆盖建议:
  - 目前 `crates/ralph-e2e` 的 `ExecutionResult.events` 只提取了 topic/payload/source_instance.
  - 可考虑在不大改现有断言的前提下,新增一条"文件级"断言:
    - 针对 `.ralph/events.jsonl`(debug log)逐行检查 `id` 字段存在且非空.
- 外部事件输入建议:
  - `crates/ralph-tui/src/external_event_writer.rs` 写入的外部事件 JSONL 目前不带 `id`.
  - 运行时会在 Supervisor 路由时补齐 `event.id` 并写入 debug log.
  - 如果你希望"外部输入文件"本身也可引用,可以为该 JSONL schema 增加可选 `id` 字段,默认填 `new_event_id()`.

## 2026-02-15 10:58 +0800 | 后续增强: parallel TUI 逐帧录制(可选)

- 需求动机:
  - 现在 `--record-session` 录的是 cassette(事件 + stdout),适合 replay/E2E.
  - 但它不包含 TUI 逐帧画面,不适合做 UI 动画回归或演示录像.
- 建议:
  - 在 `ralph-tui` 渲染 tick 里可选写入 `UxEvent::TuiFrame` 到 `SessionRecorder`.
  - 默认关闭,避免 JSONL 过大与额外 I/O 开销.
- 备注:
- `crates/ralph-core/src/session_recorder.rs` 已支持 `ux.tui.frame` 的 record 类型,属于 wiring 级补齐.

## 2026-02-15 12:26 +0800 | 后续可选: cassette `text` 字段的体积控制与旧数据补全

- 现状:
  - `ux.terminal.write` 现在会同时落盘 `bytes`(base64) + `text`(UTF-8 lossy).
  - cassette 可读性提升,但文件体积也会随之增大.
- 建议(可选):
  - 增加一个录制开关(例如 CLI flag 或 config),允许在“只做回放/CI”时关闭 `text` 以节省体积.
  - 提供一个小工具/子命令,把旧 cassette 批量补齐 `text` 字段,方便历史排障.
