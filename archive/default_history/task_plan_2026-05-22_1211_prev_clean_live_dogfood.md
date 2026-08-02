# 任务计划: dogfood worker MaxRuntime 调试

## [2026-05-20 16:33:00] [Session ID: omx-1779158263949-kticiv] 新任务启动: 单独处理 dogfood worker MaxRuntime

## 目标

找出 parent-visible topology spawn dogfood 仍然 `MaxRuntime` 的原因,明确它是 worker prompt / runtime gate / backend hooks / output parser / completion policy 哪一层的问题,并给出可验证修复或方案。

## 阶段

- [x] 阶段1: 读取 record-session、stdout/stderr、agents snapshot 和 runtime events,确认已观察事实。
- [x] 阶段2: 按现象 -> 假设 -> 验证计划拆分候选原因。
- [x] 阶段3: 做最小可证伪实验或 focused log/script,验证主假设。
- [x] 阶段4: 如需要修改代码或 prompt,先补测试再改动。
- [x] 阶段5: 运行 focused tests / dogfood 验证,并记录 ERRORFIX/WORKLOG。

## 关键问题

1. `MaxRuntime` 是否因为 worker 没发 `analysis.done`,还是发了但 runtime 没解析/路由?
2. worker 失败是 Codex CLI hook/工具噪音导致,还是 prompt 没要求结构化 event?
3. `analysis.done` 是否被配置为 completion candidate,以及 ralph#1 是否能在收到后结束?
4. 这个问题是否可以用更窄的 fake backend / fixture 复现,避免每次跑 live Codex?

## 当前证据入口

- record-session: `/tmp/ralph-topology-dogfood-guardrail-record.jsonl`
- stdout: `/tmp/ralph-topology-dogfood-guardrail.stdout`
- stderr: `/tmp/ralph-topology-dogfood-guardrail.stderr`
- 临时配置: `/tmp/ralph-topology-dogfood-guardrail.yml`
- 临时 prompt: `/tmp/ralph-topology-dogfood-prompt.txt`

## 当前假设

- 暂不确认根因。
- 候选方向包括: worker 未输出 `analysis.done`, worker 输出但 parser 没识别, worker 被 hooks/tool 噪音拖住, completion policy 等待更多 worker 结果,或 gate timeout 与 MaxRuntime 交互。

## 状态

**正在阶段1** - 先解析 durable evidence,不凭 stdout 片段下结论。

## [2026-05-20 16:34:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 解析 dogfood evidence

行动目的:
- 明确 `MaxRuntime` 的已观察事实。
- 区分 worker 未发 event、event 未解析、event 已解析但未路由、以及 completion policy 未收敛。

即将执行:
- 解析 `/tmp/ralph-topology-dogfood-guardrail-record.jsonl` 的 bus.publish、agent output 和 termination。
- 检查 stdout/stderr 中各 instance 的 state、job、错误输出和 event tag。
- 检查临时配置的 hats / completion candidate。

状态:
- **阶段1进行中** - 先收集动态证据。

## [2026-05-20 16:45:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 检查 parallel event 解析与 default_publishes

行动目的:
- 验证 parallel worker 完成但没有输出 bus event 时,runtime 当前会如何处理。
- 对比 serial path 的 default_publishes 行为,避免把配置缺口误判为 worker prompt 问题。

即将执行:
- 读取 `crates/ralph-core/src/parallel/instance.rs` 中 prompt assembly、job completed 和 event parser 路径。
- 读取 `crates/ralph-core/src/event_loop/mod.rs` 中 default_publishes 相关逻辑。
- 检查临时 dogfood config 是否配置了 completion candidate / default_publishes。

状态:
- **阶段1 -> 阶段2** - 从动态证据进入候选原因拆分。

## [2026-05-20 16:48:00] [Session ID: omx-1779158263949-kticiv] 验证记录: bounded worker 实验 A 仍 MaxRuntime 但出现 1 条 analysis.done

实验设置:
- 临时配置: `/tmp/ralph-topology-dogfood-bounded.yml`
- 只收窄 analyst instructions: 最多 2 条 shell/search 命令、最多 5 个文件、final 只能输出 1 个 `analysis.done` event。
- 保持 worker hooks 开启,只保留 coordinator `features.hooks=false`。

关键输出:
- record-session 仍然 `reason=MaxRuntime`, `elapsed_secs≈90`。
- `bus.publish` 出现 1 条 `analysis.done`,来源为 `analyst#3`。
- `analyst#2` 与 `analyst#4` 最终进入 failed/done,未发布 `analysis.done`。

当前意义:
- 已推翻“parallel parser 无法解析 worker stdout 里的 analysis.done”的强备选解释。
- 仍需继续确认两个 worker 失败是 backend/network、hooks 开销、prompt contract 不足,还是 runtime 没有 worker no-event 诊断/default fallback。

## [2026-05-20 16:53:00] [Session ID: omx-1779158263949-kticiv] 验证记录: parallel default_publishes focused test 通过

已执行:
- `cargo test -p ralph-core parallel::supervisor::routing_tests::parallel_default_publishes_injects_when_worker_finishes_without_event -- --exact --nocapture`

结果:
- passed。

当前结论:
- parallel path 之前缺少 serial `default_publishes` 的等价补偿,已用 focused regression 锁住。
- 但 bounded dogfood 还暴露第二层竞态: `analysis.done` stdout 已出现,record-session 中没有 bus.publish,随后 `_meta.termination(reason=MaxRuntime)`。

下一步:
- 继续写一个 focused test 复现 MaxRuntime 与 late `JobCompleted` 的边界,确认是否需要在 shutdown drain 中持久化/路由已完成 job 的事件。

## [2026-05-20 16:56:00] [Session ID: omx-1779158263949-kticiv] 口径更正: bounded 实验是 stdout event,不是 bus.publish

更正内容:
- 上一条记录中“`bus.publish` 出现 1 条 `analysis.done`”表述不准确。
- 准确证据是: `/tmp/ralph-topology-dogfood-bounded-record.jsonl` 第 5934 行存在 `analyst#3` 的 stdout `<event topic="analysis.done" ...>`。
- 但同一 record-session 的 `bus.publish` 仍然没有 `analysis.done`。

当前结论:
- 这不是 parser 完全不能解析的问题。
- 更像是 stdout 已到达 UX/record 流,但 job result 尚未被 Supervisor 作为 `JobCompleted` 处理时,`MaxRuntime` tick 先结束主循环。
- 如果 `JobCompleted` 随后落在 shutdown drain 里,当前 drain 逻辑会忽略 `JobCompleted` / `Published`,造成 late result 不落入 durable event log。

## [2026-05-20 17:08:00] [Session ID: omx-1779158263949-kticiv] 验证记录: 180 秒 dogfood 成功收敛

