## 2026-02-18 21:01 +0800 | WORKLOG 续档

- 旧文件因超过 1000 行已续档为 `WORKLOG_2026-02-18_210146.md`。
- 新日志从本文件继续追加。

## 2026-02-18 21:05 +0800 | feat: app_server 三种消息投递模式 + `ralph agents`

- 变更点(消息投递三模式):
  - new_instance: 新增 event 路由提示字段 `spawn_instance`。
  - turn: 保持默认语义(排队新 turn)。
  - steer: 复用既有 `turn_action="steer"`(不满足条件自动降级为排队)。

- 变更点(可观测性与易用性):
  - 并行 Supervisor 运行态持续写入 `.ralph/agents.json` 快照(实例列表 + state + 最后一次输入摘要)。
  - 新增 CLI 子命令 `ralph agents` 读取并展示该快照。

- 文档:
  - `config/all_hat.md` 补齐三种模式的选择建议与写法示例。

- 验证:
  - `cargo fmt` ✅
  - `cargo test -p ralph-core` ✅
- `cargo test -p ralph-cli` ✅

## 2026-02-27 11:32 +0800 | docs/spec: 并行事件发布通道(in-band `<event>` vs out-of-band `ralph emit`)

- 新增规格:
  - `specs/parallel-event-channels.spec.md`
  - 把你问的关键问题写死:
    - `<event ...>...</event>` 是否必须等一轮 turn/job 完结才能被系统识别与路由?
    - turn/job 期间能否多次 tool/MCP 交互? 这些交互是否会让 `<event>` 提前路由?
    - turn/job in-flight 期间如何多次发消息/steer/interrupt?

- prompt 补强(避免误解为"只能发 `<event>`"):
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - 明确: out-of-band 的 `ralph emit` 不需要等待当前 job/turn 完结,可随时执行(包含 in-flight steer/interrupt)。

- 验证:
  - `cargo fmt --check` ✅
  - `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test -p ralph-e2e` ✅
  - `cargo test` ✅

## 2026-02-18 21:25 +0800 | fix: `ralph agents` 支持子目录自动定位

- 变更点:
  - `ralph agents` 未指定 `--file` 时,会从 cwd 向上遍历父目录,选择最近的 `.ralph/agents.json`。
  - 用户显式传 `--file` 时保持原语义(不做 auto-detect)。

- 测试:
  - `crates/ralph-cli/tests/integration_agents.rs` 新增: 在 `a/b/c` 子目录执行也能读取父目录快照。

- 验证:
  - `cargo fmt` ✅
  - `cargo test -p ralph-cli --test integration_agents` ✅

## 2026-02-18 22:19 +0800 | verify: 继续回归验证通过

- 本轮动作:
  - 追加 task_plan 记录,按“四文件上下文模式”继续.
  - 重新跑格式化与全量测试,确保实现与文档没有漂移.

- 验证结果:
  - `cargo fmt --check` ✅
  - `cargo test -p ralph-cli --test integration_agents` ✅
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test` ✅

## 2026-02-18 22:28 +0800 | feat: `ralph agents --watch` 定时刷新

- 目标:
  - 让 `ralph agents` 支持 `--watch` 定时刷新,用于实时观察并行实例状态变化.

- 实现:
  - `ralph agents --watch`:
    - stdout 是 TTY: 清屏并原地刷新.
    - stdout 非 TTY: 追加分隔符输出,不写清屏控制序列,便于日志/CI.
  - `--watch-interval-ms` 可配置刷新间隔(默认 1000ms).
  - watch 模式下,未显式 `--file` 时会持续向上遍历父目录探测,快照生成后可自动发现.

- 文档:
  - `config/all_hat.md` 增加 `ralph agents --watch` 示例.

- 测试:
  - `crates/ralph-cli/tests/integration_agents.rs` 增加 watch 回归:
    - 用 spawn + kill 的方式验证 watch 至少输出一次表格.

- 验证:
  - `cargo fmt` ✅
  - `cargo test -p ralph-cli --test integration_agents` ✅
  - `cargo test -p ralph-cli` ✅
  - `cargo test` ✅

## 2026-02-19 11:30 +0800 | docs: all_hat 补齐 direct emit 决策清单 + 并行度解释

- 背景:
  - 你问“直接 emit 业务 task topic 有什么不好”以及“emit experiment.task 会不会并行起很多 CLI 实例”.

- 变更:
  - `config/all_hat.md` 增加一套可执行的决策清单:
    - 默认走 `human.message -> ralph#1`(让 coordinator 控窗口/backpressure).
    - direct emit 仅在你明确知道订阅者与payload字段,且愿意自己节流时使用.
  - `config/all_hat.md` 增加并行度解释:
    - 解释 instance/job/CLI 进程三者区别.
    - 明确并发由 `parallel.autoscale.max_running_jobs` 与 `hats.<hat>.instances` 双重约束.
    - 给出“串行化 direct emit”的示例: `--target-instance`.

- 验证:
  - 文档变更,不影响代码与测试.

## 2026-02-19 13:29 +0800 | chore: continuous-learning 收尾(归档历史版本 + 提取 skills)

- 动机:
  - 根目录残留的四文件历史版本会增加检索噪音,也容易让人误读“当前状态”。
  - 本轮出现 2 个不那么直观、但高复用的坑点,适合固化成 `self-learning.*` skill,避免只留在 ERRORFIX/notes。

- 归档(降低根目录噪音):
  - `notes_2026-02-11_204839.md` -> `archive/notes_2026-02-11_204839.md`
  - `task_plan_2026-02-18_132109.md` -> `archive/task_plan_2026-02-18_132109.md`
  - `WORKLOG_2026-02-18_210146.md` -> `archive/WORKLOG_2026-02-18_210146.md`

- 路径更正(只追加,不改旧段落):
  - 旧段落里提到的 `WORKLOG_2026-02-18_210146.md` 现已移动到 `archive/WORKLOG_2026-02-18_210146.md`。

- 新增 skills(项目级):
  - `.codex/skills/self-learning.ralph-autopilot-agent-analysis-custom-backend/SKILL.md`
  - `.codex/skills/self-learning.ralph-parallel-secondary-coordinator-prompt-drift/SKILL.md`

- 过程性产物:
  - `notes.md` 已追加本轮四文件摘要与归档路径更正。

## 2026-02-19 16:16 +0800 | verify: $parallel-engine-autopilot 复跑(PASS)

- 目的:
  - 在“包含当前工作区未提交改进”的工作树快照上,复跑一次 autopilot,观察闭环与并发证据是否仍稳定.

