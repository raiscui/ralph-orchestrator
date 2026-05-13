## [2026-04-29 22:13:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 任务名称: 修复 memory_store 中文字符边界 panic

### 任务内容

- 修复 Ralph 在 memory_store 截断包含中文的上下文时发生 byte index is not a char boundary panic 的问题。
- 顺手修复 scratchpad 尾部预算截断中的同类 UTF-8 字符边界风险。
- 添加中文内容回归测试,确保以后不会只靠 ASCII 测试误判安全。

### 完成过程

- 定位到 crates/ralph-core/src/memory_store.rs 的 truncate_to_budget 使用字符预算,但直接拿预算值作为 byte index 切片。
- 在 crates/ralph-core/src/text.rs 添加 byte_index_after_chars helper,把“按字符数保留”统一转换成 Rust 字符串可安全切片的 byte index。
- 将 memory_store 的 head budget 截断改为通过 helper 获取安全边界。
- 将 event_loop 的 scratchpad tail 截断改为先按字符数定位起点,再按换行边界调整。
- 新增三类回归测试:
  - text helper 的中文边界测试。
  - memory_store 中文预算截断测试。
  - scratchpad 中文尾部截断测试。

### 验证

- cargo fmt 通过。
- cargo test --package ralph-core --lib -- memory_store::tests::test_truncate_to_budget_is_utf8_safe_for_chinese --exact 通过。
- cargo test --package ralph-core --lib -- event_loop::tests::test_scratchpad_injection_tail_truncation_is_utf8_safe --exact 通过。
- cargo test --package ralph-core --lib -- text::tests::test_byte_index_after_chars_uses_utf8_boundaries --exact 通过。
- cargo test -p ralph-core smoke_runner 通过,12 个测试全部通过。
- cargo test 通过,包括 workspace 单测和 doctest。

### 总结感悟

- 只要注释写的是字符预算,实现就不能再碰裸 byte index。
- Rust 字符串切片 panic 很硬,这种问题必须在预算转换处统一收口。
- 中文上下文文件是这个项目的真实输入,回归测试不能只用 ASCII。