已执行:
- 临时配置: `/tmp/ralph-topology-dogfood-bounded-180.yml`
- 命令: `cargo run --bin ralph -- run --no-tui --hide-stderr --color never -c /tmp/ralph-topology-dogfood-bounded-180.yml --record-session /tmp/ralph-topology-dogfood-bounded-180-record.jsonl -P /tmp/ralph-topology-dogfood-bounded-prompt.txt`

关键证据:
- `topology.spawn_group` 发布。
- `analysis.task` 直接投递给 `analyst#2/#3/#4`。
- 3 条 `analysis.done` 全部进入 `bus.publish`。
- `ralph#1` 最终输出 `LOOP_COMPLETE`。
- `_meta.termination`: `reason=CompletionPromise`, `elapsed_secs≈85.8`, `iterations=5`。

当前结论:
- parent-visible dynamic spawn 和 worker result routing 已经可以自然收敛。
- 原始 90/120 秒 `MaxRuntime` 的主要触发因素是 worker repo-grounded live Codex 调查 + coordinator 多轮收敛时间预算不足。
- 代码层同时发现并修补了 parallel path 缺少 serial `default_publishes` 等价语义的缺口,已用 focused test 锁住。

下一步:
- 运行格式化和 focused tests。
- 记录 ERRORFIX/WORKLOG。

## [2026-05-20 17:16:00] [Session ID: omx-1779158263949-kticiv] 最终状态: 已完成验证与收尾

状态修正:
- 16:53 时计划继续写 MaxRuntime late-drain focused test。
- 后续 180 秒 dogfood 提供了更强动态证据: 同一链路能在 `elapsed_secs≈85.8` 内完成 3 条 `analysis.done` 并 `CompletionPromise` 收敛。
- 因此当前不继续扩大修改 shutdown drain,避免在证据不足时改变 MaxRuntime 的硬护栏语义。

最终结论:
- 本轮代码修复限定在 parallel `default_publishes` parity。
- live dogfood 的稳定运行方式是 bounded worker contract + 足够 runtime budget。
- 所有阶段已完成,验证门禁已通过。

最终验证:
- focused default_publishes test: passed。
- ralph-core smoke_runner: passed。
- 180 秒 bounded live dogfood: `CompletionPromise`。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test`: passed。

## [2026-05-20 17:22:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 复跑 3-worker live dogfood

行动目的:
- 按用户要求继续跑同一个 parent-visible 3-worker live dogfood。
- 使用独立 record-session 保存证据,验证修复后的链路是否再次稳定收敛。

即将执行:
- 使用 `/tmp/ralph-topology-dogfood-bounded-180.yml`。
- 使用 `/tmp/ralph-topology-dogfood-bounded-prompt.txt`。
- 输出到新的 `/tmp/ralph-topology-dogfood-bounded-180-rerun-*` 文件。

状态:
- **复跑验证进行中** - 目标是看到 3 条 `analysis.done` 和 `CompletionPromise`。

## [2026-05-20 19:01:00] [Session ID: omx-1779158263949-kticiv] 最终状态: 3-worker live dogfood 复跑完成

完成内容:
- 等待已启动的 live dogfood 结束,未重复启动新 run。
- 解析 record-session、stdout/stderr、`.ralph/agents.json`。
- 整理三个 worker 的输出并写入 `notes.md`。
- 将本次工作写入 `WORKLOG.md`。

验证证据:
- record-session: `/tmp/ralph-topology-dogfood-bounded-180-rerun-20260520-185717.jsonl`
- `ralph record summary`: `Termination reason=CompletionPromise`
- topics: `analysis.done=3`, `analysis.task=3`, `topology.spawn_group=1`, `topology.spawn.result=1`
- `_meta.termination`: `elapsed_secs=85.605838208`, `iterations=4`, `reason=CompletionPromise`
- `.ralph/agents.json`: 包含 `analyst#2/#3/#4` dynamic instances,`analyst#4` 有 fixed-role metadata。

最终结论:
- 复跑成功。
- 三个 worker 的结果有用,但价值分层不同: evidence inspect / 显示契约 / acknowledgement guardrail 可转近期任务; Claude stream-json adapter 应单独立项。
- 本轮没有发现需要立即新增 `ERRORFIX.md` 的新 bug。

## [2026-05-20 19:03:00] [Session ID: omx-1779158263949-kticiv] 收尾记录: 后续候选任务已登记

已登记到 `LATER_PLANS.md`:
- topology/capability evidence inspect。
- parent-visible spawn replay/integration guardrail。
- TUI/plain 显示验收。
- Claude stream-json adapter capability negotiation 单独评估。

EPIPHANY 检查:
- 本轮没有发现新的架构级灾难点。
- 不追加 `EPIPHANY_LOG.md`。

## [2026-05-20 19:10:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 实装统一 evidence inspect

行动目的:
- 按用户确认,优先做一个能证明 topology / child-run / agents / result / termination 的统一 evidence inspect。
- 优先改良现有 `ralph record summary`,避免新增过度分散的命令入口。

已定方向:
- 不先做 Claude stream-json。
- 不先做新的调度智能。
- 先把 record-session 和 `.ralph/agents.json` 的证据合并成用户可读摘要。

阶段:
- [ ] 阶段1: 读取现有 CLI record summary / agents snapshot / capability runtime 代码。
- [ ] 阶段2: 写规格和测试计划。
- [ ] 阶段3: 先写 focused tests。
- [ ] 阶段4: 实现 evidence inspect 输出。
- [ ] 阶段5: 运行 focused tests、smoke tests、格式检查,并用最新 dogfood record 做真实验证。

状态:
- **阶段1进行中** - 先确认现有代码路径,避免凭印象改错入口。

## [2026-05-20 19:16:00] [Session ID: omx-1779158263949-kticiv] 阶段更新: unified evidence inspect 规格已落地

已完成:
- 新增 `specs/unified-evidence-inspect.spec.md`。
- 明确 `ralph record summary` 的 `Evidence Inspect` 输出契约。
- Mermaid flowchart 和 sequenceDiagram 已用 `beautiful-mermaid-rs --ascii` 验证通过。

下一步:
- 先写 focused tests,覆盖 topology、agents snapshot、child-runs、result topics、termination。

状态:
- **阶段2完成,进入阶段3** - 准备先写测试。

## [2026-05-20 19:26:00] [Session ID: omx-1779158263949-kticiv] 阶段更新: focused test 和 CLI 接入已完成