- 运行:
  - `bash scripts/run_autopilot.sh --repo-dir /private/tmp/ralph-autopilot-repo-20260219-155244 --config examples/parallel-experimental-dev-engine/ralph.yml --out-dir /tmp/ralph-autopilot-out-parallel-20260219-155244 --child-parallel-max-running-jobs 2 --skip-agent-analysis`
  - `python3 scripts/summarize_report.py /tmp/ralph-autopilot-out-parallel-20260219-155244`

- 结果(硬断言):
  - exit_code: `0`(Pass)
  - termination_reason: `CompletionPromise`
  - required topics 全部出现(包含 `integration.applied`)
  - banned topics 未命中(包含 `routing.escalate`/`gate.*`)

- 并行度(从 report.md 的 stdout 状态机推断,runner 维度):
  - max_concurrent_running: `2`
  - runner_instances_entered_running: `3`(`experiment_runner#1`/`experiment_runner#3`/`experiment_runner#5`)

- 证据入口:
  - `/tmp/ralph-autopilot-out-parallel-20260219-155244/report.json`
  - `/tmp/ralph-autopilot-out-parallel-20260219-155244/report.md`
  - `/tmp/ralph-autopilot-out-parallel-20260219-155244/session.jsonl`
  - `/tmp/ralph-autopilot-out-parallel-20260219-155244/stdout.txt`

- 清理:
  - 已删除临时 repo: `/private/tmp/ralph-autopilot-repo-20260219-155244`

## 2026-02-19 16:26 +0800 | improve: autopilot 临时 repo 隔离方案升级(保留 git 历史)

- 动机:
  - 之前用 rsync + `git init` 的“单提交快照临时 repo”会丢失真实 git 历史.
  - 这会让 integration.task 里常见的 `commit_lookup_cmd`(例如 `git rev-list --all --grep ...`)在临时 repo 中必然 miss.
  - 进而导致 integrator 只能走 fallback(例如 file follow/blame),增加漂移与耗时.

- 改良点(脚本层,不侵入 Rust autopilot 协议):
  - `scripts/run_autopilot.sh` 新增 isolate 模式:
    - `--isolate`: `git clone --local` 保留历史 + `rsync --delete` 覆盖工作树(含 untracked/未提交改动).
    - isolate 默认 out_dir: `/tmp/ralph-autopilot-out-<timestamp>`(避免清理临时 repo 时误删证据).
    - 默认创建 snapshot commit(让 worktree 干净);可用 `--no-snapshot-commit` 关闭.
    - 清理策略: PASS 默认清理临时 repo; FAIL 默认保留;可用 `--keep-temp-repo` 强制保留.

- 文档:
  - `.codex/skills/parallel-engine-autopilot/SKILL.md` 增加 `--isolate` 推荐用法与行为说明.

- 验证:
  - `bash -n scripts/run_autopilot.sh` ✅
  - `bash scripts/run_autopilot.sh --help` ✅

## 2026-02-19 19:40 +0800 | verify: isolate 实跑 PASS + E2E 补测全绿

- autopilot isolate 实跑(硬断言 PASS,并自动清理临时 repo):
  - 命令:
    - `bash scripts/run_autopilot.sh --repo-dir . --config examples/parallel-experimental-dev-engine/ralph.yml --isolate --child-parallel-max-running-jobs 2 --skip-agent-analysis`
  - 结果:
    - out_dir: `/tmp/ralph-autopilot-out-20260219-191459`
    - exit_code: `0`(Pass)
    - report.md 已包含 Topic Counts 与并行度指标(便于审计).
    - max_concurrent_running: `2`(runner 维度)

- E2E: agents snapshot 能力未遗漏(编译 + mock-mode 回放均通过):
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -q -p ralph-e2e -- codex --mock --filter parallel-starting-event-inference` ✅

- 全量验证(避免“只补 E2E 但其他 crate 回归”):
  - `cargo fmt --check` ✅
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test -p ralph-cli` ✅
  - `cargo test` ✅

- 备注(观测到的漂移风险,未在本轮强制修复):
  - 个别 run 中 `integration.task.commit` 可能出现占位值(非 git hash),会导致 integrator 需要额外“按提交信息 grep 定位”才能继续.
  - 这类回退路径会依赖 git 历史是否完整,因此 isolate 模式“保留历史”的价值被放大.

## 2026-02-20 14:55 +0800 | sync: 复制 parallel-experimental-dev-engine example 到独立 repo

- 源目录:
  - `examples/parallel-experimental-dev-engine/`
- 目标目录:
  - `/Users/cuiluming/local_doc/l_dev/my/rust/parallel-experimental-dev-engine/examples/parallel-experimental-dev-engine/`
- 同步(仅影响目标目录本身,不会删除/覆盖目标仓库其他文件):
  - `rsync -a --delete --exclude '.DS_Store' examples/parallel-experimental-dev-engine/ <dest>/`
- 校验:
  - `diff -q` 显示 `PROMPT.md`/`ralph.yml`/`README.md` 与源文件一致.
  - `prompt_file` 仍指向 `examples/parallel-experimental-dev-engine/PROMPT.md`.
- 备注:
  - 未覆盖目标仓库根目录的 `PROMPT.md`/`ralph.yml`(它们可能仍是旧版本).如果你希望"根目录也直接等价最新 example",我可以再补一个整理动作(覆盖/软链/README 提示三选一).

## 2026-02-23 18:27 +0800 | test(e2e): 新增 `ralph emit --spawn-instance` 动态实例闭环 + 人类可读日志

- 新增 Tier 8 场景:
  - `crates/ralph-e2e/src/scenarios/parallel/emit_spawn_instance.rs`
  - 场景目标:
    - `ralph#1` 在运行中用 `ralph emit ... --spawn-instance` 创建动态 `worker#2`
    - `worker#2` 发 `spawn.done` 回执到 `ralph#1`
    - `ralph#1` 输出 `LOOP_COMPLETE` 收敛
    - 同时生成 `.e2e/human-log.md` 便于人类审计

- mock-mode 支撑改良:
  - `crates/ralph-e2e/src/mock_cli.rs`
    - 支持从 terminal writes 提取 `[E2E_CMD] ...` 命令,并按 allowlist 执行
    - 执行 `ralph ...` 时优先解析到本地构建二进制(避免 PATH 依赖)
  - `crates/ralph-e2e/src/mock.rs`
    - 默认 allowlist 增加 `ralph emit`
  - `crates/ralph-e2e/src/runner.rs`
    - cleanup 前复制 `${workspace}/.e2e/*` 到 `.e2e-tests/artifacts/<scenario-id>/`，不依赖 `--keep-workspace`
  - `crates/ralph-e2e/src/main.rs`
    - 修复: `RALPH_MOCK_ALLOW` 为空字符串时不再覆盖 CLI allowlist(避免“allowlist 变空”导致命令不执行)

