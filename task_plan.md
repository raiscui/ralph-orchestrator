# 任务计划: 修复 `ralph events` 在 payload 含非 ASCII 时可能 panic

> 创建时间: 2026-01-23 16:02 (CST)

## 目标
`ralph events` 在输出表格(`--format table`)时，即使事件 payload/hat/topic 含有多字节 UTF-8 字符(中文/emoji)，也不应 panic。

## 阶段
- [x] 阶段1: 计划和设置
- [x] 阶段2: 研究/收集信息
- [x] 阶段3: 执行/构建
- [x] 阶段4: 审查和交付

## 方案方向(至少两个)
### 方案A: 不惜代价，最佳方案
- 修复 `ralph-cli` 中所有可能发生 UTF-8 非字符边界切片的点(不仅仅是 `truncate`)。
- 追加 Unicode 回归测试，覆盖中文/emoji 在截断边界附近的场景。
- 顺手把 `ralph-e2e` 里同类的 `truncate` 也做成“永不 panic”，避免将来测试用例带 Unicode 时炸。

### 方案B: 先能用，后面再优雅
- 仅修复 `crates/ralph-cli/src/display.rs` 的 `truncate` 和 `payload_preview` 逻辑，确保 `ralph events` 不 panic。
- 只补最小必要测试。

## 做出的决定
- [决定] 先落地“方案B”的最小修复，但实现方式会尽量可复用；如果改动很小，我会顺手把 `ralph-e2e` 的同类截断也一起修掉(接近方案A的收益，但不做大范围重构)。

## 关键问题
1. panic 的根因是否是对 `&str` 使用按字节索引切片(`&s[..N]`)导致的 “not a char boundary”？
2. 修复后是否会影响表格对齐/展示效果？(允许轻微变化，但不能再 panic)
3. 是否已有类似的 UTF-8 截断修复模式可复用？(例如项目里其他 `truncate` 实现)

## 遇到的错误
- 暂无

## 状态
**目前状态：已完成**  
- 修复已完成：`ralph events` 对中文/emoji 等多字节 UTF-8 payload 不再因为截断而 panic。
- 验证已完成：`cargo test -p ralph-cli` 与 `cargo test` 均通过。

---

# 追加任务计划: 修复 `ralph-e2e` 里同类 UTF-8 截断 panic 风险

> 追加时间: 2026-01-23 16:10 (CST)

## 追加目标
`crates/ralph-e2e` 的多个 `truncate()` 目前也用 `&s[..N]` 按字节切片；当输出包含中文/emoji 时同样可能 panic。  
这会让 e2e 报告在“打印错误信息”时崩溃，影响排障效率。

## 追加阶段
- [x] 阶段5: 扫描并修复 e2e 的 truncate
- [x] 阶段6: 为 e2e 补回归测试
- [x] 阶段7: 重新验证并交付

## 追加状态
**目前状态：追加任务已完成**  
- 我已修复 `crates/ralph-e2e/src/scenarios/*` 里所有 `truncate()` 的 UTF-8 边界问题，并补了回归测试。
- 验证已完成：`cargo test -p ralph-e2e`、`cargo fmt --check`、`cargo clippy -p ralph-e2e` 均通过。

## 追加说明（进一步稳健性）
- 我用 ast-grep 扫描到 `crates/ralph-cli/src/display.rs` 在解析 `ts` 的时间字段时仍存在 `&time_str[..N]` 这种按字节切片。
- 虽然正常情况下 `ts` 是 ISO 8601 ASCII 字符串，但 agent 写入的事件可能带“异常 ts 文本”。
- 我已把这里也改成 UTF-8 安全切片，并补了回归测试，确保异常 `ts` 也不会让 `ralph events` 崩溃。
- 验证已完成：`cargo test -p ralph-cli`、`cargo fmt --check`、`cargo clippy -p ralph-cli` 均通过。

---

# 任务计划: 排查并修复 TUI 中文显示异常

> 追加时间: 2026-01-24 15:50 (+0800)

## 目标
Ralph 默认启用的 TUI 在显示中文（或包含中文的输出）时，不出现乱码、丢字、错位、异常换行等问题。  
同时保证英文/ANSI 颜色/markdown 渲染不被破坏。

## 阶段
- [x] 阶段1: 收集现象与稳定复现
- [x] 阶段2: 代码路径定位与假设列表
- [x] 阶段3: 最小实验验证根因
- [x] 阶段4: 修复 + 回归测试 + TUI 验证

## 方案方向(至少两个)
### 方案A: 不惜代价，最佳方案(倾向)
把“PTY 字节流 → UTF-8 文本流”的解码做成**增量解码器**：
- 允许 PTY read() 在任意字节边界分片。
- 缓存不完整的 UTF-8 尾部，下一次拼接后再解码。
- 对真正的非法字节序列，做明确策略（替换为 � 或直接保留原始 bytes 以便调试）。

