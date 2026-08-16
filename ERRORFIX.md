## [2026-05-28 16:25:43] [Session ID: omx-1779954714247-oab9zc] 错误修复: record_session bin target 测试 fixture 字段误放

### 现象
- 命令: `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`。
- 编译失败: `struct TopologySpawnedInstance has no field named recoverable_failures`。
- 位置: `crates/ralph-cli/src/record_session.rs:1440:17`。

### 当前判断
- 这是 fixture 结构体字段误放,不是 runtime recoverable lifecycle 失败。
- `recoverable_failures` 应属于 `AgentInstanceSnapshot`,不属于 `TopologySpawnedInstance`。

### 修复计划
- 读取 `record_session.rs` 相关测试段落。
- 删除或移动错误字段,保持 `TopologySpawnedInstance` fixture 只包含其真实字段。
- 重跑正确 target 的 focused test。

### 验证状态
- 待修复后更新。

## [2026-05-28 16:27:27] [Session ID: omx-1779954714247-oab9zc] 验证更新: record_session fixture 字段误放已修复

### 修复
- 删除 `crates/ralph-cli/src/record_session.rs` 中误加到 `TopologySpawnedInstance` fixture 的 `recoverable_failures: Vec::new()`。
- 保持 recoverable evidence fixture 位于 `AgentInstanceSnapshot.recoverable_failures`,也就是正确的观察面结构。

### 验证
- `cargo test -p ralph-cli --bin ralph record_session::tests::aggregate_collects_evidence_inspect -- --exact --nocapture`: 1 passed。

### 结论
- 上一假设成立: 该错误是测试 fixture 字段误放,不是 recoverable runtime 或 record-session 渲染逻辑问题。

## [2026-05-28 16:47:12] [Session ID: omx-1779954714247-oab9zc] 错误修复: integration_record_session fixture 漏填 recoverable_failures

### 现象
- `cargo test -p ralph-cli --quiet` 编译失败。
- 错误: `missing field recoverable_failures in initializer of AgentInstanceSnapshot`。
- 位置: `crates/ralph-cli/tests/integration_record_session.rs:377:25`。

### 当前判断
- `AgentInstanceSnapshot` 已新增 `recoverable_failures` 观察面字段。
- `integration_record_session.rs` 中的旧 fixture 需要显式填 `Vec::new()`。

### 修复计划
- 补齐 fixture 字段。
- 重跑 `cargo test -p ralph-cli --quiet`。

## [2026-05-28 16:50:23] [Session ID: omx-1779954714247-oab9zc] 验证更新: integration_record_session fixture 已补齐

### 修复
- 在 `crates/ralph-cli/tests/integration_record_session.rs` 的 `AgentInstanceSnapshot` fixture 中补充 `recoverable_failures: Vec::new()`。

### 验证
- `cargo test -p ralph-cli --quiet`: passed。

### 结论
- 上一判断成立: 这是结构扩展后的旧 fixture 漏字段,不涉及 runtime 语义。

## [2026-05-28 17:51:35] [Session ID: omx-1779954714247-oab9zc] 错误: 搜索带反引号文本时触发 shell command substitution

### 问题
- 在验证 `EXPERIENCE.md` / `AGENTS.md` / skill 内容时,曾用双引号包裹包含反引号的 `rg` pattern。
- zsh 将 pattern 中的反引号内容当成命令替换执行,出现: `zsh:1: command not found: agent-cli-recoverable-failure-retry`。

### 原因
- shell 双引号内的反引号仍会触发 command substitution。
- Markdown 文本里高频出现反引号代码片段,所以 `rg` / `grep` 搜索 pattern 也必须按 heredoc 同类风险处理。

### 修复
- 改用单引号包裹 `rg` pattern 后重跑验证。
- 复核已存在的 `self-learning.shell-heredoc-backtick-command-substitution` skill,确认其明确覆盖 `rg` / `grep` 搜索场景。

### 验证
- 使用单引号 pattern 重跑 `rg`: passed。
- `git diff --check -- EXPERIENCE.md AGENTS.md .codex/skills/self-learning.ralph-agent-cli-recoverable-failure-retry/SKILL.md task_plan.md`: passed。

## [2026-05-29 00:09:00] [Session ID: omx-1779954714247-oab9zc] 错误: WORKLOG 续档 manifest 使用未加引号 heredoc