- cassette:
  - `cassettes/e2e/parallel-emit-spawn-instance-codex.jsonl`
  - `cassettes/e2e/README.md` 已同步条目

- 人类可读证据(自动复制,不依赖 keep workspace):
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/human-log.md`
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/stdout.txt`

- 验证:
  - `cargo fmt` ✅
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- --mock --filter parallel-emit-spawn-instance` ✅
  - `cargo test` ✅

## 2026-02-23 21:12 +0800 | test(e2e): Codex App Server turn/steer 多轮注入(确定性) + 人类可读日志

- 新增 Tier 8 场景(并行 runtime):
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn.rs`
  - 场景目标:
    - `ralph#1` 使用 `session_strategy=app_server` 启动 turn
    - turn in-flight 期间,外部并发执行 2 次 `ralph emit --turn-action steer --target-instance ralph#1`
    - fake app-server 在收到两次 steer 前不会 completed,因此 steer 若未走 in-flight 通道会稳定卡死(强回归信号)
    - 最终输出 `LOOP_COMPLETE` 收敛

- 实现策略(稳定性优先,不依赖真实网络/真实模型):
  - 在 E2E workspace 内生成 fake `codex` shim:
    - `${workspace}/.e2e/bin/codex`
    - 通过 PATH 注入到 `ralph run` 子进程,覆盖 `codex app-server --listen stdio://` 路径
  - fake app-server 用最小 JSON-RPC 协议:
    - `turn/start` 后延迟发送 `turn/started`(覆盖 pending_steers flush 分支)
    - 收到 2 次 `turn/steer` 后才发送 `turn/completed`,并流式输出 echo(marker) + `LOOP_COMPLETE`

- 发现并修复的并行路由 bug(避免 steer/interrupt 语义被破坏):
  - 根因: busy `ralph#1` 时,路由层会把显式 `target_instance=ralph#1` 改投到 `ralph#2`
  - 但 `turn/steer` 与 `turn/interrupt` 属于 in-flight 控制信号,必须直达目标实例,否则无法影响正在运行的 turn
  - 修复位置: `crates/ralph-core/src/parallel/supervisor/routing.rs`
  - 回归单测: `crates/ralph-core/src/parallel/supervisor/routing_tests.rs` 新增 2 条用例锁死行为

- 人类可读证据(自动复制,不依赖 keep workspace):
  - `.e2e-tests/artifacts/parallel-app-server-steer-multi-turn/human-log.md`
  - `.e2e-tests/artifacts/parallel-app-server-steer-multi-turn/stdout.txt`

- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn` ✅
  - `cargo test -p ralph-core smoke_runner` ✅

## 2026-02-23 21:35 +0800 | improve(e2e): human-log 增补 runner 收发证据(更可读)

- 背景:
  - 你反馈 `.e2e-tests/artifacts/parallel-app-server-steer-multi-turn/human-log.md` 里看不到 runner(这里指 fake codex app-server)“收到 turn/steer 请求并回复”的可读证据。

- 改进:
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn.rs`:
    - fake `codex app-server` 增加 RPC trace 日志:
      - `recv request method=... id=...`
      - `send response id=...`
      - `send notify method=...`
    - human-log.md 增加:
      - `ralph emit` 两次注入的 stdout/stderr 摘录(证明注入命令已被 CLI 接受)
      - artifacts 路径补齐 `emit-1/2.*.txt`

- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn` ✅

## 2026-02-23 23:10 +0800 | test(e2e): 真实 Codex app-server turn/steer 多轮注入闭环 + fake vs real 差异对齐

- 修复/增强 `CodexAppServerRuntime`(真实协议兼容 + 可观测性):
  - `crates/ralph-cli/src/codex_app_server_session.rs`
    - RPC trace 兼容 JSON-RPC error response(之前只记录 result response,导致看不到 steer 失败回执).
    - stderr trace buffer 增大(避免高频 delta 下 trace 行丢失,人类审计不完整).
    - completion 信号兼容: 增加 `codex/event/task_complete|task_completed` 作为 turn 完成判定(部分 real 版本不稳定出现 `turn/completed`).
    - steer 时序修复: 等 `codex/event/task_started` 后再 flush `pending_steers`(否则会出现 "no active turn to steer").

- 新增/完善真实(live) E2E 场景,并用强证据断言 steer 真正闭环:
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn_live.rs`
    - 断言新增:
      - 必须看到 `turn/steer` send>=2.
      - 必须看到 `turn/steer` 成功 response>=2(排除 error_code).
      - 必须看到 `[ralph#1:out:job=...]`(证明真实 agent 输出走 stdout).
    - human-log.md 增强:
      - 增加“精选(握手 + steer 回执)”段落,人类一眼能审计关键收发.

- 验证:
  - `cargo test -p ralph-cli` ✅

## 2026-02-27 12:36 +0800 | fix(parallel): coordinator prompt 双通道 + `ralph emit` 子目录自动定位

### 做了什么

- 并行 coordinator(`ralph#1`) prompt 明确双通道,并允许单轮多事件:
  - in-band: 输出 `<event ...>...</event>`(允许一次输出多条)。
  - out-of-band: 当 backend 支持 tool/shell 时,允许直接执行 `ralph emit ...` 注入事件(不必等 turn/job 结束)。
  - 位置: `crates/ralph-core/src/parallel/supervisor.rs`
- `ralph emit` 与 `ralph events` 在子目录执行时,能自动定位到 active run 的 events 文件:
  - 向上遍历父目录寻找最近的 `.ralph/current-events` marker。
  - 正确解析 marker 的相对路径(以 workspace root 为基准)。
  - 位置: `crates/ralph-cli/src/main.rs`
- all-hat overlay 补充说明:
  - 有命令执行能力时可以用 `ralph emit ...` 注入,否则回退用 `<event ...>...</event>`。
  - 位置: `config/all_hat.md`

### 测试与验证

- `cargo fmt --check` ✅
- `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅
- `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test -p ralph-e2e` ✅
  - `cargo build --release -p ralph-cli --bin ralph` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn-live` ✅

## 2026-02-24 09:56 +0800 | improve(e2e): 消息 payload 增加“具体任务”(121+43=?),并验证请求/执行/反馈

### 1) spawn_instance: task -> worker 执行 -> spawn.done 回执(含 answer)

