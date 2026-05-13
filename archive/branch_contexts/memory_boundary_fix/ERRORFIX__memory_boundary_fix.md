## [2026-04-29 22:13:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 错误修复: memory_store 中文截断 panic

### 问题现象

- 用户运行 Ralph 时主线程 panic。
- panic 位置是 crates/ralph-core/src/memory_store.rs 第 311 行。
- 报错信息说明 byte index 1200 落在中文字符设的 UTF-8 字节中间。

### 根本原因

- truncate_to_budget 把 token budget 乘以 4 得到字符预算。
- 旧实现直接执行 content 的字节切片。
- 字符预算和 byte index 混用后,遇到中文或 emoji 等多字节字符就可能切进字符内部。

### 修复方式

- 新增 text helper: byte_index_after_chars。
- 任何按字符数截断的逻辑,先用 helper 找到安全 byte index。
- memory_store 的 truncation 改为使用该 helper。
- scratchpad tail truncation 的同类 byte 起点计算也同步改成字符安全路径。

### 验证结果

- 新增中文回归测试覆盖 memory_store。
- 新增中文回归测试覆盖 scratchpad tail truncation。
- 新增 text helper 单测。
- ralph-core smoke_runner 通过。
- 根目录 cargo test 通过。

### 避免再犯

- 以后看到 budget chars, max chars, tail chars 这类语义时,不要直接使用 String::len 结果做切片边界。
- 如果必须切片,先通过 char_indices 或统一 helper 得到安全 byte index。