已完成:
- 在 `crates/ralph-cli/src/record_session.rs` 增加 evidence 聚合结构。
- 在 `crates/ralph-cli/src/record_session.rs` 增加 `render_evidence_inspect`。
- 在 `crates/ralph-cli/src/record_cli.rs` 将 `Evidence Inspect` 接入 `ralph record summary`。
- 新增 `--agents-file FILE`,用于显式指定 agents snapshot sidecar。
- focused test `record_session::tests::aggregate_collects_evidence_inspect` 通过。
- 用真实 dogfood record 试跑,能看到 topology spawn、agents snapshot、child_runs、result topics 和 termination。

当前观察:
- 输出已能一眼证明 `analyst#2/#3/#4` 是 parent-visible dynamic instances。
- 输出已能证明本轮 `analysis.done=3` 且 termination 是 `CompletionPromise`。
- 输出已能证明 child_runs 为 0,不是 isolated child-run。

下一步:
- 跑格式化、focused/full relevant tests 和 smoke_runner。
- 用真实 dogfood record 做最终输出验证。

状态:
- **阶段3和阶段4完成,进入阶段5** - 开始验证门禁。

## [2026-05-21 07:25:47] [Session ID: omx-1779158263949-kticiv] 最终状态: unified evidence inspect 已完成

阶段完成情况:
- [x] 阶段1: 读取现有 CLI record summary / agents snapshot / capability runtime 代码。
- [x] 阶段2: 写规格和测试计划。
- [x] 阶段3: 先写 focused tests。
- [x] 阶段4: 实现 evidence inspect 输出。
- [x] 阶段5: 运行 focused tests、smoke tests、格式检查,并用最新 dogfood record 做真实验证。

交付内容:
- `specs/unified-evidence-inspect.spec.md`
- `crates/ralph-cli/src/record_cli.rs`
- `crates/ralph-cli/src/record_session.rs`
- `crates/ralph-cli/tests/integration_record_session.rs`

最终验证:
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-cli --test integration_record_session -- --nocapture`: passed。
- `cargo test -p ralph-cli record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo test`: passed。
- 真实 dogfood record 的 `Evidence Inspect` 输出可证明 parent-visible topology、child-run absence、agents snapshot、result topics 和 termination。

EPIPHANY 检查:
- 本轮没有新增架构级灾难点。
- 关键规律已记录在 notes/WORKLOG/LATER_PLANS,不追加 `EPIPHANY_LOG.md`。

## [2026-05-21 07:32:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 继续第 2 项 TUI/plain 显示验收

行动目的:
- 用户选择第 2 项,即补 TUI/plain 显示验收。
- 目标是让 parent-visible dynamic instance、child-run 状态、result/termination 这些关键信息不只存在于 record summary,也能在 TUI/plain 显示层被测试锁住。

阶段:
- [ ] 阶段1: 读取现有 TUI state/widgets/plain runner 输出与测试。
- [ ] 阶段2: 明确缺口和最小验收矩阵。
- [ ] 阶段3: 先补 focused tests。
- [ ] 阶段4: 如测试暴露缺口,再做最小实现修正。
- [ ] 阶段5: 运行 TUI/plain focused tests、smoke_runner、cargo test 和必要的文本/截图验证。

状态:
- **阶段1进行中** - 先做只读定位,避免重复实现已经存在的显示能力。

## [2026-05-21 07:46:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 接续 TUI/plain 显示验收实现

行动目的:
- 接续上一轮已开始但未完成的 `parallel_runner.rs` plain/no-tui 控制面事件摘要改动。
- 先检查 worktree 和当前代码,再补 focused tests,避免把 TUI 已有能力重复实现。

当前判断:
- TUI 侧已有 footer/instances/output status strip 测试覆盖较多。
- 当前缺口主要是 plain/no-tui 终端显示层: topology/capability 控制面事件应该给出一行用户可读摘要。

下一步:
- 检查 `git status` 与 `crates/ralph-cli/src/parallel_runner.rs`。
- 把 helper 接入 event observer,并补 guardrail tests。

状态:
- **阶段2进入阶段3** - 明确最小缺口后,开始用 focused tests 锁住 plain/no-tui 显示契约。

## [2026-05-21 07:55:00] [Session ID: omx-1779158263949-kticiv] 阶段更新: plain/no-tui 控制面摘要 focused tests 通过

已完成:
- 在 `parallel_runner.rs` 补 `parallel_cli_event_summary` 相关 focused tests。
- 覆盖 `topology.spawn.result`、`capability.result`、`capability.failed`、非相关 topic 忽略、quiet 不输出。
- 接入 no-tui event observer,让 plain 模式能输出低频控制面摘要,同时保留 record-session 写入。

验证:
- `cargo fmt --all`: passed。
- `cargo test -p ralph-cli parallel_runner::guardrail_tests -- --nocapture`: passed,9 tests passed。

状态:
- **阶段3和阶段4已完成** - plain/no-tui 最小实现已接入,进入阶段5验证。

## [2026-05-21 07:38:00] [Session ID: omx-1779158263949-kticiv] 最终状态: TUI/plain 显示验收已完成

时间戳更正:
- 本轮前两条行动记录误写为未来时间 `07:46` / `07:55`。
- 实际收尾时间以本条 `2026-05-21 07:38:00 CST` 为准。

阶段完成情况:
- [x] 阶段1: 读取现有 TUI state/widgets/plain runner 输出与测试。
- [x] 阶段2: 明确缺口和最小验收矩阵。
- [x] 阶段3: 先补 focused tests。
- [x] 阶段4: 如测试暴露缺口,再做最小实现修正。
- [x] 阶段5: 运行 TUI/plain focused tests、smoke_runner、cargo test 和必要的文本验证。

交付内容:
- `crates/ralph-cli/src/parallel_runner.rs`: no-tui/plain 控制面事件摘要和 guardrail tests。
- `specs/unified-evidence-inspect.spec.md`: plain runtime control-plane evidence 和 TUI display guardrails。

遇到错误:
- 初次 TUI focused test 使用短过滤名导致 0 tests,不能作为证据。
- 已用 `cargo test -p ralph-tui -- --list | rg ...` 找到完整路径,并用 `--exact` 重跑真实测试通过。

最终验证:
- `cargo test -p ralph-cli parallel_runner::guardrail_tests -- --nocapture`: passed。
- TUI 5 条 focused display tests: passed。
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `cargo test`: passed。

EPIPHANY 检查:
- 本轮没有新增架构级灾难点。
- 核心规律已经记录到 notes/WORKLOG/LATER_PLANS,不追加 `EPIPHANY_LOG.md`。

## [2026-05-21 07:43:00] [Session ID: omx-1779158263949-kticiv] 验证更新: warning 已处理并通过最终门禁

遇到错误:
- 优化 stdout 锁逻辑后,`maybe_write_parallel_cli_event_summary` 只在测试中使用,导致 `dead_code` warning。
- 根据项目规则,warning 不能忽略,所以不能把上一轮 `cargo test` 当作最终通过。

修正:
- 给测试专用 helper 增加 `#[cfg(test)]`。
- 最终全量测试使用 `RUSTFLAGS="-Dwarnings" cargo test --quiet`,确保 warning 会变成失败。