优点：这是从根因处修，最符合“流式输出”的现实情况。  
风险：需要非常小心不要影响现有的 ANSI/JSON streaming 解析逻辑。

### 方案B: 先能用，后面再优雅
如果问题主要是“宽字符宽度计算/截断”，则先在 TUI 渲染层做补丁：
- 所有截断/对齐都改用 `unicode-width` 的显示宽度，而不是 `.len()`/`chars().count()`。
- 对 emoji / CJK 组合字符做兼容。

优点：改动面可能更小。  
风险：如果根因是 UTF-8 分片导致的乱码/丢字，这个方案只能治标。

## 关键问题
1. 你看到的“中文显示问题”属于哪一类：乱码(出现 �/方块)、丢字、对齐错位、还是换行/截断异常？
2. 只在 TUI 模式发生吗？`ralph run --no-tui` 是否正常？
3. 发生时使用的 backend 是哪个（claude/kiro/gemini/codex/custom）？输出是否包含 ANSI 颜色？

## 状态
**目前状态：已完成**  
- 根因：`crates/ralph-adapters/src/pty_executor.rs` 的 `run_observe_streaming()` 逐 chunk 使用 `std::str::from_utf8(&data)`，当 PTY 读分片落在中文/emoji 多字节字符中间时会解码失败；旧逻辑在失败时会直接丢弃整个 chunk，导致流式输出（含 TUI）出现“中文丢字”。  
- 修复：引入 `Utf8StreamDecoder` 做 UTF-8 增量解码（缓存不完整尾部、继续解码），并接入 `run_observe_streaming()` 的主循环与 drain 逻辑。  
- 回归测试：新增 `test_utf8_stream_decoder_handles_split_multibyte_char` / `test_utf8_stream_decoder_replaces_invalid_bytes_and_continues`。  
- 验证：已运行 `cargo test -p ralph-adapters`、`cargo test`；并用 `.ralph/tui_chinese_custom.yml` 在 TUI 与 `--no-tui` 两种模式下人工验证 `中<MARK>` 不再丢失。

---

# 追加任务计划: 修复 TUI 中文宽字符导致的错位/缺字

> 追加时间: 2026-01-25 01:05 (+0800)

## 目标
Ralph 默认启用的 TUI 在显示“中文 + 紧随其后的英文/路径”时：
- 不出现“中文像被插入空格”的错位现象
- 不出现英文首字母被吞（例如 `search` 显示成 `earch`）的问题

## 阶段
- [x] 阶段1: 收集现象与稳定复现
- [x] 阶段2: 根因定位（宽字符宽度/continuation cell）
- [x] 阶段3: 修复渲染逻辑 + 回归测试
- [x] 阶段4: 验证（含 smoke tests）+ 记录交付

## 方案方向(至少两个)
### 方案A: 不惜代价，最佳方案（倾向）
- 在 TUI 的渲染层改为“按 grapheme cluster + 显示宽度”进行写入与软换行。
- 避免自己用 `chars()`/`x += 1` 这种按 codepoint 的方式写 buffer。
- 为“中文紧贴英文路径”的 case 增加可复现的单测，确保以后不回退。

### 方案B: 先能用，后面再优雅
- 仅修复 `ContentPane::render()` 里的光标推进逻辑：对 CJK/emoji 宽字符直接 `x += 2` 并跳过 continuation cell。
- 先覆盖中文（主要痛点），暂不完全覆盖 grapheme cluster / 组合 emoji。

## 关键问题
1. 用户看到的现象是否只发生在 `ContentPane`（主内容区），还是 header/footer 也会错位？
2. 问题是否能用 `TestBackend` 单测稳定复现（无需真实终端）？
3. 修复后是否会影响现有“软换行/搜索高亮/样式保留”测试？

## 状态
**目前状态：已完成**  
- 稳定复现：新增单测 `cjk_double_width_does_not_swallow_next_ascii_char`，复现 `"将search/notes"` 会吞掉英文首字母的问题。
- 根因确认：`ContentPane::render()` 用 `chars()` + `x += 1` 写入 buffer；遇到中文/emoji 等“占两列”的字符时，下一列是 continuation cell，终端渲染会跳过，导致紧随其后的 ASCII 首字母被吞，并呈现“中文像插入空格”的错位现象。
- 修复方案：改为按 grapheme cluster（`unicode-segmentation`）渲染，并按显示宽度（`unicode-width`）推进光标；在触发软换行时先清理本行剩余格子，避免残影。
- 验证完成：已运行 `cargo fmt`、`cargo test -p ralph-tui`、`cargo clippy -p ralph-tui`、`cargo test -p ralph-core smoke_runner`、`cargo test -p ralph-core kiro`、`cargo test` 全通过。
