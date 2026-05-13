
## [2026-05-11 14:11:52] [Session ID: omx-1778475786175-ogndry] 任务名称: experience_promotion workaround 审计修复收尾

### 任务内容
- 复核 Stop hook 指出的 `Temporary workaround for one topic only.`。
- 确认该文本位于 `crates/ralph-core/src/experience_promotion.rs` 的测试 fixture,不是运行时 fallback 分支。
- 将 fixture summary 改为 `Topic-local memory axes guidance should not remain in shared role experience.`。
- 保留 role -> topic demotion 测试语义,不改业务逻辑。

### 完成过程
- 使用精确搜索确认审计词命中位置。
- 读取测试上下文,把候选结论拆成“现象 / 假设 / 备选解释 / 验证计划”。
- 修正测试 fixture 文案,避免把 topic-local 规则误写成临时 workaround。
- 当前会话重新运行验证,没有只依赖上一轮交接摘要。

### 验证证据
- `rg -n 'Temporary workaround|workaround|bypass|fallback' crates/ralph-core/src/experience_promotion.rs || true`: 无命中。
- `cargo test --package ralph-core --lib -- experience_promotion::tests::role_demotion_to_topic_keeps_topic_reference --exact`: 1 passed, 0 failed。
- `cargo test --package ralph-core --lib experience_promotion`: 8 passed, 0 failed。
- `cargo test`: exit code 0,全量测试通过。

### 总结感悟
- hook 命中文案时,先区分运行时逻辑和测试 fixture,不要为了过 hook 直接改注释或删行为。
- 测试 fixture 的文本也会成为源码审计面。测试数据需要表达真实语义,不要使用 “temporary/workaround/fallback” 这种会误导长期维护者的词。
- 写六文件日志时,凡是 Markdown 正文含反引号,必须使用 `cat <<'EOF'`。