最终验证补充:
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-cli parallel_runner::guardrail_tests -- --nocapture`: passed。
- TUI 5 条 focused display tests: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

状态:
- **全部阶段完成** - TUI/plain 显示验收可以交付。

## [2026-05-21 07:50:00] [Session ID: omx-1779158263949-kticiv] 行动记录: 开始 parent-visible spawn replay/integration guardrail

行动目的:
- 按用户确认,开始做 parent-visible spawn replay/integration guardrail。
- 目标是把 `topology.spawn_group` 真正创建父级 dynamic instances、写入 `.ralph/agents.json`、且 `topology.spawn.result` 后不重复 publish 原 delivery topic 这条链路锁成测试。

当前约束:
- 不重写 topology runtime,优先补 replay/integration guardrail。
- 只改必要测试/规格/少量 helper,不碰无关在途改动。
- 继续使用 record-session / `.ralph/events.jsonl` / `.ralph/agents.json` 作为证据真相源。

阶段:
- [ ] 阶段1: 读取 topology runtime、agents snapshot、现有 integration/replay tests。
- [ ] 阶段2: 明确测试入口和验收矩阵。
- [ ] 阶段3: 先补失败优先的 focused/integration test。
- [ ] 阶段4: 如测试暴露缺口,做最小实现修正。
- [ ] 阶段5: 跑 focused tests、smoke_runner、deny-warnings 全量测试。

状态:
- **阶段1进行中** - 先做只读定位,避免造一个脱离真实 runtime 的假测试。

## [2026-05-21 07:58:00] [Session ID: omx-1779158263949-kticiv] 错误记录: integration_topology_spawn 首次失败

现象:
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture` 失败。
- 失败断言: `analysis.done` source 顺序期望 `builder#2,#3,#4`,实际为 `builder#3,#4,#2`。

候选假设:
- 主假设: spawned builder jobs 并行完成,`analysis.done` 事件顺序不保证稳定,测试不应依赖完成顺序。
- 备选解释: runtime delivery 顺序异常。当前 build.task direct delivery 和 lifecycle 断言在失败前未报错,更支持主假设。

修正计划:
- 只对 `analysis.done` source 列表排序后比较集合。
- 保持 build.task delivery 顺序断言不变,因为那是 spawn_group direct delivery 的 deterministic runtime evidence。

## [2026-05-21 08:03:00] [Session ID: omx-1779158263949-kticiv] 阶段更新: parent-visible spawn integration guardrail 已补

已完成:
- 新增 `crates/ralph-cli/tests/integration_topology_spawn.rs`。
- 测试用真实 `ralph run --no-tui --record-session` 驱动 `topology.spawn_group`。
- 断言 `.ralph/events.jsonl`、`.ralph/agents.json`、stdout 和 `record summary --agents-file` 证据链。
- 更新 `specs/parent-visible-topology-spawn-observability.spec.md` 的验证建议。

已修正:
- `analysis.done` source 顺序改为排序后比较,避免并发 worker 完成顺序造成测试假失败。

当前验证:
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: passed。

状态:
- **阶段3完成,阶段4无需额外 runtime 修复,进入阶段5验证**。

## [2026-05-21 08:05:00] [Session ID: omx-1779158263949-kticiv] 错误记录: 格式检查失败

现象:
- `cargo fmt --all -- --check` 失败。
- 失败位置在新测试文件 `crates/ralph-cli/tests/integration_topology_spawn.rs` 的换行格式。

结论:
- 这是格式化问题,不是 runtime 行为问题。

修正:
- 执行 `cargo fmt --all` 后重新跑完整验证。

## [2026-05-21 08:10:00] [Session ID: omx-1779158263949-kticiv] 最终状态: parent-visible spawn replay/integration guardrail 已完成

阶段完成情况:
- [x] 阶段1: 读取 topology runtime、agents snapshot、现有 integration/replay tests。
- [x] 阶段2: 明确测试入口和验收矩阵。
- [x] 阶段3: 先补 focused/integration test。
- [x] 阶段4: 如测试暴露缺口,做最小实现修正。
- [x] 阶段5: 跑 focused tests、smoke_runner、deny-warnings 全量测试。

交付内容:
- `crates/ralph-cli/tests/integration_topology_spawn.rs`
- `specs/parent-visible-topology-spawn-observability.spec.md`

关键证据:
- 新 integration test 覆盖真实 binary / custom backend / no-tui stdout / record-session / `.ralph/events.jsonl` / `.ralph/agents.json` / record summary。
- `.ralph/events.jsonl` 断言 `topology.spawn.result` 之后没有后置 `build.task`,防止 ack 被当作 replay 机制。

验证:
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::topology_spawn_group_creates_three_dynamic_instances_and_delivers_direct -- --exact --nocapture`: passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::topology_spawn_group_is_idempotent_by_request_id -- --exact --nocapture`: passed。
- `cargo test -p ralph-core smoke_runner`: passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

EPIPHANY 检查:
- 本轮没有新增架构级灾难点。
- 关键规律已记录到 notes/WORKLOG/LATER_PLANS,不追加 `EPIPHANY_LOG.md`。

## [2026-05-21 18:44:33] [Session ID: omx-1779158263949-kticiv] 计划: ralplan 生成 task-derived dynamic hat identity / role contract 方案

目标:
- 按 `` 共识规划流程,为 task-derived dynamic hat identity / role contract 生成一份可执行方案。
- 本轮只做方案,不改业务代码。

阶段:
- [x] 阶段1: 读取 ralplan/plan skill 和当前六文件上下文。
- [ ] 阶段2: 创建或复用 OMX context snapshot。
- [ ] 阶段3: 形成 Planner 初稿和 RALPLAN-DR summary。
- [ ] 阶段4: 依次完成 Architect 与 Critic 评审。
- [ ] 阶段5: 保存最终 plan,输出方案和后续执行建议。

状态:
- 当前进入阶段2,先固化 repo-grounded context,避免方案只凭会话记忆漂移。

## [2026-05-21 18:46:41] [Session ID: omx-1779158263949-kticiv] 状态: Architect agent 启动方式调整

现象:
- 第一次使用 full-history fork 并同时指定 agent_type/reasoning_effort,工具层拒绝。

处理:
- 不改变方案内容。
- 改用无 fork 的自包含提示启动 Architect,让评审只读取 draft plan 路径。

状态:
- 继续阶段4: Architect -> Critic 顺序评审。

