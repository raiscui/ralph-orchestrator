# 任务计划: 修复 Ralph memory_store 中文切片 panic

## 目标

让 Ralph 读取或裁剪包含中文等多字节 UTF-8 文本的记忆内容时不再 panic,并用回归测试锁住行为。

## 阶段

- [x] 阶段1: 建立支线上下文并确认问题边界
- [x] 阶段2: 阅读 memory_store 相关代码和现有测试
- [x] 阶段3: 修复字符边界切片问题并补回归测试
- [x] 阶段4: 运行聚焦测试与必要 smoke test,记录交付结果

## 关键问题

1. panic 是否来自固定字节索引截断字符串。
2. 修复应该放在唯一负责生成摘要或预览的位置,避免多处零散处理。
3. 需要确认现有工作区有大量用户改动,本次不能误改或回滚无关文件。

## 做出的决定

- 决定: 使用 memory_boundary_fix 支线上下文集。
  理由: 当前主线六文件很长,且已有 memory_axes 支线。这个 panic 修复是独立 bugfix,单独记录更清楚。
- 决定: 当前环境不进入 OMX runtime 的 ralph 工作流。
  理由: 本界面是 Codex App outside tmux,用户给的是 panic 证据,可以直接走源码修复和验证。

## 遇到错误

- 读代码搜索时第一次 rg 同时传入不存在的 tests 路径,命令失败。
  决议: 后续只搜索实际存在的路径,不忽略该失败。

## 状态

**全部完成** - 已修复 UTF-8 字符预算截断 panic,并通过聚焦测试、smoke_runner 和根目录 cargo test。

## [2026-04-29 22:04:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [状态更新]: 完成根因定位,进入修复

- [x] 阶段1: 建立支线上下文并确认问题边界
- [x] 阶段2: 阅读 memory_store 相关代码和现有测试
- [ ] 阶段3: 修复字符边界切片问题并补回归测试
- [ ] 阶段4: 运行聚焦测试与必要 smoke test,记录交付结果

- 根因:
  - memory_store 的 truncate_to_budget 把预算换算成字符数,但使用字节下标截断字符串。
  - 当预算落在中文等多字节字符中间时,Rust 字符串切片会 panic。
- 同类风险:
  - event_loop 的 scratchpad 尾部截断也使用字节长度倒推字符预算,同样可能切进多字节字符。
- 当前行动:
  - 在 text 工具中新增字符数到字节边界的 helper。
  - 让两个截断入口复用同一个 helper,避免以后继续分散写 unsafe-like 切片逻辑。

## [2026-04-29 22:13:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] [完成]: UTF-8 字符边界截断修复已验证

- [x] 阶段1: 建立支线上下文并确认问题边界
- [x] 阶段2: 阅读 memory_store 相关代码和现有测试
- [x] 阶段3: 修复字符边界切片问题并补回归测试
- [x] 阶段4: 运行聚焦测试与必要 smoke test,记录交付结果

- 已修改:
  - crates/ralph-core/src/text.rs
  - crates/ralph-core/src/memory_store.rs
  - crates/ralph-core/src/event_loop/mod.rs
  - crates/ralph-core/src/event_loop/tests.rs
- 已验证:
  - cargo test --package ralph-core --lib -- memory_store::tests::test_truncate_to_budget_is_utf8_safe_for_chinese --exact
  - cargo test --package ralph-core --lib -- event_loop::tests::test_scratchpad_injection_tail_truncation_is_utf8_safe --exact
  - cargo test --package ralph-core --lib -- text::tests::test_byte_index_after_chars_uses_utf8_boundaries --exact
  - cargo test -p ralph-core smoke_runner
  - cargo test
- 当前状态:
  - **全部完成**: 用户报告的 memory_store 中文截断 panic 已修复,同类 scratchpad 尾部截断风险也已一并覆盖。
