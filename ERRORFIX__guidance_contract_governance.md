
## [2026-05-11 17:18:02] [Session ID: omx-1778475786175-ogndry] 问题: `omx explore` 只读探索超时

### 现象
- 执行 `omx explore --prompt ...` 后长时间无输出。
- 最终输出显示 spark 模型 `gpt-5.3-codex-spark` 503,并且 harness 在 120000ms 后 timeout。

### 原因
- 这是外部模型通道不可用与 explore harness 超时,不是仓库代码问题。

### 修复
- 不继续等待 `omx explore`。
- 降级为本地只读命令,使用 `rg` / `find` / `sed` 直接探索仓库结构。

### 验证
- 后续探索结果将来自本地文件系统命令,不依赖失败的 explore 输出。

## [2026-05-11 17:34:05] [Session ID: omx-1778475786175-ogndry] 问题: manifest invalid_type 测试替换过宽

### 现象
- `cargo test --package ralph-core --lib agent_guidance_manifest` 首次实现后失败。
- 失败项: `agent_guidance_manifest::tests::invalid_type_fails`。
- 输出显示断言没有包含 `invalid asset type`。

### 原因
- 测试使用 `valid_manifest().replace("experience", "unknown_type")`。
- 这个替换不只改了 `type = "experience"`,也把 `project-experience` asset id 改坏。
- verifier 按顺序先发现 id 不是 kebab-case,所以没有走到 invalid type 分支。

### 修复
- 把测试替换改成只替换 `type = "experience"` 这一行。

### 验证
- 重新运行 `cargo test --package ralph-core --lib agent_guidance_manifest`。
- 结果: 8 passed,0 failed。

## [2026-05-11 18:52:41] [Session ID: omx-1778475786175-ogndry] 错误: 追加计划时未使用单引号 heredoc

### 现象
- 在向 `task_plan__guidance_contract_governance.md` 追加记录时,正文包含反引号包裹的 change 名称。
- 命令使用了未加引号的 `cat <<EOF`,shell 把反引号内容当成命令替换执行。
- 终端输出出现 `zsh: command not found: agent-guidance-catalog-cli` 和 `zsh: command not found: agent-guidance-contracts`。

### 原因
- 违反了项目规则: 向上下文 Markdown 追加包含反引号的正文时,必须使用 `cat <<'EOF'`。

### 修复
- 立即追加本错误记录。
- 后续所有包含反引号的 Markdown 追加都使用单引号 heredoc。
- 在计划文件中追加修正记录,明确真实 change 名称。

### 验证
- 已检查计划尾部,确认错误只影响上下文记录文本,没有造成代码或规格文件变更。

## [2026-05-11 19:09:54] [Session ID: omx-1778475786175-ogndry] 错误: `cargo fmt --check` 发现格式漂移

### 现象
- 运行 `cargo fmt --check` 时退出码为 1。
- 输出显示 `crates/ralph-cli/src/main.rs` 和 `crates/ralph-core/src/agent_guidance_manifest.rs` 有 rustfmt diff。

### 原因
- 新增 CLI 子命令和 verifier 测试后,部分 import、match arm、长函数调用和断言没有按 rustfmt 格式落盘。

### 修复
- 执行 `cargo fmt` 统一格式化 Rust 代码。
- 格式化后重新运行 focused tests 和 `cargo fmt --check`。

### 验证
- 待后续门禁记录补充。

### 追加验证
- 已执行 `cargo fmt`。
- 已重新执行 `cargo fmt --check`,该组合命令没有再输出 diff。
- 已重新执行 `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`,结果: 13 passed,0 failed。
- 已重新执行 `cargo test -p ralph-cli verify_agent_guidance_command -- --nocapture`,结果: 2 passed,0 failed。

## [2026-05-11 20:46:10] [Session ID: omx-1778475786175-ogndry] 错误: prompt 对齐阶段 `cargo fmt --check` 发现断言排版漂移

### 现象
- `cargo fmt --check` 失败。
- 输出指向 `crates/ralph-core/src/event_loop/tests.rs` 中新增的 `outcome:` / `evidence:` output contract anchor 断言。

### 原因
- 新增测试断言时使用单行写法,超过了 rustfmt 期望的多行格式。

### 修复
- 只对 `crates/ralph-core/src/event_loop/tests.rs` 执行 rustfmt。
- 不改动测试语义。

### 验证
- 待重跑 `cargo fmt --check`、`git diff --check` 和 `openspec validate --all --strict` 后补充。

### 追加现象
- 单独执行 `rustfmt crates/ralph-core/src/event_loop/tests.rs` 后,focused prompt test 通过。
- 但 `cargo fmt --check` 又在同一文件的另一个断言处发现排版 diff。

### 追加修复
- 改为执行项目级 `cargo fmt`,让 cargo/rustfmt 按 workspace 配置统一格式化。

### 追加验证
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 19 passed,0 failed。
- `cargo test --package ralph-core --lib event_loop::tests::test_custom_hat_with_instructions_uses_build_custom_hat -- --exact --nocapture`: 1 passed,0 failed。

## [2026-05-12 09:25:00] [Session ID: omx-1778475786175-ogndry] 错误: state operation 实现阶段 `cargo fmt --check` 发现排版漂移

### 现象
- `cargo fmt --check` 失败。
- 输出 diff 全部指向 `crates/ralph-core/src/state_operations.rs`。

### 原因
- 新增 state operation 模块和测试时,多处长链式调用、错误枚举字段和断言没有按 rustfmt 期望格式落盘。

### 修复
- 执行 `cargo fmt` 统一格式化。

### 验证
- 待重跑 `cargo fmt --check`、focused tests、smoke 和全量测试后补充。

### 追加验证
- 已执行 `cargo fmt`。
- `cargo test --package ralph-core --lib state_operations -- --nocapture`: 9 passed,0 failed,无 warning。
- `cargo fmt --check`: 通过。
- `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
- `cargo test`: 全量通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 20 passed,0 failed。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 54 assets / 35 skills。

## [2026-05-12 10:25:00] [Session ID: omx-1778475786175-ogndry] 错误: state CLI adapter 阶段 `cargo fmt --check` 发现排版漂移

### 现象
- `cargo fmt --check` 返回退出码 1。
- diff 指向 `crates/ralph-cli/src/main.rs` 的 import 排序,以及 `crates/ralph-cli/tests/integration_state.rs` 的长行/断言排版。

### 原因
- 新增 CLI handler 和 integration tests 后,部分 import、命令参数数组和断言没有按 rustfmt 规则换行。

### 修复
- 执行 `cargo fmt` 统一格式化。
- 格式化后重跑 focused state CLI tests 和 `cargo fmt --check`。

### 验证
- 待后续补充。

### 追加验证
- 已执行 `cargo fmt`。
- `cargo test -p ralph-cli --test integration_state -- --nocapture`: 5 passed,0 failed。
- `cargo fmt --check`: 通过。