## [2026-05-21 18:48:59] [Session ID: omx-1779158263949-kticiv] 状态: Architect 评审完成,进入 Critic

Architect verdict:
- ITERATE。

关键反馈:
- 需要 canonical effective contract,downstream 只能消费 runtime 归一化后的 contract。
- 显式 role_contract 与旧字段冲突要 fail closed。
- allowed topics 必须拆成 input delivery topic 与 output publish allowlist。
- prompt isolation 要有结构性 API 约束。

状态:
- 现在启动 Critic,评审初稿和 Architect 反馈。

## [2026-05-21 18:51:17] [Session ID: omx-1779158263949-kticiv] 状态: Critic 评审完成,开始修订 plan

Critic verdict:
- ITERATE。

必须修订:
- Option A 改为 raw input + canonical EffectiveRoleContract。
- allowed_topics 只表示 output publish allowlist。
- 冲突策略改成 validate + canonicalize + fail closed。
- agents summary 增加 hash/source id/schema version/persistence。
- 验证命令不能有 placeholder。

状态:
- 回到 Planner 修订步骤,产出 rev2 draft 后重新评审。

## [2026-05-21 18:55:40] [Session ID: omx-1779158263949-kticiv] 状态: Architect rev2 APPROVE,进入 Critic rev2

Architect rev2 verdict:
- APPROVE。

需要吸收到 final plan 的细节:
- objective 冲突策略不能含糊。
- canonical objective 建议永远取 member.task,raw objective 只进入 warning/evidence,不参与 downstream 决策;或直接 fail closed。

状态:
- 现在启动 Critic rev2 作为最终质量门。

## [2026-05-21 18:57:38] [Session ID: omx-1779158263949-kticiv] 状态: Critic rev2 ITERATE,修订 objective 规则

Critic rev2 verdict:
- ITERATE。

阻塞点:
- objective 冲突策略仍有含糊措辞。

采用的修订规则:
- EffectiveRoleContract.objective 永远取 member.task。
- raw role_contract.objective 只进入 warning/evidence。
- raw objective 不参与 downstream prompt / agents snapshot / record summary 的 canonical objective 决策。

状态:
- 生成 rev3,然后重新跑 Architect -> Critic。

## [2026-05-21 18:59:34] [Session ID: omx-1779158263949-kticiv] 状态: Architect rev3 APPROVE,进入 Critic rev3

Architect rev3 verdict:
- APPROVE。

状态:
- 最后启动 Critic rev3。
- 如果 Critic 批准,将 rev3 保存为最终 plan。

## [2026-05-21 19:02:44] [Session ID: omx-1779158263949-kticiv] 状态: Critic rev3 APPROVE,最终 plan 已保存

Critic rev3 verdict:
- APPROVE。

已吸收的非阻塞建议:
- 收紧 Principle 2: 兼容且冲突显式化;权限/身份冲突 fail closed;objective 冲突 canonicalize to member.task 并记录 evidence。
- 补 3 个 focused test 名称: control-plane topic denylist、empty output allowlist intersection、agents snapshot summary-only。

最终计划文件:
- .omx/plans/task-derived-dynamic-hat-identity-role-contract.md

状态:
- 进入收尾:记录 notes/WORKLOG/LATER_PLANS,然后输出方案摘要和下一步建议。

## [2026-05-21 19:03:10] [Session ID: omx-1779158263949-kticiv] 最终状态: ralplan 方案生成完成

阶段完成情况:
- [x] 阶段1: 读取 ralplan/plan skill 和当前六文件上下文。
- [x] 阶段2: 创建或复用 OMX context snapshot。
- [x] 阶段3: 形成 Planner 初稿和 RALPLAN-DR summary。
- [x] 阶段4: 依次完成 Architect 与 Critic 评审。
- [x] 阶段5: 保存最终 plan,输出方案和后续执行建议。

最终计划文件:
- .omx/plans/task-derived-dynamic-hat-identity-role-contract.md

评审结果:
- Architect rev3: APPROVE。
- Critic rev3: APPROVE。

EPIPHANY 检查:
- 本轮没有新增需要单独写入 EPIPHANY_LOG.md 的架构灾难点。
- canonical EffectiveRoleContract 规则已经进入最终 plan、notes、WORKLOG 和 LATER_PLANS。

## [2026-05-21 19:05:07] [Session ID: omx-1779158263949-kticiv] 收尾: stop hook ralplan terminal 状态已修正

现象:
- stop hook 检测到 ralplan 仍为 active/current_phase=planning。
- 实际最终 plan 已存在: .omx/plans/task-derived-dynamic-hat-identity-role-contract.md。
- Architect rev3 与 Critic rev3 均已 APPROVE。

处理:
- 将 hook 指向的 ralplan-state.json 修正为 active=false,status=complete,current_phase=complete。
- 将 consensus-short-state.json 同步为 status=complete/run_outcome=complete。

状态:
- ralplan 已终止在 complete。
- 没有执行代码实现。

## [2026-05-21 19:45:31] [Session ID: omx-1779158263949-kticiv] 计划: Ralph 执行 task-derived dynamic hat identity / role contract

目标:
- 按 .omx/plans/task-derived-dynamic-hat-identity-role-contract.md 完整实现。
- 不做 scope reduction。
- 最终必须有 focused tests、integration_topology_spawn、smoke_runner、deny-warnings 全量测试和 record summary dogfood 证据。

已确认:
- Codex goal-mode 当前没有 active goal。
- 最终 plan 已存在且 Architect/Critic APPROVE。
- 当前 worktree 已有大量既有修改,本轮只触碰 plan 指定文件,不回滚其他改动。

阶段:
- [x] 阶段1: intake,读取 plan/context/git 状态。
- [ ] 阶段2: core contract/runtime 数据模型与 canonicalization。
- [ ] 阶段3: worker prompt、agents snapshot、record summary、display 接线。
- [ ] 阶段4: focused/integration tests。
- [ ] 阶段5: full verification、dogfood、architect/deslop/收尾。

状态:
- 进入阶段2,先做代码阅读和最小实现切入点确认。

## [2026-05-21 19:59:20] [Session ID: omx-1779158263949-kticiv] 状态: 接手 compaction 后继续 Ralph 实现

现象:
- 已有 handoff summary 和两个 explorer 只读结果。
- 当前代码已经部分实现 EffectiveRoleContract、topology role_contract raw hint、runtime canonicalization、agents snapshot summary 和 worker prompt 注入。
- 尚未完成编译验证,也尚未补齐 display/TUI/record summary 与测试门禁。

下一步行动:
- 先读取最终 plan 和当前 diff。
- 运行 focused compile/test,暴露真实缺口。
- 按编译错误和验收清单补齐接线与测试。