- 改进:
  - `crates/ralph-e2e/src/scenarios/parallel/emit_spawn_instance.rs`
    - `spawn.task` payload 从仅 marker 升级为: `marker + question: 121+43=?`
    - `worker#N` 的 `spawn.done` 回执增加: `answer: 164`
    - 断言与 human-log 同时强匹配 question/answer,避免“只有 marker 的假阳性”
  - `cassettes/e2e/parallel-emit-spawn-instance-codex.jsonl`
    - 同步更新 mock cassette,保证 `--mock` 回归稳定

- 人类证据入口:
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/human-log.md`
    - 能直接看到 `question: 121+43=?` 与 `answer: 164`

### 2) app-server steer(fake): steer payload 变成任务,runner 计算并反馈 answer

- 改进:
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn.rs`
    - steer payload 增加 question:
      - `121+43=?` -> `answer: 164`
      - `10+5=?` -> `answer: 15`
    - fake codex app-server:
      - 新增 `codex/event/task_started` 通知,对齐真实 app-server 的 steer 门槛语义
      - 解析加法表达式并输出 `TASK_REQUEST/TASK_EXECUTE/TASK_FEEDBACK`(人类一眼能审计)
    - human-log 证据摘录现在包含 answer 行,覆盖“任务反馈”

- 人类证据入口:
  - `.e2e-tests/artifacts/parallel-app-server-steer-multi-turn/human-log.md`

### 3) app-server steer(live): 保持 marker-only,避免真实模型因 question 漂移导致 flake

- 说明:
  - live 场景的目标是验证真实 app-server 的 turn/steer RPC send/recv 与闭环收敛。
  - 将 question 注入 live steer payload 会引入模型行为漂移风险(可能不再按 prompt 输出 LOOP_COMPLETE)。
  - 因此 live 场景保持 marker-only,而“任务执行/反馈(answer)”由 fake 场景提供确定性覆盖。

### 验证

- `cargo test -p ralph-e2e` ✅
- `cargo run -p ralph-e2e -- --mock --filter parallel-emit-spawn-instance` ✅
- `cargo run -p ralph-e2e -- codex --filter "multiple steers"` ✅

## 2026-02-24 06:05 +0800 | improve(e2e): spawn_instance human-log 补强 runner 收发证据 + artifacts 留存 agents/events

- 背景:
  - 你问过“为什么没有激活 worker#1 而是激活 worker#2?”。
  - 同时,旧版 `.e2e/human-log.md` 更偏“结论”,缺少可直接审计的 stdout 片段,不利于排障与复核。

- 改进:
  - `crates/ralph-e2e/src/scenarios/parallel/emit_spawn_instance.rs`:
    - `ParallelEmitSpawnInstanceScenario::write_human_log()` 现在会摘录:
      - supervisor 初始实例列表(解释 `worker#1` 启动时已存在,因此新动态实例编号变成 `worker#2`)。
      - `worker#N` 输出的 `spawn.done` `<event ...>` 片段(证明 runner 收到任务并回执)。
    - 额外把强证据 best-effort 复制到 `.e2e/`:
      - `.ralph/agents.json` -> `.e2e/agents.json`
      - `.ralph/events.jsonl` -> `.e2e/events.jsonl`
    - runner 会把 `.e2e/*` 复制到 `.e2e-tests/artifacts/<scenario-id>/`，因此这些证据能稳定留存,不依赖 `--keep-workspace`。

- 人类可读证据入口:
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/human-log.md`
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/agents.json`
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/events.jsonl`
  - `.e2e-tests/artifacts/parallel-emit-spawn-instance-codex/stdout.txt`

- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- --mock --filter parallel-emit-spawn-instance` ✅

## 2026-02-24 13:30 +0800 | improve(e2e): live app-server steer payload 增加 question(更可审计)

- 变更:
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn_live.rs`:
    - steer payload 从 marker-only 改为 marker+question:
      - `121+43=?`
      - `10+5=?`
    - human-log.md 增加“任务内容(注入 payload)”段落。
    - 断言增强: 要求 question 出现在 RPC trace 的 `input_preview` 里(证明 payload 真进了 `turn/steer`)。
    - prompt 增强: 明确要求忽略 steer 输入,继续固定输出(避免真实模型输出漂移导致 flake)。

- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn-live` ✅ (真实 codex app-server)

- 人类可读证据入口:
  - `.e2e-tests/artifacts/parallel-app-server-steer-multi-turn-live/human-log.md`

## 2026-02-24 14:58 +0800 | improve(e2e): live human-log 增加 hat runner 状态与 stdout 摘录(排障无回复)

- 背景:
  - 你反馈 `parallel-app-server-steer-multi-turn-live` 的 human-log 里“看不到 runner 的输出信息”,只看得到 steer 的 RPC trace。
  - 这会导致无法判断:
    - runner 是否真的在输出.
    - “无回复”到底是模型没输出,还是 human-log 没摘录.