### 问题
- 写入 `archive/manifests/ARCHIVE_MANIFEST__default_worklog_rollover_2026-05-29_0008.md` 时误用了未加引号 heredoc。
- manifest 正文包含 Markdown 反引号,触发 zsh command substitution。
- 终端出现 `command not found` 和 `permission denied` 输出。

### 原因
- 应使用 `cat <<'EOF'`,但实际用了未加引号 `cat <<EOF`。
- 这与上一轮 `rg` pattern 反引号错误属于同一类 shell quoting 问题。

### 修复
- 立即用 `cat <<'EOF'` 重写 archive manifest。
- 保留已移动的旧 `WORKLOG.md`,因为归档目标和新 `WORKLOG.md` 均已存在且行数验证通过。

### 验证
- 重新读取 manifest,确认反引号内容按字面保留。
- 后续所有含反引号 Markdown 写入必须使用单引号 heredoc。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误: 追加 task_plan 时再次使用未加引号 heredoc

### 现象
- 追加 `task_plan.md` 的命令输出出现:
  - `zsh:2: command not found: display.rs`
  - `zsh:2: command not found: record_session.rs`
  - `zsh:2: command not found: routing_tests.rs`

### 原因
- Markdown 正文里包含反引号包裹的文件名。
- 命令使用了 `cat <<EOF`,导致 zsh 把反引号内容当成 command substitution 执行。
- 这与此前已记录的 heredoc / rg 反引号错误属于同类问题。

### 修复
- 后续所有包含反引号的 Markdown 追加都必须使用 `cat <<'EOF'`。
- 该错误只影响未 staged 的上下文文件,不会污染当前 recoverable retry staged index。

### 验证计划
- 继续通过 `git diff --cached --name-only` 和 `git status --short -- task_plan.md ERRORFIX.md` 确认上下文文件未进入 index。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误: zsh 循环变量 path 覆盖 PATH

### 现象
- 执行 staged-only blob 更新脚本时输出: `zsh:11: command not found: git`。
- 前一行 `rg` 可以正常运行,说明不是系统真的缺 `git`。

### 原因
- zsh 中小写变量 `path` 与 `PATH` 有特殊绑定关系。
- 脚本使用 `for path in ...`,把 shell 搜索路径覆盖成了普通文件路径。
- 之后命令解析找不到 `git`。

### 修复
- 后续 shell 循环禁止使用 `path` 作为变量名。
- 改用 `file_path` / `target_file` 等不会污染 PATH 的变量名。

### 验证
- 重新打开新 shell 执行 `command -v git`。
- 用安全变量名重新执行 `git update-index --cacheinfo`。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误: staged-only cargo fmt check 失败

### 现象
- 临时 worktree 应用 cached patch 后,`cargo fmt --check` 失败。
- 主要差异来自新增 CLI / record-session / routing_tests 的 rustfmt 排版,以及此前 staged `config.rs` 的多余空行。

### 原因
- 本轮使用 index-only blob 构造 staged patch,没有经过 rustfmt。
- 之前 core staged patch 中也有一处 rustfmt 会调整的空行。

### 修复
- 在临时 staged-only worktree 中运行 `cargo fmt`。
- 只把已经 staged 的 Rust 文件的格式化结果写回主仓库 index,不修改当前混杂 working tree。

### 验证
- 重新运行 `cargo fmt --check` 于 staged-only 临时 worktree。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 验证阻断: staged-only 全量 cargo test 缺未跟踪 example PROMPT.md

### 现象
- 在 staged-only 临时 worktree 中运行 `cargo test --quiet` 失败。
- 失败集中在 `crates/ralph-cli/tests/integration_examples.rs`。
- 错误为多个 `examples/parallel-*/PROMPT.md` 不存在。

### 动态证据
- 主工作区 `find examples -maxdepth 2 -name PROMPT.md` 能看到这些 `PROMPT.md`。
- `git ls-files examples` 只包含多数 `README.md` / `ralph.yml`,除 `parallel-experimental-dev-engine/PROMPT.md` 外,其它 `PROMPT.md` 未被 Git 跟踪。
- staged-only 临时 worktree 从 HEAD 检出,只包含 tracked files,所以这些未跟踪 prompt fixture 不存在。

### 当前结论
- 该失败不是 recoverable retry staged patch 引入的代码错误。
- 它揭示了一个独立的测试夹具/仓库状态问题: 全量 cargo test 依赖未跟踪 example PROMPT.md。