阶段状态:
- [x] 阶段1: intake,读取 plan/context/git 状态。
- [~] 阶段2: core contract/runtime 数据模型与 canonicalization 已有半成品,需要编译验证。
- [ ] 阶段3: worker prompt、agents snapshot、record summary、display/TUI 接线。
- [ ] 阶段4: focused/integration tests。
- [ ] 阶段5: full verification、dogfood、architect/deslop/收尾。

## [2026-05-21 20:00:40] [Session ID: omx-1779158263949-kticiv] 验证: ralph-core 编译通过,转入 CLI/TUI 接线检查

已完成:
- `cargo check -p ralph-core` 通过。

结论:
- 当前 core 层新增类型、runtime canonicalization、agents snapshot 字段至少没有基础编译错误。

下一步行动:
- 运行 `cargo check -p ralph-cli` 和 `cargo check -p ralph-tui`。
- 若失败,优先修复 display/summary/TUI 字段接线。

## [2026-05-21 20:03:10] [Session ID: omx-1779158263949-kticiv] 验证: CLI 编译通过,进入验收缺口补齐

已完成:
- `cargo check -p ralph-cli` 通过,同时编译了 `ralph-tui` 依赖。

结论:
- 当前缺口集中在验收覆盖和 evidence/display 完整性,不是基础编译失败。

下一步行动:
- 只读检查 role_contract_summary 在 core/CLI/TUI 的全部引用。
- 补最小必要测试,优先锁住 summary-only、prompt 注入、record summary、plain display、TUI badge。

## [2026-05-21 20:05:40] [Session ID: omx-1779158263949-kticiv] 发现: 核心实现已有,缺规格同步和显示/测试闭环

只读检查结论:
- `EffectiveRoleContract` / `RoleContractSummary` / runtime canonicalization / worker prompt 注入已存在。
- `record summary` 和 no-tui plain summary 已能展示部分 contract summary。
- `ralph agents` table 已有 `Role Contract` 列,但还缺 source request / schema 等摘要信息。
- TUI 仍只保存 `spawn_role_labels`,没有保存 `RoleContractSummary`,因此看不到 task-derived / temporary|fixed / hash 短码。
- `specs/ralph-prompt-role-layering.md` 与 `specs/parent-visible-topology-spawn-observability.spec.md` 尚未同步 EffectiveRoleContract 的 canonical authority 规则。

下一步行动:
- 跑现有 topology focused test 与 CLI integration test。
- 然后补规格、TUI summary map、display 细节和 regression tests。

## [2026-05-21 20:18:30] [Session ID: omx-1779158263949-kticiv] 状态: 第一轮修复完成,开始 focused 验证

已修改:
- runtime output allowlist 显式剔除 `delivery_topic`。
- core topology focused tests 增加 target hat publishes,并补 role contract positive/negative guardrails。
- record summary / plain summary / ralph agents / TUI 实例栏补 role contract summary 展示。
- integration topology dogfood backend 改为带 raw role_contract,并断言 worker prompt 收到 canonical ROLE CONTRACT。
- specs 同步 EffectiveRoleContract canonical authority 规则。

下一步行动:
- `cargo fmt --all`。
- 重跑 core topology role contract focused tests。
- 重跑 CLI integration_topology_spawn、record summary、agents、parallel runner、TUI instances focused tests。

## [2026-05-21 20:20:10] [Session ID: omx-1779158263949-kticiv] 验证: core focused role contract tests 通过

已通过:
- topology spawn 旧 payload 创建三动态实例。
- conflicting role contract fail closed。
- raw objective mismatch canonicalize 到 member.task。
- non-task-derived identity_source fail closed。
- control-plane output topic fail closed。
- empty output allowlist intersection fail closed。
- delivery_topic 从 output allowlist 剔除。
- agents snapshot summary-only。
- dynamic worker prompt 包含 ROLE CONTRACT 且不继承 coordinator-only surface。

下一步行动:
- 运行 CLI/TUI focused tests 与 integration_topology_spawn。

## [2026-05-21 20:26:10] [Session ID: omx-1779158263949-kticiv] 验证: CLI/TUI focused 缺口已修复

已通过:
- record summary evidence inspect focused test。
- plain topology.spawn.result summary focused test。
- ralph agents role contract summary integration test。
- integration_topology_spawn。
- TUI instances role contract badge test。
- EffectiveRoleContract summary/hash/worker-section 单测。

处理过的错误:
- ralph agents 测试最初把 role contract hash 当作通用 truncate 输出,实际需要固定短 hash。已把显示改为 12 字符 hash 短码并复验。

下一步行动:
- 跑受影响 test suite,再跑 fmt/check/smoke/full deny-warnings。

## [2026-05-21 20:28:20] [Session ID: omx-1779158263949-kticiv] 验证: 受影响套件通过,进入全局门禁

已通过:
- topology spawn parser role contract tests。
- topology spawn group 9 个 focused routing tests。
- integration_agents 全套 7 个 tests。
- integration_topology_spawn。

下一步行动:
- `cargo fmt --all -- --check`。
- `git diff --check`。
- `cargo test -p ralph-core smoke_runner`。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`。

## [2026-05-21 20:31:40] [Session ID: omx-1779158263949-kticiv] 验证: 全局门禁通过,进入 live dogfood

已通过:
- `cargo fmt --all -- --check`。
- `git diff --check`。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: 全量通过,无 warning。

下一步行动:
- 运行 task-derived role contract live dogfood record-session。
- 使用 `ralph record summary --agents-file .ralph/agents.json` 检查 topology / agents / result / termination 证据。

## [2026-05-21 20:30:27] [Session ID: omx-1779158263949-kticiv] 状态恢复: 接管 task-derived role contract Ralph 执行

已恢复上下文:
- 用户触发 `$ralph .omx/plans/task-derived-dynamic-hat-identity-role-contract.md`,目标是完整落地 task-derived dynamic hat identity / role contract。
- 前一轮已实现 core/runtime/CLI/TUI 大部分链路,并通过 focused tests、smoke_runner、deny-warnings 全量测试。
- 当前未完成重点是:
  1. 等待并分析 live dogfood record-session。
  2. 修复 live dogfood 暴露的 coordinator prompt schema guidance 问题: `role_contract` 被 LLM 错放到 `input` object。
  3. 补回归测试证明 prompt/event protocol 示例明确 `role_contract` 是 `instances[]` 成员字段,且 `input` 必须是字符串。
  4. 重跑 focused/global gates。
  5. 做 Ralph 要求的 architect/deslop 收尾和六文件记录。

下一步行动:
- 接管正在运行的 dogfood session,确认 termination/topology/agents/result evidence。
- 然后只修 prompt guidance,不扩大 scope。

状态:
** 目前在阶段5 ** - 继续 live dogfood evidence 和 prompt schema guardrail 收尾。

## [2026-05-21 20:38:00] [Session ID: omx-1779158263949-kticiv] 验证: live dogfood 已结束但未自然收敛

已完成:
- 接管并等待 live dogfood 完成。
- record-session: `/tmp/ralph-task-derived-role-contract-dogfood-20260521-202623.jsonl`。
- 运行状态: `RUN_STATUS=124`,record termination reason 为 `Interrupted`,elapsed_secs 约 `419.970`。

已验证事实:
- 首轮 `topology.spawn_group` 因 `input` object 失败: `instances[0]: field input must be a string when present`。
- retry 后 `topology.spawn.result` 成功创建 3 个 parent-visible dynamic builder instances: `builder#2/#3/#4`。
- `.ralph/agents.json`/Evidence Inspect 能显示 `identity_source=task-derived`, `persistence=fixed`, `contract_schema_version=1`, `role_contract_hash`, `source_spawn_request_id`。
- `builder#4` 发布了 1 条可解析 `build.done`。
- `builder#2` 的事件出现在 stderr/终端输出中,未进入 record summary 的 result topics;`builder#3` 最终未形成可解析 result。