- 改进:
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_multi_turn_live.rs`:
    - human-log 新增:
      - hat runner 状态变化摘录(包含 `[supervisor] instances` 与 `[ralph#1:state] ...`)。
      - hat runner stdout 的 head/tail 摘录(包含 `[ralph#1:out:job=...] ...`)。
    - 断言加固:
      - 要求 `.e2e/human-log.md` 至少包含一条 `[ralph#1:out:job=...]` 行,避免回归到“只有 trace 没有输出”的不可读状态。

- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-multi-turn-live` ✅

- 人类可读证据入口:
  - `.e2e-tests/artifacts/parallel-app-server-steer-multi-turn-live/human-log.md`

## 2026-02-24 20:02 +0800 | improve(e2e): 真实 codex app-server steer 场景补齐“可见回复(answer)”闭环

- 背景:
  - 你希望不仅能看到 `turn/steer` 的 send/recv 回执,还要看到 runner 的“任务请求/执行/反馈(answer)”输出,以定位“无回复”。
  - 仅做 transport 级别的断言,无法证明 steer 输入真的在模型侧被消费并形成可见回复。

- 变更:
  - 新增场景: `crates/ralph-e2e/src/scenarios/parallel/app_server_steer_live_reply_multi_turn.rs`
  - 关键策略: 两轮 turn/iteration 强制闭环(更稳定,不要求 steer 立刻打断当前输出)
    - 第 1 轮(`[task.start]`): 输出 30 行 `STEER_WINDOW_OPEN`,给外部 steer 留“窗口”。
    - 外部注入(同一 in-flight window 内): 2 次 `turn/steer`(含具体任务 payload):
      - `121+43=?` -> `164`
      - `10+5=?` -> `15`
    - 第 2 轮(emit `e2e.reply.step2`): 要求从 thread 历史读取两条输入,输出:
      - `TASK_REQUEST[n]: ...`
      - `TASK_FEEDBACK[n]: answer: ...`
      - 最终 `LOOP_COMPLETE` 收敛。
  - human-log 结构增强:
    - 同时包含 `[app-server-rpc]` trace(含 input_preview)与 hat runner stdout 摘录(含 `answer: 164/15`)。

- 验证:
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- codex --filter parallel-app-server-steer-live-reply-multi-turn` ✅

- 人类可读证据入口:
  - `.e2e-tests/artifacts/parallel-app-server-steer-live-reply-multi-turn/human-log.md`
  - `.e2e-tests/artifacts/parallel-app-server-steer-live-reply-multi-turn/stdout.txt`

## 2026-02-25 19:20 +0800 | feat(e2e): `--idle-start` 待机启动的 fake+live 闭环(含可审计 human-log)

- 新增 E2E 场景:
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_idle_start.rs`
    - id: `parallel-app-server-idle-start`(fake codex shim,0 token)
    - 证据: `.e2e-tests/artifacts/parallel-app-server-idle-start/human-log.md`
  - `crates/ralph-e2e/src/scenarios/parallel/app_server_idle_start_live.rs`
    - id: `parallel-app-server-idle-start-live`(真实 codex app-server)
    - 证据: `.e2e-tests/artifacts/parallel-app-server-idle-start-live/human-log.md`
- 场景注册:
  - `crates/ralph-e2e/src/scenarios/parallel/mod.rs`
  - `crates/ralph-e2e/src/scenarios/mod.rs`
  - `crates/ralph-e2e/src/lib.rs`
  - `crates/ralph-e2e/src/main.rs`

- E2E runner 改良(避免假失败):
  - `crates/ralph-e2e/src/executor.rs`: `resolve_ralph_binary()` 在 release/debug 都存在时按 mtime 选择更新的那个。
    - 修复开发期常见问题: 旧 release 二进制缺少新 CLI flag(例如 `--idle-start`)导致 E2E 误报失败。

- 相关修复:
  - `crates/ralph-cli/src/main.rs`: 默认 `RunArgs` 初始化补齐 `idle_start: false`(避免编译错误)。
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`:
    - turn/interrupt 属于控制信号,不一定启动新 job.
    - 单测改为断言“不会创建/改投到 ralph#2”(避免错误期望)。

- 验证:
  - `cargo test -p ralph-core` ✅
  - `cargo test -p ralph-cli` ✅
  - `cargo test -p ralph-e2e` ✅
  - `cargo run -p ralph-e2e -- codex --filter "fake codex shim"` ✅
  - `cargo run -p ralph-e2e -- codex --filter idle-start-live` ✅

## 2026-02-25 23:53 +0800 | fix(example): 子目录运行时可正确读取 PROMPT.md

- 修复 `examples/parallel-experimental-dev-engine` 在子目录内直接 `ralph run` 报错 `Prompt file ... not found` 的问题.
- 变更:
  - `examples/parallel-experimental-dev-engine/ralph.yml`: `event_loop.prompt_file` 改为 `PROMPT.md`,并同步注释/说明.
  - `examples/parallel-experimental-dev-engine/README.md`: 推荐 `cd examples/parallel-experimental-dev-engine` 后运行;仓库根目录运行需显式加 `-P examples/parallel-experimental-dev-engine/PROMPT.md`.
  - `crates/ralph-cli/tests/integration_examples.rs`: 新增回归测试,确保 example prompt_file 不再写死仓库根路径.
- 验证:
  - `cargo test -p ralph-cli` ✅

## 2026-02-26 10:08 +0800 | fix(parallel): 防止 TUI idle chat 的 human.message 自我对话回路

- 背景:
  - 在并行 TUI 的 chat idle 模式下,给 `ralph#1` 发 `human.message` 后,会出现 ralph#1 “回复自己的 human.message”并进入循环。
  - 复现证据: `/Users/cuiluming/local_doc/l_dev/my/rust/ralph-talk-example.jsonl`。

- 修复(确定性,不依赖模型行为):
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`:
    - 对 `topic=="human.message"` 且带 `source/source_instance` 的事件做 UI-only early-return:
      - 仍推送给 TUI event_observer 用于展示。
      - 但不再参与后续 routing/delivery,从机制上打断自我对话回路。

- 回归测试:
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`:
    - 新增 `parallel_does_not_route_hat_sourced_human_message_to_prevent_self_chat_loop`。

- 验证:
  - `cargo test -p ralph-core` ✅

## 2026-02-26 12:21 +0800 | improve(parallel): reply.human.message(回复输出) + app-server trace 展示 turn/start prompt

- 主题1: 回复 topic 语义拆分(避免自问自答)
  - `crates/ralph-core/src/parallel/supervisor/routing.rs`:
    - 新增 UI-only 护栏: `reply.human.message` 只用于 UI 展示,不参与路由。
  - `crates/ralph-core/src/parallel/supervisor.rs`:
    - ralph#1 内置协调器指令新增规则:
      - human-facing reply 必须用 `<event topic="reply.human.message" reply="...">...`。
  - `config/all_hat.md`:
    - 补充 `human.message` vs `reply.human.message` 的语义说明与示例。
  - `crates/ralph-tui/src/state.rs`:
    - Radar 因果链忽略 `reply.human.message`(控制面噪音)。
  - `crates/ralph-cli/src/parallel_runner.rs`:
    - 事件转发策略把 `reply.human.message` 视为控制面事件之一(避免 UI 侧被误过滤)。

- 主题2: Codex app-server trace 打印 turn/start prompt
  - `crates/ralph-cli/src/codex_app_server_session.rs`:
    - 在 `RALPH_CODEX_APP_SERVER_TRACE=1` 时,为 `turn/start` 增加:
      - `input_len`
      - 可选 `input_preview`(需 `RALPH_CODEX_APP_SERVER_TRACE_STEER_INPUT=1`,截断)。
  - `.envrc`:
    - 补齐 trace 环境变量的默认值与注释,便于 direnv 开关。

- 回归测试:
  - `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`:
    - 新增 `parallel_does_not_route_reply_human_message_topic`。

- 验证:
  - `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅

## 2026-02-26 10:56 +0800 | fix: app-server 默认回显 prompt transcript(彩色) + 并行 TUI 防自问自答

- 防循环(topic 语义):
  - ralph 回复 human 改用 `reply.human.message`.
  - Supervisor 对 `reply.human.message` 做 UI-only early-return,不再路由回 hats,避免 `ralph#1`(订阅 "*")自问自答.

- 可观测性(codex app-server):
  - `CodexAppServerRuntime` 在每次 `turn/start` 前,默认把完整 prompt 以 transcript 写入 stderr.
  - transcript 支持 ANSI 色彩,并受 `--color` 控制.

- ANSI 色彩保真:
  - TUI: stderr chunk 含 ANSI 时不再强制 muted 覆盖 fg.
  - log mode: stderr 行含 ANSI 时不再外层包 GRAY.

- 测试:
  - `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅

- 额外全量验证:
  - `cargo test` ✅

## 2026-02-26 14:50 +0800 | fix(app-server): thinking 回显 + reply 事件 EOF 容错(避免 chat 看起来“没回复”)

- app-server thinking 回显(不影响事件解析):
  - `crates/ralph-cli/src/codex_app_server_session.rs`
  - 当输出源为 `AgentMessageDelta` 时:
    - `item/reasoning/summaryTextDelta` 作为 thinking 输出持续回显到 stderr.
    - 但不进入 `HatJobResult.output`(仍坚持 stdout-only 事件解析边界)。
  - 保持 fallback: 若只有 summary 输出源,仍沿用原行为(保证在隐藏 stderr 的场景下也可见输出)。

- 回复事件更稳健(EOF 缺 `</event>` 也能回收):
  - `crates/ralph-core/src/event_parser.rs`
  - 新增最小容错:
    - 仅对 `reply.human.message` 且位于输出开头(忽略前导空白)时,允许把 EOF 当作隐式 `</event>`。
  - 同时避免“缺闭合导致吞掉后续事件”:
    - 若 payload 里出现新的 `<event `,则认为当前 event 未闭合,跳过并继续扫描后续事件。

- 验证:
  - `cargo fmt --check` ✅
  - `cargo test -p ralph-core -p ralph-cli -p ralph-tui` ✅

## 2026-02-26 21:29 +0800 | fix(record-session): 中断/错误退出也 flush JSONL(避免“没回复就 end”假象)

- 问题:
  - 并行模式被 Ctrl+C/SIGTERM/SIGHUP 打断时,`--record-session` JSONL 可能缺少尾部:
    - 看不到 `reply.human.message`
    - 看不到 `_meta.termination`
  - 用户打开 record-session 文件会误以为“没有回答就 end 了”。

- 修复:
  - `crates/ralph-cli/src/parallel_runner.rs`
    - interrupt 早退前补:
      - 写入 `_meta.termination`
      - `recorder.flush()`
    - supervisor.run Err(e) 早退前同样补 flush(保留证据)。

- 验证:
  - `cargo test -p ralph-cli` ✅

## 2026-02-27 11:32 +0800 | 补记: 并行事件发布通道规格 + prompt 明确 `ralph emit` 可随时使用

- 说明:
  - 本条为“按 append-only 规则追加到文件尾部”的补记。
  - 之前同名段落因 patch 上下文误命中,出现在文件中部;不删除旧段落,以这里为准。

- 规格:
  - `specs/parallel-event-channels.spec.md`

- prompt 补强:
  - `crates/ralph-core/src/parallel/supervisor.rs`
  - 明确: out-of-band `ralph emit` 不需要等待当前 job/turn 完结,可随时执行(包含 in-flight steer/interrupt)。

- 验证:
  - `cargo test` ✅

## 2026-02-27 12:41 +0800 | test(guardrails): 落地 openclaw 风格的可执行护栏(先锁死 stdout-only + scratchpad truncate)

- 背景:
  - 你同意继续推进后,我优先把 openclaw 那套“把不变量写成可失败检查”的思路,落到 Ralph 的 Rust 单测里。
  - 目标是阻断两类最常见且最难排查的并行回归:
    1) stderr 污染事件解析输出,导致假事件/假 completion/E2E 波动
    2) fresh run 清理 scratchpad 误删,导致 `--continue/--resume` 不稳定

- 变更:
  - `crates/ralph-cli/src/parallel_runner.rs`
    - 新增 guardrail 单测: stderr 仍流式输出,但不会拼进 `HatJobResult.output`(stdout-only).
    - fresh run 的 scratchpad 清理改为复用 `loop_runner::clear_scratchpad_for_fresh_run()`。
  - `crates/ralph-cli/src/loop_runner.rs`
    - 抽出 `clear_scratchpad_for_fresh_run()` 并增加两条回归测试(截断但不删除;缺失时 no-op).

- 验证:
  - `cargo test -p ralph-cli` ✅
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test` ✅

## 2026-02-27 12:10 +0800 | research: 阅读 openclaw(openclaw/openclaw),提炼可迁移机制(doctor/wizard/guardrails/lanes)

- 目标:
  - 从 `openclaw/openclaw` 的源码中提炼"可迁移到 ralph-orchestrator 的工程机制",而不是停留在口号。

- 我重点看的模块(与 Ralph 直接相关):
  - Wizard/Doctor(first-run UX + 可修复路径):
    - `src/wizard/onboarding.ts`
    - `src/commands/doctor.ts`
  - 并发与队列(lane/generation/draining):
    - `src/process/command-queue.ts`
    - `src/process/lanes.ts`
    - `src/gateway/server-lanes.ts`
  - guardrails(把约定变成可执行的失败条件):
    - `scripts/check-no-random-messaging-tmp.mjs`
    - `src/agents/acp-binding-architecture.guardrail.test.ts`
  - backend runner 稳健性(no-output watchdog,serialize,scopeKey):
    - `src/agents/cli-runner.ts`
    - `src/process/supervisor/supervisor.ts`
  - 安全边界与信任模型:
    - `SECURITY.md`
    - `VISION.md`

- 产出:
  - 研究笔记与可迁移点汇总: `notes.md`(末尾 "源码研究: openclaw..." 段落)。
  - 后续可落地清单(未实施,仅记录): `LATER_PLANS.md`(末尾 "借鉴 openclaw..." 段落)。

## 2026-02-27 13:08 +0800 | improve(parallel): HatJobResult 输出语义拆分(从类型层面防止 stderr 污染事件解析)

- 背景:
  - 并行模式下 `EventParser` 只能解析 stdout-only,否则 stderr 里的示例 `<event ...>`/prompt transcript/后端日志会触发假事件与 flaky。
  - 之前的 guardrail 已能卡住行为,但类型层面的 `HatJobResult.output` 仍容易误导维护者。

- 变更:
  - `HatJobResult` 从 `output` 拆成:
    - `output_for_parsing`(stdout-only,供 `EventParser`/路由/收敛判断)
    - `observed_stderr`(诊断用,不参与解析,best-effort)
  - core 的解析/收敛判断全部改为读取 `output_for_parsing`。
  - cli executor 先用最小行为变化补齐字段(当前统一 `observed_stderr=""`)。

- 验证:
  - `cargo test -p ralph-core -p ralph-cli` ✅
  - `cargo test` ✅

## 2026-02-27 13:31 +0800 | spec: 新增 `ralph doctor`(借鉴 openclaw doctor 的可修复路径)

- 目的:
  - 把 "配置/环境/工作区" 的常见启动失败,固化为一个可执行的诊断入口。
  - 避免 "跑不起来只能猜" 或 "只能让人去翻代码/看日志"。

- 产出:
  - 规格: `specs/ralph-doctor.spec.md`。
  - 补充研究笔记: `notes.md` 追加 openclaw doctor 的选项与 config-guard 模式摘录。

- 额外同步:
  - `specs/parallel-event-channels.spec.md` 已跟随实现更新: 事件解析输入字段改为 `output_for_parsing`。

## 2026-02-27 16:51 +0800 | feat(cli): 新增 `ralph doctor`(诊断常见启动失败 + 安全 `--fix`)

- 新增命令:
  - `ralph doctor` / `ralph doctor --fix` / `ralph doctor --strict`.

- 主要能力(最小可用,对齐 spec):
  - Config 加载(file/builtin/remote) + `config.validate()`.
  - Hats 拓扑检查(对齐 `ralph hats validate` 的核心规则),并为 warn/err 输出 "Fix:" 建议.
  - Backend 可用性检查:
    - `cli.backend=auto` 时跑 auto-detect.
    - `cli.backend=custom` 时验证 `cli.command` 可运行.
  - Workspace 健康度:
    - scratchpad 目录/文件存在性与可写性检查,`--fix` 可创建缺失目录/空文件.
    - `.ralph/current-events` marker 指向的 events 文件可写性检查,`--fix` 可创建.
  - Build freshness:
    - `config/all_hat.md` 比当前可执行文件更新时,提示需要重编译.

- 代码位置:
  - `crates/ralph-cli/src/doctor.rs`
  - `crates/ralph-cli/src/main.rs`
  - `crates/ralph-cli/src/hats.rs`(复用 check 输出工具)

- 验证:
  - `cargo test -p ralph-cli` ✅
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test` ✅

## 2026-02-27 17:42 +0800 | fix(parallel/app-server): Codex app-server job 支持 timeout/stale watchdog,并在超时后重启 session

- 背景:
  - openclaw 的 `cli-runner.ts` 有明确的 no-output watchdog 思路: “长期无输出就 kill 并给出专门原因”.
  - Ralph 里大多数 runner 已有 `HatJob.timeout/output_stale_timeout` 语义,但 app-server runtime 之前没有消费它:
    - cancel 时不会退出,容易导致 instance 永远 Running。
    - 卡死时只能靠 Supervisor max_runtime 止损,不可控且不够可审计。

- 变更:
  - `crates/ralph-cli/src/codex_app_server_session.rs`:
    - `CodexAppServerRuntime::execute_job()` 对齐 `HatJob.timeout/output_stale_timeout`:
      - tick 到期时判断输出是否停滞,若停滞则 `timed_out=true` 并立即返回。
      - cancel 触发时 `canceled=true` 并立即返回(不再无限等待 `turn/completed`/`task_complete`)。
    - timeout 时输出一行 stderr 证据(不参与事件解析),并重启该 instance 的 app-server session:
      - 从 runtime map 移除并 kill child,避免残留 turn/thread 污染后续 job。
    - 修复潜在 busy loop: 当 stderr/control/cancel 通道关闭时禁用对应 select 分支,避免空转占用 CPU。
  - 为单测提供可注入的 `codex` 命令:
    - 新增 `CodexAppServerRuntime::new_with_command()`(默认仍为 `codex`)。
  - 回归测试:
    - `codex_app_server_session::tests::app_server_timeout_triggers_and_returns_timed_out`:
      - 使用 fake `codex app-server` shim 模拟“无输出卡死”,断言 timed_out 语义。

- 验证:
  - `cargo fmt` ✅
  - `cargo test -p ralph-cli` ✅
  - `cargo test` ✅

## 2026-02-28 10:55 +0800 | feat(doctor): context window guard(借鉴 openclaw,把上下文窗不足/Prompt 过大变成可执行护栏)

- 背景:
  - openclaw 的启发是: context window 是硬资源,不足就应该提前 warn/block,而不是跑到一半才失败。
  - Ralph 目前无法从各 CLI 后端稳定获取“模型上下文窗”信息,因此采用配置驱动: 由用户显式声明窗口大小。

- 配置(schema):
  - `crates/ralph-core/src/config.rs`:
    - `AdapterSettings` 新增 `context_window_tokens: Option<u32>`。

- `ralph doctor` 增强:
  - `crates/ralph-cli/src/doctor.rs`:
    - 新增 D3.5 `check_context_window_guard`:
      - 若未配置 `adapters.<backend>.context_window_tokens`: `[ok] Skipped ...` 并给出配置示例。
      - 若 window < 32k: `[warn]`。
      - 若 window < 16k: `[err]`(阻断)。
      - prompt-fit 粗估:
        - 用 core `EventLoop` 真实组装一次“非并行”ralph prompt。
        - 用 chars/4 粗估 tokens,当 prompt>=85% window 时 warn,>=100% 时 err。

- 回归测试:
  - `crates/ralph-core/src/config.rs`:
    - 扩展 `test_adapter_settings` 覆盖 `context_window_tokens` 解析。
  - `crates/ralph-cli/src/doctor.rs`:
    - 新增 `doctor_fails_when_context_window_below_hard_min`。

- 文档/规格同步:
  - `specs/ralph-doctor.spec.md`: 把 context window guard 从“后续扩展”升级为 D3.5 检查项。
  - `docs/advanced/context-management.md`: 增加 `adapters.<backend>.context_window_tokens` 的配置说明与 `ralph doctor --strict` 用法。

- 验证:
  - `cargo fmt` ✅
  - `cargo check` ✅
  - `cargo test` ❌(当前机器未接受 Xcode license,链接阶段会失败; 需要 `sudo xcodebuild -license accept`)

## 2026-02-28 15:26 +0800 | 修正: macOS 下 cargo test 的 Xcode license 报错可用 CLT 绕过

- `cargo test` 并不“依赖 Xcode license”,实际是 macOS 编译/链接会走 Apple toolchain。
- 当 `xcode-select -p` 指向 Xcode.app 且未接受 license 时,`xcrun` 会拒绝执行,从而让 `cargo test` 链接失败(exit 69)。
- 本机验证: 显式指定 Command Line Tools 后,测试全量通过:
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test -p ralph-cli` ✅
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test -p ralph-core smoke_runner` ✅
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test` ✅

## 2026-02-28 15:26 +0800 | docs/dev: 记录 macOS `cargo test` 的 Xcode license 绕过方式

- 为避免后续再次误判“cargo test 需要 Xcode license”,补了两处工程化提示:
  - `.envrc`: 若存在 `/Library/Developer/CommandLineTools` 且未显式设置 `DEVELOPER_DIR`,则默认使用 CLT(绕过 Xcode.app license).
  - `DEVELOPMENT.md`: 增加 macOS 说明,给出 `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test` 的无 sudo 方案。

## 2026-02-28 15:41 +0800 | feat(parallel): 引入 command lanes(对齐 openclaw lane+generation+draining),串行化 workspace.git 并改良 shutdown 收尾

- 新增 spec:
  - `specs/parallel-command-lanes.spec.md`

- ralph-core:
  - `crates/ralph-core/src/parallel/command_queue.rs`
    - 新增 in-process `CommandQueue`(acquire-based): lanes + generation + draining + clear/reset.
    - 单元测试覆盖: 同 lane 串行、draining 拒绝、clear_lane 拒绝 queued waiter、reset 忽略 stale release。
  - `crates/ralph-core/src/parallel/supervisor.rs`
    - Supervisor 持有 `CommandQueue` 并注入到每个 HatInstance。
  - `crates/ralph-core/src/parallel/instance.rs`
    - 将高风险 git 副作用动作纳入 `workspace.git` lane:
      - `git worktree add/remove` 串行化。
      - clone 模式的 `git fetch (clone->main)` 也串行化。
    - HatInstance shutdown 进入 draining:
      - 停止启动新 job,并在退出前 best-effort workspace cleanup(跳过 hooks,保证退出可控)。
      - clone backend 的 shutdown cleanup 只做目录清理,不触碰主仓库 refs。

- 验证:
  - `cargo fmt` ✅
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test -p ralph-core` ✅
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test` ✅

## 2026-02-28 17:17 +0800 | chore: 续档 task_plan + continuous-learning 摘要 + 归档历史版本

- 续档:
  - `task_plan.md` 超过 1000 行,已续档并新建(保持根目录低噪音)。
- continuous-learning(按规则执行):
  - `notes.md` 追加“四文件摘要”,明确本轮没有新增需要提取的 skill。
- 归档(减少根目录噪音):
  - `task_plan_2026-02-28_1717.md` -> `archive/task_plan_2026-02-28_1717.md`
  - `notes_2026-02-28_1706.md` -> `archive/notes_2026-02-28_1706.md`
- 答复:
  - 汇总 openclaw 对 Ralph 的可迁移启发(guardrails/doctor/lanes/watchdog/context-window)。
  - 解释 macOS 下 `cargo test` 触发 Xcode license 的真实原因(Apple toolchain/xcrun/xcode-select),并指向 `.envrc` 与 `DEVELOPMENT.md` 的工程化规避方案。

## 2026-02-28 18:40 +0800 | research: 阅读 zeroclaw 源码,提炼对 ralph 的启发

- 研究对象: https://github.com/zeroclaw-labs/zeroclaw
- 本地 clone: `/tmp/zeroclaw-20260228-1832`

- 重点阅读(用于提炼可迁移机制,而不是泛泛而谈):
  - 文档:
    - `README.md`, `AGENTS.md`
    - `docs/README.md`, `docs/security/README.md`, `docs/sandboxing.md`
    - `deny.toml`
  - 代码:
    - `src/tools/mod.rs`, `src/runtime/traits.rs`, `src/providers/traits.rs`
    - `src/approval/mod.rs`, `src/doctor/mod.rs`
    - `src/security/prompt_guard.rs`, `src/security/estop.rs`, `src/security/firejail.rs`

- 提炼的可迁移启发(摘要):
  - Tool 系统: Tool trait + JSON schema + runtime 能力门控 + SecurityPolicy 注入.
  - ApprovalManager: supervised 工具审批 + session allowlist + audit log + 非 CLI pending approvals.
  - Doctor: 结构化 diag + 人类报告 + 错误分类(尤其 auth/access vs error).
  - 安全: PromptGuard(Aho-Corasick + regex),E-stop fail-closed,Sandbox trait + wrap_command + tests.
  - 文档 IA: docs hub + 决策树 + "Current-Behavior" vs "Proposal/Roadmap" 分层.
  - 供应链治理: cargo-deny 的 license/source allowlist + ignore reason.

- 映射到 ralph 的高 ROI 建议(只列最值得做的 3 类):
  - docs 入口补 "10 秒决策树" + "按受众" 导航,降低新用户学习成本.
  - doctor 输出支持结构化 JSON 模式(若现状缺),便于 TUI/CI 复用.
  - tool/gate/approval 的审计落盘更明确(谁触发,原因,结果),为回放和排障提供证据.

- 详细笔记: `notes.md` 2026-02-28 18:31 +0800 条目.

## 2026-02-28 21:03 +0800 | feat: `ralph doctor` 支持 JSON 输出(schema v1) + 稳定错误分类字段

- 背景/动机:
  - 你明确说当前只对 code agent 负责.
  - 现状的 doctor 是纯文本,CI/TUI/agent 只能解析 stdout,不稳定且难分流.

- 变更点:
  - `ralph doctor` 新增输出格式:
    - `--format json` 输出单个 JSON 对象(机器可读).
    - `--json` 作为 `--format json` 的便捷别名.
  - JSON schema v1(最小稳定字段):
    - `schema_version`, `verdict`, `counts`, `args`, `checks[]`.
    - 每条 check 都包含 `id/category/status/message`,并从 message 里提取可选 `fix`.
  - doctor 内部改为 reporter 统一记录 check,同时保持原有文本输出不变.

- 规格同步:
  - `specs/ralph-doctor.spec.md` 已把 JSON 输出从"后续扩展"提升为正式能力,并写明 schema 最小字段约束.

- 回归测试:
  - 新增 doctor JSON 模式测试:
    - 错误场景 stdout 是可解析 JSON(且包含 config.load=err + fix 提取).
    - 成功场景 stdout 是可解析 JSON(且 verdict=pass,errors=0).

- 验证:
  - `cargo fmt` ✅
  - `cargo test -p ralph-cli` ✅
  - `cargo test -p ralph-core smoke_runner` ✅
  - `cargo test` ✅
