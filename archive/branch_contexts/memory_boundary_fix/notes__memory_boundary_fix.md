## [2026-04-29 22:04:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 笔记: UTF-8 截断 panic 根因

## 来源

### 来源1: 用户 panic 文本

- 要点:
  - panic 位置是 crates/ralph-core/src/memory_store.rs 第 311 行。
  - 报错为 byte index 1200 is not a char boundary。
  - 被切开的字符是中文设,说明固定字节索引落在 UTF-8 多字节字符中间。

### 来源2: crates/ralph-core/src/memory_store.rs

- 要点:
  - truncate_to_budget 使用 budget 乘 4 得到 char_budget。
  - 但随后执行 content 的字节切片。
  - 这与注释中的字符预算语义不一致。

### 来源3: crates/ralph-core/src/event_loop/mod.rs

- 要点:
  - scratchpad 注入同样以 4000 tokens 约等于 16000 chars 为预算。
  - 当前实现用 content.len 倒推 byte start,再从该 byte index 切片。
  - 这是同类 UTF-8 字符边界风险。

## 综合发现

### 修复方向

- 单一修复点应该是通用 text helper,负责把保留字符数转换为安全 byte index。
- memory_store 负责 head budget 截断。
- scratchpad 注入负责 tail budget 截断。
- 测试需要覆盖中文内容,而不是只用 ASCII padding。