结论边界:
- 可以确认 task-derived parent-visible spawn 与 role contract display/evidence 主链路已经工作。
- 不能宣称 live 3-worker dogfood 自然完成,因为 termination 是 Interrupted 且 result topics 只有 1 条 `build.done`。

下一步行动:
- 修复 coordinator prompt/event protocol schema guidance,避免再次把 `role_contract` 错放进 `input` object。
- 另外记录后续项: worker artifact 写入、stderr event 不被 parse、live dogfood 预算/收敛问题。

状态:
** 目前在阶段5 ** - 根据 live dogfood 动态证据修 prompt guidance 并补回归测试。

## [2026-05-21 20:40:30] [Session ID: omx-1779158263949-kticiv] 修复: topology.spawn_group role_contract schema guidance

已修改:
- `crates/ralph-core/src/event_emission_protocol.rs`
  - 将 `role_contract` 加入 `instances[]` optional sibling fields。
  - 明确 `input` 存在时必须是 string。
  - 明确禁止把 `role_contract` 放进 `input`。
  - 增加带 `role_contract` sibling 的 JSON 示例。
- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - coordinator prompt 测试新增断言,锁住 role_contract sibling guidance。
- `specs/parent-visible-topology-spawn-observability.spec.md`
  - 补充 `input` string 与 `role_contract` sibling field 规则。

下一步行动:
- 运行 `cargo fmt --all`。
- 运行 event protocol unit test 和 coordinator prompt focused test。
- 然后进入受影响 integration/global gates。

## [2026-05-21 20:42:30] [Session ID: omx-1779158263949-kticiv] 验证: prompt schema guidance focused tests 通过

已通过:
- `cargo test -p ralph-core event_emission_protocol::tests::topology_spawn_prompt_documents_parent_visible_group_spawn_contract -- --exact --nocapture`
- `cargo test -p ralph-core parallel::supervisor::routing_tests::runtime_capability_catalog_is_injected_only_into_ralph_prompt -- --exact --nocapture`

结论:
- 静态协议文案已锁住 `role_contract` sibling field、`input` string、禁止把 role_contract 放进 input。
- coordinator prompt 已包含该 guidance。

下一步行动:
- 跑 topology spawn focused routing tests。
- 跑 CLI integration_topology_spawn。
- 跑 fmt/check/smoke/full deny-warnings。

## [2026-05-21 20:45:00] [Session ID: omx-1779158263949-kticiv] 验证: topology/CLI 受影响测试通过

已通过:
- `cargo test -p ralph-core parallel::supervisor::routing_tests::topology_spawn_group -- --nocapture`：9 passed。
- `cargo test -p ralph-core parallel::supervisor::routing_tests::dynamic_worker_prompt_contains_effective_role_contract -- --exact --nocapture`：1 passed。
- `cargo test -p ralph-cli --test integration_topology_spawn -- --nocapture`：1 passed。