### 当前处理
- 不把这些 example PROMPT.md 混入本 recoverable retry scoped commit。
- 保留 focused / smoke / OpenSpec strict 作为本轮可重复 staged-only 证据。

## [2026-05-29 00:00:00] [Session ID: native-codex-20260529] 错误修复: integration_record_session watch 测试固定 sleep 过短

### 现象
- staged-only `cargo test -p ralph-cli --test integration_record_session --quiet` 中,`record_watch_auto_locates_latest_pointer_and_streams_lines` 失败。
- 单独重跑该测试仍失败,断言为 stdout 未包含 `_meta.session_start`。

### 验证
- 手工复现同一路径,将等待时间从 200ms 放宽到 0.8s,`ralph record watch --from-start` 能正常输出 `_meta.session_start`。

### 结论
- 该失败是既有测试时序问题,不是 recoverable retry 或 `record_cli` summary 入口引起。
- 固定 sleep 200ms 对当前 staged-only 临时 worktree 的二进制启动不稳定。

### 修复
- 将该测试改为等待 stdout 文件出现目标内容或超时,避免依赖固定 sleep。


## [2026-05-29 18:03:28] [Session ID: omx-1779004640353-blcixq] 错误: continuous-learning 写入 EXPERIENCE 时 f-string 误解析花括号

### 现象
- 使用 Python f-string 追加 EXPERIENCE.md 时,正文包含 OutputBlock::{Text, Image} 形式的 Rust 枚举示例。
- Python 将 {Text, Image} 当作 f-string 表达式解析,导致 NameError。

### 原因
- 这是脚本写入方式错误,不是项目代码错误。
- 包含 Rust / JSON / Mermaid / Markdown 示例时,正文常会含花括号,不适合直接放进 f-string。

### 修复
- 保留已经成功写入的 notes.md 和 LATER_PLANS.md。
- 改用普通字符串模板替换时间戳,避免正文被 f-string 解释。

### 验证
- 已重新追加 EXPERIENCE.md 标记,并用 rg 检查经验条目存在。


## [2026-05-29 18:57:29] [Session ID: omx-1779004640353-blcixq] 错误修复: example PROMPT.md fixture 未进入 Git 真相源

### 现象
- 干净 worktree / staged-only worktree 中运行 full `cargo test --quiet` 时,`crates/ralph-cli/tests/integration_examples.rs` 会因为多个 `examples/parallel-*/PROMPT.md` 不存在而失败。
- 主工作区能看到这些 `PROMPT.md`,但它们没有被 Git 跟踪。

### 原因
- `.gitignore` 全局忽略 `PROMPT.md`。
- 之前只给 `examples/parallel-experimental-dev-engine/PROMPT.md` 开了例外。
- 其它 runnable parallel examples 的 prompt templates 实际存在,但被 ignore 规则挡在 Git 真相源之外。

### 修复
- 将 `.gitignore` 的单个 example prompt 例外改为 `!examples/parallel-*/PROMPT.md`。
- 将 24 个现有 `examples/parallel-*/PROMPT.md` 加入 scoped staged patch。
- 保持测试契约不变,因为 specs / README / integration test 都要求这些 example 自包含。

### 验证
- `git diff --cached --check`: passed。
- `cargo test -p ralph-cli --test integration_examples --quiet`: 26 passed。
- staged-only clean worktree + 当前 cached patch:
  - `cargo test -p ralph-cli --test integration_examples --quiet`: 26 passed。
  - `cargo test --quiet`: passed。
- staged forbidden context check 无输出,说明六文件上下文没有进入 fixture patch。

## [2026-08-02 23:50:00] [Session ID: omx-1785579233065-awidzo] ERRORFIX: codex app-server 不接受 --profile 导致 ralph#1 启动失败

### 现象
- ralph-example 加 `-p deepseek` 后, ralph#1 状态 running → failed(零输出)
- 手动 `codex app-server --profile deepseek` 报错: unexpected argument '--profile'

### 原因
- app_server.rs build_codex_app_server_process_args 把 profile 透传为 `--profile`
- codex(0.146)的 app-server 子命令只支持 --listen/--config, 不支持 --profile

### 修复
- app_server.rs: 不再透传 --profile, 改为 warn(提示用 --config 表达等价语义)
- 保留 parse 逻辑(未来 codex 支持时可恢复透传)

