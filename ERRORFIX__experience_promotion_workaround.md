
## [2026-05-11 14:03:02] [Session ID: omx-1778475786175-ogndry] 问题: 单测命令参数位置错误

### 现象
- 执行 `cargo test --package ralph-core --lib experience_promotion::tests::role_demotion_to_topic_keeps_topic_reference --exact` 失败。
- Cargo 输出: `error: unexpected argument '--exact' found`。

### 原因
- `--exact` 是传给 test harness 的参数,必须放在 `--` 之后。
- 这是命令用法错误,不是源码行为错误。

### 修复
- 改用 `cargo test --package ralph-core --lib -- experience_promotion::tests::role_demotion_to_topic_keeps_topic_reference --exact`。

### 验证
- 下一步重跑正确命令并读取输出。

## [2026-05-11 14:09:34] [Session ID: omx-1778475786175-ogndry] 问题: 追加 Markdown 时未加引号 heredoc 触发反引号命令替换

### 现象
- 追加 `task_plan__experience_promotion_workaround.md` 时,终端输出 `zsh:2: command not found: experience_promotion.rs`。
- 刚追加的计划段落里,原本应该保留的 `` `experience_promotion.rs` `` 被 shell 命令替换吞掉,变成了空白文本。

### 原因
- 我错误使用了未加引号的 `cat <<EOF`。
- Markdown 正文包含反引号代码片段时,zsh 会把反引号里的内容当作 command substitution 执行。

### 修复
- 立即读取 `self-learning.shell-heredoc-backtick-command-substitution` skill。
- 后续追加 Markdown 改用 `cat <<'EOF'` 的单引号 heredoc。
- 对已经写坏的 append-only 记录,不在中间改写历史,而是在 `task_plan__experience_promotion_workaround.md` 末尾追加更正说明。

### 验证
- 写入后用 `tail -n` 复查末尾文本。
- 后续验证命令仍需要重新执行,不能因为写日志错误而跳过。

## [2026-05-11 14:11:52] [Session ID: omx-1778475786175-ogndry] 问题: Stop hook 拦截测试 fixture 中的 workaround 文案

### 现象
- Stop hook 报告 `crates/ralph-core/src/experience_promotion.rs` 中存在新增源码行: `Temporary workaround for one topic only.`。

### 原因
- 该文本不是业务代码 fallback,而是 `role_demotion_to_topic_keeps_topic_reference` 测试里的 `ExperienceEntry::new(...)` summary fixture。
- fixture 使用了 “Temporary workaround” 表述,容易让审计器和维护者误判为未落地临时绕过。

### 修复
- 将测试 fixture summary 改成语义更明确的 topic-local rule 文案:
  - `Topic-local memory axes guidance should not remain in shared role experience.`
- 没有改动 promotion/demotion 业务逻辑。

### 验证
- 审计搜索无命中。
- 精确测试通过: `role_demotion_to_topic_keeps_topic_reference`。
- `experience_promotion` 模块 8 个测试通过。
- `cargo test` 全量测试 exit code 0。