下一步行动:
- `cargo fmt --all -- --check`。
- `git diff --check`。
- `cargo test -p ralph-core smoke_runner`。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`。

## [2026-05-21 20:47:30] [Session ID: omx-1779158263949-kticiv] 验证: 全局门禁通过

已通过:
- `cargo fmt --all -- --check`。
- `git diff --check`。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: 全量通过,无 warning。

下一步行动:
- 执行 Ralph 收尾要求: architect-style verification。
- 执行 ai-slop-cleaner changed-files-only pass 或等价手工 deslop,并复跑必要门禁。

## [2026-05-21 20:51:20] [Session ID: omx-1779158263949-kticiv] 错误: post-deslop deny-warnings 全量测试失败

现象:
- `RUSTFLAGS="-Dwarnings" cargo test --quiet` 失败。
- 失败测试: `crates/ralph-core/tests/event_loop_ralph.rs::test_reads_actual_events_jsonl_with_object_payloads`。
- 失败信息: `events.jsonl should have records`。

当前假设:
- 主假设: 测试读取了某个实际 `.ralph/events.jsonl` 或 fixture,live dogfood/测试隔离导致该文件为空或路径改变。
- 备选解释: deslop 改动不相关,但本轮 live dogfood 或其它并发测试写入了运行态文件,暴露了 test fixture 对工作目录状态的依赖。

下一步行动:
- 读取该测试和被读取的 events.jsonl 路径。
- 单独复跑失败测试,确认是否稳定复现。
- 根据证据修复或清理运行态数据,不能直接忽略。

## [2026-05-21 20:55:20] [Session ID: omx-1779158263949-kticiv] 修复: event_loop_ralph 环境依赖测试

已修复:
- `test_reads_actual_events_jsonl_with_object_payloads` 不再读取工作区 `.ralph/events.jsonl`。
- 改为使用临时 fixture,同时断言 object payload 和 string payload。
- 同步修正测试输出文案,避免仍声称读取 `.ralph/events.jsonl`。

已通过:
- `cargo test -p ralph-core --test event_loop_ralph test_reads_actual_events_jsonl_with_object_payloads -- --exact --nocapture`。

下一步行动:
- 复跑 post-fix 全局门禁。

## [2026-05-21 21:02:00] [Session ID: omx-1779158263949-kticiv] 完成: task-derived dynamic hat identity / role contract

最终状态:
- [x] 阶段1: 计划和上下文恢复。
- [x] 阶段2: 研究/收集 live dogfood 和静态 prompt 证据。
- [x] 阶段3: 执行/构建 EffectiveRoleContract、spawn payload、runtime canonicalization、display/evidence 接线。
- [x] 阶段4: focused/integration/global gates。
- [x] 阶段5: live dogfood、architect-style 审计、deslop、post-deslop regression 和六文件收尾。

最终验证:
- `cargo fmt --all -- --check`: passed。
- `git diff --check`: passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `RUSTFLAGS="-Dwarnings" cargo test --quiet`: passed。

重要边界:
- live dogfood 证明了 parent-visible spawn 和 role contract evidence,但没有自然收敛,不能当作 3-worker live collaboration completion 证据。
- 外部 architect reviewer 超时未返回,已关闭。采用本地 architect-style 审计 + 全量门禁作为收尾证据。
- `get_goal` 返回 no active goal,因此没有调用 `update_goal`。
- Ralph state 已通过 `omx state write` 标记 complete。

## [2026-05-22 11:43:13] [Session ID: omx-1779158263949-kticiv] 计划: clean live dogfood 配置验证 3-worker 自然收敛

目标:
- 给 task-derived role contract 的 live dogfood 单独制作 clean config,不改长期 ralph.yml。
- 关闭 confessor 干扰,让 builder 目标 hat 明确 publishes analysis.done。
- 通过真实 ralph run --record-session 和 record summary --agents-file 验证 topology.spawn_group、agents snapshot、analysis.done result topics、reply.human.message 和 termination。

阶段:
- [x] 阶段1: 恢复历史上下文与本轮目标。
- [ ] 阶段2: 基于 ralph.yml 与 integration_topology_spawn 生成临时 clean config。
- [ ] 阶段3: 运行 3-worker live dogfood 并保存 record summary。
- [ ] 阶段4: 分析是否自然收敛,区分现象、假设、验证证据、结论。
- [ ] 阶段5: 更新 notes/WORKLOG/必要后续计划并交付。

当前状态:
- 正在进入阶段2,先读取当前主配置和测试用 clean 配置,再生成 /tmp 下临时 dogfood 配置。

## [2026-05-22 11:44:41] [Session ID: omx-1779158263949-kticiv] 进展: clean dogfood 临时配置已生成

已完成:
- 生成 `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 生成 `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- 配置中未包含 `confessor` / `confession_handler`。
- `builder.publishes` 包含 `analysis.done`,并保留 `build.blocked` 作为异常出口。
- coordinator role_args 使用 `-c features.hooks=false`,worker role_args 保持空数组。
- worker instructions 明确禁止通过 shell / `ralph emit` / `echo` 输出 final event,要求最终 assistant message 直接输出 `analysis.done`。

阶段状态:
- [x] 阶段2: 基于 ralph.yml 与 integration_topology_spawn 生成临时 clean config。
- [ ] 阶段3: 运行 3-worker live dogfood 并保存 record summary。

下一步行动:
- 运行 `ralph run --no-tui --record-session`。
- 随后执行 `ralph record summary --agents-file .ralph/agents.json`。

## [2026-05-22 11:45:46] [Session ID: omx-1779158263949-kticiv] 错误: clean config 保留了无发布者的 complete_publishes

现象:
- 首次 `ralph run` 在配置校验阶段失败。
- 错误: `Invalid value for 'event_loop.complete_publishes': topic workflow.complete must be declared in at least one hat's publishes`。
- 因为 runtime 未启动,record-session 文件没有创建,后续 summary 也无法打开文件。

当前假设:
- 主假设: clean config 移除了 `confession_handler`,但仍保留 `complete_publishes: workflow.complete`,导致配置校验正确拒绝。
- 备选解释: 需要显式配置一个 coordinator hat 发布 `workflow.complete`,但当前任务只需要 completion promise,不需要 workflow.complete 事件。

修正动作:
- 从临时 clean config 移除 `complete_publishes`。
- 保留 `completion_promise: LOOP_COMPLETE` 作为本次 live dogfood 的自然收敛条件。

下一步行动:
- 更新 /tmp clean config。
- 重新运行 3-worker live dogfood。

## [2026-05-22 12:07:32] [Session ID: omx-1779158263949-kticiv] 分析: 核验 agents snapshot 缺少 builder#4

现象:
- 第二次 clean dogfood `RUN_STATUS=0`。
- `record summary` 显示 `Termination.reason=CompletionPromise`、`analysis.done: 3`、`source_instances=builder#2,builder#3,builder#4`。
- 同一个 summary 的 Agents Snapshot 只列出 builder#1、builder#2、builder#3、ralph#1,缺少 builder#4。

当前假设:
- 主假设: clean config 设置 `dynamic_idle_ttl_secs=20`,builder#4 最先完成并在最终 sidecar 写入前被动态 idle 回收,所以 agents sidecar 是最后快照,不是完整历史拓扑。
- 备选解释: `build_agents_snapshot` 或 `write_agents_snapshot_best_effort` 对 done/idle dynamic instance 有漏写 bug。

验证计划:
- 读取 dynamic idle 回收和 agents snapshot 代码。
- 从 record-session 重建 topology.spawn/result/analysis.done/termination 时间线。
- 判断缺失 builder#4 是回收语义还是 snapshot bug。

## [2026-05-22 12:09:52] [Session ID: omx-1779158263949-kticiv] 完成: clean live dogfood 自然收敛验证

最终状态:
- [x] 阶段1: 恢复历史上下文与本轮目标。
- [x] 阶段2: 基于 ralph.yml 与 integration_topology_spawn 生成临时 clean config。
- [x] 阶段3: 运行 3-worker live dogfood 并保存 record summary。
- [x] 阶段4: 分析是否自然收敛,区分现象、假设、验证证据、结论。
- [x] 阶段5: 更新 notes/WORKLOG/必要后续计划并交付。

最终验证:
- 临时 config: `/tmp/ralph-clean-task-derived-dogfood-20260522.yml`。
- 临时 prompt: `/tmp/ralph-clean-task-derived-dogfood-20260522.prompt.md`。
- record-session: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.jsonl`。
- summary: `/tmp/ralph-clean-task-derived-role-contract-dogfood-20260522-114604.summary.txt`。
- `RUN_STATUS=0`。
- `Termination.reason=CompletionPromise`。
- `analysis.done: 3 source_instances=builder#2,builder#3,builder#4`。
- `topology.spawn_group: 1`, `topology.spawn.result: 1`, `topology.spawn.failed: 0`。
- `capability.request/result/failed: 0`。

结论:
- clean config 方案可用。
- 上次 live dogfood 不自然收敛的问题,本轮通过关闭 confessor、对齐 builder publishes 到 `analysis.done`、要求 final event 走 stdout 后得到验证性解决。
- 新发现的剩余问题是 `.ralph/agents.json` 作为 current registry snapshot 会丢掉已被 TTL 回收的动态实例,已记录到 LATER_PLANS。