### 验证
- 修复后 ralph#1 恢复正常(idle/running 交替, app-server 通道工作)
- demo 用 deepseek 模型仍无法稳定闭环(模型行为漂移: human.message 乱入/审计不完整) — 模型兼容性问题, 非本 bug

## [2026-08-16 13:35:00] [Session ID: omx-1786600320381-z290x9] 错误修复: parallel-hat-instances `--full-auto` minimax 不兼容

### 现象
- 命令: `RALPH_E2E_CODEX_PROFILE=minimax cargo run -p ralph-e2e -- codex --filter parallel-hat-instances`
- 失败: codex CLI 拒绝 `--full-auto` flag (minimax provider wrapper 不支持)
- 影响: parallel-hat-instances + parallel-hat-instances-zh 在 minimax 下不能跑,只能用 default codex account

### 根因
- `crates/ralph-e2e/scenarios/hat-instances.yaml` line 21 与 `hat-instances-zh.yaml` line 20 残留 `- --full-auto`
- `--full-auto` 是 OpenAI codex CLI 专属组合 flag (sandbox + ask-for-approval),minimax provider 不识别
- 历史 task plan (2026-08-14) 已经 work-around 同样问题: emit-spawn-instance.yaml 删 `--full-auto` 替换 `--sandbox danger-full-access`
- 该 fix 没覆盖 parallel-hat-instances (被标为 "独立 fix, 跟本次 fix 无关")

### 修复
- 跟 emit-spawn-instance 完全对称:
  - `sed -i '' 's/        - --full-auto/        - --sandbox\
        - danger-full-access/'` 在两个 YAML 上
- 改动范围: 2 文件, 4 insertions, 2 deletions (最小对称 diff)
- 不动 Rust: code-defined `parallel/hat_instances.rs` 不再注册, declarative YAML 是 source of truth

### 验证
- `cargo check -p ralph-e2e` 无 error (仅有 296 个无关的 deprecation warning)
- `cargo run -p ralph-e2e -- --list` 仍列出 parallel-hat-instances + parallel-hat-instances-zh
- `cargo test -p ralph-e2e --lib -- all_scenario_yamls` 1 passed (YAML schema 验证)
- 未跑 live: 需要 minimax account 上有 minimax 模型可用 + minimax API 高负载缓解 (后置条件)

### 结论
- 上一次假设成立: 同样的 `--full-auto` 残留问题, 同样的 `--sandbox danger-full-access` 替代方案
- 后续类似 bug pattern 已识别: `starting-event-inference.yaml` + `starting-event-inference-multi-candidate.yaml` 仍残留 `--full-auto`, 已在 LATER_PLANS 跟踪

## [2026-08-16 13:45:00] [Session ID: omx-1786600320381-z290x9] 错误修复: starting-event-inference `--full-auto` minimax 不兼容 (平行 fix)

### 现象
- 同 parallel-hat-instances: minimax provider 不支持 `--full-auto`
- 影响: `parallel-starting-event-inference` + `parallel-starting-event-inference-multi-candidate` 在 minimax 下不能跑

### 根因
- 跟前一次 fix (parallel-hat-instances) 同源, 都是 declarative YAML 残留 `--full-auto`
- 是同一 git commit 周期的 scope 扩展

### 修复
- 跟前一次 fix 完全对称:
  - `sed -i '' 's/        - --full-auto/        - --sandbox\
        - danger-full-access/'` 在两个 YAML 上
- 改动范围: 2 文件, 4 insertions, 2 deletions (最小对称 diff)
- 当前 declarative YAML 路径下 `--full-auto` 残留 = 0

### 验证
- `cargo test -p ralph-e2e --lib -- all_scenario_yamls` 1 passed
- `cargo run -p ralph-e2e -- --list` 正常列出两个场景
- `grep -rln "        - --full-auto" crates/ralph-e2e/scenarios/` → 0 (全仓库 YAML 清理干净)

### 结论
- 上一假设成立: 也是 `--full-auto` 残留, 同一 `--sandbox danger-full-access` 替代方案
- 后续: Rust code-defined scenarios (dead code) 仍有 `--full-auto` 残留, 但因为不再 Imperative 注册, 实际不会跑
- 后续可清理: 4 个 Rust 文件 (parallel/hat_instances.rs, emit_spawn_instance.rs, starting_event_inference.rs, mod.rs, parallel_trigger_routing_example.rs) 在 Wave 3.4 物理删除时一并清理
